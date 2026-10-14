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
    let intel = if cards
        .iter()
        .any(|c| matches!(c.driver.as_str(), "i915" | "xe") && !c.suspended)
    {
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
        if !matches!(
            driver.as_str(),
            "amdgpu" | "i915" | "xe" | "mgag200" | "ast"
        ) {
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
    if vendor == "8086"
        && let Some(name) = intel_name(&device_id)
    {
        return format!("Intel {name}");
    }
    if (vendor.as_str(), device_id.as_str()) == ("102b", "0532") {
        return "Matrox MGA G200eW WPCM450".to_owned();
    }
    fn intel_name(device: &str) -> Option<&'static str> {
        Some(match device {
            // GMA
            "2582" | "2592" => "GMA 900",
            "2772" | "27a2" | "27ae" => "GMA 950",
            "2972" => "GMA 3000",
            "29a2" => "GMA X3000",
            "2a02" => "GMA X3100",
            "2a42" => "GMA 4500MHD",
            "2e12" | "2e22" | "2e32" | "2e42" => "GMA X4500",
            // Ironlake
            "0042" | "0046" => "HD Graphics",
            // Sandy Bridge
            "0102" => "HD Graphics 2000",
            "0112" | "0122" | "0116" | "0126" => "HD Graphics 3000",
            "0106" => "HD Graphics",
            "010a" => "HD Graphics P3000",
            // Ivy Bridge
            "0152" | "0156" => "HD Graphics 2500",
            "0162" | "0166" => "HD Graphics 4000",
            "016a" => "HD Graphics P4000",
            // Haswell
            "0412" | "0416" => "HD Graphics 4600",
            "0a16" => "HD Graphics 4400",
            "0a1e" => "HD Graphics 4200",
            "0a26" => "HD Graphics 5000",
            "0a2e" => "Iris Graphics 5100",
            "0d22" | "0d26" => "Iris Pro Graphics 5200",
            // Broadwell
            "1612" => "HD Graphics 5600",
            "1616" => "HD Graphics 5500",
            "161e" => "HD Graphics 5300",
            "1622" => "Iris Pro Graphics 6200",
            "1626" => "HD Graphics 6000",
            "162b" => "Iris Graphics 6100",
            // Skylake
            "1906" => "HD Graphics 510",
            "1912" | "191b" => "HD Graphics 530",
            "1916" => "HD Graphics 520",
            "191e" => "HD Graphics 515",
            "1926" => "Iris Graphics 540",
            "1927" => "Iris Graphics 550",
            "193b" => "Iris Pro Graphics 580",
            // Kaby Lake
            "5902" | "5906" => "HD Graphics 610",
            "5912" | "591b" => "HD Graphics 630",
            "5916" => "HD Graphics 620",
            "591e" => "HD Graphics 615",
            "5926" => "Iris Plus Graphics 640",
            "5927" => "Iris Plus Graphics 650",
            // Apollo Lake, Gemini Lake
            "5a84" => "HD Graphics 505",
            "5a85" => "HD Graphics 500",
            "3184" => "UHD Graphics 605",
            "3185" => "UHD Graphics 600",
            // Coffee Lake, Whiskey Lake, Amber Lake, Comet Lake
            "3e90" | "3e93" => "UHD Graphics 610",
            "3e91" | "3e92" | "3e98" | "3e9b" => "UHD Graphics 630",
            "3ea0" => "UHD Graphics 620",
            "3ea5" => "Iris Plus Graphics 655",
            "3ea6" => "Iris Plus Graphics 645",
            "87c0" => "UHD Graphics 617",
            "9b41" => "UHD Graphics",
            // Ice Lake, Tiger Lake
            "8a52" | "8a51" => "Iris Plus Graphics G7",
            "8a5a" | "8a5c" => "Iris Plus Graphics G4",
            "8a56" => "UHD Graphics G1",
            "9a49" | "9a40" => "Iris Xe Graphics",
            "9a60" | "9a68" => "UHD Graphics",
            // Alder Lake, Raptor Lake
            "4680" | "4690" | "a780" => "UHD Graphics 770",
            "4682" | "4692" => "UHD Graphics 730",
            "46a6" => "Iris Xe Graphics",
            // Arc
            "56a0" => "Arc A770",
            "56a1" => "Arc A750",
            "56a5" => "Arc A380",
            "56a6" => "Arc A310",
            "5690" => "Arc A770M",
            "5691" => "Arc A730M",
            "5693" => "Arc A370M",
            _ => return None,
        })
    }
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
        "102b" => "Matrox",
        "1a03" => "ASPEED",
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
    use super::{intel_name, pci_name};

    const IDS: &str = "\
# comment
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t164e  Raphael
\t744c  Navi 31 [Radeon RX 7900 XT/7900 XTX/7900 GRE/7900M]
\t\t1002 0e3b  Radeon RX 7900 GRE
14e4  Broadcom Inc. and subsidiaries
\t164e  NetXtreme II BCM57710 10-Gigabit PCIe [Everest]
102b  Matrox Electronics Systems Ltd.
\t0532  MGA G200eW WPCM450
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
        assert_eq!(
            pci_name(IDS, "102b", "0532").as_deref(),
            Some("Matrox MGA G200eW WPCM450")
        );
    }
    #[test]
    fn intel_names() {
        assert_eq!(intel_name("3ea0"), Some("UHD Graphics 620"));
        assert_eq!(intel_name("2a42"), Some("GMA 4500MHD"));
        assert_eq!(intel_name("0116"), Some("HD Graphics 3000"));
        assert_eq!(intel_name("ffff"), None);
    }
}
