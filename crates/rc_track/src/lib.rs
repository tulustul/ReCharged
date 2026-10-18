//! # rc_track — tracks (L1)
//!
//! - **Responsibilities:** track data (`assets/data/tracks/<id>.ron`) → spline, geometry buffers,
//!   colliders, checkpoints, respawn points, grid, item rows, racing line & speed profile, ground
//!   areas and props (`assets/data/props.ron`); the `Track` query resource; variants (reverse,
//!   mirror); resetting dynamic props to their spawn poses in `OnEnter(RacePhase::Setup)`.
//! - **Inputs:** `RaceConfig::variant`, `UserPaths` (data files), the session lifecycle states.
//! - **Outputs:** `Track`, `TrackSummary`, session-scoped collider entities (`TrackPiece`,
//!   `Surface`, `Prop`), `Teleported` (prop resets), `LoadingDone("track")`.
//! - **Must never:** depend on presentation or flow crates, spawn visuals (rc_render builds meshes
//!   from the geometry), touch car bodies, or keep session state past `OnEnter(Unloading)`.
//!
//! Status: skeleton (T01); implemented by T02 (`docs/tasks/T02-track.md`).

use bevy::prelude::*;
use rc_core::prelude::*;

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "track";

/// Builds the session's track while `AppState::Loading`.
pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_systems(OnEnter(AppState::Loading), report_loaded);
    }
}

// ponytail: stub — reports "loaded" at once so the gate opens; T02 builds the track first.
fn report_loaded(mut done: MessageWriter<LoadingDone>) {
    done.write(LoadingDone(LOADING_KEY));
}
