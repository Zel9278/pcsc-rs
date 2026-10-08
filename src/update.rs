use std::{path::Path, process, thread, time::Duration};

use self_update::{
    cargo_crate_version,
    update::{UpdateConfig, UpdateStrategy},
};

use crate::config::OnUpdate;

const REPO_OWNER: &str = "Zel9278";
const REPO_NAME: &str = "pcsc-rs";
/// Long-running clients look for a new release this often, not only at start.
const CHECK_INTERVAL: Duration = Duration::from_hours(6);

/// The package manager that owns the binary, if any. Nix installs it into its
/// read-only store and updates it itself, so replacing it here would fail.
fn managed_by() -> Option<&'static str> {
    let exe = std::env::current_exe().ok()?;
    exe.starts_with("/nix/store").then_some("Nix")
}

/// Replaces the running binary with the latest release when there is one,
/// then follows `PCSC_UPDATED`. Failures are logged and otherwise ignored.
pub fn check(on_update: OnUpdate) {
    if let Some(manager) = managed_by() {
        println!("Installed by {manager}; updates come from {manager}, not from GitHub releases");
        return;
    }
    match update() {
        Ok(Some(bin)) => {
            println!("Updated to the latest release");
            match on_update {
                OnUpdate::Restart => restart(&bin),
                OnUpdate::Terminate => process::exit(0),
                OnUpdate::Nothing => {}
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("Update check failed: {e}"),
    }
}

pub fn spawn_periodic(on_update: OnUpdate) {
    if managed_by().is_some() {
        return;
    }
    let spawned = thread::Builder::new()
        .name("Updater".into())
        .spawn(move || {
            loop {
                thread::sleep(CHECK_INTERVAL);
                check(on_update);
            }
        });
    if let Err(e) = spawned {
        eprintln!("Failed to start the update checker: {e}");
    }
}

/// `Ok(Some(path))` when the binary at `path` was replaced.
fn update() -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("pcsc-rs")
        .show_download_progress(false)
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        // Jump across major versions too (v1 -> v2): the server keeps accepting older clients.
        .update_strategy(UpdateStrategy::Latest)
        .build()?;

    let bin = updater.bin_install_path().to_path_buf();
    let status = updater.update()?;
    Ok(status.is_updated().then_some(bin))
}

fn restart(bin: &Path) -> ! {
    if let Err(e) = process::Command::new(bin).spawn() {
        eprintln!("Failed to restart the program: {e}");
        process::exit(1);
    }
    process::exit(0);
}
