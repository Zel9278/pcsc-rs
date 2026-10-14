//! What a PC's owner keeps off PC Status (`PCSC_HIDE=gpu_usage,temperature`). A hidden
//! item is not sent (or sent as zero where the wire format has no "unknown"), and its
//! name goes in the status's `hidden` list, so the server shows it as not public rather
//! than as a value.

/// Every item that can be hidden, and what it covers.
pub const ITEMS: [(&str, &str); 12] = [
    ("os", "the OS name and version"),
    ("cpu", "CPU usage, per core"),
    (
        "cpu_model",
        "the CPU's name and its cores' kinds and top frequencies",
    ),
    ("memory", "memory and swap"),
    ("storage", "disks"),
    ("gpu", "GPUs altogether"),
    (
        "gpu_usage",
        "GPU usage only (names, memory and temperatures stay)",
    ),
    ("temperature", "temperature sensors, GPUs' too"),
    ("battery", "the battery"),
    ("uptime", "time since boot"),
    ("loadavg", "load average"),
    ("iowait", "IO wait"),
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hidden(Vec<&'static str>);

impl Hidden {
    /// `PCSC_HIDE`: names separated by commas or spaces, in any case. Also returns the
    /// names it does not know, to warn about.
    pub fn parse(value: &str) -> (Self, Vec<String>) {
        let mut items = Vec::new();
        let mut unknown = Vec::new();
        for name in value
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            let name = name.to_ascii_lowercase();
            match ITEMS.iter().find(|(item, _)| *item == name) {
                Some((item, _)) if !items.contains(item) => items.push(*item),
                Some(_) => {}
                None => unknown.push(name),
            }
        }
        // All of the GPU covers its usage
        if items.contains(&"gpu") {
            items.retain(|item| *item != "gpu_usage");
        }
        items.sort_unstable();
        (Self(items), unknown)
    }

    pub fn has(&self, item: &str) -> bool {
        self.0.contains(&item) || (item == "gpu_usage" && self.0.contains(&"gpu"))
    }

    pub fn names(&self) -> &[&'static str] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Hidden;

    #[test]
    fn parses_names() {
        let (hidden, unknown) = Hidden::parse("GPU_usage, temperature  uptime,,nope");
        assert_eq!(hidden.names(), ["gpu_usage", "temperature", "uptime"]);
        assert_eq!(unknown, ["nope"]);
        assert!(hidden.has("gpu_usage"));
        assert!(!hidden.has("gpu"));

        let (gpu, _) = Hidden::parse("gpu_usage gpu gpu");
        assert_eq!(gpu.names(), ["gpu"]);
        assert!(gpu.has("gpu_usage"));

        assert!(Hidden::parse("").0.is_empty());
    }
}
