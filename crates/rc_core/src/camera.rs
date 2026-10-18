//! Camera markers and viewport layout (ARCHITECTURE §4.4, §4.6, §5.8). Spawned and written by
//! rc_camera; read by render (post-processing), audio (listeners), vfx (precipitation layers), ui.

use bevy::prelude::*;

use crate::catalog::PlayerSlot;

/// Where the menu showroom's car stands (turntable centre, on the floor). rc_render builds the
/// showroom around it, rc_camera frames it.
pub const SHOWROOM_FOCUS: Vec3 = Vec3::ZERO;

/// A local player's race camera (session-scoped; re-bound to the slot's racer in
/// `OnEnter(RaceAttempt)`, so restarts never despawn it).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameCamera {
    /// The player it follows.
    pub slot: PlayerSlot,
}

/// The menu showroom camera.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuCamera;

/// Full-window UI overlay camera used in split-screen.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverlayCamera;

/// The camera currently hosting the React UI (`IsDefaultUiCamera`); exactly one exists.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UiHost;

/// Each local player's viewport in logical pixels (HUD layout). Written only by rc_camera.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct PlayerViewports(pub Vec<(PlayerSlot, Rect)>);
