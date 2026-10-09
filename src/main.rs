#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, io::Write, path::Path, process};

/// Like eprintln!, without panicking when stderr is closed or broken.
fn report(message: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{message}");
}

/// Loads `.env` from the working directory. The environment wins, except for `HOSTNAME`:
/// shells (Fedora's /etc/profile, Git Bash), Docker and adb export their own HOSTNAME, and
/// the name set in `.env` is there to replace the machine's real one.
fn load_env_file() {
    let path = Path::new(".env");
    if !path.exists() {
        return;
    }
    let entries = match dotenvy::from_path_iter(path) {
        Ok(entries) => entries,
        Err(e) => return report(&format!("Failed to read .env: {e}")),
    };
    for entry in entries {
        match entry {
            Ok((key, value)) => {
                if key == "HOSTNAME" || env::var_os(&key).is_none() {
                    // SAFETY: nothing else runs yet; no other thread reads the environment
                    unsafe { env::set_var(key, value) };
                }
            }
            Err(e) => {
                // The parser cannot go on after a bad line; the lines after it are not read
                report(&format!(
                    "Failed to read .env (it is read only up to this line): {e}"
                ));
                break;
            }
        }
    }
}

fn main() {
    load_env_file();

    if !sysinfo::IS_SUPPORTED_SYSTEM {
        report("This OS isn't supported (yet?).");
        process::exit(95);
    }

    let Some(config) = pcsc_rs::Config::from_env() else {
        report("The environment variable Password (PASS) is not specified.");
        process::exit(95);
    };

    pcsc_rs::start(&config);
}
