//! A more useful CPU description than sysinfo's `brand()` on ARM (where it is empty or
//! "ARMv7 Processor rev 4"), and the real core count when the kernel takes cores offline.
//!
//! * model: `<SoC> (4×Cortex-A55 + 4×Cortex-A75)`, from the SoC name (Android properties,
//!   device tree, `/sys/devices/soc0`) and the CPU implementer/part of every core.
//! * cores: `/sys/devices/system/cpu/possible` counts cores that are currently offline too.
//!   Phones power cores down (hotplug); `/proc/stat` and `/proc/cpuinfo` then list only the
//!   online ones, so a dual-core SoC with one core asleep looked like a single core.

#![cfg_attr(not(any(target_os = "linux", target_os = "android")), allow(dead_code))]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

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
                // Single-core kernels print no `processor` line at all
                if cores.is_empty() && matches!(key, "CPU implementer" | "CPU part") {
                    cores.push(CoreInfo::default());
                }
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
    // Old 32-bit ARM kernels print the implementer/part once, after all the `processor`
    // entries, so only the last core got them. Every core without its own is taken to be
    // like the ones that have them (such SoCs have one kind of core).
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
    // One core of one kind (single-core SoCs): just its name, no count
    let single = groups.len() == 1 && groups[0].1.0 == 1;
    let parts: Vec<String> = groups
        .iter()
        .map(|((i, p), (n, ..))| {
            let name = hint
                .map(str::to_owned)
                .or_else(|| core_name(*i, *p).map(str::to_owned))
                .unwrap_or_else(|| format!("0x{i:02x}:0x{p:03x}"));
            if single { name } else { format!("{n}×{name}") }
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
    let platform = getprop("ro.board.platform");
    if let Some(p) = platform.as_deref() {
        let brand = brand_of(p);
        if brand != p.trim() {
            return Some(if brand == "Tegra" {
                tegra_name(cores)
            } else {
                brand
            });
        }
    }
    getprop("ro.hardware.chipname")
        .or_else(|| cores.iter().find_map(|c| c.hardware.clone()))
        .or(platform)
        .map(|raw| brand_of(&raw))
}

/// ボードのコード名("msm7k", "QCT MSM8255 SURF")を通り名("Snapdragon")にいたしますわ。
fn brand_of(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let qualcomm = [
        "msm", "qsd", "apq", "mdm", "sdm", "sm8", "sm7", "sm6", "sm4",
    ]
    .iter()
    .any(|p| lower.starts_with(p))
        || lower.contains("qualcomm")
        || lower.contains("qct")
        || lower.contains("snapdragon");
    if qualcomm {
        "Snapdragon".to_owned()
    } else if lower.starts_with("tegra") {
        "Tegra".to_owned()
    } else {
        raw.trim().to_owned()
    }
}

/// "tegra" is the board platform of every Tegra. Tegra 3 is four Cortex-A9 plus a
/// companion core the kernel cannot see, whose cluster switch lives in /sys/kernel/cluster;
/// Tegra 2 is two Cortex-A9 without it. Anything else stays "Tegra".
fn tegra_name(cores: &[CoreInfo]) -> String {
    let a9 = !cores.is_empty()
        && cores
            .iter()
            .all(|c| (c.implementer, c.part) == (Some(0x41), Some(0xc09)));
    let companion = std::path::Path::new("/sys/kernel/cluster").is_dir();
    match cores.len() {
        4 if a9 && companion => "Tegra 3",
        2 if a9 && !companion => "Tegra 2",
        _ => "Tegra",
    }
    .to_owned()
}
fn is_tegra3(cores: &[CoreInfo]) -> bool {
    getprop("ro.board.platform").is_some_and(|p| p.trim().to_lowercase().starts_with("tegra"))
        && tegra_name(cores) == "Tegra 3"
}

/// Cores the OS cannot see: Tegra 3's companion core (the fifth, low-power one, which runs
/// instead of the four main ones when the load is light). 0 on everything else.
pub fn companion_cores() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| {
        let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let cores = parse_cpuinfo(&text);
        let cores = match possible_cores() {
            Some(n) => fill_offline(cores, n),
            None => cores,
        };
        usize::from(is_tegra3(&cores))
    })
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
    let freqs = read_max_freqs(&cores);
    let layout = layout(&cores, &freqs, hint).map(|l| {
        // Tegra 3: four main cores plus the companion core
        if is_tegra3(&cores) {
            l.replacen("4×", "4+1×", 1)
        } else {
            l
        }
    });
    compose(soc_name(&cores).as_deref(), layout.as_deref())
        .or_else(|| brand_is_name.then(|| brand.to_owned()))
        .or_else(|| cores.iter().find_map(|c| c.name.clone()))
        .unwrap_or_default()
}

/// `cpuinfo_max_freq` (kHz) of each core's cpufreq policy. An offline core often has no
/// cpufreq directory; it takes the figure of the nearest lower core that has one.
fn read_max_freqs(cores: &[CoreInfo]) -> BTreeMap<usize, u64> {
    let mut freqs: BTreeMap<usize, u64> = BTreeMap::new();
    for c in cores {
        let path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq",
            c.index
        );
        if let Some(khz) = read(&path).and_then(|t| t.parse().ok()) {
            freqs.insert(c.index, khz);
        }
    }
    for c in cores {
        if !freqs.contains_key(&c.index)
            && let Some((_, khz)) = freqs.range(..c.index).next_back()
        {
            let khz = *khz;
            freqs.insert(c.index, khz);
        }
    }
    freqs
}

/// One kind of core: its name and the top frequency of its cpufreq policy. Cores of one
/// design in separate clusters (a prime core beside the big ones) differ in the frequency.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoreKind {
    pub name: String,
    pub max_mhz: Option<u32>,
}

/// The kind of each core, at its CPU number. `None` where nothing is known (no CPU part,
/// as on x86).
fn kinds_of(
    cores: &[CoreInfo],
    freqs: &BTreeMap<usize, u64>,
    hint: Option<&str>,
) -> Vec<Option<CoreKind>> {
    if cores
        .iter()
        .any(|c| c.implementer.is_none() || c.part.is_none())
    {
        return Vec::new();
    }
    let uniform = cores
        .windows(2)
        .all(|w| (w[0].implementer, w[0].part) == (w[1].implementer, w[1].part));
    let mut kinds: Vec<Option<CoreKind>> = Vec::new();
    for c in cores {
        let (i, p) = (c.implementer.unwrap_or(0), c.part.unwrap_or(0));
        let name = hint
            .filter(|_| uniform)
            .map(str::to_owned)
            .or_else(|| core_name(i, p).map(str::to_owned))
            .unwrap_or_else(|| format!("0x{i:02x}:0x{p:03x}"));
        let max_mhz = freqs
            .get(&c.index)
            .and_then(|khz| u32::try_from(khz / 1000).ok());
        if kinds.len() <= c.index {
            kinds.resize(c.index + 1, None);
        }
        kinds[c.index] = Some(CoreKind { name, max_mhz });
    }
    kinds
}

/// True when the cores are of more than one kind (big.LITTLE).
pub fn is_mixed(kinds: &[Option<CoreKind>]) -> bool {
    kinds.iter().flatten().collect::<BTreeSet<_>>().len() > 1
}

/// How long a guess about offline cores is kept before looking again.
const KINDS_RECHECK: Duration = Duration::from_secs(10);

/// The kind of each core, indexed by CPU number. Empty where cores have no CPU part.
/// A kind never changes, so it is read once; but while some cores were offline their
/// kind was only guessed (like the nearest lower core), so that is checked again now and then.
#[cfg(not(windows))]
pub fn max_mhz_per_core(total: usize) -> Vec<Option<u32>> {
    static CACHE: Mutex<Option<(Instant, usize, bool, Vec<Option<u32>>)>> = Mutex::new(None);
    let total = total.max(possible_cores().unwrap_or(0));
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((at, n, complete, mhz)) = cache.as_ref()
        && *n == total
        && (*complete || at.elapsed() < KINDS_RECHECK)
    {
        return mhz.clone();
    }
    let cores: Vec<CoreInfo> = (0..total)
        .map(|index| CoreInfo {
            index,
            ..CoreInfo::default()
        })
        .collect();
    let freqs = read_max_freqs(&cores);
    let complete = (0..total).all(|i| {
        read(&format!(
            "/sys/devices/system/cpu/cpu{i}/cpufreq/cpuinfo_max_freq"
        ))
        .is_some()
    });
    let mhz: Vec<Option<u32>> = (0..total)
        .map(|i| freqs.get(&i).and_then(|khz| u32::try_from(khz / 1000).ok()))
        .collect();
    *cache = Some((Instant::now(), total, complete, mhz.clone()));
    mhz
}
#[cfg(windows)]
pub fn max_mhz_per_core(total: usize) -> Vec<Option<u32>> {
    use windows_sys::Win32::System::Power::{
        CallNtPowerInformation, PROCESSOR_POWER_INFORMATION, ProcessorInformation,
    };
    static CACHE: OnceLock<Vec<Option<u32>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            if total == 0 {
                return Vec::new();
            }
            // SAFETY: the structure is plain integers; the buffer holds `total` of them
            // and its byte size is passed along, so the call writes within it.
            let mut buf: Vec<PROCESSOR_POWER_INFORMATION> =
                (0..total).map(|_| unsafe { std::mem::zeroed() }).collect();
            let bytes = u32::try_from(std::mem::size_of_val(buf.as_slice())).unwrap_or(0);
            let status = unsafe {
                CallNtPowerInformation(
                    ProcessorInformation,
                    std::ptr::null(),
                    0,
                    buf.as_mut_ptr().cast(),
                    bytes,
                )
            };
            if status < 0 {
                return vec![None; total];
            }
            buf.iter()
                .map(|p| (p.MaxMhz != 0).then_some(p.MaxMhz))
                .collect()
        })
        .clone()
}
pub fn core_kinds(core_hint: Option<&str>) -> Vec<Option<CoreKind>> {
    static CACHE: Mutex<Option<(Instant, bool, Vec<Option<CoreKind>>)>> = Mutex::new(None);
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((at, complete, kinds)) = cache.as_ref()
        && (*complete || at.elapsed() < KINDS_RECHECK)
    {
        return kinds.clone();
    }
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let listed = parse_cpuinfo(&text);
    let possible = possible_cores();
    let complete = possible.is_none_or(|n| listed.len() >= n);
    let cores = match possible {
        Some(n) => fill_offline(listed, n),
        None => listed,
    };
    let freqs = read_max_freqs(&cores);
    let kinds = kinds_of(&cores, &freqs, core_hint);
    *cache = Some((Instant::now(), complete, kinds.clone()));
    kinds
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
    fn big_little_layout() {
        let cores = parse_cpuinfo(SD855);
        assert_eq!(cores.len(), 3);
        assert_eq!(
            cores[0].hardware.as_deref(),
            Some("Qualcomm Technologies, Inc SM8150")
        );
        let freq = BTreeMap::from([(0, 1_800_000), (1, 1_800_000), (4, 2_840_000)]);
        assert_eq!(
            layout(&cores, &freq, None).unwrap(),
            "1×Cortex-A76 + 2×Cortex-A55"
        );
        // Without frequencies the part ID decides
        assert_eq!(
            layout(&cores, &BTreeMap::new(), None).unwrap(),
            "1×Cortex-A76 + 2×Cortex-A55"
        );
    }

    #[test]
    fn old_kernel_prints_part_once() {
        // Krait 400 (Linux 3.4): four cores, one implementer/part block at the end
        let text = "Processor\t: ARMv7 Processor rev 0 (v7l)\nprocessor\t: 0\nBogoMIPS\t: 38.40\n\nprocessor\t: 1\nBogoMIPS\t: 38.40\n\nprocessor\t: 2\nBogoMIPS\t: 38.40\n\nprocessor\t: 3\nBogoMIPS\t: 38.40\n\nFeatures\t: swp half thumb\nCPU implementer\t: 0x51\nCPU part\t: 0x06f\n\nHardware\t: Qualcomm MSM 8974\n";
        let cores = parse_cpuinfo(text);
        assert_eq!(cores.len(), 4);
        assert_eq!(
            layout(&cores, &BTreeMap::new(), Some("Krait 400")).unwrap(),
            "4×Krait 400"
        );
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

    fn eight_cores(extra_freqs: bool) -> (Vec<CoreInfo>, BTreeMap<usize, u64>) {
        let mut text = String::new();
        for i in 0..8 {
            let part = if i < 4 { "0xd05" } else { "0xd0b" };
            text += &format!("processor\t: {i}\nCPU implementer\t: 0x41\nCPU part\t: {part}\n\n");
        }
        let cores = parse_cpuinfo(&text);
        let mut freqs = BTreeMap::new();
        for i in 0..8 {
            let khz = match i {
                0..=3 => 1_800_000,
                7 if extra_freqs => 2_840_000,
                _ => 2_420_000,
            };
            freqs.insert(i, khz);
        }
        (cores, freqs)
    }

    #[test]
    fn kinds_split_by_name_and_frequency() {
        let (cores, freqs) = eight_cores(true);
        let kinds = kinds_of(&cores, &freqs, None);
        assert_eq!(kinds.len(), 8);
        assert_eq!(
            kinds[0],
            Some(CoreKind {
                name: "Cortex-A55".into(),
                max_mhz: Some(1800)
            })
        );
        // the prime core is the same design as the big ones, told apart by its frequency
        assert_eq!(kinds[4].as_ref().unwrap().name, "Cortex-A76");
        assert_eq!(kinds[4].as_ref().unwrap().max_mhz, Some(2420));
        assert_eq!(kinds[7].as_ref().unwrap().max_mhz, Some(2840));
        assert!(is_mixed(&kinds));
    }

    #[test]
    fn one_kind_is_not_mixed() {
        let cores = parse_cpuinfo(
            "processor : 0\nprocessor : 1\nprocessor : 2\nprocessor : 3\nCPU implementer : 0x51\nCPU part : 0x06f\n",
        );
        let freqs = BTreeMap::from([
            (0, 2_150_000),
            (1, 2_150_000),
            (2, 2_150_000),
            (3, 2_150_000),
        ]);
        let kinds = kinds_of(&cores, &freqs, Some("Krait 400"));
        assert_eq!(kinds.len(), 4);
        assert_eq!(kinds[3].as_ref().unwrap().name, "Krait 400");
        assert!(!is_mixed(&kinds));
        // no CPU part (x86): nothing is known
        let x86 = parse_cpuinfo("processor : 0\nmodel name : Intel(R) Core(TM) i7\n");
        assert!(kinds_of(&x86, &BTreeMap::new(), None).is_empty());
    }

    #[test]
    fn offline_cores_take_the_nearest_lower_kind() {
        let cores = parse_cpuinfo(
            "processor : 0\nCPU implementer : 0x41\nCPU part : 0xd05\n\nprocessor : 4\nCPU implementer : 0x41\nCPU part : 0xd0b\n",
        );
        let cores = fill_offline(cores, 8);
        let kinds = kinds_of(&cores, &BTreeMap::new(), None);
        assert_eq!(kinds.len(), 8);
        assert_eq!(kinds[1].as_ref().unwrap().name, "Cortex-A55");
        assert_eq!(kinds[4].as_ref().unwrap().name, "Cortex-A76");
        assert_eq!(kinds[7].as_ref().unwrap().name, "Cortex-A76");
    }

    #[test]
    fn ranges() {
        assert_eq!(parse_range_count("0-1\n"), Some(2));
        assert_eq!(parse_range_count("0"), Some(1));
        assert_eq!(parse_range_count("0-3,6-7"), Some(6));
        assert_eq!(parse_range_count("x"), None);
    }

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
    fn compose_names() {
        assert_eq!(
            compose(Some("Raspberry Pi 4"), Some("4×Cortex-A72")).unwrap(),
            "4×Cortex-A72 (Raspberry Pi 4)"
        );
        assert_eq!(compose(None, Some("2×Cortex-A9")).unwrap(), "2×Cortex-A9");
        assert!(compose(None, None).is_none());
    }
    #[test]
    fn max_mhz_per_core_has_one_entry_per_core() {
        // Whatever this machine is, the list has one slot per core asked for
        assert!(max_mhz_per_core(2).len() >= 2);
    }
    #[test]
    fn tegra_is_named_by_its_cores() {
        assert_eq!(brand_of("tegra"), "Tegra");
        let a9 = |n: usize| {
            (0..n)
                .map(|index| CoreInfo {
                    index,
                    implementer: Some(0x41),
                    part: Some(0xc09),
                    ..CoreInfo::default()
                })
                .collect::<Vec<_>>()
        };
        // 4 cores: "Tegra 3" where /sys/kernel/cluster exists, else plain "Tegra"
        assert!(matches!(tegra_name(&a9(4)).as_str(), "Tegra 3" | "Tegra"));
        assert_eq!(tegra_name(&[]), "Tegra");
    }
}
