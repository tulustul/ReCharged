//! # rc_track — tracks (L1)
//!
//! - **Responsibilities:** track data (`assets/maps/<id>/track.ron`, [`TrackDef`]) → variants
//!   (reverse, mirror) → centreline spline with banked frames → geometry buffers
//!   ([`TrackGeometry`]) → checkpoints, respawn points, grid, item rows, racing line & speed
//!   profile, ground areas, props (`assets/data/props/*.ron`, [`PropCatalog`]); the [`Track`]
//!   query resource; session colliders; resetting dynamic props in `OnEnter(RacePhase::Setup)`.
//! - **Inputs:** `RaceConfig::variant`, `UserPaths::assets_dir`, [`TrackDraft`] (rc_editor), the
//!   session lifecycle states.
//! - **Outputs:** [`Track`], `TrackSummary`, session-scoped collider entities (`TrackPiece`,
//!   `Surface`, `Prop` + `Interpolated` for dynamic props) on rc_core collision layers,
//!   `Teleported` (prop resets), `LoadingDone("track")` / `LoadingFailed`.
//! - **Must never:** depend on presentation or flow crates, spawn visuals (rc_render builds
//!   meshes from [`Track::geometry`]), touch car bodies, or keep session state past
//!   `OnEnter(Unloading)`.
//!
//! Everything but the plugin is pure and usable without an `App` ([`Track::load`]); the
//! authoring guide is `docs/design/rc_track.md`, the plot tool
//! `cargo run -p rc_track --example plot -- <track> [--rev] [--mirror] [--out <dir>]`.

mod analysis;
mod build;
mod decor;
mod def;
mod geometry;
mod layout;
mod mesh;
mod place;
mod props;
mod racing;
mod spawn;
mod spline;
mod track;
mod turns;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use analysis::{MAX_BANK_GUIDE_DEG, MAX_CHOKEPOINTS, MAX_SLOPE_DEG, STACK_CLEARANCE_U};
pub use build::{KILL_FLOOR_DEPTH_U, WALL_THICKNESS_U, variant_name};
pub use decor::{DECOR_FILE, DecorItem, decor_to_ron, load_decor, parse_decor};
pub use def::*;
pub use geometry::{GroundMesh, RibbonMesh, TrackGeometry};
pub use mesh::MeshData;
pub use place::{
    CHECKPOINT_SPACING_U, GRID_COLUMN_U, GRID_DROP_U, GRID_FRONT_U, GRID_ROW_U, GRID_STAGGER_U,
    ITEM_EDGE_MARGIN_U, ITEM_HEIGHT_U, JUMP_RUNUP_U, LANDING_RESPAWN_U, MIN_CHECKPOINTS,
    RESPAWN_DROP_U, RESPAWN_SPACING_U,
};
pub use props::{MAX_DYNAMIC_PROPS, PROPS_DIR, Primitive, PropCatalog, PropDef};
pub use racing::{LINE_MARGIN_U, LOCKED_OFFSET_U, SpeedModel};
pub use spline::{MIN_UP_Y, SAMPLE_SPACING_U};
pub use track::*;
pub use turns::{MERGE_U, MIN_ANGLE_DEG, MIN_TURN_RATE, SMOOTH_U};

/// What the session builds and draws instead of `maps/<id>/track.ron` and `maps/<id>/decor.ron`
/// when it races track `id`: the track editor's unsaved edits (written only by rc_editor; the
/// game never has one).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TrackDraft {
    /// Track it stands in for.
    pub id: TrackId,
    /// The layout.
    pub def: TrackDef,
    /// The decorative models.
    pub decor: Vec<DecorItem>,
}

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "track";

/// Builds the session's track while `AppState::Loading`, resets dynamic props on every
/// `RacePhase::Setup`, cleans up in `OnEnter(AppState::Unloading)`. Needs rapier
/// (`rc_vehicle::PhysicsPlugin`) for the colliders to do anything.
pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_systems(OnEnter(AppState::Loading), spawn::load_track)
            .add_systems(OnEnter(RacePhase::Setup), spawn::reset_props)
            .add_systems(OnEnter(AppState::Unloading), spawn::remove_track);
    }
}

#[cfg(test)]
mod tests;
