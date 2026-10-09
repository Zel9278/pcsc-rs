//! Battery level and charging state, when the machine has a battery.
//!
//! - Linux: `/sys/class/power_supply`
//! - Android: `dumpsys battery` (`adb shell` cannot read `/sys/class/power_supply`)
//! - Windows: `GetSystemPowerStatus`
//! - macOS: `pmset -g batt`

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;

/// The level changes slowly; Android and macOS spawn a command to read it.
const INTERVAL: Duration = Duration::from_secs(5);

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BatteryData {
    /// 0–100
    pub(crate) level: f64,
    pub(crate) state: BatteryState,
    /// Connected to a charger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) plugged: Option<bool>,
    /// °C, where the system reports it (Android)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f64>,
}

pub fn get() -> Option<BatteryData> {
    static LAST: Mutex<Option<(Instant, Option<BatteryData>)>> = Mutex::new(None);
    let mut last = LAST.lock().ok()?;
    if let Some((at, data)) = &*last
        && at.elapsed() < INTERVAL
    {
        return data.clone();
    }
    let data = read();
    *last = Some((Instant::now(), data.clone()));
    data
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn read() -> Option<BatteryData> {
    if crate::android::is_android() {
        let output = crate::cmd::run(std::process::Command::new("dumpsys").arg("battery")).ok()?;
        return parse::dumpsys(&output.stdout);
    }
    sysfs::read()
}

#[cfg(target_os = "windows")]
fn read() -> Option<BatteryData> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // SAFETY: the pointer is to a SYSTEM_POWER_STATUS that lives for the call
    if unsafe { GetSystemPowerStatus(&raw mut status) } == 0 {
        return None;
    }
    parse::windows(
        status.ACLineStatus,
        status.BatteryFlag,
        status.BatteryLifePercent,
    )
}

#[cfg(target_os = "macos")]
fn read() -> Option<BatteryData> {
    let output = crate::cmd::run(std::process::Command::new("pmset").args(["-g", "batt"])).ok()?;
    parse::pmset(&output.stdout)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "windows",
    target_os = "macos"
)))]
fn read() -> Option<BatteryData> {
    None
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod sysfs {
    use std::fs;

    use super::{BatteryData, parse};

    const DIR: &str = "/sys/class/power_supply";

    pub fn read() -> Option<BatteryData> {
        let mut battery = None;
        let mut plugged = None;
        for entry in fs::read_dir(DIR).ok()?.flatten() {
            let path = entry.path();
            let read = |name: &str| {
                fs::read_to_string(path.join(name))
                    .ok()
                    .map(|s| s.trim().to_owned())
            };
            match read("type").as_deref() {
                // The first battery that reports a level (laptops sometimes list an empty BAT0)
                Some("Battery") if battery.is_none() && read("present").as_deref() != Some("0") => {
                    if let Some(level) = read("capacity").and_then(|c| c.parse::<f64>().ok()) {
                        battery = Some((
                            level,
                            parse::sysfs_state(read("status").as_deref().unwrap_or("")),
                        ));
                    }
                }
                Some("Mains" | "USB" | "USB_C" | "USB_PD" | "Wireless") => {
                    if let Some(online) = read("online") {
                        plugged = Some(plugged.unwrap_or(false) || online == "1");
                    }
                }
                _ => {}
            }
        }
        let (level, state) = battery?;
        Some(BatteryData {
            level: level.clamp(0.0, 100.0),
            state,
            plugged,
            temperature: None,
        })
    }
}

mod parse {
    #![allow(dead_code)] // each platform uses its own

    use super::{BatteryData, BatteryState};

    pub fn sysfs_state(status: &str) -> BatteryState {
        match status {
            "Charging" => BatteryState::Charging,
            "Discharging" => BatteryState::Discharging,
            "Full" => BatteryState::Full,
            "Not charging" => BatteryState::NotCharging,
            _ => BatteryState::Unknown,
        }
    }

    /// `dumpsys battery`: `level: 85`, `status: 2` (`BatteryManager`: 2 charging, 3 discharging,
    /// 4 not charging, 5 full), `temperature: 370` (tenths of °C), `AC powered: true` …
    pub fn dumpsys(text: &str) -> Option<BatteryData> {
        let value = |key: &str| {
            text.lines()
                .find_map(|l| l.trim().strip_prefix(key)?.strip_prefix(':'))
                .map(str::trim)
        };
        let level: f64 = value("level")?.parse().ok()?;
        let state = match value("status").and_then(|s| s.parse::<u8>().ok()) {
            Some(2) => BatteryState::Charging,
            Some(3) => BatteryState::Discharging,
            Some(4) => BatteryState::NotCharging,
            Some(5) => BatteryState::Full,
            _ => BatteryState::Unknown,
        };
        let plugged = [
            "AC powered",
            "USB powered",
            "Wireless powered",
            "Dock powered",
        ]
        .iter()
        .filter_map(|k| value(k))
        .map(|v| v == "true")
        .reduce(|a, b| a || b);
        let temperature = value("temperature")
            .and_then(|t| t.parse::<f64>().ok())
            .map(|t| t / 10.0);
        Some(BatteryData {
            level: level.clamp(0.0, 100.0),
            state,
            plugged,
            temperature,
        })
    }

    /// `SYSTEM_POWER_STATUS`: `ACLineStatus` 1 = on AC; `BatteryFlag` 128 = no battery, 8 = charging;
    /// `BatteryLifePercent` 255 = unknown.
    pub fn windows(ac_line: u8, flag: u8, percent: u8) -> Option<BatteryData> {
        if flag == 128 || flag == 255 || percent > 100 {
            return None;
        }
        let plugged = match ac_line {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let state = if flag & 8 != 0 {
            BatteryState::Charging
        } else if plugged == Some(true) {
            if percent >= 100 {
                BatteryState::Full
            } else {
                BatteryState::NotCharging
            }
        } else if plugged == Some(false) {
            BatteryState::Discharging
        } else {
            BatteryState::Unknown
        };
        Some(BatteryData {
            level: f64::from(percent),
            state,
            plugged,
            temperature: None,
        })
    }

    /// `pmset -g batt`:
    /// ```text
    /// Now drawing from 'AC Power'
    ///  -InternalBattery-0 (id=1234)<TAB>85%; charging; 1:02 remaining present: true
    /// ```
    pub fn pmset(text: &str) -> Option<BatteryData> {
        let line = text.lines().find(|l| l.contains("InternalBattery"))?;
        let after_tab = line.split_once('\t').map_or(line, |(_, rest)| rest);
        let mut fields = after_tab.split(';').map(str::trim);
        let level: f64 = fields.next()?.trim_end_matches('%').parse().ok()?;
        let state = match fields.next().unwrap_or("") {
            "charging" | "finishing charge" => BatteryState::Charging,
            "discharging" => BatteryState::Discharging,
            "charged" => BatteryState::Full,
            "AC attached" | "not charging" => BatteryState::NotCharging,
            _ => BatteryState::Unknown,
        };
        Some(BatteryData {
            level: level.clamp(0.0, 100.0),
            state,
            plugged: Some(text.contains("'AC Power'")),
            temperature: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BatteryState, parse};

    #[test]
    fn android_dumpsys() {
        let text = "Current Battery Service state:\n  AC powered: true\n  USB powered: false\n  Wireless powered: false\n  status: 5\n  level: 100\n  temperature: 370\n";
        let b = parse::dumpsys(text).unwrap();
        assert_eq!(
            (b.level, b.state, b.plugged, b.temperature),
            (100.0, BatteryState::Full, Some(true), Some(37.0))
        );
        let b =
            parse::dumpsys("  AC powered: false\n  USB powered: false\n  status: 3\n  level: 42\n")
                .unwrap();
        assert_eq!(
            (b.state, b.plugged, b.temperature),
            (BatteryState::Discharging, Some(false), None)
        );
        assert!(parse::dumpsys("Can't find service: battery").is_none());
    }

    #[test]
    fn windows_power_status() {
        assert!(parse::windows(1, 128, 255).is_none());
        let b = parse::windows(1, 8, 60).unwrap();
        assert_eq!((b.state, b.plugged), (BatteryState::Charging, Some(true)));
        assert_eq!(parse::windows(1, 1, 100).unwrap().state, BatteryState::Full);
        assert_eq!(
            parse::windows(0, 1, 70).unwrap().state,
            BatteryState::Discharging
        );
    }

    #[test]
    fn macos_pmset() {
        let text = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=4653155)\t85%; charging; 1:02 remaining present: true\n";
        let b = parse::pmset(text).unwrap();
        assert_eq!(
            (b.level, b.state, b.plugged),
            (85.0, BatteryState::Charging, Some(true))
        );
        let text = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=4653155)\t52%; discharging; 3:10 remaining present: true\n";
        assert_eq!(parse::pmset(text).unwrap().state, BatteryState::Discharging);
        assert!(parse::pmset("Now drawing from 'AC Power'\n").is_none());
    }

    #[test]
    fn sysfs_status() {
        assert_eq!(
            parse::sysfs_state("Not charging"),
            BatteryState::NotCharging
        );
        assert_eq!(parse::sysfs_state("Full"), BatteryState::Full);
        assert_eq!(parse::sysfs_state("???"), BatteryState::Unknown);
    }
}
