use std::env;

use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use sysinfo::{Cpu, CpuExt, DiskExt, System, SystemExt};

use crate::{gpu, unix_to_date};

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct CpuData {
    pub(crate) model: String,
    pub(crate) cpus: Vec<CoreData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RamData {
    pub(crate) free: u64,
    pub(crate) total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StorageData {
    pub(crate) free: u64,
    pub(crate) total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpuMemory {
    pub(crate) free: u64,
    pub(crate) total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpuData {
    pub(crate) name: String,
    pub(crate) usage: Option<u64>,
    pub(crate) memory: GpuMemory,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SystemStatus {
    pub(crate) _os: String,
    pub(crate) hostname: String,
    pub(crate) version: String,
    pub(crate) cpu: CpuData,
    pub(crate) ram: RamData,
    pub(crate) storage: StorageData,
    #[serde(rename = "loadavg")]
    pub(crate) load_average: Option<[f64; 3]>,
    pub(crate) uptime: String,
    pub(crate) gpu: Option<GpuData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StatusDataWithPass {
    #[serde(flatten)]
    pub(crate) status: SystemStatus,
    pub(crate) pass: String,
}

impl SystemStatus {
    pub fn get(sys: &mut System) -> Self {
        let cpu_name = sys.cpus()[0].brand().to_string();
        let os_name = sys.name().expect("Failed to get os name");
        let os_version = sys
            .os_version()
            .or(sys.kernel_version())
            .expect("Failed to get os version");

        let hostname = env::var("HOSTNAME")
            .unwrap_or_else(|_| sys.host_name().expect("Failed to get hostname"));

        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let load_average = None;
            } else {
                let load_average = sys.load_average();
                let load_average = Some([load_average.one, load_average.five, load_average.fifteen]);
            }
        };

        let cpus: Vec<CoreData> = sys.cpus().iter().map(Into::into).collect();

        let cpu = CpuData {
            model: cpu_name.clone(),
            cpus,
        };

        let ram = RamData {
            free: sys.free_memory(),
            total: sys.total_memory(),
        };

        let uptime = unix_to_date::new(sys.uptime());

        let disk = sys.disks().iter().next();

        let storage = StorageData {
            free: disk.unwrap().available_space(),
            total: disk.unwrap().total_space(),
        };

        let gpu = gpu::get_info();

        Self {
            _os: format!("{} {}", os_name.clone(), os_version.clone()),
            version: "Rust".into(),
            hostname,
            cpu,
            ram,
            load_average,
            uptime,
            storage,
            gpu,
        }
    }

    pub fn with_pass(self, pass: String) -> StatusDataWithPass {
        StatusDataWithPass { status: self, pass }
    }
}
