//! Temperatures of the machine's sensors.
//!
//! - PCs: sysinfo's components (Linux hwmon, Windows ACPI thermal zone through WMI, macOS SMC/IOKit)
//! - Android: `/sys/class/thermal`, grouped into CPU, GPU, … (there are dozens of zones)

use std::time::{Duration, Instant};

use serde::Serialize;

/// Sensors change slowly, and the Windows ones go through WMI
const INTERVAL: Duration = Duration::from_secs(5);
/// The wire format keeps the list short; the server caps it as well
const MAX_SENSORS: usize = 32;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Temperature {
    pub(crate) label: String,
    /// °C
    pub(crate) value: f64,
}

/// Keeps the sensors open between samples.
#[derive(Default)]
pub struct Tracker {
    /// Created on the monitor thread: sysinfo initialises COM per thread on Windows
    components: Option<sysinfo::Components>,
    at: Option<Instant>,
    latest: Vec<Temperature>,
}

impl Tracker {
    pub fn refresh(&mut self) {
        if self.at.is_some_and(|at| at.elapsed() < INTERVAL) {
            return;
        }
        self.at = Some(Instant::now());

        #[cfg(any(target_os = "linux", target_os = "android"))]
        if crate::android::is_android() {
            self.latest = zones::read();
            return;
        }

        let components = self
            .components
            .get_or_insert_with(sysinfo::Components::new_with_refreshed_list);
        components.refresh(false);
        let sensors: Vec<(String, f64)> = components
            .list()
            .iter()
            .filter_map(|c| Some((c.label().to_owned(), f64::from(c.temperature()?))))
            .collect();
        let latest = parse::components(&sensors);
        // VMs and some boards have no hwmon sensor but do have thermal zones
        #[cfg(target_os = "linux")]
        let latest = if latest.is_empty() {
            zones::read()
        } else {
            latest
        };
        self.latest = latest;
    }

    pub fn temperatures(&self) -> Vec<Temperature> {
        self.latest.clone()
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod zones {
    use std::fs;

    use super::{Temperature, parse};

    pub fn read() -> Vec<Temperature> {
        let Ok(dir) = fs::read_dir("/sys/class/thermal") else {
            return Vec::new();
        };
        let zones = dir.flatten().filter_map(|entry| {
            let path = entry.path();
            if !path.file_name()?.to_str()?.starts_with("thermal_zone") {
                return None;
            }
            let kind = fs::read_to_string(path.join("type")).ok()?;
            let millis = fs::read_to_string(path.join("temp"))
                .ok()?
                .trim()
                .parse::<i64>()
                .ok()?;
            Some((kind.trim().to_owned(), millis))
        });
        parse::zones(zones, crate::android::is_android())
    }
}

mod parse {
    #![allow(dead_code)] // each platform uses its own

    use super::{MAX_SENSORS, Temperature};

    /// Anything outside this is a sensor that is off or not a temperature
    fn plausible(celsius: f64) -> bool {
        celsius > 0.0 && celsius < 150.0
    }

    fn round(celsius: f64) -> f64 {
        (celsius * 10.0).round() / 10.0
    }

    /// sysinfo's components, e.g. `coretemp Package id 0`, `coretemp Core 3`, `nvme Composite …`.
    /// Per-core and secondary `NVMe` sensors are left out when the package / composite one is there.
    pub fn components(sensors: &[(String, f64)]) -> Vec<Temperature> {
        let has = |prefix: &str| sensors.iter().any(|(l, _)| l.starts_with(prefix));
        let has_package = has("coretemp Package");
        let has_composite = has("nvme Composite");
        let mut out: Vec<Temperature> = sensors
            .iter()
            .filter(|(label, value)| {
                plausible(*value)
                    && !(has_package && label.starts_with("coretemp Core"))
                    && !(has_composite && label.starts_with("nvme Sensor"))
            })
            .take(MAX_SENSORS)
            .map(|(label, value)| Temperature {
                // Chips with a single unnamed sensor: `acpitz temp1` → `acpitz`
                label: label.strip_suffix(" temp1").unwrap_or(label).to_owned(),
                value: round(*value),
            })
            .collect();
        // sysinfo lists them in directory order; keep the order stable for the viewer
        out.sort_by(|a, b| a.label.cmp(&b.label));
        out
    }

    /// The group an Android thermal zone belongs to, by its type. Battery, current and
    /// voltage limits (`bcl-*`, `ibat`, `vbat`, `socd`) and PMIC zones are not shown.
    pub fn android_group(kind: &str) -> Option<&'static str> {
        let kind = kind.to_ascii_lowercase();
        let starts = |p: &str| kind.starts_with(p);
        Some(if kind.contains("gpu") || kind == "g3d" {
            "GPU"
        } else if starts("cpu")
            || kind.contains("cpuss")
            || matches!(kind.as_str(), "big" | "mid" | "little")
        {
            "CPU"
        } else if starts("ddr") {
            "Memory"
        } else if starts("mdm") || starts("modem") {
            "Modem"
        } else if starts("nsp") || kind.contains("npu") {
            "NPU"
        } else if starts("battery") {
            "Battery"
        } else if starts("sys-therm") || kind.contains("skin") {
            "Device"
        } else {
            return None;
        })
    }

    const ANDROID_ORDER: [&str; 7] = ["CPU", "GPU", "Memory", "NPU", "Modem", "Battery", "Device"];

    /// `(type, millidegrees)` of each thermal zone. On Android they are grouped (the hottest
    /// zone of each group); elsewhere each type is listed once.
    pub fn zones(zones: impl Iterator<Item = (String, i64)>, android: bool) -> Vec<Temperature> {
        let mut out: Vec<Temperature> = Vec::new();
        for (kind, millis) in zones {
            #[allow(clippy::cast_precision_loss)]
            let celsius = millis as f64 / 1000.0;
            if !plausible(celsius) {
                continue;
            }
            let label = if android {
                match android_group(&kind) {
                    Some(group) => group.to_owned(),
                    None => continue,
                }
            } else {
                kind
            };
            match out.iter_mut().find(|t| t.label == label) {
                Some(t) => t.value = t.value.max(round(celsius)),
                None => out.push(Temperature {
                    label,
                    value: round(celsius),
                }),
            }
        }
        if android {
            out.sort_by_key(|t| ANDROID_ORDER.iter().position(|g| *g == t.label));
        } else {
            out.sort_by(|a, b| a.label.cmp(&b.label));
        }
        out.truncate(MAX_SENSORS);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Temperature, parse};

    fn t(label: &str, value: f64) -> Temperature {
        Temperature {
            label: label.into(),
            value,
        }
    }

    #[test]
    fn components() {
        let sensors = [
            ("acpitz temp1", 73.0),
            ("nvme Composite WD PC SN530", 47.85),
            ("nvme Sensor 1 WD PC SN530", 47.85),
            ("coretemp Package id 0", 75.0),
            ("coretemp Core 0", 62.0),
            ("iwlwifi_1 temp1", 49.0),
            ("broken", -273.0),
        ]
        .map(|(l, v)| (l.to_owned(), v));
        assert_eq!(
            parse::components(&sensors),
            [
                t("acpitz", 73.0),
                t("coretemp Package id 0", 75.0),
                t("iwlwifi_1", 49.0),
                t("nvme Composite WD PC SN530", 47.9),
            ]
        );
        // Without a package sensor (AMD, ARM) every core stays
        let cores = vec![("coretemp Core 0".to_owned(), 50.0)];
        assert_eq!(parse::components(&cores), [t("coretemp Core 0", 50.0)]);
    }

    #[test]
    fn android_zones() {
        let zones = [
            ("cpu-1-0-0", 41_200),
            ("cpuss-0", 43_000),
            ("gpuss-1", 39_500),
            ("ddr", 36_000),
            ("bcl-lvl0", 0),
            ("ibat", 1_200),
            ("socd", 12_000),
            ("pm7325_tz", 40_000),
            ("sys-therm-1", 33_900),
            ("mdmss-0", 38_000),
            ("nsphvx-0", 37_000),
            ("camera-0", -40_000),
        ]
        .map(|(k, m)| (k.to_owned(), m));
        assert_eq!(
            parse::zones(zones.into_iter(), true),
            [
                t("CPU", 43.0),
                t("GPU", 39.5),
                t("Memory", 36.0),
                t("NPU", 37.0),
                t("Modem", 38.0),
                t("Device", 33.9),
            ]
        );
    }

    #[test]
    fn linux_zones() {
        let zones = [
            ("x86_pkg_temp", 78_000),
            ("acpitz", 73_000),
            ("INT3400 Thermal", 20_000),
            ("acpitz", 74_000),
        ]
        .map(|(k, m)| (k.to_owned(), m));
        assert_eq!(
            parse::zones(zones.into_iter(), false),
            [
                t("INT3400 Thermal", 20.0),
                t("acpitz", 74.0),
                t("x86_pkg_temp", 78.0)
            ]
        );
    }
}
