//! Track contracts: surfaces (GDD §5.2), track/prop markers and the track summary
//! (ARCHITECTURE §4.4, §4.6). The `Track` query resource itself lives in rc_track.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Grip multiplier of full snow cover on any outdoor surface (GDD §5.2).
pub const SNOW_GRIP_MUL: f32 = 0.70;

/// Drivable surface material.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceKind {
    /// Roads, concrete floors, curbs, drains.
    #[default]
    Asphalt,
    /// Patio, paving.
    Stone,
    /// Plastic toy-track ribbons.
    ToyTrack,
    /// Floors, deck boards, stairs, plywood ramps, shelves.
    Wood,
    /// Tiles, marble, vinyl, polished floors.
    Tile,
    /// Rugs, play-mats, rubber mats.
    Carpet,
    /// Boxes, cardboard ramps.
    Cardboard,
    /// Lawns.
    Grass,
    /// Paths, sandbox.
    Dirt,
    /// Freezer aisle.
    Ice,
    /// Workshop oil stains.
    Oil,
    /// Puddles, wash bay.
    Water,
}

/// Physical properties of a surface (GDD §5.2). Multipliers are relative to Asphalt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceProps {
    /// Tyre friction multiplier.
    pub grip: f32,
    /// Top-speed factor (rolling resistance).
    pub top_speed: f32,
    /// Grip multiplier when fully wet (outdoor tracks only).
    pub wet_grip: f32,
    /// Top-speed multiplier when fully wet (wet dirt turns to mud).
    pub wet_top_speed: f32,
}

impl SurfaceKind {
    /// Every surface.
    pub const ALL: [SurfaceKind; 12] = [
        Self::Asphalt,
        Self::Stone,
        Self::ToyTrack,
        Self::Wood,
        Self::Tile,
        Self::Carpet,
        Self::Cardboard,
        Self::Grass,
        Self::Dirt,
        Self::Ice,
        Self::Oil,
        Self::Water,
    ];

    /// The GDD §5.2 table row of this surface.
    pub fn props(self) -> SurfaceProps {
        let (grip, top_speed, wet_grip, wet_top_speed) = match self {
            Self::Asphalt => (1.00, 1.00, 0.85, 1.0),
            Self::Stone => (1.00, 1.00, 0.75, 1.0),
            Self::ToyTrack => (1.00, 1.02, 1.0, 1.0),
            Self::Wood => (0.95, 1.00, 1.0, 1.0),
            Self::Tile => (0.82, 1.00, 0.75, 1.0),
            Self::Carpet => (1.05, 0.92, 1.0, 1.0),
            Self::Cardboard => (0.90, 0.95, 1.0, 1.0),
            Self::Grass => (0.75, 0.85, 0.85, 1.0),
            Self::Dirt => (0.80, 0.92, 0.80, 0.90),
            Self::Ice => (0.45, 1.00, 1.0, 1.0),
            Self::Oil => (0.50, 1.00, 1.0, 1.0),
            Self::Water => (0.65, 0.85, 1.0, 1.0),
        };
        SurfaceProps {
            grip,
            top_speed,
            wet_grip,
            wet_top_speed,
        }
    }
}

impl SurfaceProps {
    /// Grip and top speed under weather: `wetness` and `snow_cover` (0–1) scale the modifiers
    /// linearly. Pass zeros on indoor tracks (`WeatherState` already reports zeros there).
    pub fn with_weather(self, wetness: f32, snow_cover: f32) -> Self {
        let w = wetness.clamp(0.0, 1.0);
        let s = snow_cover.clamp(0.0, 1.0);
        let lerp = |to: f32, t: f32| 1.0 + (to - 1.0) * t;
        Self {
            grip: self.grip * lerp(self.wet_grip, w) * lerp(SNOW_GRIP_MUL, s),
            top_speed: self.top_speed * lerp(self.wet_top_speed, w),
            ..self
        }
    }
}

/// Which part of the track a collider entity is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TrackPieceKind {
    /// A ribbon run (drivable, carries a [`Surface`]).
    #[default]
    Ribbon,
    /// A barrier wall.
    Wall,
    /// A ground area polygon (drivable, carries a [`Surface`]).
    Ground,
    /// Room shell / kill floor.
    Shell,
}

/// Marker on track collider entities (session-scoped, spawned by rc_track).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrackPiece {
    /// What this piece is.
    pub kind: TrackPieceKind,
}

/// The surface of a drivable collider (wheel ray hits read it).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Surface(pub SurfaceKind);

/// A prop placed by the track (session-scoped; dynamic props are reset in `OnEnter(Setup)`).
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Prop {
    /// Prop kind id from `assets/data/props.ron`.
    pub kind: String,
    /// Knockable (dynamic rigid body, also `Interpolated`).
    pub dynamic: bool,
}

/// A corner of the layout (found by rc_track from the centreline's heading): the HUD's turn
/// indicator and the turn signs along the track.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TrackTurn {
    /// Arc length where the turn starts (u, from the start line in the travel direction).
    pub start_s: f32,
    /// Arc length it lasts (u); `start_s + span` may run past the start line.
    pub span: f32,
    /// Turns right (seen from above, in the travel direction).
    pub right: bool,
    /// Heading change (radians, positive).
    pub angle: f32,
    /// How hard: 1 easy, 2 medium, 3 sharp (a hairpin or a hard stop).
    pub severity: u8,
}

/// Track facts for UI and records. Written by rc_track when the track is built (session resource).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TrackSummary {
    /// Top-down centreline outline (world X/Z) for the minimap.
    pub outline_2d: Vec<Vec2>,
    /// The racing edges (world X/Z: left, right) at the `outline_2d` points.
    pub edges_2d: Vec<[Vec2; 2]>,
    /// Checkpoint gates across the racing width (world X/Z: left, right end), in order.
    pub gates_2d: Vec<[Vec2; 2]>,
    /// Corners in travel order.
    pub turns: Vec<TrackTurn>,
    /// Lap length in u.
    pub length: f32,
    /// Number of checkpoints per lap.
    pub checkpoints: usize,
    /// Hash of the built geometry; records and ghosts with another hash are discarded.
    pub geometry_hash: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_table_is_sane() {
        for s in SurfaceKind::ALL {
            let p = s.props();
            assert!(p.grip > 0.0 && p.grip <= 1.05, "{s:?}");
            assert!(p.top_speed > 0.0 && p.top_speed <= 1.02, "{s:?}");
            assert!(
                p.wet_grip <= 1.0 && p.wet_top_speed <= 1.0,
                "wet never helps: {s:?}"
            );
        }
        let grip = |s: SurfaceKind| s.props().grip;
        assert!(grip(SurfaceKind::Ice) < grip(SurfaceKind::Tile));
        assert!(grip(SurfaceKind::Tile) < grip(SurfaceKind::Asphalt));
        assert!(grip(SurfaceKind::Oil) < grip(SurfaceKind::Water));
        let fastest = SurfaceKind::ALL
            .into_iter()
            .max_by(|a, b| a.props().top_speed.total_cmp(&b.props().top_speed));
        assert_eq!(fastest, Some(SurfaceKind::ToyTrack));
    }

    #[test]
    fn weather_scales_linearly() {
        let dry = SurfaceKind::Asphalt.props();
        assert_eq!(dry.with_weather(0.0, 0.0), dry);
        assert!((dry.with_weather(1.0, 0.0).grip - 0.85).abs() < 1e-6);
        assert!((dry.with_weather(0.5, 0.0).grip - 0.925).abs() < 1e-6);
        assert!((dry.with_weather(0.0, 1.0).grip - SNOW_GRIP_MUL).abs() < 1e-6);
        let mud = SurfaceKind::Dirt.props().with_weather(1.0, 0.0);
        assert!((mud.grip - 0.64).abs() < 1e-6 && (mud.top_speed - 0.828).abs() < 1e-6);
        assert_eq!(SurfaceKind::Ice.props().with_weather(2.0, -1.0).grip, 0.45);
    }
}
