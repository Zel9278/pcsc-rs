//! GPUs on macOS (Apple Silicon, Intel and AMD) from the `PerformanceStatistics`
//! that each `IOAccelerator` publishes in the I/O Registry (`ioreg`).
//!
//! - Usage: `Device Utilization %`.
//! - Memory: Apple Silicon (and Intel) GPUs share the system memory, so they report
//!   what they use of it (`In use system memory`) against the whole of it; AMD GPUs
//!   report their VRAM (`vramUsedBytes`, `vramFreeBytes`).
//! - Name: Apple Silicon's accelerator has a `model`; Intel's and AMD's do not, so
//!   theirs comes once from `system_profiler`.

use std::{process::Command, sync::OnceLock};

use crate::status::{GpuData, GpuMemory};

const MIB: u64 = 1024 * 1024;

pub fn get_info() -> Vec<GpuData> {
    let Ok(output) = crate::cmd::run(Command::new("ioreg").args([
        "-r",
        "-d",
        "1",
        "-w",
        "0",
        "-c",
        "IOAccelerator",
    ])) else {
        return Vec::new();
    };
    parse(&output.stdout)
        .into_iter()
        .map(|accelerator| {
            let memory = match (
                accelerator.stat("vramUsedBytes"),
                accelerator.stat("vramFreeBytes"),
            ) {
                (Some(used), Some(free)) => GpuMemory {
                    free: free / MIB,
                    total: (used + free) / MIB,
                },
                _ => match (accelerator.stat("In use system memory"), ram_mib()) {
                    (Some(used), Some(total)) => GpuMemory {
                        free: total.saturating_sub(used / MIB),
                        total,
                    },
                    _ => GpuMemory { free: 0, total: 0 },
                },
            };
            #[allow(clippy::cast_precision_loss)]
            let usage = accelerator
                .stat("Device Utilization %")
                .map_or(0.0, |p| (p as f64).clamp(0.0, 100.0));
            GpuData {
                name: name(&accelerator),
                usage,
                memory,
                temperature: None,
                usage_known: true,
            }
        })
        .collect()
}

#[derive(Debug, Default, PartialEq)]
struct Accelerator {
    class: String,
    model: Option<String>,
    stats: Vec<(String, u64)>,
}

impl Accelerator {
    fn stat(&self, key: &str) -> Option<u64> {
        self.stats.iter().find(|(k, _)| k == key).map(|&(_, v)| v)
    }
}

fn name(accelerator: &Accelerator) -> String {
    if let Some(model) = &accelerator.model {
        return model.clone();
    }
    let vendor = if accelerator.class.starts_with("Intel") {
        "Intel"
    } else if accelerator.class.starts_with("AMD") {
        "AMD"
    } else {
        return accelerator.class.clone();
    };
    displays()
        .iter()
        .find(|model| model.contains(vendor) || (vendor == "AMD" && model.contains("Radeon")))
        .cloned()
        .unwrap_or_else(|| format!("{vendor} GPU"))
}

/// The GPUs' names from `system_profiler` (`Intel UHD Graphics 630`), once: it takes a while.
fn displays() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        crate::cmd::run(Command::new("system_profiler").args(["SPDisplaysDataType", "-json"]))
            .ok()
            .and_then(|output| serde_json::from_str::<serde_json::Value>(&output.stdout).ok())
            .and_then(|json| {
                Some(
                    json.get("SPDisplaysDataType")?
                        .as_array()?
                        .iter()
                        .filter_map(|gpu| Some(gpu.get("sppci_model")?.as_str()?.to_owned()))
                        .collect(),
                )
            })
            .unwrap_or_default()
    })
}

fn ram_mib() -> Option<u64> {
    static RAM: OnceLock<Option<u64>> = OnceLock::new();
    *RAM.get_or_init(|| {
        let output = crate::cmd::run(Command::new("sysctl").args(["-n", "hw.memsize"])).ok()?;
        Some(output.stdout.trim().parse::<u64>().ok()? / MIB)
    })
}

/// The accelerators in `ioreg -r -d 1 -w 0 -c IOAccelerator`.
fn parse(ioreg: &str) -> Vec<Accelerator> {
    let mut accelerators: Vec<Accelerator> = Vec::new();
    for line in ioreg.lines() {
        let line = line.trim();
        if line.starts_with("+-o ") {
            accelerators.push(Accelerator::default());
            continue;
        }
        let Some(current) = accelerators.last_mut() else {
            continue;
        };
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        match key {
            "\"IOClass\"" => unquote(value).clone_into(&mut current.class),
            // A string, or data holding one: <"AMD Radeon Pro 5500M">
            "\"model\"" => {
                let model = unquote(value.trim_start_matches('<').trim_end_matches('>'));
                if !model.is_empty() {
                    current.model = Some(model.to_owned());
                }
            }
            "\"PerformanceStatistics\"" => current.stats = parse_stats(value),
            _ => {}
        }
    }
    accelerators
        .into_iter()
        .filter(|a| a.stat("Device Utilization %").is_some())
        .collect()
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches('"')
}

/// `{"Device Utilization %"=12,"In use system memory"=355778560}` → its numbers.
fn parse_stats(value: &str) -> Vec<(String, u64)> {
    let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
    inner
        .split(',')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((unquote(key).to_owned(), value.trim().parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse;

    const IOREG: &str = r#"+-o AGXAcceleratorG14X  <class AGXAcceleratorG14X, id 0x1000003f5, registered, matched, active, busy 0 (0 ms), retain 140>
    {
      "IOClass" = "AGXAcceleratorG14X"
      "model" = "Apple M2"
      "PerformanceStatistics" = {"In use system memory (driver)"=0,"Alloc system memory"=1599913984,"Tiler Utilization %"=3,"Renderer Utilization %"=4,"Device Utilization %"=5,"In use system memory"=355778560}
      "gpu-core-count" = 10
    }

+-o AMDRadeonX6000_AMDNavi14GraphicsAccelerator  <class AMDRadeonX6000_AMDNavi14GraphicsAccelerator, id 0x1000004a1, registered, matched, active, busy 0 (0 ms), retain 30>
    {
      "IOClass" = "AMDRadeonX6000_AMDNavi14GraphicsAccelerator"
      "PerformanceStatistics" = {"Device Utilization %"=27,"vramUsedBytes"=1073741824,"vramFreeBytes"=3221225472}
    }
+-o SomethingElse  <class SomethingElse>
    {
      "IOClass" = "SomethingElse"
    }
"#;

    #[test]
    fn reads_accelerators() {
        let gpus = parse(IOREG);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].model.as_deref(), Some("Apple M2"));
        assert_eq!(gpus[0].stat("Device Utilization %"), Some(5));
        assert_eq!(gpus[0].stat("In use system memory"), Some(355_778_560));
        assert_eq!(gpus[1].class, "AMDRadeonX6000_AMDNavi14GraphicsAccelerator");
        assert_eq!(gpus[1].model, None);
        assert_eq!(gpus[1].stat("vramFreeBytes"), Some(3_221_225_472));
    }
}
