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
    let output = crate::cmd::run(Command::new(GETPROP).arg(key)).ok()?;
    let value = output.stdout.trim().to_owned();
    (!value.is_empty()).then_some(value)
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

/// The shell user, and the groups adbd gives it. inet (3003) is needed for name lookups:
/// netd's DNS socket is root:inet 0660.
const SHELL_UID: u32 = 2000;
const SHELL_GROUPS: [u32; 15] = [
    2000, 3003, 1004, 1007, 1011, 1015, 1028, 1078, 1079, 3001, 3002, 3006, 3009, 3011, 3012,
];

/// `pcsc-rs --as-shell-user COMMAND...`: the Android app's start when it has root (root
/// mode, or Shizuku started by root). Root runs only this copy of the client, from the APK
/// (which the shell user cannot change); it becomes the shell user with adb's groups and
/// runs COMMAND, the loop that keeps the client running. So nothing the shell user can
/// write is run as root, and the client and its files are the same in every mode.
pub fn exec_as_shell_user(mut command: impl Iterator<Item = std::ffi::OsString>) -> ! {
    use std::os::unix::process::CommandExt;
    let fail = |message: String| -> ! {
        elog!("{message}");
        std::process::exit(1)
    };
    if !is_android() {
        fail("--as-shell-user is for Android".to_owned());
    }
    let Some(program) = command.next() else {
        fail("--as-shell-user needs a command".to_owned());
    };
    // SAFETY: plain libc calls; this process runs nothing else.
    unsafe {
        if libc::getuid() == 0
            && (libc::setgroups(SHELL_GROUPS.len(), SHELL_GROUPS.as_ptr()) != 0
                || libc::setgid(SHELL_UID) != 0
                || libc::setuid(SHELL_UID) != 0)
        {
            fail(format!(
                "Could not switch to the shell user: {}",
                std::io::Error::last_os_error()
            ));
        }
        if libc::getuid() != SHELL_UID {
            fail(format!(
                "--as-shell-user: running as uid {}, not root or the shell user",
                libc::getuid()
            ));
        }
    }
    let error = std::process::Command::new(&program).args(command).exec();
    fail(format!(
        "Failed to run {}: {error}",
        Path::new(&program).display()
    ))
}

/// Where the CA bundle goes: next to the client first (its own directory, so a bundle a
/// different user left elsewhere cannot get in the way), then the temporary directory.
fn ca_bundle_dirs() -> Vec<PathBuf> {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    exe_dir.into_iter().chain([temp_dir()]).collect()
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
    for dir in ca_bundle_dirs() {
        let path = dir.join("pcsc-rs-ca.pem");
        match fs::write(&path, &bundle) {
            Ok(()) => {}
            // A bundle another user wrote (e.g. root) still has the same roots
            Err(_)
                if fs::metadata(&path).is_ok_and(|m| m.len() > 0)
                    && fs::File::open(&path).is_ok() => {}
            Err(e) => {
                elog!("Failed to write {}: {e}", path.display());
                continue;
            }
        }
        // SAFETY: called at the very start, before any other thread exists.
        unsafe { env::set_var("SSL_CERT_FILE", &path) };
        return;
    }
}
