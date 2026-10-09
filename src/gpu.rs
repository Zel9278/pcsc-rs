//! GPU usage: NVIDIA through `nvidia-smi`, and phone GPUs (Qualcomm Adreno,
//! Samsung's Mali / Xclipse) through sysfs. Anything unexpected means "no GPU".

use std::process::Command;

use crate::status::{GpuData, GpuMemory};

pub fn get_info() -> Vec<GpuData> {
    let gpus = nvidia();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if gpus.is_empty() {
        return mobile::get_info().into_iter().collect();
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

/// Phone GPUs, read from sysfs: Qualcomm Adreno (kgsl) and the Mali / Xclipse
/// GPUs in Samsung's Exynos (Samsung's `/sys/kernel/gpu`). The files are
/// readable from `adb shell` (or through Shizuku), not from an ordinary app.
///
/// There is no VRAM: the GPU shares the device's memory. On Android the memory
/// is reported as what the GPU uses out of the whole device memory (`dumpsys
/// gpu`, which works for any GPU); elsewhere it is unknown (free and total 0).
#[cfg(any(target_os = "linux", target_os = "android"))]
mod mobile {
    use std::{
        fs,
        process::Command,
        sync::Mutex,
        time::{Duration, Instant},
    };

    use crate::status::{GpuData, GpuMemory};

    const KGSL: &str = "/sys/class/kgsl/kgsl-3d0";
    const SAMSUNG: &str = "/sys/kernel/gpu";
    /// `dumpsys` takes a few tens of milliseconds; once in a while is enough
    const MEMORY_INTERVAL: Duration = Duration::from_secs(5);

    pub fn get_info() -> Option<GpuData> {
        let (name, usage) = adreno().or_else(samsung)?;
        Some(GpuData {
            name,
            usage,
            memory: shared_memory().unwrap_or(GpuMemory { free: 0, total: 0 }),
        })
    }

    fn adreno() -> Option<(String, f64)> {
        // Busy time and total time since the previous read (reading resets them)
        let busy = fs::read_to_string(format!("{KGSL}/gpubusy")).ok()?;
        let model = fs::read_to_string(format!("{KGSL}/gpu_model")).unwrap_or_default();
        Some((name(model.trim()), usage(&busy)?))
    }

    /// Galaxy phones with Exynos: `gpu_busy` holds the usage in percent (`12 %`).
    fn samsung() -> Option<(String, f64)> {
        let busy = fs::read_to_string(format!("{SAMSUNG}/gpu_busy")).ok()?;
        let model = fs::read_to_string(format!("{SAMSUNG}/gpu_model")).unwrap_or_default();
        let model = model.trim();
        Some((
            if model.is_empty() {
                "GPU".into()
            } else {
                model.into()
            },
            percent(&busy)?,
        ))
    }

    /// The first number in the text as a percentage: `12 %`, `12%`, `12`.
    pub(super) fn percent(text: &str) -> Option<f64> {
        let start = text.find(|c: char| c.is_ascii_digit())?;
        let number: String = text[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        Some(number.parse::<f64>().ok()?.clamp(0.0, 100.0))
    }

    /// GPU memory in MiB as used / whole device memory.
    fn shared_memory() -> Option<GpuMemory> {
        static LAST: Mutex<Option<(Instant, Option<u64>)>> = Mutex::new(None);
        let used = {
            let mut last = LAST.lock().ok()?;
            match *last {
                Some((at, used)) if at.elapsed() < MEMORY_INTERVAL => used,
                _ => {
                    let used = gpu_used_bytes();
                    *last = Some((Instant::now(), used));
                    used
                }
            }
        }?;
        let total = mem_total_kib(&fs::read_to_string("/proc/meminfo").ok()?)? / 1024;
        let used = used / (1024 * 1024);
        Some(GpuMemory {
            free: total.saturating_sub(used),
            total,
        })
    }

    fn gpu_used_bytes() -> Option<u64> {
        if !crate::android::is_android() {
            return None;
        }
        let output = Command::new("dumpsys")
            .args(["gpu", "--gpumem"])
            .output()
            .ok()?;
        global_total(&String::from_utf8_lossy(&output.stdout))
    }

    /// `Global total: 773124096` from `dumpsys gpu --gpumem`.
    pub(super) fn global_total(dump: &str) -> Option<u64> {
        dump.lines()
            .find_map(|l| l.trim().strip_prefix("Global total:"))
            .and_then(|v| v.trim().parse().ok())
    }

    /// `MemTotal:       11629912 kB` from `/proc/meminfo`.
    pub(super) fn mem_total_kib(meminfo: &str) -> Option<u64> {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix("MemTotal:"))
            .and_then(|v| v.split_whitespace().next()?.parse().ok())
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
        use super::mobile::{global_total, mem_total_kib, name, percent, usage};
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
        assert_eq!(percent("12 %\n"), Some(12.0));
        assert_eq!(percent("7%"), Some(7.0));
        assert_eq!(percent(" 99.5\n"), Some(99.5));
        assert_eq!(percent("250 %"), Some(100.0));
        assert_eq!(percent("busy"), None);
        assert_eq!(
            global_total(
                "Memory snapshot for GPU 0:\nGlobal total: 773124096\nProc 461 total: 96882688\n"
            ),
            Some(773_124_096)
        );
        assert_eq!(global_total("Can't find service: gpu\n"), None);
        assert_eq!(
            mem_total_kib("MemTotal:       11629912 kB\nMemFree:  1 kB\n"),
            Some(11_629_912)
        );
    }
}
