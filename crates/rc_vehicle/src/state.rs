//! Private per-car runtime state shared by the vehicle systems (never read by other crates).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::model::CarSpec;

/// One wheel between ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct WheelRuntime {
    pub grounded: bool,
    /// Hardpoint → wheel centre, u.
    pub extension: f32,
    /// Steering angle, rad.
    pub steer: f32,
    /// Spin angle, rad (positive = rolling forward).
    pub spin: f32,
    /// Spin rate, rad/s.
    pub spin_rate: f32,
    pub surface: Option<SurfaceKind>,
    pub slip: f32,
    /// Contact normal (when grounded).
    pub normal: Vec3,
    /// Ray hit point at the start of the tick (when grounded), world space.
    pub contact: Vec3,
}

/// A touchdown being judged (all four wheels within the window, tilt).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Landing {
    pub air_time: f32,
    pub strength: f32,
    pub ticks: u32,
    /// Bit per wheel that touched since the first contact.
    pub touched: u8,
    pub tilt_ok: bool,
}

/// Runtime state of one car.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct CarState {
    pub wheels: [WheelRuntime; 4],
    pub grounded: u8,
    /// Linear velocity at the start of the latest tick (landing strength, path rate).
    pub pre_velocity: Vec3,
    /// Largest closing speed into the ground seen by the wheel rays during the current descent
    /// (before the landing absorber), u/s.
    pub closing_speed: f32,
    /// Seconds without any wheel on the ground.
    pub air_time: f32,
    /// Seconds of free flight: no wheel down and not resting on the body (charge, clean landings).
    pub flight_time: f32,
    /// Seconds of continuous body contact while no wheel is down.
    pub body_contact: f32,
    /// Rear lateral grip multiplier (handbrake ramp).
    pub rear_grip: f32,
    /// Handbrake drift direction: +1 sliding right of the heading (left-hand drift), −1 the
    /// other way, 0 = no drift engaged.
    pub drift_dir: f32,
    /// The charge boost is on (button held, charge left).
    pub charge_boost: bool,
    /// The charge boost is the boost in effect: charge drains.
    pub draining: bool,
    /// Any boost in effect.
    pub boosting: bool,
    /// Effective top speed of the latest tick, u/s.
    pub top_speed: f32,
    pub rpm: f32,
    pub throttle_load: f32,
    /// Body slip angle, rad (+ = sliding right of the heading).
    pub slip_angle: f32,
    pub drifting: bool,
    /// Seconds of continuous drift.
    pub drift_time: f32,
    /// Charge gained per second in the latest tick (drift, air, draft).
    pub charge_rate: f32,
    pub surface: Option<SurfaceKind>,
    /// Seconds resting on the side/roof (auto-right).
    pub flip_timer: f32,
    /// Seconds of auto-right manoeuvre left.
    pub righting: f32,
    pub landing: Option<Landing>,
    pub clean_landing: bool,
    /// A clean landing's charge bonus waits for the charge system.
    pub landing_bonus: bool,
    pub ghosted: bool,
    /// Tick counter of the impact debounce and the strength of the latest reported impact.
    pub last_impact: Option<(u32, f32)>,
}

impl CarState {
    pub fn new(spec: &CarSpec) -> Self {
        let wheels = spec.wheels.map(|w| WheelRuntime {
            extension: w.rest,
            ..default()
        });
        Self {
            wheels,
            grounded: 0,
            pre_velocity: Vec3::ZERO,
            closing_speed: 0.0,
            air_time: 0.0,
            flight_time: 0.0,
            body_contact: 0.0,
            rear_grip: 1.0,
            drift_dir: 0.0,
            charge_boost: false,
            draining: false,
            boosting: false,
            top_speed: spec.params.top_speed,
            rpm: 0.0,
            throttle_load: 0.0,
            slip_angle: 0.0,
            drifting: false,
            drift_time: 0.0,
            charge_rate: 0.0,
            surface: None,
            flip_timer: 0.0,
            righting: 0.0,
            landing: None,
            clean_landing: false,
            landing_bonus: false,
            ghosted: false,
            last_impact: None,
        }
    }

    /// After a teleport: forget the motion history, keep charge-independent flags.
    pub fn reset_motion(&mut self, spec: &CarSpec) {
        *self = Self {
            ghosted: self.ghosted,
            ..Self::new(spec)
        };
    }
}
