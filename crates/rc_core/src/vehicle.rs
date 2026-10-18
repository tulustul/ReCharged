//! Car contracts: driving intent, telemetry, charge and the vehicle messages
//! (ARCHITECTURE §4.3, §4.5). Rapier components on a racer are private to rc_vehicle.

use bevy::prelude::*;

use crate::track::SurfaceKind;

/// Driving intent of a racer for the current tick.
///
/// Writers: rc_input (`LocalPlayer` without `AiDriver`) or rc_ai (`AiDriver`) in
/// `SimSet::Drivers`; the steering assist may change only `steer` in `SimSet::Assist`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct CarControl {
    /// Throttle 0–1.
    pub throttle: f32,
    /// Brake / reverse 0–1.
    pub brake: f32,
    /// Steering −1 (left) … 1 (right).
    pub steer: f32,
    /// Handbrake / drift held.
    pub drift: bool,
    /// Spend charge (held).
    pub boost: bool,
    /// Use the held item: an edge, consumed in the tick it is set.
    pub use_item: bool,
    /// Look back (held; also fires items backward).
    pub look_back: bool,
    /// Reset button level; rc_race times the hold in fixed ticks (tap = flip, hold = respawn).
    pub reset_held: bool,
}

/// Per-wheel state for visuals (wheel spin, steer, suspension) and effects.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelState {
    /// The wheel touches the ground.
    pub grounded: bool,
    /// Suspension compression 0 (fully extended) … 1 (bottomed out).
    pub compression: f32,
    /// Accumulated spin angle in radians.
    pub spin: f32,
    /// Steering angle in radians.
    pub steer: f32,
    /// Surface under the wheel, if grounded.
    pub surface: Option<SurfaceKind>,
    /// Slip ratio/angle magnitude 0–1 (skid sounds, marks).
    pub slip: f32,
}

/// Physical state of a car after the latest tick. Written only by rc_vehicle (`SimSet::PostPhysics`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct VehicleTelemetry {
    /// Speed magnitude in u/s.
    pub speed: f32,
    /// Speed along the car's forward axis (−Z) in u/s; negative when reversing.
    pub forward_speed: f32,
    /// Engine rpm, normalised 0–1.
    pub rpm: f32,
    /// Engine load 0–1 (audio).
    pub throttle_load: f32,
    /// Wheels touching the ground (0–4).
    pub grounded_wheels: u8,
    /// Seconds since the last ground contact (0 when grounded).
    pub airborne_time: f32,
    /// Body slip angle in radians.
    pub slip_angle: f32,
    /// Currently drifting.
    pub drifting: bool,
    /// Dominant surface under the car.
    pub surface: Option<SurfaceKind>,
    /// Wheels in order FL, FR, RL, RR.
    pub wheels: [WheelState; 4],
    /// Any boost is active.
    pub boosting: bool,
    /// Dot product of the car's up axis with world up (< 0.3 = on its side/roof).
    pub up_dot: f32,
    /// The latest landing was clean (all wheels within 0.15 s, tilt < 20°).
    pub clean_landing: bool,
}

/// Battery charge 0–1 (GDD §5.3). Written only by rc_vehicle.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct Charge(pub f32);

/// AI catch-up top-speed scaling, ±0.06 (GDD §8.1). Written only by rc_ai.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct CatchUp(pub f32);

/// A car hit something hard (producer vehicle; consumers audio, vfx, input rumble, camera shake).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct CarImpact {
    /// The car.
    pub car: Entity,
    /// The other body, if it was an entity we know (car, prop).
    pub other: Option<Entity>,
    /// Impact strength 0–1.
    pub strength: f32,
    /// Contact point in world space.
    pub point: Vec3,
    /// Contact normal pointing away from the other body.
    pub normal: Vec3,
}

/// A car landed after being airborne.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct CarLanded {
    /// The car.
    pub car: Entity,
    /// Seconds in the air.
    pub air_time: f32,
    /// Landing strength 0–1.
    pub strength: f32,
    /// Clean landing (charge bonus).
    pub clean: bool,
}

/// How to put a car back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RespawnKind {
    /// Upright in place (reset tap while on the side/roof).
    #[default]
    FlipInPlace,
    /// Back on the track centre at a respawn point (hold, kill height, off-track, stuck).
    ToTrack,
}

/// Request to respawn a car (producer race — the sole decider; consumer vehicle).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct RespawnCar {
    /// The car.
    pub car: Entity,
    /// Target pose.
    pub transform: Transform,
    /// Flip or respawn.
    pub kind: RespawnKind,
}

/// A respawn was executed (producer vehicle; consumers audio, vfx, ui, ai).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarRespawned {
    /// The car.
    pub car: Entity,
    /// What happened.
    pub kind: RespawnKind,
}

/// Instant impulse on a car, e.g. a tumble (producer items; consumer vehicle, which scales it by
/// `(reference mass / mass)^0.5`).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ApplyImpulse {
    /// The car.
    pub car: Entity,
    /// Linear impulse in world space.
    pub linear: Vec3,
    /// Angular impulse in world space.
    pub angular: Vec3,
}

/// Remove a fraction of a car's charge (EMP, Leech Coil victim).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct DrainCharge {
    /// The car.
    pub car: Entity,
    /// Fraction 0–1 of the current charge.
    pub fraction: f32,
}

/// Add charge to a car (Leech Coil owner).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct GrantCharge {
    /// The car.
    pub car: Entity,
    /// Absolute charge 0–1 to add (clamped by the vehicle).
    pub amount: f32,
}
