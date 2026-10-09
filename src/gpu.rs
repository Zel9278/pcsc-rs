//! GPU usage: NVIDIA through `nvidia-smi`, and Qualcomm Adreno (Android) through
//! the kgsl driver's sysfs files. Anything unexpected means "no GPU".

use std::process::Command;

use crate::status::{GpuData, GpuMemory};

pub fn get_info() -> Vec<GpuData> {
    let gpus = nvidia();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if gpus.is_empty() {
        return adreno::get_info().into_iter().collect();
    }
    gpus
}

fn nvidia() -> Vec<GpuData> {
    let mut command = Command::new("nvidia-smi");
    command.args([
        "--format=csv,noheader,nounits",
        "--query-gpu=name,utilization.gpu,memory.free,memory.total",
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    match command.output() {
        Ok(output) if output.status.success() => parse(&String::from_utf8_lossy(&output.stdout)),
        _ => Vec::new(),
    }
}

/// One line per GPU, e.g. `NVIDIA GeForce RTX 3050 Ti Laptop GPU, 0, 3784, 4096`.
fn parse(output: &str) -> Vec<GpuData> {
    output.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<GpuData> {
    if line.trim().is_empty() {
        return None;
    }
    // The name itself may contain commas, so read the numbers from the right.
    let mut fields = line.rsplitn(4, ',').map(str::trim);
    let total = fields.next()?.parse().ok()?;
    let free = fields.next()?.parse().ok()?;
    // "[N/A]" on some GPUs; reported as 0 (the wire format has no "unknown")
    let usage = fields.next()?.parse().unwrap_or(0.0);
    let name = fields.next()?.to_owned();
    Some(GpuData {
        name,
        usage,
        memory: GpuMemory { free, total },
    })
}

/// Adreno GPUs on Qualcomm phones. The files are readable from `adb shell` (or
/// through Shizuku), not from an ordinary app. The memory is shared with the
/// CPU, so there is no VRAM to report.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod adreno {
    use std::fs;

    use crate::status::{GpuData, GpuMemory};

    const DIR: &str = "/sys/class/kgsl/kgsl-3d0";

    pub fn get_info() -> Option<GpuData> {
        // Busy time and total time since the previous read (reading resets them)
        let busy = fs::read_to_string(format!("{DIR}/gpubusy")).ok()?;
        let model = fs::read_to_string(format!("{DIR}/gpu_model")).unwrap_or_default();
        Some(GpuData {
            name: name(model.trim()),
            usage: usage(&busy)?,
            memory: GpuMemory { free: 0, total: 0 },
        })
    }

    /// `"  23464 1006294"` → 2.33. A powered-down GPU reports `0 0`, which is idle.
    #[allow(clippy::cast_precision_loss)]
    pub(super) fn usage(gpubusy: &str) -> Option<f64> {
        let mut fields = gpubusy.split_whitespace().map(str::parse::<u64>);
        let busy = fields.next()?.ok()?;
        let total = fields.next()?.ok()?;
        Some(if total == 0 {
            0.0
        } else {
            (busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
        })
    }

    /// `Adreno752v2` → `Adreno 752v2`.
    pub(super) fn name(model: &str) -> String {
        match model.strip_prefix("Adreno") {
            Some(rest) if !rest.is_empty() && !rest.starts_with(' ') => format!("Adreno {rest}"),
            _ if model.is_empty() => "Adreno".into(),
            _ => model.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::status::{GpuData, GpuMemory};

    #[test]
    fn parses_a_gpu() {
        assert_eq!(
            parse("NVIDIA GeForce RTX 3050 Ti Laptop GPU, 7, 3784, 4096\n"),
            vec![GpuData {
                name: "NVIDIA GeForce RTX 3050 Ti Laptop GPU".into(),
                usage: 7.0,
                memory: GpuMemory {
                    free: 3784,
                    total: 4096
                },
            }]
        );
    }

    #[test]
    fn several_gpus() {
        let gpus = parse("GPU A, 1, 2, 3\nGPU B, 4, 5, 6\n");
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[1].name, "GPU B");
        assert!((gpus[1].usage - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn usage_not_available() {
        let gpus = parse("Tesla K80, [N/A], 11000, 11441\r\n");
        assert!(gpus[0].usage.abs() < f64::EPSILON);
        assert_eq!(gpus[0].memory.total, 11441);
    }

    #[test]
    fn name_with_comma() {
        assert_eq!(
            parse("Vendor, Model X, 50, 100, 200")[0].name,
            "Vendor, Model X"
        );
    }

    #[test]
    fn errors_mean_no_gpu() {
        assert_eq!(parse(""), []);
        assert_eq!(
            parse("NVIDIA-SMI has failed because it couldn't communicate with the NVIDIA driver."),
            []
        );
        assert_eq!(parse("GPU, 1, [N/A], [N/A]"), []);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn adreno() {
        use super::adreno::{name, usage};
        assert_eq!(
            usage("  23464 1006294"),
            Some(23464.0 / 1_006_294.0 * 100.0)
        );
        assert_eq!(usage("      0       0"), Some(0.0));
        assert_eq!(usage(""), None);
        assert_eq!(usage("x 1"), None);
        assert_eq!(name("Adreno752v2"), "Adreno 752v2");
        assert_eq!(name("Adreno 740"), "Adreno 740");
        assert_eq!(name(""), "Adreno");
    }
}
