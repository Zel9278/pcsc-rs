#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::Path, process};

fn main() {
    if Path::new(".env").exists()
        && let Err(e) = dotenvy::dotenv()
    {
        eprintln!("Failed to read .env: {e}");
    }

    if !sysinfo::IS_SUPPORTED_SYSTEM {
        println!("This OS isn't supported (yet?).");
        process::exit(95);
    }

    let Some(config) = pcsc_rs::Config::from_env() else {
        println!("The environment variable Password (PASS) is not specified.");
        process::exit(95);
    };

    pcsc_rs::start(&config);
}
