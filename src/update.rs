use std::{
    path::Path,
    process,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use self_update::{
    cargo_crate_version,
    update::{UpdateConfig, UpdateStrategy},
};

use crate::config::OnUpdate;

const REPO_OWNER: &str = "Zel9278";
const REPO_NAME: &str = "pcsc-rs";
/// Long-running clients look for a new release this often, not only at start.
const CHECK_INTERVAL: Duration = Duration::from_hours(6);
/// For the GitHub API call and the download
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Set once this process has replaced its binary but keeps running (`PCSC_UPDATED=nothing`):
/// it still reports the old version, so every later check would install the same release again.
static INSTALLED: AtomicBool = AtomicBool::new(false);

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
        log!("Installed by {manager}; updates come from {manager}, not from GitHub releases");
        return;
    }
    if INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    match update() {
        Ok(Some((bin, version))) => {
            log!("Updated to {version}");
            match on_update {
                OnUpdate::Restart => restart(&bin),
                OnUpdate::Terminate => process::exit(0),
                OnUpdate::Nothing => {
                    log!("The new version runs after a restart");
                    INSTALLED.store(true, Ordering::Relaxed);
                }
            }
        }
        Ok(None) => {}
        Err(e) => elog!("Update check failed: {e}"),
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
        elog!("Failed to start the update checker: {e}");
    }
}

/// `Ok(Some((path, version)))` when the binary at `path` was replaced.
fn update() -> Result<Option<(std::path::PathBuf, String)>, Box<dyn std::error::Error>> {
    log!("Checking for a new release ({})", self_update::get_target());
    let updater = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("pcsc-rs")
        // self_update prints with println!, which panics on a broken stdout
        .show_output(false)
        .show_download_progress(false)
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .timeout(REQUEST_TIMEOUT)
        // Only this build's own asset. The default matching falls back to "same CPU and OS"
        // and takes aarch64-linux-android for aarch64-unknown-linux-musl (and the other way
        // round) while a release is still uploading its assets.
        .asset_matcher(|assets| {
            assets
                .iter()
                .find(|a| is_asset_for(a.name(), self_update::get_target()))
                .cloned()
        })
        // Jump across major versions too (v1 -> v2): the server keeps accepting older clients.
        .update_strategy(UpdateStrategy::Latest)
        .build()?;

    let bin = updater.bin_install_path().to_path_buf();
    let status = updater.update()?;
    Ok(status
        .is_updated()
        .then(|| (bin, status.version().to_owned())))
}

/// Release assets are `pcsc-rs-v<version>-<target>` (`.exe` on Windows).
fn is_asset_for(name: &str, target: &str) -> bool {
    let name = name.strip_suffix(".exe").unwrap_or(name);
    name.starts_with("pcsc-rs-v") && name.ends_with(&format!("-{target}"))
}

fn restart(bin: &Path) -> ! {
    if let Err(e) = process::Command::new(bin).spawn() {
        elog!("Failed to restart the program: {e}");
        process::exit(1);
    }
    process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::is_asset_for;

    #[test]
    fn picks_only_this_targets_asset() {
        let musl = "aarch64-unknown-linux-musl";
        let android = "aarch64-linux-android";
        assert!(is_asset_for(
            "pcsc-rs-v2.6.0-aarch64-unknown-linux-musl",
            musl
        ));
        assert!(!is_asset_for("pcsc-rs-v2.6.0-aarch64-linux-android", musl));
        assert!(is_asset_for(
            "pcsc-rs-v2.6.0-aarch64-linux-android",
            android
        ));
        assert!(!is_asset_for(
            "pcsc-rs-v2.6.0-aarch64-unknown-linux-musl",
            android
        ));
        assert!(!is_asset_for("pcsc-rs-v2.6.0-android.apk", android));
        assert!(is_asset_for(
            "pcsc-rs-v2.6.0-x86_64-pc-windows-msvc.exe",
            "x86_64-pc-windows-msvc"
        ));
    }
}
