//! The kinds of core in the CPU. ARM chips mix several (big.LITTLE): a Snapdragon 7+ Gen 3
//! has 3 Cortex-A520, 4 Cortex-A720 and 1 Cortex-X4, each kind its own cpufreq policy
//! with its own top frequency. sysinfo names each core from its `CPU part`.
//!
//! On ARM Linux and Android, cores are grouped by name and the top frequency of their
//! policy, so cores of one design in separate clusters (a prime core beside the big
//! ones) are told apart, while servers with a policy per core are not split up.
//! Elsewhere (x86, where every core has the same name) they are grouped by name only.

/// One kind of core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kind {
    pub name: String,
    /// Top frequency of the core's cpufreq policy
    pub max_mhz: Option<u32>,
}

/// One kind per core, in core order. `brands` are sysinfo's names for the cores.
pub fn per_core(brands: &[&str]) -> Vec<Kind> {
    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "aarch64", target_arch = "arm")
    ))]
    {
        let ids = arm::part_ids();
        brands
            .iter()
            .enumerate()
            .map(|(i, brand)| Kind {
                // sysinfo leaves a part it does not know unnamed; show its ID instead
                name: if brand.is_empty() {
                    ids.get(&i).cloned().unwrap_or_default()
                } else {
                    (*brand).to_owned()
                },
                max_mhz: arm::max_mhz(i),
            })
            .collect()
    }
    #[cfg(not(all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "aarch64", target_arch = "arm")
    )))]
    {
        brands
            .iter()
            .map(|brand| Kind {
                name: (*brand).to_owned(),
                max_mhz: None,
            })
            .collect()
    }
}

/// The kinds with how many cores of each, in the order they first appear.
pub fn group(cores: &[Kind]) -> Vec<(&Kind, usize)> {
    let mut groups: Vec<(&Kind, usize)> = Vec::new();
    for core in cores {
        match groups.iter_mut().find(|(kind, _)| *kind == core) {
            Some((_, count)) => *count += 1,
            None => groups.push((core, 1)),
        }
    }
    groups
}

/// The CPU's name. Where the cores differ, each kind with its count:
/// `3× Cortex-A520 + 4× Cortex-A720 + 1× Cortex-X4`. A name that more than one kind
/// shares gets the top frequency: `6× Kryo-4XX-Silver + 1× Kryo-4XX-Gold 2.21 GHz +
/// 1× Kryo-4XX-Gold 2.40 GHz`.
pub fn model(cores: &[Kind]) -> String {
    let groups = group(cores);
    match groups.as_slice() {
        [] => String::new(),
        [(kind, _)] => kind.name.clone(),
        _ => groups
            .iter()
            .map(|(kind, count)| {
                let shared = groups.iter().filter(|(k, _)| k.name == kind.name).count() > 1;
                match kind.max_mhz {
                    Some(mhz) if shared => {
                        format!("{count}× {} {:.2} GHz", kind.name, f64::from(mhz) / 1000.0)
                    }
                    _ => format!("{count}× {}", kind.name),
                }
            })
            .collect::<Vec<_>>()
            .join(" + "),
    }
}

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    any(target_arch = "aarch64", target_arch = "arm")
))]
mod arm {
    use std::{collections::HashMap, fs};

    /// `cpuinfo_max_freq` of the core's policy, in MHz.
    pub fn max_mhz(core: usize) -> Option<u32> {
        let khz = fs::read_to_string(format!(
            "/sys/devices/system/cpu/cpu{core}/cpufreq/cpuinfo_max_freq"
        ))
        .ok()?;
        khz.trim().parse::<u32>().ok().map(|khz| khz / 1000)
    }

    /// `implementer:part` of each core from /proc/cpuinfo, e.g. `0x41:0xd82`.
    pub fn part_ids() -> HashMap<usize, String> {
        fs::read_to_string("/proc/cpuinfo")
            .map(|s| super::parse_part_ids(&s))
            .unwrap_or_default()
    }
}

/// `implementer:part` of each core in /proc/cpuinfo.
#[cfg_attr(
    not(all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "aarch64", target_arch = "arm")
    )),
    allow(dead_code)
)]
fn parse_part_ids(cpuinfo: &str) -> std::collections::HashMap<usize, String> {
    let mut ids = std::collections::HashMap::new();
    for block in cpuinfo.split("\n\n") {
        let value = |key: &str| {
            block.lines().find_map(|line| {
                let (k, v) = line.split_once(':')?;
                (k.trim() == key).then(|| v.trim().to_owned())
            })
        };
        if let (Some(Ok(core)), Some(implementer), Some(part)) = (
            value("processor").map(|p| p.parse::<usize>()),
            value("CPU implementer"),
            value("CPU part"),
        ) {
            ids.insert(core, format!("{implementer}:{part}"));
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::{Kind, model, parse_part_ids};

    fn kinds(list: &[(&str, u32, usize)]) -> Vec<Kind> {
        list.iter()
            .flat_map(|&(name, mhz, count)| {
                std::iter::repeat_n(
                    Kind {
                        name: name.into(),
                        max_mhz: Some(mhz),
                    },
                    count,
                )
            })
            .collect()
    }

    #[test]
    fn names_the_kinds() {
        let phone = kinds(&[
            ("Cortex-A520", 1900, 3),
            ("Cortex-A720", 2611, 4),
            ("Cortex-X4", 2803, 1),
        ]);
        assert_eq!(
            model(&phone),
            "3× Cortex-A520 + 4× Cortex-A720 + 1× Cortex-X4"
        );
        // Snapdragon 765G: a prime core and a big core of one design
        let prime = kinds(&[
            ("Kryo-4XX-Silver", 1804, 6),
            ("Kryo-4XX-Gold", 2208, 1),
            ("Kryo-4XX-Gold", 2400, 1),
        ]);
        assert_eq!(
            model(&prime),
            "6× Kryo-4XX-Silver + 1× Kryo-4XX-Gold 2.21 GHz + 1× Kryo-4XX-Gold 2.40 GHz"
        );
        let pc = vec![
            Kind {
                name: "Intel(R) Core(TM) i7-11800H @ 2.30GHz".into(),
                max_mhz: None,
            };
            16
        ];
        assert_eq!(model(&pc), "Intel(R) Core(TM) i7-11800H @ 2.30GHz");
        assert_eq!(model(&[]), "");
    }

    #[test]
    fn reads_part_ids() {
        let cpuinfo = "processor\t: 0\nBogoMIPS\t: 38.40\nCPU implementer\t: 0x41\nCPU part\t: 0xd80\n\n\
                       processor\t: 7\nCPU implementer\t: 0x41\nCPU part\t: 0xd8f\n\n\
                       Hardware\t: Qualcomm\n";
        let ids = parse_part_ids(cpuinfo);
        assert_eq!(ids.get(&0).map(String::as_str), Some("0x41:0xd80"));
        assert_eq!(ids.get(&7).map(String::as_str), Some("0x41:0xd8f"));
        assert_eq!(ids.len(), 2);
    }
}
