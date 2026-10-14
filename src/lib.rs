#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Without the client, the parts only it uses (TLS setup, PCSC_HIDE parsing, logging) are idle
#![cfg_attr(not(feature = "client"), allow(dead_code, unused_macros))]

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
#[cfg(feature = "client")]
mod client;
mod cmd;
#[cfg(feature = "client")]
mod config;
mod cores;
mod gpu;
mod hide;
mod io;
mod monitor;
mod status;
mod thermal;
#[cfg(feature = "client")]
mod update;

#[cfg(feature = "client")]
pub use config::Config;

/// The system monitor alone, for programs that show this PC's status locally (such as a
/// chat box overlay): it samples the system every second on a thread of its own, as the client
/// does before sending.
pub struct Monitor(monitor::SharedStatus);

impl Monitor {
    /// Starts sampling. The first sample is taken before this returns.
    #[must_use]
    pub fn start() -> Self {
        Self(monitor::spawn(status::Identity::default()))
    }

    /// The latest sample, in the format the client sends to PC Status (see the README):
    /// `cpu.cpus[].cpu`, `ram.free`/`ram.total`, `gpus[]`, `storages[]` and so on.
    #[must_use]
    pub fn status(&self) -> serde_json::Value {
        serde_json::to_value(&**self.0.load()).unwrap_or_default()
    }
}

// With both, tungstenite and self_update pick native-tls, which fails on XP
#[cfg(all(feature = "native-tls", feature = "xp"))]
compile_error!(
    "the xp feature replaces native-tls: build with --no-default-features --features xp"
);

/// `--as-shell-user COMMAND...` (Android): runs COMMAND as the shell user; see
/// [`android::exec_as_shell_user`].
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn exec_as_shell_user(command: impl Iterator<Item = std::ffi::OsString>) -> ! {
    android::exec_as_shell_user(command)
}

#[cfg(feature = "client")]
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
    if !config.unknown_hidden.is_empty() {
        let items: Vec<&str> = hide::ITEMS.iter().map(|(item, _)| *item).collect();
        elog!(
            "PCSC_HIDE: unknown {} (one of: {})",
            config.unknown_hidden.join(", "),
            items.join(", ")
        );
    }
    if !config.hidden.is_empty() {
        log!(
            "Not sent to PC Status: {}",
            config.hidden.names().join(", ")
        );
    }
    let status = monitor::spawn(status::Identity {
        hostname,
        dev: config.dev,
        hidden: config.hidden.clone(),
    });

    log!("This OS is supported!");
    log!("Hello, world! {}", config.uri);

    client::run(config, &status)
}
