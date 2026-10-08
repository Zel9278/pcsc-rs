//! The status sent to the PC Status server. Field names and shapes are part of
//! the wire format the server (and its dashboard) expect; keep them as they are.

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

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct GpuData {
    pub(crate) name: String,
    /// `None` when the driver reports `[N/A]`.
    pub(crate) usage: Option<u64>,
    pub(crate) memory: GpuMemory,
}

#[derive(Serialize, Clone, Debug)]
pub struct SystemStatus {
    pub(crate) _os: String,
    pub(crate) hostname: String,
    pub(crate) version: String,
    pub(crate) cpu: CpuData,
    pub(crate) ram: MemoryData,
    pub(crate) swap: MemoryData,
    pub(crate) storages: Vec<StorageData>,
    #[serde(rename = "loadavg")]
    pub(crate) load_average: Option<[f64; 3]>,
    pub(crate) uptime: String,
    pub(crate) gpu: Option<GpuData>,
}

/// The first status (`hi`) carries the password.
#[derive(Serialize)]
pub struct StatusWithPass<'a> {
    #[serde(flatten)]
    pub(crate) status: &'a SystemStatus,
    pub(crate) pass: &'a str,
}

/// `git describe` of the build, or the crate version when built outside a repository.
const VERSION: &str = match option_env!("GIT_DESCRIBE") {
    Some(describe) => describe,
    None => env!("CARGO_PKG_VERSION"),
};

impl SystemStatus {
    pub fn collect(sampler: &Sampler, hostname: Option<&str>) -> Self {
        let Sampler { system, disks } = sampler;

        let os_name = System::name().unwrap_or_else(|| "Unknown OS".into());
        let os_version = System::os_version()
            .or_else(System::kernel_version)
            .unwrap_or_default();
        let hostname = hostname
            .map(str::to_owned)
            .or_else(System::host_name)
            .unwrap_or_else(|| "unknown".into());

        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let load_average = None;
            } else {
                let load = System::load_average();
                let load_average = Some([load.one, load.five, load.fifteen]);
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
        // the like); list it once.
        let mut storages: Vec<StorageData> = Vec::new();
        for disk in disks.iter().filter(|d| d.total_space() != 0) {
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
            load_average,
            uptime: format_uptime(System::uptime()),
            gpu: gpu::get_info(),
        }
    }

    pub fn with_pass<'a>(&'a self, pass: &'a str) -> StatusWithPass<'a> {
        StatusWithPass { status: self, pass }
    }
}

/// The server and dashboard read this exact shape.
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = seconds % 86400 / 3600;
    let minutes = seconds % 3600 / 60;
    let seconds = seconds % 60;
    format!("{days} days {hours} hours {minutes} minutes {seconds} seconds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_format() {
        assert_eq!(format_uptime(0), "0 days 0 hours 0 minutes 0 seconds");
        assert_eq!(
            format_uptime(5 * 86400 + 27 * 60 + 2),
            "5 days 0 hours 27 minutes 2 seconds"
        );
    }

    #[test]
    fn wire_format() {
        let status = SystemStatus {
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
            load_average: Some([0.5, 0.25, 0.125]),
            uptime: "0 days 0 hours 0 minutes 1 seconds".into(),
            gpu: Some(GpuData {
                name: "gpu".into(),
                usage: None,
                memory: GpuMemory { free: 5, total: 6 },
            }),
        };
        let json = serde_json::to_value(status.with_pass("secret")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "_os": "Arch Linux rolling",
                "hostname": "host",
                "version": "Rust client test",
                "cpu": { "model": "cpu", "cpus": [{ "cpu": 12.5 }] },
                "ram": { "free": 1, "total": 2 },
                "swap": { "free": 0, "total": 0 },
                "storages": [{ "name": "/dev/sda1", "free": 3, "total": 4 }],
                "loadavg": [0.5, 0.25, 0.125],
                "uptime": "0 days 0 hours 0 minutes 1 seconds",
                "gpu": { "name": "gpu", "usage": null, "memory": { "free": 5, "total": 6 } },
                "pass": "secret",
            })
        );
    }
}
