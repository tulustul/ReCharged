//! # rc_camera — cameras & the UI host (P)
//!
//! - **Responsibilities:** the UI host rig (exactly one camera marked `IsDefaultUiCamera`: the
//!   full-window 3D camera in single view, an `OverlayCamera` in split-screen); menu/loading
//!   cameras; one session-scoped `GameCamera` per local player, re-bound to the slot's racer in
//!   `OnEnter(RaceAttempt)`; split-screen viewports (`PlayerViewports`); camera modes, look-back,
//!   obstruction pull-in, FOV, shake, intro fly-by, finish orbit, spectate.
//! - **Inputs:** `LocalPlayers`, `LocalPlayer` racers and their `Visual` proxies, `Settings`,
//!   `Track`, `CarImpact` / `CarLanded` / `Explosion`, `MenuBackdrop`.
//! - **Outputs:** camera entities (`GameCamera`, `MenuCamera`, `OverlayCamera`, `UiHost`),
//!   `PlayerViewports`.
//! - **Must never:** write gameplay state, follow physics bodies when a proxy exists, or despawn
//!   the UI host on a restart.
//!
//! Status (T01): a single full-window 3D backdrop camera hosts the UI in every state; T04 builds
//! the real rig (`docs/tasks/T04-input-camera.md`).

use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;
use rc_core::prelude::*;

/// Backdrop colour until rc_render provides a scene (dark navy, sRGB).
const BACKDROP: Color = Color::srgb(0.035, 0.05, 0.1);

/// Cameras and viewports.
pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui_host);
    }
}

// ponytail: one persistent single-view UI host for menus, loading and races; T04 adds the
// showroom/game cameras and the split-screen overlay.
fn spawn_ui_host(mut commands: Commands) {
    commands.spawn((
        Name::new("UiHost camera"),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
        Transform::from_xyz(0.0, 6.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
        UiHost,
        IsDefaultUiCamera,
    ));
}
