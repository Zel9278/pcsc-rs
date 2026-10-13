//! A more useful CPU description than sysinfo's `brand()` on ARM (where it is empty or
//! "ARMv7 Processor rev 4"), and the real core count when the kernel takes cores offline.
//!
//! * model: `<SoC> (4×Cortex-A55 + 4×Cortex-A75)`, from the SoC name (Android properties,
//!   device tree, `/sys/devices/soc0`) and the CPU implementer/part of every core.
//! * cores: `/sys/devices/system/cpu/possible` counts cores that are currently offline too.
//!   Phones power cores down (hotplug); `/proc/stat` and `/proc/cpuinfo` then list only the
//!   online ones, so a dual-core SoC with one core asleep looked like a single core.

#![cfg_attr(not(any(target_os = "linux", target_os = "android")), allow(dead_code))]

use std::{collections::BTreeMap, fs, sync::OnceLock};

/// One core as `/proc/cpuinfo` describes it.
#[derive(Debug, Default, Clone, PartialEq)]
struct CoreInfo {
    index: usize,
    implementer: Option<u32>,
    part: Option<u32>,
    /// `model name`, `Model` or `Hardware`, whichever the kernel printed
    name: Option<String>,
    hardware: Option<String>,
}

fn parse_hex(v: &str) -> Option<u32> {
    u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok()
}

/// Cores in `/proc/cpuinfo`, in order. x86 and ARM format alike.
fn parse_cpuinfo(text: &str) -> Vec<CoreInfo> {
    let mut cores: Vec<CoreInfo> = Vec::new();
    let mut hardware = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "processor" => cores.push(CoreInfo {
                index: value.parse().unwrap_or(cores.len()),
                ..CoreInfo::default()
            }),
            // After the cores on ARM: applies to the whole machine
            "Hardware" => hardware = Some(value.to_owned()),
            _ => {
                let Some(core) = cores.last_mut() else {
                    continue;
                };
                match key {
                    "CPU implementer" => core.implementer = parse_hex(value),
                    "CPU part" => core.part = parse_hex(value),
                    "model name" | "Model Name" | "cpu model" | "Processor"
                        if !value.is_empty() =>
                    {
                        core.name = Some(value.to_owned());
                    }
                    _ => {}
                }
            }
        }
    }
    let known = cores
        .iter()
        .rev()
        .find_map(|c| Some((c.implementer?, c.part?)));
    for core in &mut cores {
        if let (None, None, Some((i, p))) = (core.implementer, core.part, known) {
            core.implementer = Some(i);
            core.part = Some(p);
        }
        core.hardware.clone_from(&hardware);
    }
    cores
}

/// Name of a core by its implementer and part ID (the `MIDR` fields).
fn core_name(implementer: u32, part: u32) -> Option<&'static str> {
    Some(match (implementer, part) {
        // Arm
        (0x41, 0xb76) => "ARM1176",
        (0x41, 0xc05) => "Cortex-A5",
        (0x41, 0xc07) => "Cortex-A7",
        (0x41, 0xc08) => "Cortex-A8",
        (0x41, 0xc09) => "Cortex-A9",
        (0x41, 0xc0d) => "Cortex-A12",
        (0x41, 0xc0e) => "Cortex-A17",
        (0x41, 0xc0f) => "Cortex-A15",
        (0x41, 0xd01) => "Cortex-A32",
        (0x41, 0xd03) => "Cortex-A53",
        (0x41, 0xd04) => "Cortex-A35",
        (0x41, 0xd05) => "Cortex-A55",
        (0x41, 0xd07) => "Cortex-A57",
        (0x41, 0xd08) => "Cortex-A72",
        (0x41, 0xd09) => "Cortex-A73",
        (0x41, 0xd0a) => "Cortex-A75",
        (0x41, 0xd0b) => "Cortex-A76",
        (0x41, 0xd0c) => "Neoverse N1",
        (0x41, 0xd0d) => "Cortex-A77",
        (0x41, 0xd0e) => "Cortex-A76AE",
        (0x41, 0xd40) => "Neoverse V1",
        (0x41, 0xd41) => "Cortex-A78",
        (0x41, 0xd44) => "Cortex-X1",
        (0x41, 0xd46) => "Cortex-A510",
        (0x41, 0xd47) => "Cortex-A710",
        (0x41, 0xd48) => "Cortex-X2",
        (0x41, 0xd49) => "Neoverse N2",
        (0x41, 0xd4a) => "Neoverse E1",
        (0x41, 0xd4b) => "Cortex-A78C",
        (0x41, 0xd4d) => "Cortex-A715",
        (0x41, 0xd4e) => "Cortex-X3",
        (0x41, 0xd80) => "Cortex-A520",
        (0x41, 0xd81) => "Cortex-A720",
        (0x41, 0xd82) => "Cortex-X4",
        (0x41, 0xd84) => "Neoverse V2",
        (0x41, 0xd85) => "Cortex-X925",
        (0x41, 0xd87) => "Cortex-A725",
        // Qualcomm
        (0x51, 0x00f) => "Scorpion",
        (0x51, 0x02d) => "Scorpion",
        (0x51, 0x04d | 0x06f) => "Krait",
        (0x51, 0x201 | 0x205 | 0x211) => "Kryo",
        (0x51, 0x800) => "Kryo 260 Gold (Cortex-A73)",
        (0x51, 0x801) => "Kryo 260 Silver (Cortex-A53)",
        (0x51, 0x802) => "Kryo 385 Gold (Cortex-A75)",
        (0x51, 0x803) => "Kryo 385 Silver (Cortex-A55)",
        (0x51, 0x804) => "Kryo 485 Gold (Cortex-A76)",
        (0x51, 0x805) => "Kryo 485 Silver (Cortex-A55)",
        (0x51, 0xc00) => "Falkor",
        (0x51, 0xc01) => "Saphira",
        // Others
        (0x53, 0x001) => "Exynos M1",
        (0x4e, 0x000) => "Denver",
        (0x4e, 0x004) => "Carmel",
        (0x48, 0xd01) => "TaiShan v110",
        (0x48, 0xd02) => "Cortex-A55 (HiSilicon)",
        (0x61, _) => "Apple",
        _ => return None,
    })
}

/// `4×Cortex-A55 + 4×Cortex-A75`, the cores found most (and the fastest first when
/// frequencies are known) listed first. `None` when no core has an implementer/part.
fn layout(
    cores: &[CoreInfo],
    max_freq: &BTreeMap<usize, u64>,
    hint: Option<&str>,
) -> Option<String> {
    // (implementer, part) → (count, best max freq, first index)
    let mut groups: Vec<((u32, u32), (usize, u64, usize))> = Vec::new();
    for core in cores {
        let (Some(i), Some(p)) = (core.implementer, core.part) else {
            continue;
        };
        let freq = max_freq.get(&core.index).copied().unwrap_or(0);
        if let Some((_, g)) = groups.iter_mut().find(|(k, _)| *k == (i, p)) {
            g.0 += 1;
            g.1 = g.1.max(freq);
        } else {
            groups.push(((i, p), (1, freq, core.index)));
        }
    }
    if groups.is_empty() {
        return None;
    }
    // Fastest first. Without frequencies the bigger part ID is the bigger core
    // within a vendor (A55 = 0xd05 < A75 = 0xd0a), which is the usual case.
    groups.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(b.0.1.cmp(&a.0.1)));
    // A single kind of core: the caller may know its name better ("Krait 400" for "Krait")
    let hint = hint.filter(|_| groups.len() == 1);
    let parts: Vec<String> = groups
        .iter()
        .map(|((i, p), (n, ..))| {
            let name = hint
                .map(str::to_owned)
                .or_else(|| core_name(*i, *p).map(str::to_owned))
                .unwrap_or_else(|| format!("0x{i:02x}:0x{p:03x}"));
            format!("{n}×{name}")
        })
        .collect();
    Some(parts.join(" + "))
}

/// Cores that exist but were not listed (offline) are assumed to be like the closest lower one.
fn fill_offline(mut cores: Vec<CoreInfo>, possible: usize) -> Vec<CoreInfo> {
    for index in 0..possible {
        if cores.iter().any(|c| c.index == index) {
            continue;
        }
        let like = cores
            .iter()
            .filter(|c| c.index < index)
            .max_by_key(|c| c.index)
            .or(cores.first())
            .cloned();
        if let Some(mut c) = like {
            c.index = index;
            cores.push(c);
        }
    }
    cores.sort_by_key(|c| c.index);
    cores
}

/// `0-7`, `0,2-3`, `0` → number of CPUs.
fn parse_range_count(text: &str) -> Option<usize> {
    let mut total = 0;
    for part in text.trim().split(',') {
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.parse::<usize>().ok()?, b.parse::<usize>().ok()?),
            None => {
                let n = part.parse::<usize>().ok()?;
                (n, n)
            }
        };
        total += b.checked_sub(a)? + 1;
    }
    Some(total)
}

/// Joins the SoC name and the core layout, avoiding the same words twice.
fn compose(soc: Option<&str>, layout: Option<&str>) -> Option<String> {
    match (soc.map(str::trim).filter(|s| !s.is_empty()), layout) {
        (Some(s), Some(l)) => Some(format!("{l} ({s})")),
        (Some(s), None) => Some(s.to_owned()),
        (None, Some(l)) => Some(l.to_owned()),
        (None, None) => None,
    }
}

/// Places sysinfo's per-core usage at its CPU number (`cpu3` → 3) and gives the offline
/// cores 0, so the list always has `total` entries and keeps each core in its position.
/// Lists `total` cores from sysinfo's per-core usage, each at its CPU number (`cpu3` → 3).
/// PC Status shows one figure for the CPU (the average of this list), so an offline core
/// gets the average of the cores that are running instead of 0 %: a dual-core SoC with one
/// core asleep reads as two cores at the running core's load, not as half as busy.
pub fn spread_usage(named: &[(String, f32)], total: usize) -> Vec<f32> {
    let index_of = |name: &str| {
        name.trim_start_matches(|c: char| !c.is_ascii_digit())
            .parse::<usize>()
            .ok()
    };
    let indexed: Option<Vec<(usize, f32)>> = named
        .iter()
        .map(|(n, u)| Some((index_of(n)?, *u)))
        .collect();
    // Without CPU numbers in the names, keep sysinfo's order
    let indexed = indexed.unwrap_or_else(|| {
        named
            .iter()
            .enumerate()
            .map(|(i, (_, u))| (i, *u))
            .collect()
    });
    let total = total
        .max(named.len())
        .max(indexed.iter().map(|(i, _)| i + 1).max().unwrap_or(0));
    let average = if indexed.is_empty() {
        0.0
    } else {
        indexed.iter().map(|(_, u)| u).sum::<f32>() / indexed.len() as f32
    };
    let mut out = vec![average; total];
    for (i, u) in indexed {
        out[i] = u;
    }
    out
}

// ---- reading the machine -------------------------------------------------------------

fn read(path: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    (!text.is_empty()).then(|| text.to_owned())
}

fn getprop(name: &str) -> Option<String> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    return crate::android::getprop(name);
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = name;
        None
    }
}

/// Number of cores the kernel could ever run, online or not.
pub fn possible_cores() -> Option<usize> {
    static N: OnceLock<Option<usize>> = OnceLock::new();
    *N.get_or_init(|| read("/sys/devices/system/cpu/possible").and_then(|t| parse_range_count(&t)))
}

fn soc_name(cores: &[CoreInfo]) -> Option<String> {
    // Android 12+: the SoC's own name ("Snapdragon 8 Gen 1", "Tensor G3", "Dimensity 9200")
    let manufacturer = getprop("ro.soc.manufacturer");
    if let Some(model) = getprop("ro.soc.model") {
        return Some(match manufacturer {
            Some(m) if !model.to_lowercase().contains(&m.to_lowercase()) => format!("{m} {model}"),
            _ => model,
        });
    }
    // Device tree (Raspberry Pi, most SBCs and phones)
    if let Some(model) = read("/proc/device-tree/model") {
        return Some(model);
    }
    // Qualcomm and others expose it here
    if let Some(machine) = read("/sys/devices/soc0/machine") {
        return Some(match read("/sys/devices/soc0/family") {
            Some(family) if !machine.contains(&family) => format!("{family} {machine}"),
            _ => machine,
        });
    }
    getprop("ro.hardware.chipname")
        .or_else(|| cores.iter().find_map(|c| c.hardware.clone()))
        .or_else(|| getprop("ro.board.platform"))
}

fn compute(brand: &str, hint: Option<&str>) -> String {
    let brand = brand.trim();
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cores = parse_cpuinfo(&text);
    // x86 and the like: the CPU already has a proper name
    let brand_is_name = !brand.is_empty()
        && !brand.starts_with("ARMv")
        && !brand.starts_with("AArch64")
        && !brand.starts_with("Processor");
    if brand_is_name && cores.iter().all(|c| c.implementer.is_none()) {
        return brand.to_owned();
    }
    let cores = match possible_cores() {
        Some(n) => fill_offline(cores, n),
        None => cores,
    };
    let freqs: BTreeMap<usize, u64> = cores
        .iter()
        .filter_map(|c| {
            let f = read(&format!(
                "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq",
                c.index
            ))?;
            Some((c.index, f.parse().ok()?))
        })
        .collect();
    let layout = layout(&cores, &freqs, hint);
    compose(soc_name(&cores).as_deref(), layout.as_deref())
        .or_else(|| brand_is_name.then(|| brand.to_owned()))
        .or_else(|| cores.iter().find_map(|c| c.name.clone()))
        .unwrap_or_default()
}

/// The CPU's description; read once, since it does not change while running.
/// `core_hint` names the core when the platform knows it better than the CPU part
/// does (android.rs: "Krait 400"); it is used when all cores are of one kind.
pub fn model(brand: &str, core_hint: Option<&str>) -> String {
    static MODEL: OnceLock<String> = OnceLock::new();
    MODEL.get_or_init(|| compute(brand, core_hint)).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SD855: &str = "processor\t: 0\nCPU implementer\t: 0x41\nCPU part\t: 0xd05\n\nprocessor\t: 1\nCPU implementer\t: 0x41\nCPU part\t: 0xd05\n\nprocessor\t: 4\nCPU implementer\t: 0x41\nCPU part\t: 0xd0b\n\nHardware\t: Qualcomm Technologies, Inc SM8150\n";

    #[test]
    fn spreads_usage() {
        // offline cores take the average of the running ones
        let named = vec![("cpu0".to_owned(), 10.0), ("cpu3".to_owned(), 30.0)];
        assert_eq!(spread_usage(&named, 4), vec![10.0, 20.0, 20.0, 30.0]);
        // dual core, one asleep: both read as the running core's load
        let one = vec![("cpu0".to_owned(), 40.0)];
        assert_eq!(spread_usage(&one, 2), vec![40.0, 40.0]);
        // names without numbers keep their order
        let plain = vec![("cpu".to_owned(), 5.0)];
        assert_eq!(spread_usage(&plain, 1), vec![5.0]);
        // nothing known
        assert_eq!(spread_usage(&[], 2), vec![0.0, 0.0]);
        // possible unreadable: as many as sysinfo has
        assert_eq!(spread_usage(&named, 0), vec![10.0, 20.0, 20.0, 30.0]);
    }

    #[test]
    fn offline_cores_come_back() {
        // dual core, second one asleep
        let cores = parse_cpuinfo("processor : 0\nCPU implementer : 0x41\nCPU part : 0xc09\n");
        let cores = fill_offline(cores, 2);
        assert_eq!(cores.len(), 2);
        assert_eq!(
            layout(&cores, &BTreeMap::new(), None).unwrap(),
            "2×Cortex-A9"
        );
    }

    #[test]
    fn hint_names_uniform_cores_only() {
        let krait = parse_cpuinfo(
            "processor : 0\nCPU implementer : 0x51\nCPU part : 0x06f\nprocessor : 1\nCPU implementer : 0x51\nCPU part : 0x06f\n",
        );
        assert_eq!(
            layout(&krait, &BTreeMap::new(), Some("Krait 400")).unwrap(),
            "2×Krait 400"
        );
        let mixed = parse_cpuinfo(SD855);
        assert_eq!(
            layout(&mixed, &BTreeMap::new(), Some("Krait 400")).unwrap(),
            "1×Cortex-A76 + 2×Cortex-A55"
        );
    }

    #[test]
    fn ranges() {
        assert_eq!(parse_range_count("0-1\n"), Some(2));
        assert_eq!(parse_range_count("0"), Some(1));
        assert_eq!(parse_range_count("0-3,6-7"), Some(6));
        assert_eq!(parse_range_count("x"), None);
    }

    #[test]
    fn compose_names() {
        assert_eq!(
            compose(Some("Raspberry Pi 4"), Some("4x Cortex-A72")).unwrap(),
            "4x Cortex-A72 (Raspberry Pi 4)"
        );
        assert_eq!(compose(None, Some("2×Cortex-A9")).unwrap(), "2×Cortex-A9");
        assert!(compose(None, None).is_none());
    }
}
