//! GPU usage: NVIDIA through `nvidia-smi`; AMD and Intel through sysfs on Linux
//! ([`drm`]), the GPU performance counters on Windows ([`windows`]) and the I/O Registry on
//! macOS ([`macos`]); phone GPUs (Qualcomm Adreno, Samsung's Mali / Xclipse)
//! through sysfs. Anything unexpected means "no GPU".

use std::{
    process::Command,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::status::{GpuData, GpuMemory};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod drm;
#[cfg(target_os = "macos")]
mod macos;
// XP has neither the GPU performance counters nor DXGI
#[cfg(all(target_os = "windows", not(feature = "xp")))]
mod windows;

pub fn get_info() -> Vec<GpuData> {
    #[allow(unused_mut)]
    let mut gpus = nvidia();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if !crate::android::is_android() {
            gpus.extend(drm::get_info());
        }
        if gpus.is_empty() {
            return mobile::get_info().into_iter().collect();
        }
    }
    // The counters cover NVIDIA's GPUs too; nvidia-smi's figures (with the temperature) win
    #[cfg(all(target_os = "windows", not(feature = "xp")))]
    gpus.extend(windows::get_info(!gpus.is_empty()));
    #[cfg(target_os = "macos")]
    gpus.extend(macos::get_info());
    gpus
}

/// When `nvidia-smi` is missing, hangs or fails, try it again only this often. Trying every
/// second costs a process each time, and with glibc the failed child's SIGCHLD can interrupt
/// the socket read (#674).
const NVIDIA_SMI_RETRY: Duration = Duration::from_secs(300);

fn nvidia() -> Vec<GpuData> {
    // Asking a runtime-suspended GPU wakes it up and keeps it awake (hybrid laptops would never
    // power the dGPU down); while it sleeps, report the last memory figures as idle
    static LAST: Mutex<Vec<GpuData>> = Mutex::new(Vec::new());
    #[cfg(target_os = "linux")]
    if nvidia_suspended() {
        let last = LAST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        return last
            .iter()
            .map(|g| GpuData {
                usage: 0.0,
                temperature: None,
                ..g.clone()
            })
            .collect();
    }
    let gpus = query_nvidia();
    LAST.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone_from(&gpus);
    gpus
}

/// True when every NVIDIA GPU on the PCI bus is runtime-suspended (D3); false when there is
/// none or the kernel does not suspend them.
#[cfg(target_os = "linux")]
fn nvidia_suspended() -> bool {
    let Ok(devices) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return false;
    };
    let mut found = false;
    for device in devices.flatten() {
        let path = device.path();
        let read = |name: &str| std::fs::read_to_string(path.join(name)).unwrap_or_default();
        // Display controllers (class 0x03xxxx) from NVIDIA (0x10de)
        if read("vendor").trim() != "0x10de" || !read("class").trim().starts_with("0x03") {
            continue;
        }
        if read("power/runtime_status").trim() != "suspended" {
            return false;
        }
        found = true;
    }
    found
}

fn query_nvidia() -> Vec<GpuData> {
    static MISSING_SINCE: Mutex<Option<Instant>> = Mutex::new(None);
    let mut missing = MISSING_SINCE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if missing.is_some_and(|since| since.elapsed() < NVIDIA_SMI_RETRY) {
        return Vec::new();
    }

    let mut command = Command::new("nvidia-smi");
    command.args([
        "--format=csv,noheader,nounits",
        "--query-gpu=name,utilization.gpu,memory.free,memory.total,temperature.gpu",
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = crate::cmd::run(&mut command);
    *missing = match &output {
        Ok(output) if output.success => None,
        // Missing, hung or failing every time (no driver loaded): try again later
        _ => Some(Instant::now()),
    };
    match output {
        Ok(output) if output.success => parse(&output.stdout),
        _ => Vec::new(),
    }
}

/// One line per GPU, e.g. `NVIDIA GeForce RTX 3050 Ti Laptop GPU, 0, 3784, 4096, 45`.
fn parse(output: &str) -> Vec<GpuData> {
    output.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<GpuData> {
    if line.trim().is_empty() {
        return None;
    }
    // The name itself may contain commas, so read the numbers from the right.
    let mut fields = line.rsplitn(5, ',').map(str::trim);
    // "[N/A]" when the GPU has no sensor
    let temperature = fields.next()?.parse().ok();
    let total = fields.next()?.parse().ok()?;
    let free = fields.next()?.parse().ok()?;
    // "[N/A]" on some GPUs; reported as 0 (the wire format has no "unknown")
    let usage = fields.next()?.parse().unwrap_or(0.0);
    let name = fields.next()?.to_owned();
    Some(GpuData {
        name,
        usage,
        memory: GpuMemory { free, total },
        temperature,
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
        sync::{Mutex, OnceLock},
        time::{Duration, Instant},
    };

    use crate::status::{GpuData, GpuMemory};

    const KGSL: &str = "/sys/class/kgsl/kgsl-3d0";
    const SAMSUNG: &str = "/sys/kernel/gpu";

    /// `dumpsys` takes a few tens of milliseconds; once in a while is enough
    const MEMORY_INTERVAL: Duration = Duration::from_secs(5);

    pub fn get_info() -> Option<GpuData> {
        let (name, usage) = adreno()
            .or_else(samsung)
            .or_else(mediatek_mali)
            .or_else(generic_mali)
            .or_else(powervr)
            .or_else(tegra)
            .or_else(raspberry_pi)
            .or_else(name_only)?;
        // 汎用名（"Arm Mali" など）しか分からないときは、GLES の名前に置き換える
        let name = if matches!(
            name.as_str(),
            "Samsung" | "MediaTek" | "Arm Mali" | "PowerVR" | "Adreno"
        ) {
            renderer_name().unwrap_or(name)
        } else {
            name
        };
        Some(GpuData {
            name,
            usage,
            memory: shared_memory().unwrap_or(GpuMemory { free: 0, total: 0 }),
            // In the thermal zones (the "GPU" temperature)
            temperature: None,
        })
    }

    fn adreno() -> Option<(String, f64)> {
        // Busy time and total time since the previous read (reading resets them)
        let busy = fs::read_to_string(format!("{KGSL}/gpubusy")).ok()?;
        let model = fs::read_to_string(format!("{KGSL}/gpu_model")).unwrap_or_default();
        let model = if model.trim().is_empty() {
            crate::android::gpu_model().unwrap_or_default()
        } else {
            model
        };
        Some((name(model.trim()), usage(&busy)?))
    }

    /// Galaxy phones with Exynos: `gpu_busy` holds the usage in percent (`12 %`).
    fn samsung() -> Option<(String, f64)> {
        let busy = fs::read_to_string(format!("{SAMSUNG}/gpu_busy")).ok()?;
        let model = fs::read_to_string(format!("{SAMSUNG}/gpu_model")).unwrap_or_default();
        let model = model.trim();
        Some((
            if model.is_empty() {
                "Samsung".into()
            } else {
                model.into()
            },
            percent(&busy)?,
        ))
    }
    fn mediatek_mali() -> Option<(String, f64)> {
        // `gpu_utilization` is "<active> <block> <idle>" in percent (`0 0 100`); some kernels
        // only have `total_gpu_utilization` (one number). Root only: closed to the shell user.
        let busy = fs::read_to_string("/sys/kernel/ged/hal/gpu_utilization")
            .or_else(|_| fs::read_to_string("/sys/kernel/ged/hal/total_gpu_utilization"))
            .ok()?;
        Some(("MediaTek".into(), first_percent(&busy)?))
    }

    /// The first number of the text, as a percentage.
    pub(super) fn first_percent(text: &str) -> Option<f64> {
        Some(
            text.split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()?
                .clamp(0.0, 100.0),
        )
    }
    fn generic_mali() -> Option<(String, f64)> {
        let raw_val = fs::read_to_string("/sys/class/misc/mali0/device/utilization")
            .or_else(|_| fs::read_to_string("/sys/devices/platform/mali.0/utilization"))
            .ok()?;

        let val = raw_val.trim().parse::<f64>().ok()?;
        // ドライバーによっては 0-255 で出力されるため、100を超える場合は255で割る
        let usage = if val > 100.0 {
            (val / 255.0) * 100.0
        } else {
            val
        };
        Some(("Arm Mali".into(), usage.clamp(0.0, 100.0)))
    }
    fn powervr() -> Option<(String, f64)> {
        if let Ok(busy) = fs::read_to_string("/sys/class/misc/pvr/load") {
            return Some(("PowerVR".into(), percent(&busy)?));
        }
        None
    }
    /// GPU name from SurfaceFlinger (`GLES: Qualcomm, Adreno (TM) 640, …` → `Adreno 640`).
    /// Read once: the dump is large and the GPU does not change.
    fn renderer_name() -> Option<String> {
        static NAME: OnceLock<Option<String>> = OnceLock::new();
        NAME.get_or_init(|| {
            if !crate::android::is_android() {
                return None;
            }
            let output = crate::cmd::run(Command::new("dumpsys").arg("SurfaceFlinger")).ok()?;
            gles_renderer(&output.stdout)
        })
        .clone()
    }

    pub(super) fn gles_renderer(dump: &str) -> Option<String> {
        let line = dump.lines().find_map(|l| l.trim().strip_prefix("GLES:"))?;
        let renderer = line.split(',').nth(1)?.trim().replace(" (TM)", "");
        (!renderer.is_empty()).then_some(renderer)
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
        let output = crate::cmd::run(Command::new("dumpsys").args(["gpu", "--gpumem"])).ok()?;
        global_total(&output.stdout)
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
    #[allow(clippy::cast_precision_loss)]
    fn tegra() -> Option<(String, f64)> {
        // Tegra 3 only (the one this was checked on)
        if crate::cpuinfo::companion_cores() == 0 {
            return None;
        }
        static LAST: Mutex<Option<(Instant, u64)>> = Mutex::new(None);
        let on_ms: u64 = fs::read_to_string("/sys/kernel/debug/clock/3d/time_on")
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let now = Instant::now();
        let mut last = LAST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let usage = match *last {
            Some((at, before)) => {
                let wall_ms = now.duration_since(at).as_secs_f64() * 1000.0;
                if wall_ms > 0.0 {
                    (on_ms.saturating_sub(before) as f64 / wall_ms * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            }
            None => 0.0,
        };
        *last = Some((now, on_ms));
        Some(("ULP GeForce (Tegra)".to_owned(), usage))
    }
    pub(super) fn pi_gpu(model: &str) -> Option<&'static str> {
        let model = model.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        let rest = model.strip_prefix("Raspberry Pi ")?;
        // The 4, 400, Compute Module 4 and the 5 have newer GPUs; Zero, Zero W, Zero 2 W,
        // 1, 2, 3 and Compute Module 1/3 have VideoCore IV
        let newer = ["4", "5", "Compute Module 4", "Compute Module 5"]
            .iter()
            .any(|p| rest.starts_with(p));
        (!newer).then_some("VideoCore IV")
    }
    fn raspberry_pi() -> Option<(String, f64)> {
        let model = fs::read_to_string("/proc/device-tree/model").ok()?;
        Some((pi_gpu(&model)?.to_owned(), 0.0))
    }
    fn name_only() -> Option<(String, f64)> {
        Some((renderer_name()?, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::status::{GpuData, GpuMemory};

    #[test]
    fn parses_a_gpu() {
        assert_eq!(
            parse("NVIDIA GeForce RTX 3050 Ti Laptop GPU, 7, 3784, 4096, 45\n"),
            vec![GpuData {
                name: "NVIDIA GeForce RTX 3050 Ti Laptop GPU".into(),
                usage: 7.0,
                memory: GpuMemory {
                    free: 3784,
                    total: 4096
                },
                temperature: Some(45.0),
            }]
        );
    }

    #[test]
    fn several_gpus() {
        let gpus = parse("GPU A, 1, 2, 3, 40\nGPU B, 4, 5, 6, 50\n");
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[1].name, "GPU B");
        assert!((gpus[1].usage - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn usage_not_available() {
        let gpus = parse("Tesla K80, [N/A], 11000, 11441, [N/A]\r\n");
        assert!(gpus[0].usage.abs() < f64::EPSILON);
        assert_eq!(gpus[0].memory.total, 11441);
        assert_eq!(gpus[0].temperature, None);
    }

    #[test]
    fn name_with_comma() {
        assert_eq!(
            parse("Vendor, Model X, 50, 100, 200, 60")[0].name,
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
        assert_eq!(parse("GPU, 1, [N/A], [N/A], 40"), []);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn adreno() {
        use super::mobile::{
            first_percent, gles_renderer, global_total, mem_total_kib, name, percent, pi_gpu, usage,
        };
        use super::mobile::{
            first_percent, gles_renderer, global_total, mem_total_kib, name, percent, usage,
        };
        use super::mobile::{gles_renderer, global_total, mem_total_kib, name, percent, usage};
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
        assert_eq!(
            gles_renderer("GLES: Qualcomm, Adreno (TM) 640, OpenGL ES 3.2 V@415.0\n").as_deref(),
            Some("Adreno 640")
        );
        assert_eq!(
            gles_renderer("GLES: ARM, Mali-G78 MC10, OpenGL ES 3.2 v1.r32p1\n").as_deref(),
            Some("Mali-G78 MC10")
        );
        assert_eq!(gles_renderer("nothing"), None);
        assert_eq!(first_percent("0 0 100\n"), Some(0.0));
        assert_eq!(first_percent("35 20 45"), Some(35.0));
        assert_eq!(first_percent("42"), Some(42.0));
        assert_eq!(first_percent("x"), None);
        assert_eq!(
            pi_gpu("Raspberry Pi 3 Model B Rev 1.2\0"),
            Some("VideoCore IV")
        );
        assert_eq!(pi_gpu("Raspberry Pi Zero W Rev 1.1"), Some("VideoCore IV"));
        assert_eq!(
            pi_gpu("Raspberry Pi Zero 2 W Rev 1.0"),
            Some("VideoCore IV")
        );
        assert_eq!(
            pi_gpu("Raspberry Pi 2 Model B Rev 1.1"),
            Some("VideoCore IV")
        );
        assert_eq!(pi_gpu("Raspberry Pi Model B Rev 2"), Some("VideoCore IV"));
        assert_eq!(pi_gpu("Raspberry Pi 4 Model B Rev 1.5"), None);
        assert_eq!(pi_gpu("Raspberry Pi 400 Rev 1.0"), None);
        assert_eq!(pi_gpu("Raspberry Pi 5 Model B Rev 1.0"), None);
        assert_eq!(pi_gpu("Some Other Board"), None);
    }
}
