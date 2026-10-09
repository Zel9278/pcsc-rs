//! Running the Linux build on Android (from `adb shell` or Termux). Android is
//! detected at run time, so the same static musl binary works on both.

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
    let output = Command::new(GETPROP).arg(key).output().ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!value.is_empty()).then_some(value)
}

/// The device model, e.g. `SH-M28`; Android's own hostname is just `localhost`.
pub fn device_name() -> Option<String> {
    getprop("ro.product.model")
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
    getprop("ro.build.version.release").map(|release| format!("Android {release}"))
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
        eprintln!("No CA certificates found on this Android device");
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
        eprintln!("Failed to write {}: {e}", path.display());
        return;
    }
    // SAFETY: called at the very start, before any other thread exists.
    unsafe { env::set_var("SSL_CERT_FILE", &path) };
}
