//! Automation hooks owned by the binary: `--exit-after`, `--screenshot*`, startup notes.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use rc_core::prelude::*;
use rc_ui::UiStatus;

/// Capture time when `--screenshot` comes without `--screenshot-at`, in wall-clock seconds.
const DEFAULT_SCREENSHOT_SECS: f32 = 3.0;

/// Wall-clock automation (`Time<Real>`, so pausing never delays it).
pub struct AutomationPlugin;

impl Plugin for AutomationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_flags)
            .add_systems(Last, (take_screenshots, exit_after).chain());
    }
}

fn log_flags(cfg: Res<AutomationConfig>, paths: Res<UserPaths>) {
    info!("assets: {}", paths.assets_dir.display());
    info!("user data: {}", paths.data_dir.display());
    if cfg.unlock_all {
        info!("--unlock-all: everything unlocked (in memory only)");
    }
    for flag in crate::cli::pending_flags(&cfg) {
        warn!("{flag}: not implemented yet");
    }
}

fn take_screenshots(
    cfg: Res<AutomationConfig>,
    time: Res<Time<Real>>,
    mut taken: Local<u32>,
    mut commands: Commands,
) {
    let Some(plan) = &cfg.screenshot else {
        return;
    };
    let first = plan.at.unwrap_or(DEFAULT_SCREENSHOT_SECS);
    let due = match plan.every {
        Some(every) if every > 0.0 => first + every * *taken as f32,
        _ if *taken == 0 => first,
        _ => return,
    };
    if time.elapsed_secs() < due {
        return;
    }
    let path = match plan.every {
        Some(_) => numbered(&plan.path, *taken),
        None => plan.path.clone(),
    };
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        error!("screenshot directory {}: {e}", dir.display());
    }
    info!("screenshot -> {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    *taken += 1;
}

/// `shot.png` → `shot_003.png`.
fn numbered(path: &Path, n: u32) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem}_{n:03}.{}", ext.to_string_lossy()),
        None => format!("{stem}_{n:03}"),
    };
    path.with_file_name(name)
}

/// Quits after `--exit-after` seconds; the exit code is non-zero when the UI never became ready
/// or reported errors, so automated runs catch a broken UI.
fn exit_after(
    cfg: Res<AutomationConfig>,
    time: Res<Time<Real>>,
    ui: Option<Res<UiStatus>>,
    mut done: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(limit) = cfg.exit_after else {
        return;
    };
    if *done || time.elapsed_secs() < limit {
        return;
    }
    *done = true;
    match ui.as_deref() {
        Some(ui) if ui.ready && ui.errors == 0 => {
            info!("--exit-after {limit}: done");
            exit.write(AppExit::Success);
        }
        status => {
            error!("--exit-after {limit}: the UI is not healthy ({status:?})");
            exit.write(AppExit::error());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_screenshots_keep_the_extension() {
        assert_eq!(
            numbered(Path::new("out/shot.png"), 3),
            Path::new("out/shot_003.png")
        );
        assert_eq!(numbered(Path::new("shot"), 12), Path::new("shot_012"));
    }
}
