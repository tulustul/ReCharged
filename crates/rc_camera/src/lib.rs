//! # rc_camera — cameras & the UI host (P)
//!
//! - **Responsibilities:** the UI host rig (exactly one camera carries `UiHost` +
//!   `IsDefaultUiCamera`: the menu camera, the loading camera, the full-window game camera in a 1P
//!   race, or the `OverlayCamera` over the split-screen views, see [`rig`]); the menu showroom
//!   drift; one session-scoped `GameCamera` per local player, re-bound to the slot's racer in
//!   `OnEnter(RaceAttempt)`; split-screen viewports and `PlayerViewports`; chase / far / hood
//!   modes, look-back, obstruction pull-in, Hor+ FOV with speed/boost widening and the 2P clamp,
//!   shake, intro fly-by, finish orbit and spectating.
//! - **Inputs:** `RaceConfig` (local players), `LocalPlayer` racers with their `Visual` proxy
//!   (else their interpolated `PoseHistory`), `VehicleTelemetry`, `CarControl::look_back`,
//!   `RaceProgress`, `Standings`, `Settings` (camera mode, shake, split, FOV), `CycleCamera`,
//!   `CarImpact` / `CarLanded` / `Explosion`, `Teleported`, `MenuBackdrop`, rapier (fixed colliders).
//! - **Outputs:** camera entities (`GameCamera`, `MenuCamera`, `OverlayCamera`, `UiHost`) with
//!   their `Camera`, `Transform` and `Projection`; `PlayerViewports`; the `ClearColor`.
//! - **Must never:** write gameplay state, follow the physics body when a proxy exists, add
//!   post-processing (rc_render decorates the cameras), or despawn the UI host on a restart.

mod director;
mod follow;
mod layout;
mod menu;
mod pose;
mod rig;
mod shake;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use rig::LoadingCamera;

/// Cameras and viewports.
pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        // Also registered by rc_input; registering twice is a no-op.
        app.add_message::<CycleCamera>();
        rig::build(app);
        director::build(app);
        menu::build(app);
        app.add_systems(
            Update,
            (
                menu::drift.run_if(in_state(AppState::Menu)),
                rig::layout_viewports,
                director::tick_intro,
                director::bind_targets,
                director::cycle_views,
                director::finish_orbit,
                shake::add_trauma,
                follow::follow_racers,
            )
                .chain()
                .in_set(CameraSet),
        );
    }
}
