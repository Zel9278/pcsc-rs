//! The status sent to the PC Status server. The shape follows
//! pc-status-monorepo-rs (`StatusData`): `gpus` is a list and `uptime` is in
//! seconds. Keep it in step with the server.

use cfg_if::cfg_if;
use serde::Serialize;
use std::time::Duration;

use sysinfo::{Cpu, Disks, System};

use crate::{
    battery::{self, BatteryData},
    gpu, io,
    monitor::Sampler,
    thermal::{self, Temperature},
};

#[derive(Serialize, Clone, Debug)]
pub struct CoreData {
    #[serde(rename = "cpu")]
    pub(crate) usage: f32,
}

impl From<&Cpu> for CoreData {
    fn from(value: &Cpu) -> Self {
        Self {
            usage: value.cpu_usage(),
        }
    }
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
}

/// `git describe` of the build, or the crate version when built outside a repository.
const VERSION: &str = match option_env!("GIT_DESCRIBE") {
    Some(describe) => describe,
    None => env!("CARGO_PKG_VERSION"),
};

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
        }

        let cpu = CpuData {
            model: system
                .cpus()
                .first()
                .map(|c| c.brand().to_owned())
                .unwrap_or_default(),
            cpus: system.cpus().iter().map(Into::into).collect(),
        };

        let storages = storages(disks, io, *interval);

        let (battery, temperatures) = sensors(thermal);

        Self {
            dev: identity.dev,
            _os: format!("{os_name} {os_version}").trim_end().to_owned(),
            hostname,
            version: format!("Rust client {VERSION}"),
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
            gpus: gpu::get_info(),
            battery,
            temperatures,
            index: 0,
            histories: [],
        }
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

#[cfg(test)]
pub(crate) fn sample() -> SystemStatus {
    SystemStatus {
        dev: false,
        _os: "Arch Linux rolling".into(),
        hostname: "host".into(),
        version: "Rust client test".into(),
        cpu: CpuData {
            model: "cpu".into(),
            cpus: vec![CoreData { usage: 12.5 }],
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
        index: 0,
        histories: [],
    }
}

#[cfg(test)]
mod tests {
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
    fn wire_format() {
        assert_eq!(
            serde_json::to_value(super::sample()).unwrap(),
            serde_json::json!({
                "dev": false,
                "_os": "Arch Linux rolling",
                "hostname": "host",
                "version": "Rust client test",
                "cpu": { "model": "cpu", "cpus": [{ "cpu": 12.5 }] },
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
