//! Disk activity that sysinfo does not report: the share of CPU time spent
//! waiting for IO (iowait, what Proxmox calls "IO delay") and how busy each
//! disk was (iostat's %util). Both come from `/proc` and exist on Linux only;
//! elsewhere they are reported as unknown.

/// Keeps the previous counters so each sample can report the change since the last one.
#[derive(Default)]
pub struct Tracker {
    #[cfg(target_os = "linux")]
    inner: linux::Tracker,
}

impl Tracker {
    pub fn new() -> Self {
        let mut tracker = Self::default();
        tracker.refresh();
        tracker
    }

    pub fn refresh(&mut self) {
        #[cfg(target_os = "linux")]
        self.inner.refresh();
    }

    /// Percentage of CPU time spent waiting for IO since the previous refresh.
    pub fn iowait(&self) -> Option<f64> {
        #[cfg(target_os = "linux")]
        return self.inner.iowait;
        #[cfg(not(target_os = "linux"))]
        None
    }

    /// Percentage of time the device (e.g. `/dev/sda1`) was busy since the previous refresh.
    #[cfg_attr(not(target_os = "linux"), allow(clippy::unused_self))]
    pub fn busy(&self, device: &str) -> Option<f64> {
        #[cfg(target_os = "linux")]
        return self.inner.busy(device);
        #[cfg(not(target_os = "linux"))]
        {
            let _ = device;
            None
        }
    }
}

#[cfg(any(target_os = "linux", test))]
mod parse {
    /// CPU time counters from the first `cpu` line of `/proc/stat`.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct CpuTimes {
        pub iowait: u64,
        pub total: u64,
    }

    pub fn cpu_times(stat: &str) -> Option<CpuTimes> {
        let line = stat.lines().find(|l| l.starts_with("cpu "))?;
        // user nice system idle iowait irq softirq steal (guest time is already in user)
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .take(8)
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        (fields.len() >= 5).then(|| CpuTimes {
            iowait: fields[4],
            total: fields.iter().sum(),
        })
    }

    /// Milliseconds each device spent doing IO (`io_ticks`), by name (`sda1`, `dm-0`, …).
    pub fn io_ticks(diskstats: &str) -> Vec<(String, u64)> {
        diskstats
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace().skip(2);
                let name = fields.next()?.to_owned();
                // reads, merged, sectors, ms, writes, merged, sectors, ms, in progress, io_ticks
                let ticks = fields.nth(9)?.parse().ok()?;
                Some((name, ticks))
            })
            .collect()
    }

    /// `part / whole` as a percentage, kept within 0–100.
    #[allow(clippy::cast_precision_loss)]
    pub fn percent(part: u64, whole: u64) -> Option<f64> {
        (whole > 0).then(|| (part as f64 / whole as f64 * 100.0).clamp(0.0, 100.0))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{collections::HashMap, fs, path::Path, time::Instant};

    use super::parse::{self, CpuTimes};

    #[derive(Default)]
    pub struct Tracker {
        cpu: Option<CpuTimes>,
        ticks: HashMap<String, u64>,
        at: Option<Instant>,
        pub iowait: Option<f64>,
        busy: HashMap<String, f64>,
    }

    impl Tracker {
        pub fn refresh(&mut self) {
            let now = Instant::now();

            let cpu = fs::read_to_string("/proc/stat")
                .ok()
                .and_then(|s| parse::cpu_times(&s));
            self.iowait = match (self.cpu, cpu) {
                (Some(old), Some(new)) => parse::percent(
                    new.iowait.saturating_sub(old.iowait),
                    new.total.saturating_sub(old.total),
                ),
                _ => None,
            };
            self.cpu = cpu;

            let ticks: HashMap<String, u64> = fs::read_to_string("/proc/diskstats")
                .map(|s| parse::io_ticks(&s).into_iter().collect())
                .unwrap_or_default();
            self.busy.clear();
            if let Some(at) = self.at {
                let elapsed = u64::try_from(now.duration_since(at).as_millis()).unwrap_or(u64::MAX);
                for (name, &new) in &ticks {
                    if let Some(&old) = self.ticks.get(name)
                        && let Some(busy) = parse::percent(new.saturating_sub(old), elapsed)
                    {
                        self.busy.insert(name.clone(), busy);
                    }
                }
            }
            self.ticks = ticks;
            self.at = Some(now);
        }

        /// `/dev/sda1` → `sda1`, `/dev/mapper/vg-root` → `dm-0` (the name in `/proc/diskstats`).
        pub fn busy(&self, device: &str) -> Option<f64> {
            if !device.starts_with("/dev/") {
                return None;
            }
            let real = fs::canonicalize(device).ok()?;
            let name = Path::new(&real).file_name()?.to_str()?;
            self.busy.get(name).copied()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse::{CpuTimes, cpu_times, io_ticks, percent};

    #[test]
    fn reads_cpu_times() {
        let stat = "cpu  100 5 50 800 40 3 2 0 7 0\ncpu0 50 2 25 400 20 1 1 0 3 0\nintr 1 2 3\n";
        assert_eq!(
            cpu_times(stat),
            Some(CpuTimes {
                iowait: 40,
                total: 1000
            })
        );
        assert_eq!(cpu_times("intr 1 2 3\n"), None);
    }

    #[test]
    fn reads_io_ticks() {
        let diskstats = "\
   8       0 sda 1000 10 20000 300 500 5 8000 200 0 450 600 0 0 0 0 0 0
   8       1 sda1 900 10 18000 280 480 5 7600 190 0 420 570 0 0 0 0 0 0
 253       0 dm-0 50 0 400 10 20 0 160 5 0 12 15
   7       0 loop0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
";
        assert_eq!(
            io_ticks(diskstats),
            [
                ("sda".to_owned(), 450),
                ("sda1".to_owned(), 420),
                ("dm-0".to_owned(), 12),
                ("loop0".to_owned(), 0)
            ]
        );
        assert_eq!(io_ticks("8 0 short 1 2 3\n"), []);
    }

    #[test]
    fn percentages() {
        assert_eq!(percent(40, 1000), Some(4.0));
        assert_eq!(percent(1500, 1000), Some(100.0));
        assert_eq!(percent(1, 0), None);
    }
}
