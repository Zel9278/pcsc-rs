#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// println! that never panics: a hung-up terminal (EIO), a closed pipe (EPIPE) or a full
/// disk behind the log (ENOSPC) must not stop the client.
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout().lock(), $($arg)*);
    }};
}

/// eprintln! that never panics (see `log!`).
macro_rules! elog {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr().lock(), $($arg)*);
    }};
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod android;
mod battery;
mod client;
mod cmd;
mod config;
mod gpu;
mod io;
#[cfg(all(target_os = "linux", target_arch = "arm", target_env = "musl"))]
mod legacy_dns;
mod monitor;
mod status;
mod thermal;
#[cfg(not(feature = "android-legacy"))]
mod update;

pub use config::Config;

// Rustls replaces native-tls for both targets that cannot use the system OpenSSL.
#[cfg(all(
    feature = "native-tls",
    any(feature = "xp", feature = "android-legacy")
))]
compile_error!(
    "xp and android-legacy replace native-tls: build with --no-default-features and one target feature"
);

/// `--as-shell-user COMMAND...` (Android): runs COMMAND as the shell user; see
/// [`android::exec_as_shell_user`].
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn exec_as_shell_user(command: impl Iterator<Item = std::ffi::OsString>) -> ! {
    android::exec_as_shell_user(command)
}

/// Checks for an update, starts the system monitor, keeps a connection to the
/// PC Status server, and never returns.
pub fn start(config: &Config) -> ! {
    // Before the updater starts, whose HTTP client starts threads.
    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        feature = "native-tls"
    ))]
    android::prepare_tls();

    #[cfg(not(feature = "android-legacy"))]
    {
        update::check(config.on_update);
        update::spawn_periodic(config.on_update);
    }

    let hostname = config.hostname.clone();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let hostname = hostname.filter(|h| !android::is_shell_hostname(h));
    let status = monitor::spawn(status::Identity {
        hostname,
        dev: config.dev,
    });

    log!("This OS is supported!");
    log!("Hello, world! {}", config.uri);

    client::run(config, &status)
}
