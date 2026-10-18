//! Automation hooks owned by the binary: `--exit-after`, `--screenshot*`, the host side of
//! `--input-script` (UI readiness for `wait ui`, `shot` screenshots, `exit`), startup notes.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use rc_core::prelude::*;
use rc_input::script::{InputScript, ScriptShot};
use rc_ui::UiStatus;

/// Capture time when `--screenshot` comes without `--screenshot-at`, in wall-clock seconds.
const DEFAULT_SCREENSHOT_SECS: f32 = 3.0;

/// Wall-clock automation (`Time<Real>`, so pausing never delays it).
pub struct AutomationPlugin;

impl Plugin for AutomationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_flags)
            .add_systems(First, ui_ready_for_script)
            // Before OnAppExitSystems so rc_game's save flush sees the AppExit in the same frame.
            .add_systems(
                Last,
                (take_screenshots, script_shots, exit_after)
                    .chain()
                    .before(bevy::app::OnAppExitSystems),
            );
    }
}

fn log_flags(cfg: Res<AutomationConfig>, paths: Res<UserPaths>) {
    info!("assets: {}", paths.assets_dir.display());
    info!("user data: {}", paths.data_dir.display());
    if cfg.unlock_all {
        info!("--unlock-all: everything unlocked (in memory only)");
    }
}

/// `wait ui` in an input script waits for React's `ui.ready`.
fn ui_ready_for_script(ui: Option<Res<UiStatus>>, script: Option<ResMut<InputScript>>) {
    if let (Some(ui), Some(mut script)) = (ui, script)
        && script.ui_ready != ui.ready
    {
        script.ui_ready = ui.ready;
    }
}

/// `shot <name>` → `<user-data-dir>/screenshots/<name>.png`.
fn script_shots(
    mut shots: MessageReader<ScriptShot>,
    paths: Res<UserPaths>,
    mut commands: Commands,
) {
    for ScriptShot(name) in shots.read() {
        let path = paths
            .data_dir
            .join("screenshots")
            .join(format!("{name}.png"));
        capture(&mut commands, path);
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
    capture(&mut commands, path);
    *taken += 1;
}

/// Saves the next frame of the primary window to `path`.
fn capture(commands: &mut Commands, path: PathBuf) {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        error!("screenshot directory {}: {e}", dir.display());
    }
    info!("screenshot -> {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
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

/// Quits after `--exit-after` seconds, or when the input script's `exit` step ran (once its
/// screenshots are saved). The exit code is non-zero when the UI never became ready or reported
/// errors, or the script did not finish, so automated runs catch a broken UI or a stuck flow.
fn exit_after(
    cfg: Res<AutomationConfig>,
    time: Res<Time<Real>>,
    (ui, script): (Option<Res<UiStatus>>, Option<Res<InputScript>>),
    capturing: Query<(), With<Screenshot>>,
    mut done: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let scripted = script.as_ref().is_some_and(|s| s.exit_requested()) && capturing.is_empty();
    let timed_out = cfg
        .exit_after
        .is_some_and(|limit| time.elapsed_secs() >= limit);
    if *done || !(scripted || timed_out) {
        return;
    }
    *done = true;
    let stuck = script.as_ref().and_then(|s| s.pending());
    match (ui.as_deref(), stuck) {
        (_, Some(step)) => {
            error!("--input-script did not finish; it is at {step}");
            exit.write(AppExit::error());
        }
        (Some(ui), None) if ui.ready && ui.errors == 0 => {
            info!("automation done after {:.1} s", time.elapsed_secs());
            exit.write(AppExit::Success);
        }
        (status, None) => {
            error!("automation: the UI is not healthy ({status:?})");
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
