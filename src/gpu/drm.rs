//! AMD and Intel GPUs on Linux, read from the DRM devices in sysfs
//! (`/sys/class/drm/cardN/device`). NVIDIA's come from `nvidia-smi` instead.
//!
//! - AMD (`amdgpu`): `gpu_busy_percent`, VRAM from `mem_info_vram_*`, and the
//!   edge temperature from its hwmon.
//! - Intel (`i915`, `xe`): sysfs has no usage. It comes from the DRM clients'
//!   fdinfo (`/proc/PID/fdinfo`), as in nvtop and `intel_gpu_top`: the time each
//!   program kept each kind of engine busy, the busiest kind being the usage.
//!   Only the programs this client may look at count, so run as root it sees every
//!   one, and as a user only that user's. (How long the GPU idled, RC6, is no
//!   measure: an awake GPU with nothing to do is not in RC6.) Integrated GPUs have
//!   no VRAM of their own, so their memory is unknown.
//!
//! A runtime-suspended GPU (the dGPU of a hybrid laptop, an unused iGPU) is not
//! read, since reading wakes it up; it is reported as idle.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::status::{GpuData, GpuMemory};

mod engines;

const DRM: &str = "/sys/class/drm";
const PCI_IDS: [&str; 3] = [
    "/usr/share/hwdata/pci.ids",
    "/usr/share/misc/pci.ids",
    "/usr/share/pci.ids",
];

pub fn get_info() -> Vec<GpuData> {
    let Ok(entries) = fs::read_dir(DRM) else {
        return Vec::new();
    };
    // card0, card1, …, not the connectors (card0-HDMI-A-1)
    let mut cards: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("card"))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    cards.sort_by_key(|p| card_number(p));
    let cards: Vec<Card> = cards.iter().filter_map(|card| Card::new(card)).collect();
    // Reading every program's fdinfo is the costly part; only when an Intel GPU is awake
    let intel = if cards.iter().any(|c| c.driver != "amdgpu" && !c.suspended) {
        engines::usage()
    } else {
        HashMap::new()
    };
    cards.iter().map(|card| card.gpu(&intel)).collect()
}

struct Card {
    device: PathBuf,
    driver: String,
    suspended: bool,
    /// PCI slot (`0000:00:02.0`), which fdinfo calls `drm-pdev`
    slot: String,
}

impl Card {
    fn new(card: &Path) -> Option<Self> {
        let device = card.join("device");
        let driver = fs::read_link(device.join("driver")).ok()?;
        let driver = driver.file_name()?.to_str()?.to_owned();
        if !matches!(driver.as_str(), "amdgpu" | "i915" | "xe") {
            return None;
        }
        let slot = fs::canonicalize(&device)
            .ok()
            .and_then(|p| Some(p.file_name()?.to_str()?.to_owned()))
            .unwrap_or_default();
        Some(Self {
            suspended: read(device.join("power/runtime_status")).as_deref() == Some("suspended"),
            device,
            driver,
            slot,
        })
    }

    fn gpu(&self, intel: &HashMap<String, f64>) -> GpuData {
        let (usage, memory) = if self.driver == "amdgpu" {
            amd(&self.device, self.suspended)
        } else if self.suspended {
            (0.0, unknown_memory())
        } else {
            (
                intel.get(&self.slot).copied().unwrap_or(0.0),
                unknown_memory(),
            )
        };
        GpuData {
            name: name(&self.device),
            usage,
            memory,
            temperature: if self.suspended {
                None
            } else {
                temperature(&self.device)
            },
        }
    }
}

fn card_number(card: &Path) -> u32 {
    card.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("card"))
        .and_then(|n| n.parse().ok())
        .unwrap_or(u32::MAX)
}

fn read(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn unknown_memory() -> GpuMemory {
    GpuMemory { free: 0, total: 0 }
}

/// Usage and VRAM. While suspended, the VRAM figures from the last sample, as idle.
fn amd(device: &Path, suspended: bool) -> (f64, GpuMemory) {
    static LAST_MEMORY: Mutex<Option<HashMap<PathBuf, GpuMemory>>> = Mutex::new(None);
    let mut last = LAST_MEMORY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let last = last.get_or_insert_with(HashMap::new);
    if suspended {
        let memory = last.get(device).cloned().unwrap_or_else(unknown_memory);
        return (0.0, memory);
    }
    let usage = read(device.join("gpu_busy_percent"))
        .and_then(|s| s.parse::<f64>().ok())
        .map_or(0.0, |p| p.clamp(0.0, 100.0));
    let bytes = |name: &str| read(device.join(name)).and_then(|s| s.parse::<u64>().ok());
    let memory = match (bytes("mem_info_vram_total"), bytes("mem_info_vram_used")) {
        (Some(total), Some(used)) if total > 0 => {
            let total = total / (1024 * 1024);
            GpuMemory {
                free: total.saturating_sub(used / (1024 * 1024)),
                total,
            }
        }
        _ => unknown_memory(),
    };
    last.insert(device.to_owned(), memory.clone());
    (usage, memory)
}

/// °C of the first hwmon sensor (amdgpu's `edge`; Intel discrete GPUs).
fn temperature(device: &Path) -> Option<f64> {
    let hwmon = fs::read_dir(device.join("hwmon")).ok()?.flatten().next()?;
    let milli: f64 = read(hwmon.path().join("temp1_input"))?.parse().ok()?;
    Some(milli / 1000.0)
}

/// The board's own name when the driver has one, else the PCI ID's name from pci.ids:
/// `Intel UHD Graphics`, `AMD Radeon RX 7900 XT/7900 XTX/7900 GRE/7900M`. Looked up once
/// per device: pci.ids is large.
fn name(device: &Path) -> String {
    static NAMES: Mutex<Option<HashMap<PathBuf, String>>> = Mutex::new(None);
    let mut names = NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    names
        .get_or_insert_with(HashMap::new)
        .entry(device.to_owned())
        .or_insert_with(|| look_up_name(device))
        .clone()
}

fn look_up_name(device: &Path) -> String {
    if let Some(product) = read(device.join("product_name")).filter(|s| !s.is_empty()) {
        return product;
    }
    let id = |name: &str| {
        read(device.join(name))
            .map(|s| s.trim_start_matches("0x").to_lowercase())
            .unwrap_or_default()
    };
    let (vendor, device_id) = (id("vendor"), id("device"));
    PCI_IDS
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())
        .and_then(|ids| pci_name(&ids, &vendor, &device_id))
        .unwrap_or_else(|| format!("{} GPU ({vendor}:{device_id})", vendor_name(&vendor, None)))
}

fn vendor_name<'a>(vendor: &str, from_ids: Option<&'a str>) -> &'a str {
    match vendor {
        "8086" => "Intel",
        "1002" => "AMD",
        _ => from_ids.unwrap_or("GPU"),
    }
}

/// The name of `vendor:device` in pci.ids. A marketing name in brackets is preferred:
/// `TigerLake-H GT1 [UHD Graphics]` → `Intel UHD Graphics`.
fn pci_name(ids: &str, vendor: &str, device: &str) -> Option<String> {
    let mut in_vendor: Option<&str> = None;
    for line in ids.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            if in_vendor.is_some() {
                // The vendor's section ended without the device
                return None;
            }
            if let Some(rest) = line.strip_prefix(vendor).and_then(|r| r.strip_prefix("  ")) {
                in_vendor = Some(rest.trim());
            }
            continue;
        }
        let Some(vendor_full) = in_vendor else {
            continue;
        };
        // Subsystems are indented twice
        let Some(rest) = line.strip_prefix('\t').filter(|r| !r.starts_with('\t')) else {
            continue;
        };
        let Some(model) = rest.strip_prefix(device).and_then(|r| r.strip_prefix("  ")) else {
            continue;
        };
        let model = model.trim();
        let model = match (model.find('['), model.rfind(']')) {
            (Some(start), Some(end)) if start < end => &model[start + 1..end],
            _ => model,
        };
        return Some(format!(
            "{} {model}",
            vendor_name(vendor, Some(vendor_full))
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::pci_name;

    const IDS: &str = "\
# comment
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t164e  Raphael
\t744c  Navi 31 [Radeon RX 7900 XT/7900 XTX/7900 GRE/7900M]
\t\t1002 0e3b  Radeon RX 7900 GRE
14e4  Broadcom Inc. and subsidiaries
\t164e  NetXtreme II BCM57710 10-Gigabit PCIe [Everest]
8086  Intel Corporation
\t9a60  TigerLake-H GT1 [UHD Graphics]
";

    #[test]
    fn names_from_pci_ids() {
        assert_eq!(
            pci_name(IDS, "8086", "9a60").as_deref(),
            Some("Intel UHD Graphics")
        );
        assert_eq!(
            pci_name(IDS, "1002", "744c").as_deref(),
            Some("AMD Radeon RX 7900 XT/7900 XTX/7900 GRE/7900M")
        );
        // Not Broadcom's 164e
        assert_eq!(
            pci_name(IDS, "1002", "164e").as_deref(),
            Some("AMD Raphael")
        );
        assert_eq!(pci_name(IDS, "1002", "ffff"), None);
        assert_eq!(pci_name(IDS, "10de", "2504"), None);
    }
}
