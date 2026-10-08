use std::{sync::Arc, thread, time::Duration};

use arc_swap::ArcSwap;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::status::{Identity, SystemStatus};

pub type SharedStatus = Arc<ArcSwap<SystemStatus>>;

/// sysinfo state kept between samples (CPU usage needs the previous one).
pub struct Sampler {
    pub system: System,
    pub disks: Disks,
}

fn refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
        .with_memory(MemoryRefreshKind::everything())
}

impl Sampler {
    fn new() -> Self {
        Self {
            system: System::new_with_specifics(refresh_kind()),
            disks: Disks::new_with_refreshed_list_specifics(
                DiskRefreshKind::nothing().with_storage(),
            ),
        }
    }

    fn refresh(&mut self) {
        self.system.refresh_specifics(refresh_kind());
        self.disks
            .refresh_specifics(true, DiskRefreshKind::nothing().with_storage());
    }
}

/// Samples the system every second on its own thread. The returned handle
/// always holds the latest sample, so sending never waits for a measurement.
pub fn spawn(identity: Identity) -> SharedStatus {
    let mut sampler = Sampler::new();
    let shared: SharedStatus = Arc::new(ArcSwap::from_pointee(SystemStatus::collect(
        &sampler, &identity,
    )));

    let writer = Arc::clone(&shared);
    thread::Builder::new()
        .name("System Monitor".into())
        .spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));
                sampler.refresh();
                writer.store(Arc::new(SystemStatus::collect(&sampler, &identity)));
            }
        })
        .expect("Failed to start the system monitor thread");

    shared
}
