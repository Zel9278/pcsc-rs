use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::{
    io,
    status::{Identity, SystemStatus},
};

pub type SharedStatus = Arc<ArcSwap<SystemStatus>>;

/// State kept between samples: CPU usage, disk reads/writes and IO wait are
/// all the change since the previous sample.
pub struct Sampler {
    pub system: System,
    pub disks: Disks,
    pub io: io::Tracker,
    /// Time between the last two samples; zero before the second one.
    pub interval: Duration,
    sampled_at: Instant,
}

fn refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
        .with_memory(MemoryRefreshKind::everything())
}

fn disk_refresh_kind() -> DiskRefreshKind {
    DiskRefreshKind::nothing().with_storage().with_io_usage()
}

impl Sampler {
    fn new() -> Self {
        Self {
            system: System::new_with_specifics(refresh_kind()),
            disks: Disks::new_with_refreshed_list_specifics(disk_refresh_kind()),
            io: io::Tracker::new(),
            interval: Duration::ZERO,
            sampled_at: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        self.system.refresh_specifics(refresh_kind());
        self.disks.refresh_specifics(true, disk_refresh_kind());
        self.io.refresh();
        let now = Instant::now();
        self.interval = now.duration_since(self.sampled_at);
        self.sampled_at = now;
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
