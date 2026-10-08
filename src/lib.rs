#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod config;
mod gpu;
mod io;
mod monitor;
mod status;
mod update;

pub use config::Config;

/// Checks for an update, starts the system monitor, keeps a connection to the
/// PC Status server, and never returns.
pub fn start(config: &Config) -> ! {
    update::check(config.on_update);
    update::spawn_periodic(config.on_update);

    let status = monitor::spawn(status::Identity {
        hostname: config.hostname.clone(),
        dev: config.dev,
    });

    println!("This OS is supported!");
    println!("Hello, world! {}", config.uri);

    client::run(config, &status)
}
