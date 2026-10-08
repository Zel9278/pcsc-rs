//! The status sent to the PC Status server. The shape follows
//! pc-status-monorepo-rs (`StatusData`): `gpus` is a list and `uptime` is in
//! seconds. Keep it in step with the server.

use cfg_if::cfg_if;
use serde::Serialize;
use sysinfo::{Cpu, System};

use crate::{gpu, monitor::Sampler};

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

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct StorageData {
    pub(crate) name: String,
    pub(crate) free: u64,
    pub(crate) total: u64,
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
    pub(crate) gpus: Vec<GpuData>,
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
        let Sampler { system, disks } = sampler;

        let os_name = System::name().unwrap_or_else(|| "Unknown OS".into());
        let os_version = System::os_version()
            .or_else(System::kernel_version)
            .unwrap_or_default();
        let hostname = identity
            .hostname
            .clone()
            .or_else(System::host_name)
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

        // The same device can be mounted several times (btrfs subvolumes and
        // the like); list it once. Read-only mounts (AppImage, snap, ISO) are
        // images that always look full, so leave them out.
        let mut storages: Vec<StorageData> = Vec::new();
        for disk in disks
            .iter()
            .filter(|d| d.total_space() != 0 && !d.is_read_only())
        {
            let storage = StorageData {
                name: disk.name().to_string_lossy().into_owned(),
                free: disk.available_space(),
                total: disk.total_space(),
            };
            if !storages.contains(&storage) {
                storages.push(storage);
            }
        }

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
            gpus: gpu::get_info(),
            index: 0,
            histories: [],
        }
    }
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
        }],
        uptime: 61,
        loadavg: [0.5, 0.25, 0.125],
        gpus: vec![GpuData {
            name: "gpu".into(),
            usage: 7.0,
            memory: GpuMemory { free: 5, total: 6 },
        }],
        index: 0,
        histories: [],
    }
}

#[cfg(test)]
mod tests {
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
                "storages": [{ "name": "/dev/sda1", "free": 3, "total": 4 }],
                "uptime": 61,
                "loadavg": [0.5, 0.25, 0.125],
                "gpus": [{ "name": "gpu", "usage": 7.0, "memory": { "free": 5, "total": 6 } }],
                "index": 0,
                "histories": [],
            })
        );
    }
}
