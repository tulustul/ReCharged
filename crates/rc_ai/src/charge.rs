//! Charge (GDD §5.3): drift through long corners to fill the battery, spend it on straights.

use rc_core::prelude::*;

use crate::brain::{AiBrain, TrackView};
use crate::data::{AiTuning, DifficultyProfile, Personality};
use crate::scene::Scene;

/// Top-speed multiplier of the charge boost (GDD §5.3).
pub const CHARGE_BOOST_TOP: f32 = 1.30;
/// A drift that has not started charging after this long is abandoned (s; rc_vehicle charges
/// after 0.4 s of continuous drift).
const DRIFT_PROVE_SECS: f32 = 0.7;
/// Beyond this body slip (rad) the drift is a spin: let go (rc_vehicle stops charging at 45°).
const DRIFT_MAX_SLIP_RAD: f32 = 0.7;
/// A cautious driver (boost eagerness 0) waits for this much charge above the minimum.
const CAUTIOUS_EXTRA_CHARGE: f32 = 0.2;
/// RNG key offset of the per-corner drift decision.
const DRIFT_KEY: u64 = 5;
/// A drift starts only from stable grip driving: body slip below this (rad) …
const DRIFT_ENTRY_SLIP_RAD: f32 = 0.14;
/// … within this of the planned lateral (u), with every wheel down …
const DRIFT_ENTRY_LINE_U: f32 = 1.5;
/// … and while the battery is below this (a full battery gains nothing).
const DRIFT_MAX_CHARGE: f32 = 0.7;
/// A drifting car is turned ~30° to its path: its line keeps this much more room to the edges
/// (u), and the drift is abandoned when the car gets closer than [`DRIFT_MIN_EDGE_ROOM_U`] to its
/// legal edge anyway.
pub const DRIFT_EDGE_EXTRA_U: f32 = 1.0;
const DRIFT_MIN_EDGE_ROOM_U: f32 = 0.5;
/// After a drift, the throttle is eased to this until the slide is caught (body slip below
/// [`DRIFT_ENTRY_SLIP_RAD`]): full power while the rear regains grip spins the car.
pub const CATCH_THROTTLE: f32 = 0.4;

/// A released drift is still being caught: ease the throttle.
pub fn catching(tel: &VehicleTelemetry, drifting: bool) -> bool {
    !drifting && tel.slip_angle.abs() > DRIFT_ENTRY_SLIP_RAD && tel.forward_speed > 0.0
}

/// `s` lies in the zone `[from, to)` of a loop of `length` (zones may wrap).
pub fn in_zone(s: f32, (from, to): (f32, f32), length: f32) -> bool {
    let d = (s - from).rem_euclid(length);
    d < (to - from).rem_euclid(length)
}

/// Holds the handbrake through drift zones (taken with the difficulty's chance), while the drift
/// is charging (`VehicleTelemetry::charge_rate`), fast enough and not spinning.
/// What the drift decision sees of the car this tick.
pub(crate) struct DriftSense<'a> {
    pub view: &'a TrackView,
    pub tel: &'a VehicleTelemetry,
    pub top: f32,
    pub steer: f32,
    pub charge: f32,
    /// Distance from the planned lateral (u).
    pub off_line: f32,
    /// Room between the car and its legal lateral range's nearer edge (u, < 0 = outside).
    pub edge_room: f32,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn drift(
    w: &Scene,
    brain: &mut AiBrain,
    skill: &DifficultyProfile,
    key: u64,
    sense: DriftSense,
) -> bool {
    let DriftSense {
        view,
        tel,
        top,
        steer,
        charge,
        off_line,
        edge_room,
    } = sense;
    let length = brain.profile.length();
    let zone = brain
        .drift_zones
        .iter()
        .position(|&z| in_zone(view.s, z, length));
    let fast = tel.forward_speed > w.tuning.charge.drift_min_speed * top;
    match (brain.drift, zone) {
        (Some((i, secs)), Some(z)) if z == i => {
            let charging = secs < DRIFT_PROVE_SECS || tel.charge_rate > 0.0;
            // The corner turns right when its curvature is positive; steering the other way
            // harder than the limit = the drift is too tight.
            let corner = brain.profile.kappa_at(view.s).signum();
            let counter = -steer * corner > w.tuning.charge.drift_counter_steer;
            let keep = charging
                && fast
                && !counter
                && edge_room > DRIFT_MIN_EDGE_ROOM_U
                && tel.grounded_wheels >= 3
                && tel.slip_angle.abs() < DRIFT_MAX_SLIP_RAD;
            brain.drift = keep.then_some((i, secs + SIM_DT));
            if !keep {
                brain.skip_zone = Some(i);
            }
            keep
        }
        (Some(_), _) => {
            brain.drift = None;
            false
        }
        (None, Some(z)) if brain.skip_zone != Some(z) => {
            let stable = tel.grounded_wheels == 4
                && tel.slip_angle.abs() < DRIFT_ENTRY_SLIP_RAD
                && off_line < DRIFT_ENTRY_LINE_U
                && edge_room > DRIFT_EDGE_EXTRA_U;
            let take = fast
                && stable
                && charge < DRIFT_MAX_CHARGE
                && rng::chance(
                    w.seed,
                    w.tick,
                    RngStream::AiMistakes,
                    key + DRIFT_KEY,
                    skill.drift,
                );
            match take {
                true => brain.drift = Some((z, 0.0)),
                false => brain.skip_zone = Some(z),
            }
            take
        }
        (None, None) => {
            brain.skip_zone = None;
            false
        }
        (None, Some(_)) => false,
    }
}

/// How long the plan must stay at the boosted top speed before `personality` starts a boost.
pub fn straight_secs(personality: Personality, tuning: &AiTuning) -> f32 {
    tuning.charge.boost_straight_secs * (1.5 - personality.boost)
}

/// Holds the charge boost on straights: starts with enough charge when the plan ahead stays at
/// the boosted top speed for [`straight_secs`] (`straight`), stops when braking is due or the
/// battery is empty.
pub(crate) fn boost(
    brain: &mut AiBrain,
    charge: f32,
    tel: &VehicleTelemetry,
    top: f32,
    brake_to: f32,
    straight: bool,
) -> bool {
    let eager = brain.personality.boost;
    let able = tel.grounded_wheels >= 3 && brain.drift.is_none() && tel.forward_speed > 0.5 * top;
    brain.boosting = if brain.boosting {
        able && charge > 0.0 && brake_to >= tel.forward_speed
    } else {
        able && straight && charge >= BOOST_MIN_CHARGE + (1.0 - eager) * CAUTIOUS_EXTRA_CHARGE
    };
    brain.boosting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_wrap_around_the_start_line() {
        assert!(in_zone(5.0, (0.0, 10.0), 100.0));
        assert!(!in_zone(10.0, (0.0, 10.0), 100.0));
        assert!(in_zone(98.0, (95.0, 3.0), 100.0));
        assert!(in_zone(1.0, (95.0, 3.0), 100.0));
        assert!(!in_zone(50.0, (95.0, 3.0), 100.0));
    }
}
