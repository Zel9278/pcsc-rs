//! GPU usage from the DRM clients' fdinfo (`Documentation/gpu/drm-usage-stats.rst`).
//! Every open DRM file of every program reports how long it kept each kind of engine
//! busy: i915 as `drm-engine-<class>: <ns> ns`, xe as GPU cycles
//! (`drm-cycles-<class>` out of `drm-total-cycles-<class>`).

use std::{collections::HashMap, fs, sync::Mutex, time::Instant};

/// What one client has used, by engine class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Client {
    /// Busy nanoseconds (i915)
    ns: HashMap<String, u64>,
    /// Engines of the class; their busy times add up
    capacity: HashMap<String, u64>,
    /// Busy cycles and the GPU's cycles meanwhile (xe)
    cycles: HashMap<String, (u64, u64)>,
}

/// (PCI slot, client ID) → its use.
type Clients = HashMap<(String, String), Client>;

/// Usage in percent per PCI slot, since the previous call (the first call has none).
pub fn usage() -> HashMap<String, f64> {
    static LAST: Mutex<Option<(Instant, Clients)>> = Mutex::new(None);
    let now = Instant::now();
    let clients = clients();
    let mut last = LAST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let usage = match last.as_ref() {
        Some((at, before)) => {
            let elapsed = u64::try_from(now.duration_since(*at).as_nanos()).unwrap_or(u64::MAX);
            compare(before, &clients, elapsed)
        }
        None => HashMap::new(),
    };
    *last = Some((now, clients));
    usage
}

/// Every DRM client this process may look at.
fn clients() -> Clients {
    let mut clients = Clients::new();
    let Ok(procs) = fs::read_dir("/proc") else {
        return clients;
    };
    for proc in procs.flatten() {
        let pid = proc.file_name();
        if !pid
            .to_str()
            .is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit()))
        {
            continue;
        }
        // Other users' programs fail here unless running as root
        let Ok(fds) = fs::read_dir(proc.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if !fs::read_link(fd.path()).is_ok_and(|target| target.starts_with("/dev/dri/")) {
                continue;
            }
            let Ok(info) = fs::read_to_string(proc.path().join("fdinfo").join(fd.file_name()))
            else {
                continue;
            };
            // Files shared between programs (or opened twice) are one client
            if let Some((key, client)) = parse(&info) {
                clients.entry(key).or_insert(client);
            }
        }
    }
    clients
}

/// One fdinfo file: its (PCI slot, client ID) and use. None for other files.
fn parse(info: &str) -> Option<((String, String), Client)> {
    let mut slot = None;
    let mut id = None;
    let mut client = Client::default();
    let mut total_cycles = HashMap::new();
    for line in info.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let number = || value.split_whitespace().next()?.parse::<u64>().ok();
        match key {
            "drm-pdev" => slot = Some(value.to_owned()),
            "drm-client-id" => id = Some(value.to_owned()),
            _ => {
                if let Some(class) = key.strip_prefix("drm-engine-capacity-") {
                    client.capacity.insert(class.to_owned(), number()?);
                } else if let Some(class) = key.strip_prefix("drm-engine-") {
                    client.ns.insert(class.to_owned(), number()?);
                } else if let Some(class) = key.strip_prefix("drm-total-cycles-") {
                    total_cycles.insert(class.to_owned(), number()?);
                } else if let Some(class) = key.strip_prefix("drm-cycles-") {
                    client.cycles.insert(class.to_owned(), (number()?, 0));
                }
            }
        }
    }
    for (class, cycles) in &mut client.cycles {
        cycles.1 = total_cycles.get(class).copied().unwrap_or(0);
    }
    Some(((slot?, id?), client))
}

/// Percent per PCI slot: the busiest class of engine between two looks `elapsed_ns` apart.
/// A client that was not there before counts from zero.
#[allow(clippy::cast_precision_loss)]
fn compare(before: &Clients, now: &Clients, elapsed_ns: u64) -> HashMap<String, f64> {
    // (slot, class) → busy ns, and busy cycles with the GPU's cycles meanwhile
    let mut ns: HashMap<(&str, &str), u64> = HashMap::new();
    let mut capacity: HashMap<(&str, &str), u64> = HashMap::new();
    let mut cycles: HashMap<(&str, &str), (u64, u64)> = HashMap::new();
    for ((slot, id), client) in now {
        let old = before.get(&(slot.clone(), id.clone()));
        for (class, &busy) in &client.ns {
            let was = old.and_then(|o| o.ns.get(class)).copied().unwrap_or(0);
            *ns.entry((slot, class)).or_default() += busy.saturating_sub(was);
            if let Some(&n) = client.capacity.get(class) {
                capacity.insert((slot, class), n.max(1));
            }
        }
        for (class, &(busy, total)) in &client.cycles {
            let Some(&(was_busy, was_total)) = old.and_then(|o| o.cycles.get(class)) else {
                continue;
            };
            let entry = cycles.entry((slot, class)).or_default();
            entry.0 += busy.saturating_sub(was_busy);
            // Every client sees the same GPU clock
            entry.1 = entry.1.max(total.saturating_sub(was_total));
        }
    }
    let mut usage: HashMap<String, f64> = HashMap::new();
    let mut record = |slot: &str, percent: f64| {
        let entry = usage.entry(slot.to_owned()).or_default();
        *entry = entry.max(percent.clamp(0.0, 100.0));
    };
    if elapsed_ns > 0 {
        for (&(slot, class), &busy) in &ns {
            let engines = capacity.get(&(slot, class)).copied().unwrap_or(1);
            record(slot, busy as f64 / (elapsed_ns * engines) as f64 * 100.0);
        }
    }
    for (&(slot, _), &(busy, total)) in &cycles {
        if total > 0 {
            record(slot, busy as f64 / total as f64 * 100.0);
        }
    }
    usage
}

#[cfg(test)]
mod tests {
    use super::{Clients, compare, parse};

    const I915: &str = "pos:\t0\nflags:\t02100002\nmnt_id:\t26\nino:\t1234\n\
drm-driver:\ti915\ndrm-client-id:\t7\ndrm-pdev:\t0000:00:02.0\n\
drm-total-system0:\t4 KiB\ndrm-engine-render:\t1000000 ns\ndrm-engine-copy:\t0 ns\n\
drm-engine-video:\t200 ns\ndrm-engine-capacity-video:\t2\n";

    const XE: &str = "drm-driver:\txe\ndrm-client-id:\t3\ndrm-pdev:\t0000:03:00.0\n\
drm-cycles-rcs:\t500\ndrm-total-cycles-rcs:\t10000\n";

    fn clients(files: &[&str]) -> Clients {
        files.iter().filter_map(|f| parse(f)).collect()
    }

    #[test]
    fn reads_fdinfo() {
        let ((slot, id), client) = parse(I915).unwrap();
        assert_eq!((slot.as_str(), id.as_str()), ("0000:00:02.0", "7"));
        assert_eq!(client.ns.get("render"), Some(&1_000_000));
        assert_eq!(client.ns.get("video"), Some(&200));
        assert_eq!(client.capacity.get("video"), Some(&2));
        let (_, xe) = parse(XE).unwrap();
        assert_eq!(xe.cycles.get("rcs"), Some(&(500, 10000)));
        // Not a DRM file
        assert_eq!(parse("pos:\t0\nflags:\t02\n"), None);
    }

    #[test]
    fn usage_between_looks() {
        let before = clients(&[I915, XE]);
        let now = clients(&[
            &I915
                .replace("1000000 ns", "301000000 ns")
                .replace("video:\t200 ns", "video:\t1000000200 ns"),
            &XE.replace("cycles-rcs:\t500", "cycles-rcs:\t2500")
                .replace("total-cycles-rcs:\t10000", "total-cycles-rcs:\t20000"),
        ]);
        let usage = compare(&before, &now, 1_000_000_000);
        // render 30 %, video 1 s over 2 engines = 50 %: the busiest
        assert!((usage["0000:00:02.0"] - 50.0).abs() < 1e-9);
        // 2000 of 10000 cycles
        assert!((usage["0000:03:00.0"] - 20.0).abs() < 1e-9);
    }
}
