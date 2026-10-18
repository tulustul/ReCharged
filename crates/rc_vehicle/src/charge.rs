//! Battery charge & boost (GDD §5.3, §5.5): gains from drifting, air time, clean landings and
//! drafting (with the anti-farming thresholds), the charge boost, boost resolution (boosts never
//! stack; the Turbo Cell freezes the drain), `DrainCharge` / `GrantCharge`.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::model::CarSpec;
use crate::state::CarState;

/// Charge spent per second while the charge boost is in effect (it starts at
/// `rc_core::BOOST_MIN_CHARGE`).
const BOOST_DRAIN_PER_SEC: f32 = 0.35;
/// Charge boost: top-speed and drive-force multipliers.
const CHARGE_BOOST_TOP: f32 = 1.30;
const CHARGE_BOOST_FORCE: f32 = 1.50;
/// Drift: peak gain per second, minimum speed (share of top speed), minimum continuous drift,
/// slip-angle window (deg) and the yaw rate (rad/s) that earns the full rate.
const DRIFT_GAIN_PER_SEC: f32 = 0.22;
const DRIFT_MIN_SPEED_SHARE: f32 = 0.4;
pub(crate) const DRIFT_MIN_SECS: f32 = 0.4;
pub(crate) const DRIFT_SLIP_MIN_DEG: f32 = 10.0;
pub(crate) const DRIFT_SLIP_MAX_DEG: f32 = 45.0;
const DRIFT_FULL_YAW_RATE: f32 = 1.8;
/// Air time: gain per second after the first `AIR_MIN_SECS` of free flight.
const AIR_GAIN_PER_SEC: f32 = 0.15;
const AIR_MIN_SECS: f32 = 0.3;
/// One-shot bonus of a clean landing.
const CLEAN_LANDING_BONUS: f32 = 0.10;
/// Drafting: gain per second within range behind another car, inside the cone around its
/// direction of travel, above a share of the own top speed.
const DRAFT_GAIN_PER_SEC: f32 = 0.10;
const DRAFT_RANGE_U: f32 = 6.0;
const DRAFT_CONE_DEG: f32 = 15.0;
const DRAFT_MIN_SPEED_SHARE: f32 = 0.5;

/// Charge-boost activation and boost resolution for this tick; returns the (top-speed, force)
/// multipliers of the boost in effect. Boosts never stack: the strongest of the charge boost
/// and the status boost applies; charge drains only while the charge boost is the one in effect
/// and no status boost freezes it (Turbo Cell).
pub(crate) fn resolve_boost(
    state: &mut CarState,
    effects: &StatusEffects,
    button: bool,
    charge: f32,
) -> (f32, f32) {
    let keep = state.charge_boost && charge > 0.0;
    state.charge_boost = button && (keep || charge >= BOOST_MIN_CHARGE);
    let status = effects.boost();
    let charge_wins = state.charge_boost && status.is_none_or(|b| CHARGE_BOOST_TOP > b.top_speed);
    let frozen = status.is_some_and(|b| b.freeze_charge);
    state.draining = charge_wins && !frozen;
    state.boosting = state.charge_boost || status.is_some();
    match (charge_wins, status) {
        (true, _) => (CHARGE_BOOST_TOP, CHARGE_BOOST_FORCE),
        (false, Some(b)) => (b.top_speed, b.force),
        (false, None) => (1.0, 1.0),
    }
}

/// Charge per second earned by a drift: only above 40 % of top speed, after 0.4 s of
/// continuous drift, at a slip angle of 10–45° (none while spinning), scaled by yaw rate.
pub(crate) fn drift_gain(speed: f32, top: f32, drift_secs: f32, slip: f32, yaw_rate: f32) -> f32 {
    let slip_deg = slip.abs().to_degrees();
    let window = (DRIFT_SLIP_MIN_DEG..=DRIFT_SLIP_MAX_DEG).contains(&slip_deg);
    if speed < DRIFT_MIN_SPEED_SHARE * top || drift_secs < DRIFT_MIN_SECS || !window {
        return 0.0;
    }
    DRIFT_GAIN_PER_SEC * (yaw_rate.abs() / DRIFT_FULL_YAW_RATE).min(1.0)
}

/// Charge per second earned in the air (after the first 0.3 s of free flight).
pub(crate) fn air_gain(flight_secs: f32) -> f32 {
    if flight_secs > AIR_MIN_SECS {
        AIR_GAIN_PER_SEC
    } else {
        0.0
    }
}

/// `me` drafts behind a car at `lead_pos` moving with `lead_vel`: within range, inside the cone
/// behind its direction of travel, and fast enough itself.
// ponytail: a straight cone behind the leader ignores track curvature; follow its recent path
// if drafting in long bends matters.
pub(crate) fn drafting(
    me_pos: Vec3,
    me_speed: f32,
    me_top: f32,
    lead_pos: Vec3,
    lead_vel: Vec3,
) -> bool {
    let gap = me_pos - lead_pos;
    let back = -lead_vel.normalize_or_zero();
    if me_speed < DRAFT_MIN_SPEED_SHARE * me_top || back == Vec3::ZERO {
        return false;
    }
    let dist = gap.length();
    dist <= DRAFT_RANGE_U
        && dist > 1e-3
        && gap.dot(back) >= dist * DRAFT_CONE_DEG.to_radians().cos()
}

/// `SimSet::PostPhysics`: charge gains and spend, then drains and grants from items.
pub(crate) fn update_charge(
    mut others: Local<Vec<(Entity, Vec3, Vec3)>>,
    mut cars: Query<(
        Entity,
        &CarSpec,
        &mut CarState,
        &mut Charge,
        &Transform,
        &Velocity,
    )>,
    mut drains: MessageReader<DrainCharge>,
    mut grants: MessageReader<GrantCharge>,
) {
    others.clear();
    others.extend(
        cars.iter()
            .map(|(e, .., t, v)| (e, t.translation, v.linear)),
    );
    for (entity, spec, mut state, mut charge, transform, velocity) in &mut cars {
        let speed = velocity.linear.length();
        let top = spec.params.top_speed;
        let draft = others.iter().any(|&(e, pos, vel)| {
            e != entity && drafting(transform.translation, speed, top, pos, vel)
        });
        let up = transform.rotation * Vec3::Y;
        let yaw = velocity.angular.dot(up);
        let drift = match state.drifting {
            true => drift_gain(speed, top, state.drift_time, state.slip_angle, yaw),
            false => 0.0,
        };
        let rate =
            drift + air_gain(state.flight_time) + if draft { DRAFT_GAIN_PER_SEC } else { 0.0 };
        state.charge_rate = rate;
        let mut c = charge.0 + rate * SIM_DT;
        if std::mem::take(&mut state.landing_bonus) {
            c += CLEAN_LANDING_BONUS;
        }
        if state.draining {
            c -= BOOST_DRAIN_PER_SEC * SIM_DT;
        }
        charge.0 = c.clamp(0.0, 1.0);
    }
    for d in drains.read() {
        if let Ok((.., mut charge, _, _)) = cars.get_mut(d.car) {
            charge.0 *= 1.0 - d.fraction.clamp(0.0, 1.0);
        }
    }
    for g in grants.read() {
        if let Ok((.., mut charge, _, _)) = cars.get_mut(g.car) {
            charge.0 = (charge.0 + g.amount.max(0.0)).min(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> CarState {
        let p =
            ron::from_str(include_str!("../../../assets/data/vehicles/zip.ron")).expect("parses");
        CarState::new(&CarSpec::new(&p))
    }

    fn turbo(freeze: bool) -> StatusEffects {
        StatusEffects {
            boost: Some(TimedBoost {
                top_speed: 1.45,
                force: 1.8,
                freeze_charge: freeze,
                remaining: 2.0,
            }),
            ..default()
        }
    }

    #[test]
    fn charge_boost_needs_15_percent_to_start_then_runs_dry() {
        let mut s = state();
        let none = StatusEffects::default();
        assert_eq!(resolve_boost(&mut s, &none, true, 0.10), (1.0, 1.0));
        assert!(!s.charge_boost && !s.boosting);
        assert_eq!(
            resolve_boost(&mut s, &none, true, 0.15),
            (CHARGE_BOOST_TOP, CHARGE_BOOST_FORCE)
        );
        assert!(s.draining && s.boosting);
        resolve_boost(&mut s, &none, true, 0.01);
        assert!(s.charge_boost, "keeps running below the start threshold");
        resolve_boost(&mut s, &none, true, 0.0);
        assert!(!s.charge_boost, "stops when empty");
        resolve_boost(&mut s, &none, false, 0.5);
        assert!(!s.charge_boost, "button released");
    }

    #[test]
    fn boosts_never_stack_and_turbo_freezes_the_drain() {
        let mut s = state();
        assert_eq!(resolve_boost(&mut s, &turbo(true), true, 0.8), (1.45, 1.8));
        assert!(
            s.charge_boost && !s.draining && s.boosting,
            "turbo wins, charge not drained"
        );
        assert_eq!(resolve_boost(&mut s, &turbo(true), false, 0.8), (1.45, 1.8));
        let start = StatusEffects {
            boost: Some(TimedBoost {
                top_speed: CHARGE_BOOST_TOP,
                force: CHARGE_BOOST_FORCE,
                freeze_charge: false,
                remaining: 1.0,
            }),
            ..default()
        };
        resolve_boost(&mut s, &start, true, 0.8);
        assert!(!s.draining, "an equal free boost is used before the charge");
    }

    #[test]
    fn drift_gain_follows_the_anti_farming_rules() {
        let (top, slip, yaw) = (30.0, 20f32.to_radians(), 2.0);
        assert!((drift_gain(20.0, top, 1.0, slip, yaw) - DRIFT_GAIN_PER_SEC).abs() < 1e-6);
        assert_eq!(
            drift_gain(11.0, top, 1.0, slip, yaw),
            0.0,
            "below 40 % of top speed"
        );
        assert_eq!(drift_gain(20.0, top, 0.3, slip, yaw), 0.0, "not yet 0.4 s");
        assert_eq!(
            drift_gain(20.0, top, 1.0, 8f32.to_radians(), yaw),
            0.0,
            "too straight"
        );
        assert_eq!(
            drift_gain(20.0, top, 1.0, 60f32.to_radians(), yaw),
            0.0,
            "spinning"
        );
        assert!(
            drift_gain(20.0, top, 1.0, -slip, 0.9) < DRIFT_GAIN_PER_SEC,
            "scaled by yaw rate"
        );
        assert_eq!(
            drift_gain(20.0, top, 1.0, slip, 0.0),
            0.0,
            "sideways without rotation"
        );
    }

    #[test]
    fn air_time_charges_after_the_first_moments() {
        assert_eq!(air_gain(0.2), 0.0);
        assert_eq!(air_gain(0.5), AIR_GAIN_PER_SEC);
    }

    #[test]
    fn drafting_needs_range_cone_and_speed() {
        let lead_vel = Vec3::NEG_Z * 25.0;
        let behind = Vec3::new(0.0, 0.0, 4.0);
        assert!(drafting(behind, 25.0, 30.0, Vec3::ZERO, lead_vel));
        assert!(
            !drafting(Vec3::new(0.0, 0.0, 7.0), 25.0, 30.0, Vec3::ZERO, lead_vel),
            "range"
        );
        assert!(
            !drafting(Vec3::new(3.0, 0.0, 4.0), 25.0, 30.0, Vec3::ZERO, lead_vel),
            "cone"
        );
        assert!(
            !drafting(behind, 10.0, 30.0, Vec3::ZERO, lead_vel),
            "too slow"
        );
        assert!(
            !drafting(Vec3::new(0.0, 0.0, -4.0), 25.0, 30.0, Vec3::ZERO, lead_vel),
            "ahead"
        );
        assert!(
            !drafting(behind, 25.0, 30.0, Vec3::ZERO, Vec3::ZERO),
            "parked lead"
        );
    }
}
