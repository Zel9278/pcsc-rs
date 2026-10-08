#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod config;
mod gpu;
mod monitor;
mod status;
mod update;

use std::thread;

pub use config::Config;

/// Checks for an update, starts the system monitor, keeps a connection to the
/// PC Status server, and never returns.
pub fn start(config: &Config) -> ! {
    update::check(config.on_update);
    update::spawn_periodic(config.on_update);

    let status = monitor::spawn(config.hostname.clone());

    println!("This OS is supported!");
    println!("Hello, world! {}", config.uri);

    let _client = client::connect(config, &status);

    // The socket.io client runs on its own threads; this one just stays alive.
    loop {
        thread::park();
    }
}
