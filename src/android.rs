//! Running the Linux build on Android (from `adb shell` or Termux) and
//! ARM Linux boards. Android is detected at run time, so the same static musl
//! binary works on both.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

const GETPROP: &str = "/system/bin/getprop";
/// Trusted roots: the updatable Conscrypt module (Android 14+) first, then the system copy.
const CA_DIRS: [&str; 2] = [
    "/apex/com.android.conscrypt/cacerts",
    "/system/etc/security/cacerts",
];

pub fn is_android() -> bool {
    static ANDROID: OnceLock<bool> = OnceLock::new();
    *ANDROID.get_or_init(|| Path::new(GETPROP).exists())
}

fn getprop(key: &str) -> Option<String> {
    let output = crate::cmd::run(Command::new(GETPROP).arg(key)).ok()?;
    let value = output.stdout.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

/// The CPU brand is often empty in sysinfo on old Android releases. Prefer
/// the SoC-specific name, then fall back to the CPU part from `/proc/cpuinfo`.
pub fn cpu_model() -> Option<String> {
    static MODEL: OnceLock<Option<String>> = OnceLock::new();
    MODEL
        .get_or_init(|| {
            let platform = getprop("ro.board.platform")
                .unwrap_or_default()
                .to_ascii_lowercase();
            let soc_model = if platform.starts_with("msm8974") || platform.starts_with("apq8074") {
                Some("Krait 400")
            } else if platform.starts_with("msm8960") || platform.starts_with("msm8930") {
                Some("Krait")
            } else {
                None
            };
            soc_model.map(str::to_owned).or_else(|| cpu_part_model())
        })
        .clone()
}

fn cpu_part_model() -> Option<String> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").ok()?;
    let implementer = cpuinfo
        .lines()
        .find_map(|line| line.strip_prefix("CPU implementer")?.split(':').nth(1))
        .and_then(|value| parse_hex(value.trim()))?;
    let part = cpuinfo
        .lines()
        .find_map(|line| line.strip_prefix("CPU part")?.split(':').nth(1))
        .and_then(|value| parse_hex(value.trim()))?;
    let name = match (implementer, part) {
        (0x41, 0xB76) => "ARM1176JZF-S",
        (0x41, 0xC05) => "Cortex-A5",
        (0x41, 0xC07) => "Cortex-A7",
        (0x41, 0xC08) => "Cortex-A8",
        (0x41, 0xC09) => "Cortex-A9",
        (0x41, 0xC0D) => "Cortex-A12",
        (0x41, 0xC0E) => "Cortex-A17",
        (0x41, 0xC0F) => "Cortex-A15",
        (0x41, 0xD03) => "Cortex-A53",
        (0x41, 0xD04) => "Cortex-A35",
        (0x41, 0xD05) => "Cortex-A55",
        (0x41, 0xD08) => "Cortex-A72",
        (0x41, 0xD09) => "Cortex-A73",
        (0x41, 0xD0A) => "Cortex-A75",
        (0x41, 0xD0B) => "Cortex-A76",
        (0x41, 0xD0D) => "Cortex-A77",
        (0x41, 0xD41) => "Cortex-A78",
        (0x51, 0x06F) => "Krait",
        _ => return None,
    };
    Some(name.into())
}

fn parse_hex(value: &str) -> Option<u32> {
    u32::from_str_radix(value.strip_prefix("0x").unwrap_or(value), 16).ok()
}

/// Old Qualcomm kernels do not expose `gpu_model` in KGSL. Infer the Adreno
/// model from the board platform when the sysfs name is unavailable.
pub fn gpu_model() -> Option<String> {
    static MODEL: OnceLock<Option<String>> = OnceLock::new();
    MODEL
        .get_or_init(|| {
            let platform = getprop("ro.board.platform")?.to_ascii_lowercase();
            let model = if platform.starts_with("msm8974") || platform.starts_with("apq8074") {
                "Adreno 330"
            } else if platform.starts_with("msm8960") {
                "Adreno 225"
            } else if platform.starts_with("msm8930") {
                "Adreno 305"
            } else if platform.starts_with("msm8994") {
                "Adreno 430"
            } else if platform.starts_with("msm8992") {
                "Adreno 418"
            } else if platform.starts_with("msm8996") {
                "Adreno 530"
            } else if platform.starts_with("msm8998") {
                "Adreno 540"
            } else if platform.starts_with("msm8953") {
                "Adreno 506"
            } else {
                return None;
            };
            Some(model.into())
        })
        .clone()
}

/// The device model, e.g. `SH-M28`; Android's own hostname is just `localhost`.
pub fn device_name() -> Option<String> {
    // Fixed for the life of the process; getprop is a process each time
    static NAME: OnceLock<Option<String>> = OnceLock::new();
    NAME.get_or_init(|| getprop("ro.product.model")).clone()
}

/// `adb shell` exports `HOSTNAME` as the device codename (`ro.product.device`,
/// e.g. `Orga`); that is not a name the user chose, so it is ignored.
pub fn is_shell_hostname(name: &str) -> bool {
    is_android() && getprop("ro.product.device").as_deref() == Some(name)
}

/// Storage worth showing: the user data partition and SD cards / USB drives.
/// Android mounts many small system partitions, and `/storage/emulated` (FUSE)
/// is `/data` again.
pub fn is_user_storage(mount_point: &Path) -> bool {
    mount_point == Path::new("/data") || mount_point.starts_with("/mnt/media_rw")
}

/// e.g. `Android 16`.
pub fn os_name() -> Option<String> {
    static NAME: OnceLock<Option<String>> = OnceLock::new();
    NAME.get_or_init(|| {
        getprop("ro.build.version.release").map(|release| format!("Android {release}"))
    })
    .clone()
}

/// `/tmp` does not exist on Android; Termux sets `TMPDIR`, `adb shell` can write here.
fn temp_dir() -> PathBuf {
    env::var_os("TMPDIR").map_or_else(|| PathBuf::from("/data/local/tmp"), PathBuf::from)
}

/// OpenSSL cannot use Android's CA directory (its files are named by an older
/// hash), so join the certificates into one PEM file and point `SSL_CERT_FILE`
/// at it. Must run before any other thread starts, since it sets an
/// environment variable.
pub fn prepare_tls() {
    if !is_android()
        || env::var_os("SSL_CERT_FILE").is_some()
        || env::var_os("SSL_CERT_DIR").is_some()
    {
        return;
    }
    let Some(dir) = CA_DIRS.iter().map(Path::new).find(|d| d.is_dir()) else {
        elog!("No CA certificates found on this Android device");
        return;
    };
    let mut bundle = String::new();
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        // Each file holds one PEM certificate, plus a text dump that OpenSSL skips
        if let Ok(text) = fs::read_to_string(entry.path()) {
            bundle.push_str(&text);
            bundle.push('\n');
        }
    }
    if bundle.is_empty() {
        return;
    }
    let path = temp_dir().join("pcsc-rs-ca.pem");
    if let Err(e) = fs::write(&path, bundle) {
        elog!("Failed to write {}: {e}", path.display());
        return;
    }
    // SAFETY: called at the very start, before any other thread exists.
    unsafe { env::set_var("SSL_CERT_FILE", &path) };
}
