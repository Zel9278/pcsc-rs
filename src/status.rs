//! The status sent to the PC Status server. The shape follows
//! pc-status-monorepo-rs (`StatusData`): `gpus` is a list and `uptime` is in
//! seconds. Keep it in step with the server.

use cfg_if::cfg_if;
use serde::Serialize;
use std::time::Duration;
use sysinfo::{Disks, System};

use crate::{
    battery::{self, BatteryData},
    gpu,
    hide::Hidden,
    io,
    monitor::Sampler,
    thermal::{self, Temperature},
};

#[derive(Serialize, Clone, Debug)]
pub struct CoreData {
    #[serde(rename = "cpu")]
    pub(crate) usage: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    /// Top frequency of the core in MHz (cpufreq policy on Linux and Android, the rated
    /// figure on Windows), sent where known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_mhz: Option<u32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CpuData {
    pub(crate) model: String,
    pub(crate) cpus: Vec<CoreData>,
}

/// `free` is the memory available to programs, not only the unused part.
#[derive(Serialize, Clone, Debug)]
pub struct MemoryData {
    pub(crate) free: u64,
    pub(crate) total: u64,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct StorageData {
    pub(crate) name: String,
    pub(crate) free: u64,
    pub(crate) total: u64,
    /// Bytes read per second since the previous sample.
    pub(crate) read: u64,
    /// Bytes written per second since the previous sample.
    pub(crate) written: u64,
    /// Percentage of time the device was busy (Linux only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) busy: Option<f64>,
}

/// Memory in MiB, as `nvidia-smi` reports it.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct GpuMemory {
    pub(crate) free: u64,
    pub(crate) total: u64,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct GpuData {
    pub(crate) name: String,
    pub(crate) usage: f64,
    pub(crate) memory: GpuMemory,
    /// °C (NVIDIA only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f64>,
    /// Whether `usage` was actually read. A GPU whose usage cannot be read is not sent
    /// as 0%: when none can be read, `gpu_usage` is hidden (see `SystemStatus::hide`).
    #[serde(skip)]
    pub usage_known: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct SystemStatus {
    pub(crate) dev: bool,
    pub(crate) _os: String,
    pub(crate) hostname: String,
    pub(crate) version: String,
    pub(crate) cpu: CpuData,
    pub(crate) ram: MemoryData,
    pub(crate) swap: MemoryData,
    pub(crate) storages: Vec<StorageData>,
    /// Seconds since boot.
    pub(crate) uptime: u64,
    /// Zeros on Windows, which has no load average.
    pub(crate) loadavg: [f64; 3],
    /// Percentage of CPU time spent waiting for IO (Linux only; Proxmox's "IO delay").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) iowait: Option<f64>,
    pub(crate) gpus: Vec<GpuData>,
    /// Only on machines with a battery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) battery: Option<BatteryData>,
    /// Empty where no sensor is readable (Windows without administrator rights, most VMs).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) temperatures: Vec<Temperature>,
    /// Items kept off PC Status (`PCSC_HIDE`, and `gpu_usage` when no GPU's usage can be
    /// read): left out or zero above.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub(crate) hidden: Vec<&'static str>,
    /// Kept by the server; sent for compatibility with the monorepo server.
    pub(crate) index: u32,
    /// Kept by the server; sent for compatibility with the monorepo server.
    pub(crate) histories: [(); 0],
}

/// Identity of this client, fixed for its lifetime.
#[derive(Clone, Debug, Default)]
pub struct Identity {
    pub hostname: Option<String>,
    pub dev: bool,
    pub hidden: Hidden,
}

/// Version shown for this client in PC Status.
const CLIENT_VERSION: &str = concat!(
    "mimizk-rs v",
    env!("CARGO_PKG_VERSION"),
    env!("PCSC_ARCH_SUFFIX")
);

impl SystemStatus {
    pub fn collect(sampler: &Sampler, identity: &Identity) -> Self {
        let Sampler {
            system,
            disks,
            io,
            thermal,
            interval,
            ..
        } = sampler;

        let (os_name, os_version, system_hostname) = platform();
        let hostname = identity
            .hostname
            .clone()
            .or(system_hostname)
            .unwrap_or_else(|| "unknown".into());

        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let loadavg = [0.0; 3];
            } else {
                let load = System::load_average();
                let loadavg = [load.one, load.five, load.fifteen];
            }
        };

        let cpu = CpuData {
            model: cpu_model(system),
            cpus: cpu_cores(system),
        };

        let storages = storages(disks, io, *interval);

        let (battery, temperatures) = sensors(thermal);

        let status = Self {
            dev: identity.dev,
            _os: format!("{os_name} {os_version}").trim_end().to_owned(),
            hostname,
            version: CLIENT_VERSION.into(),
            cpu,
            ram: MemoryData {
                free: system.available_memory(),
                total: system.total_memory(),
            },
            swap: MemoryData {
                free: system.free_swap(),
                total: system.total_swap(),
            },
            storages,
            uptime: System::uptime(),
            loadavg,
            iowait: io.iowait(),
            // Not even asked for when hidden (nvidia-smi is a process each time)
            gpus: if identity.hidden.has("gpu") {
                Vec::new()
            } else {
                gpu::get_info()
            },
            battery,
            temperatures,
            hidden: Vec::new(),
            index: 0,
            histories: [],
        };
        status.hide(&identity.hidden)
    }

    /// Takes out what the owner keeps off PC Status. Fields the wire format requires
    /// become zero or empty; the `hidden` list tells the server which.
    /// `gpu_usage` is added by itself when no GPU's usage can be read.
    // `_os` is the wire format's name for the OS
    #[allow(clippy::used_underscore_binding)]
    fn hide(mut self, hidden: &Hidden) -> Self {
        let hidden = if !self.gpus.is_empty() && self.gpus.iter().all(|g| !g.usage_known) {
            hidden.clone().with("gpu_usage")
        } else {
            hidden.clone()
        };
        let zero = || MemoryData { free: 0, total: 0 };
        for &item in hidden.names() {
            match item {
                "os" => self._os.clear(),
                "cpu" => self.cpu.cpus.iter_mut().for_each(|c| c.usage = 0.0),
                "cpu_name" => {
                    self.cpu.model.clear();
                    self.cpu.cpus.iter_mut().for_each(|c| c.model = None);
                }
                "cpu_frequency" => self.cpu.cpus.iter_mut().for_each(|c| c.max_mhz = None),
                "memory" => (self.ram, self.swap) = (zero(), zero()),
                "storage" => self.storages.clear(),
                "gpu" => self.gpus.clear(),
                "gpu_name" => self.gpus.iter_mut().for_each(|g| g.name.clear()),
                "gpu_usage" => self.gpus.iter_mut().for_each(|g| g.usage = 0.0),
                "gpu_memory" => self
                    .gpus
                    .iter_mut()
                    .for_each(|g| g.memory = GpuMemory { free: 0, total: 0 }),
                "temperature" => {
                    self.temperatures.clear();
                    self.gpus.iter_mut().for_each(|g| g.temperature = None);
                    if let Some(battery) = &mut self.battery {
                        battery.temperature = None;
                    }
                }
                "battery" => self.battery = None,
                "uptime" => self.uptime = 0,
                "loadavg" => self.loadavg = [0.0; 3],
                "iowait" => self.iowait = None,
                _ => {}
            }
        }
        self.hidden = hidden.names().to_vec();
        self
    }
}

fn cpu_model(system: &System) -> String {
    let brand = system
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_owned())
        .unwrap_or_default();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // Krait 400 など、android.rs が SoC から割り出した名前
        let hint = crate::android::cpu_model();
        let model = crate::cpuinfo::model(&brand, hint.as_deref());
        if !model.is_empty() {
            return model;
        }
        if brand.is_empty() {
            return hint.unwrap_or(brand);
        }
    }
    brand
}

/// Cores the kernel has taken offline (phones do this to save power) are not in
/// sysinfo's list; show them as idle cores so a dual-core SoC does not look like one core.
fn cpu_cores(system: &System) -> Vec<CoreData> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let named: Vec<(String, f32)> = system
            .cpus()
            .iter()
            .map(|c| (c.name().to_owned(), c.cpu_usage()))
            .collect();
        let mut usages =
            crate::cpuinfo::spread_usage(&named, crate::cpuinfo::possible_cores().unwrap_or(0));
        let kinds = crate::cpuinfo::core_kinds(crate::android::cpu_model().as_deref());
        let mixed = crate::cpuinfo::is_mixed(&kinds);
        let freqs = crate::cpuinfo::max_mhz_per_core(usages.len());
        // Tegra 3's companion core is invisible to the OS: listed at the running cores' average
        let average = usages.iter().sum::<f32>() / usages.len().max(1) as f32;
        usages.extend(std::iter::repeat_n(
            average,
            crate::cpuinfo::companion_cores(),
        ));
        usages
            .into_iter()
            .enumerate()
            .map(|(i, usage)| {
                let kind = kinds.get(i).and_then(Option::as_ref).filter(|_| mixed);
                CoreData {
                    usage,
                    model: kind.map(|k| k.name.clone()),
                    max_mhz: freqs.get(i).copied().flatten(),
                }
            })
            .collect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let freqs = crate::cpuinfo::max_mhz_per_core(system.cpus().len());
        system
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, c)| CoreData {
                usage: c.cpu_usage(),
                model: None,
                max_mhz: freqs.get(i).copied().flatten(),
            })
            .collect()
    }
}

fn storages(disks: &Disks, io: &io::Tracker, interval: Duration) -> Vec<StorageData> {
    // Bytes since the previous sample, per second; nothing to compare with on the first one
    let per_second = |bytes: u64| {
        let millis = u64::try_from(interval.as_millis()).unwrap_or(u64::MAX);
        bytes.saturating_mul(1000).checked_div(millis).unwrap_or(0)
    };
    // The same device can be mounted several times (btrfs subvolumes and
    // the like); list it once. Read-only mounts (AppImage, snap, ISO) are
    // images that always look full, so leave them out.
    let mut storages: Vec<StorageData> = Vec::new();
    for disk in disks
        .iter()
        .filter(|d| d.total_space() != 0 && !d.is_read_only() && !is_view(d.file_system()))
    {
        let name = disk.name().to_string_lossy().into_owned();
        // Every ZFS dataset is a mount with the pool's free space; show the pool once
        let name = if disk.file_system() == "zfs" {
            zfs_pool(&name).to_owned()
        } else {
            name
        };
        // On Android device names like /dev/block/dm-67 mean nothing to the user; show
        // where it is mounted, and only the user's own storage
        #[cfg(any(target_os = "linux", target_os = "android"))]
        let name = if crate::android::is_android() {
            if !crate::android::is_user_storage(disk.mount_point()) {
                continue;
            }
            disk.mount_point().to_string_lossy().into_owned()
        } else {
            name
        };
        let (free, total) = (disk.available_space(), disk.total_space());
        if let Some(pool) = storages.iter_mut().find(|s| s.name == name)
            && disk.file_system() == "zfs"
        {
            // A dataset's size is its own use plus the pool's free space; the biggest is closest
            if total > pool.total {
                pool.total = total;
                pool.free = free;
            }
            continue;
        }
        if storages
            .iter()
            .any(|s| s.name == name && s.free == free && s.total == total)
        {
            continue;
        }
        let usage = disk.usage();
        let (read, written) = (
            per_second(usage.read_bytes),
            per_second(usage.written_bytes),
        );
        // Android only gives system-wide totals; they belong to the data partition
        #[cfg(any(target_os = "linux", target_os = "android"))]
        let (read, written) = if disk.mount_point() == std::path::Path::new("/data")
            && let Some(rates) = io.system_io()
        {
            rates
        } else {
            (read, written)
        };
        storages.push(StorageData {
            // by the device, not the name shown (a mount point on Android)
            busy: io.busy(&disk.name().to_string_lossy()),
            name,
            free,
            total,
            read,
            written,
        });
    }
    storages
}

/// File systems that show another one's space: Docker's overlay mounts and FUSE mounts
/// (sshfs, mergerfs, rclone, …). `fuseblk` is a real disk (NTFS, exFAT through FUSE).
fn is_view(file_system: &std::ffi::OsStr) -> bool {
    let fs = file_system.to_string_lossy();
    fs == "overlay" || fs.starts_with("fuse.")
}

/// `rpool/data/subvol-101-disk-0` → `rpool`
fn zfs_pool(dataset: &str) -> &str {
    dataset.split('/').next().unwrap_or(dataset)
}

fn sensors(thermal: &thermal::Tracker) -> (Option<BatteryData>, Vec<Temperature>) {
    let battery = battery::get();
    let mut temperatures = thermal.temperatures();
    // Android reports the battery's own sensor with the battery
    if let Some(celsius) = battery.as_ref().and_then(|b| b.temperature) {
        temperatures.retain(|t| t.label != "Battery");
        temperatures.push(Temperature {
            label: "Battery".into(),
            value: celsius,
        });
    }
    (battery, temperatures)
}

/// OS name, version and hostname. On Android sysinfo finds no os-release and the
/// hostname is `localhost`, so they come from the system properties instead.
fn platform() -> (String, String, Option<String>) {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if crate::android::is_android() {
        return (
            crate::android::os_name().unwrap_or_else(|| "Android".into()),
            String::new(),
            crate::android::device_name().or_else(System::host_name),
        );
    }
    // Proxmox runs on Debian, whose os-release it keeps
    #[cfg(target_os = "linux")]
    if let Some((name, version)) = proxmox() {
        return (name, version, System::host_name());
    }
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut name = System::name().unwrap_or_else(|| "Unknown OS".into());
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut version = System::os_version()
        .or_else(System::kernel_version)
        .unwrap_or_default();
    // sysinfo takes the major version from CurrentMajorVersionNumber, which only
    // exists since Windows 10, so older versions come out as "0 (2600)". Use the
    // product name ("Microsoft Windows XP") there instead.
    #[cfg(windows)]
    if version.starts_with("0 ")
        && let Some(product) = System::long_os_version()
    {
        version.drain(.."0 ".len());
        name = product;
    }
    (name, version, System::host_name())
}

/// `Proxmox VE 9.2.6` (or Proxmox Backup Server) on the host itself, not in its guests:
/// its configuration directory, and the version of its package. Read once; dpkg's status
/// file is large.
#[cfg(target_os = "linux")]
fn proxmox() -> Option<(String, String)> {
    static PROXMOX: std::sync::OnceLock<Option<(String, String)>> = std::sync::OnceLock::new();
    PROXMOX
        .get_or_init(|| {
            let (name, package) = [
                ("/etc/pve", "Proxmox VE", "pve-manager"),
                (
                    "/etc/proxmox-backup",
                    "Proxmox Backup Server",
                    "proxmox-backup-server",
                ),
            ]
            .into_iter()
            .find(|(dir, ..)| std::path::Path::new(dir).is_dir())
            .map(|(_, name, package)| (name, package))?;
            let status = std::fs::read_to_string("/var/lib/dpkg/status").ok()?;
            Some((name.to_owned(), dpkg_version(&status, package)?))
        })
        .clone()
}

/// The version of an installed package in dpkg's status file.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn dpkg_version(status: &str, package: &str) -> Option<String> {
    status.split("\n\n").find_map(|entry| {
        let field = |name: &str| {
            entry
                .lines()
                .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
                .map(str::trim)
        };
        (field("Package") == Some(package) && field("Status")?.ends_with(" installed"))
            .then(|| field("Version").map(str::to_owned))?
    })
}

#[cfg(test)]
pub(crate) fn sample() -> SystemStatus {
    SystemStatus {
        dev: false,
        _os: "Arch Linux rolling".into(),
        hostname: "host".into(),
        version: "Rust client test".into(),
        cpu: CpuData {
            model: "cpu".into(),
            cpus: vec![
                CoreData {
                    usage: 12.5,
                    model: None,
                    max_mhz: None,
                },
                CoreData {
                    usage: 50.0,
                    model: Some("Cortex-X4".into()),
                    max_mhz: Some(2803),
                },
            ],
        },
        ram: MemoryData { free: 1, total: 2 },
        swap: MemoryData { free: 0, total: 0 },
        storages: vec![StorageData {
            name: "/dev/sda1".into(),
            free: 3,
            total: 4,
            read: 1024,
            written: 2048,
            busy: Some(12.5),
        }],
        uptime: 61,
        loadavg: [0.5, 0.25, 0.125],
        iowait: Some(1.5),
        gpus: vec![GpuData {
            name: "gpu".into(),
            usage: 7.0,
            memory: GpuMemory { free: 5, total: 6 },
            temperature: Some(55.0),
            usage_known: true,
        }],
        battery: Some(BatteryData {
            level: 80.0,
            state: battery::BatteryState::Charging,
            plugged: Some(true),
            temperature: None,
        }),
        temperatures: vec![Temperature {
            label: "coretemp Package id 0".into(),
            value: 61.5,
        }],
        hidden: Vec::new(),
        index: 0,
        histories: [],
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dpkg_version() {
        let status = "Package: pve-manager\nStatus: install ok installed\nVersion: 9.2.6\n\n\
                      Package: pve-kernel\nStatus: deinstall ok config-files\nVersion: 1.0\n\n\
                      Package: proxmox-backup-server\nStatus: install ok installed\n\
                      Architecture: amd64\nVersion: 4.0.11\n";
        assert_eq!(
            super::dpkg_version(status, "pve-manager").as_deref(),
            Some("9.2.6")
        );
        assert_eq!(
            super::dpkg_version(status, "proxmox-backup-server").as_deref(),
            Some("4.0.11")
        );
        // Removed, and not there at all
        assert_eq!(super::dpkg_version(status, "pve-kernel"), None);
        assert_eq!(super::dpkg_version(status, "pve"), None);
    }

    #[test]
    fn storage_filters() {
        use std::ffi::OsStr;
        assert!(super::is_view(OsStr::new("overlay")));
        assert!(super::is_view(OsStr::new("fuse.sshfs")));
        assert!(!super::is_view(OsStr::new("fuseblk")));
        assert!(!super::is_view(OsStr::new("ext4")));
        assert_eq!(super::zfs_pool("rpool/data/subvol-101-disk-0"), "rpool");
        assert_eq!(super::zfs_pool("tank"), "tank");
    }

    #[test]
    #[allow(clippy::used_underscore_binding)]
    fn hides_items() {
        let (hidden, _) = crate::hide::Hidden::parse("gpu_usage memory os gpu_name");
        let status = super::sample().hide(&hidden);
        assert_eq!(status.hidden, ["gpu_name", "gpu_usage", "memory", "os"]);
        assert!(status.gpus.iter().all(|g| g.name.is_empty()));
        assert_eq!(status._os, "");
        assert_eq!((status.ram.total, status.swap.total), (0, 0));
        assert!(status.gpus.iter().all(|g| g.usage == 0.0));
        // Not hidden: left as it was
        assert_eq!(status.cpu.cpus[0].usage, super::sample().cpu.cpus[0].usage);
        // Nothing hidden: no list on the wire
        let json =
            serde_json::to_value(super::sample().hide(&crate::hide::Hidden::default())).unwrap();
        assert!(json.get("hidden").is_none());
    }

    #[test]
    fn hides_gpu_usage_that_cannot_be_read() {
        // None readable: hidden by itself
        let mut unreadable = super::sample();
        unreadable
            .gpus
            .iter_mut()
            .for_each(|g| g.usage_known = false);
        let status = unreadable.hide(&crate::hide::Hidden::default());
        assert_eq!(status.hidden, ["gpu_usage"]);
        assert!(status.gpus.iter().all(|g| g.usage == 0.0));

        // Readable: left alone
        let status = super::sample().hide(&crate::hide::Hidden::default());
        assert_eq!(status.hidden, [] as [&str; 0]);
        assert_eq!(status.gpus[0].usage, 7.0);
    }

    #[test]
    fn wire_format() {
        assert_eq!(
            serde_json::to_value(super::sample()).unwrap(),
            serde_json::json!({
                "dev": false,
                "_os": "Arch Linux rolling",
                "hostname": "host",
                "version": "Rust client test",
                "cpu": { "model": "cpu", "cpus": [{ "cpu": 12.5 }, { "cpu": 50.0, "model": "Cortex-X4", "max_mhz": 2803 }] },
                "ram": { "free": 1, "total": 2 },
                "swap": { "free": 0, "total": 0 },
                "storages": [{ "name": "/dev/sda1", "free": 3, "total": 4, "read": 1024, "written": 2048, "busy": 12.5 }],
                "uptime": 61,
                "loadavg": [0.5, 0.25, 0.125],
                "iowait": 1.5,
                "gpus": [{ "name": "gpu", "usage": 7.0, "memory": { "free": 5, "total": 6 }, "temperature": 55.0 }],
                "battery": { "level": 80.0, "state": "charging", "plugged": true },
                "temperatures": [{ "label": "coretemp Package id 0", "value": 61.5 }],
                "index": 0,
                "histories": [],
            })
        );
    }
}
