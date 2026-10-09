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
mod config;
mod gpu;
mod io;
mod monitor;
mod status;
mod thermal;
mod update;

pub use config::Config;

// With both, tungstenite and self_update pick native-tls, which fails on XP
#[cfg(all(feature = "native-tls", feature = "xp"))]
compile_error!(
    "the xp feature replaces native-tls: build with --no-default-features --features xp"
);

/// Checks for an update, starts the system monitor, keeps a connection to the
/// PC Status server, and never returns.
pub fn start(config: &Config) -> ! {
    // Before update::check, whose HTTP client starts threads
    #[cfg(any(target_os = "linux", target_os = "android"))]
    android::prepare_tls();

    update::check(config.on_update);
    update::spawn_periodic(config.on_update);

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
