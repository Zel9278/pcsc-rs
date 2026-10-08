use std::{path::Path, process, thread, time::Duration};

use self_update::{cargo_crate_version, update::UpdateConfig};

use crate::config::OnUpdate;

const REPO_OWNER: &str = "Zel9278";
const REPO_NAME: &str = "pcsc-rs";
/// Long-running clients look for a new release this often, not only at start.
const CHECK_INTERVAL: Duration = Duration::from_hours(6);

/// Replaces the running binary with the latest release when there is one,
/// then follows `PCSC_UPDATED`. Failures are logged and otherwise ignored.
pub fn check(on_update: OnUpdate) {
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
