//! `SimSet::PostPhysics`: speed cap, air/flight timers, landings (`CarLanded`, landing speed
//! loss), drift state, `VehicleTelemetry`, `CarImpact`.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::charge::{DRIFT_SLIP_MAX_DEG, DRIFT_SLIP_MIN_DEG};
use crate::model::CarSpec;
use crate::state::{CarState, Landing};

/// `CarLanded` is sent only after at least this much air time, s (small bumps stay silent).
const LANDING_MIN_AIR_SECS: f32 = 0.25;
/// Clean landing (GDD §5.3): after more than this free flight, s …
const CLEAN_MIN_FLIGHT_SECS: f32 = 0.8;
/// … all four wheels down within this many ticks (0.15 s) of the first …
const CLEAN_WINDOW_TICKS: u32 = 9;
/// … with the car tilted less than this against the ground, degrees.
const CLEAN_MAX_TILT_DEG: f32 = 20.0;
/// Body contact longer than this (s) without wheels on the ground means resting, not flying.
const BODY_REST_SECS: f32 = 0.2;
/// Landing speed along the ground normal that counts as strength 1, u/s.
const LANDING_FULL_UPS: f32 = 15.0;
/// Below this speed (u/s) there is no slip angle and no drift.
const DRIFT_MIN_UPS: f32 = 3.0;
/// A drift continues down to this slip angle (hysteresis), degrees; beyond the spin angle the
/// car is turning around, not drifting.
const DRIFT_EXIT_DEG: f32 = 7.0;
const SPIN_DEG: f32 = 90.0;
/// Safety bound of the body's angular speed, rad/s.
const MAX_ANGULAR_SPEED: f32 = 30.0;
/// Velocity change (u/s) of a contact that reports strength 1.
const IMPACT_FULL_DV_UPS: f32 = 15.0;
/// Weakest impact reported (strength 0–1).
const IMPACT_MIN_STRENGTH: f32 = 0.12;
/// After an impact, weaker ones are ignored for this many ticks (0.2 s).
const IMPACT_COOLDOWN_TICKS: u32 = 12;
/// Contacts whose normal is this close to vertical (cos of 40°) are landings, not impacts.
const GROUND_NORMAL_Y: f32 = 0.766;

/// Hard speed cap, timers, landings and drift state from this tick's wheels and the new velocity.
pub(crate) fn motion(
    mut cars: Query<(
        Entity,
        &CarSpec,
        &mut CarState,
        &Transform,
        &mut Velocity,
        &CollidingEntities,
    )>,
    mut landed: MessageWriter<CarLanded>,
) {
    for (entity, spec, mut state, transform, mut velocity, touching) in &mut cars {
        cap_speed(spec, &mut velocity);
        if state.grounded == 0 {
            state.air_time += SIM_DT;
            // Resting on the body (roof, side) is not flight; brief scrapes and bumper taps are.
            state.body_contact = match touching.is_empty() {
                true => 0.0,
                false => state.body_contact + SIM_DT,
            };
            state.flight_time = match state.body_contact > BODY_REST_SECS {
                true => 0.0,
                false => state.flight_time + SIM_DT,
            };
        } else {
            if state.air_time >= LANDING_MIN_AIR_SECS && state.landing.is_none() {
                touch_down(
                    entity,
                    spec,
                    &mut state,
                    transform,
                    &mut velocity,
                    &mut landed,
                );
            }
            state.air_time = 0.0;
            state.flight_time = 0.0;
            state.body_contact = 0.0;
            state.closing_speed = 0.0;
        }
        judge_landing(entity, &mut state, &mut landed);
        drift_state(&mut state, transform, &velocity);
    }
}

/// Hard cap at `TOP_SPEED_CAP` × base top speed (GDD §5.1); bad numbers reset the body.
fn cap_speed(spec: &CarSpec, velocity: &mut Velocity) {
    if !(velocity.linear.is_finite() && velocity.angular.is_finite()) {
        error_once!("vehicles: non-finite car velocity reset to zero");
        *velocity = Velocity::zero();
        return;
    }
    let cap = spec.params.top_speed * TOP_SPEED_CAP;
    if velocity.linear.length_squared() > cap * cap {
        velocity.linear = velocity.linear.normalize() * cap;
    }
    if velocity.angular.length_squared() > MAX_ANGULAR_SPEED * MAX_ANGULAR_SPEED {
        velocity.angular = velocity.angular.normalize() * MAX_ANGULAR_SPEED;
    }
}

/// First wheel back on the ground after real air time: strength, landing speed loss (heavier
/// cars lose more) and the start of the clean-landing judgement.
fn touch_down(
    entity: Entity,
    spec: &CarSpec,
    state: &mut CarState,
    transform: &Transform,
    velocity: &mut Velocity,
    landed: &mut MessageWriter<CarLanded>,
) {
    let normal = state
        .wheels
        .iter()
        .filter(|w| w.grounded)
        .map(|w| w.normal)
        .sum::<Vec3>()
        .normalize_or(Vec3::Y);
    let into_ground = (-state.pre_velocity.dot(normal))
        .max(state.closing_speed)
        .max(0.0);
    let strength = (into_ground / LANDING_FULL_UPS).min(1.0);
    let along = velocity.linear - normal * velocity.linear.dot(normal);
    velocity.linear -= along * (spec.params.landing_loss * strength);
    let up = transform.rotation * Vec3::Y;
    let landing = Landing {
        air_time: state.air_time,
        strength,
        ticks: 0,
        touched: 0,
        tilt_ok: up.dot(normal) >= CLEAN_MAX_TILT_DEG.to_radians().cos(),
    };
    if state.flight_time > CLEAN_MIN_FLIGHT_SECS {
        state.landing = Some(landing);
    } else {
        finish_landing(entity, state, landing, false, landed);
    }
}

/// Clean-landing window: every wheel must have touched within it (a wheel bouncing off for a
/// tick does not spoil it); judged once all four touched or the window ran out.
fn judge_landing(car: Entity, state: &mut CarState, landed: &mut MessageWriter<CarLanded>) {
    let Some(mut l) = state.landing.take() else {
        return;
    };
    l.ticks += 1;
    l.touched |= state
        .wheels
        .iter()
        .enumerate()
        .filter(|(_, w)| w.grounded)
        .fold(0, |m, (i, _)| m | 1 << i);
    match (l.touched == 0b1111, l.ticks > CLEAN_WINDOW_TICKS) {
        (true, _) => finish_landing(car, state, l, l.tilt_ok, landed),
        (_, true) => finish_landing(car, state, l, false, landed),
        _ => state.landing = Some(l),
    }
}

fn finish_landing(
    car: Entity,
    state: &mut CarState,
    l: Landing,
    clean: bool,
    landed: &mut MessageWriter<CarLanded>,
) {
    state.clean_landing = clean;
    state.landing_bonus |= clean;
    landed.write(CarLanded {
        car,
        air_time: l.air_time,
        strength: l.strength,
        clean,
    });
}

/// Body slip angle and the drift state (hysteresis; a spin resets the continuous-drift timer).
fn drift_state(state: &mut CarState, transform: &Transform, velocity: &Velocity) {
    let v = velocity.linear;
    let speed = v.length();
    let fwd = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;
    state.slip_angle = match speed > DRIFT_MIN_UPS {
        true => v.dot(right).atan2(v.dot(fwd)),
        false => 0.0,
    };
    let deg = state.slip_angle.abs().to_degrees();
    let threshold = if state.drifting {
        DRIFT_EXIT_DEG
    } else {
        DRIFT_SLIP_MIN_DEG
    };
    state.drifting =
        state.grounded >= 2 && speed > DRIFT_MIN_UPS && deg >= threshold && deg < SPIN_DEG;
    state.drift_time = match state.drifting && deg <= DRIFT_SLIP_MAX_DEG {
        true => state.drift_time + SIM_DT,
        false => 0.0,
    };
}

/// `VehicleTelemetry` of every car.
pub(crate) fn write_telemetry(
    mut cars: Query<(
        &CarSpec,
        &CarState,
        &Transform,
        &Velocity,
        &mut VehicleTelemetry,
    )>,
) {
    for (spec, state, transform, velocity, mut tel) in &mut cars {
        let fwd = transform.rotation * Vec3::NEG_Z;
        let wheels = std::array::from_fn(|i| {
            let (w, r) = (&spec.wheels[i], &state.wheels[i]);
            WheelState {
                grounded: r.grounded,
                compression: (1.0 - r.extension / w.travel).clamp(0.0, 1.0),
                spin: r.spin,
                steer: r.steer,
                surface: r.surface.filter(|_| r.grounded),
                slip: r.slip,
                offset: w.rest - r.extension,
            }
        });
        *tel = VehicleTelemetry {
            speed: velocity.linear.length(),
            forward_speed: velocity.linear.dot(fwd),
            rpm: state.rpm,
            throttle_load: state.throttle_load,
            grounded_wheels: state.grounded,
            airborne_time: state.air_time,
            slip_angle: state.slip_angle,
            drifting: state.drifting,
            surface: state.surface,
            wheels,
            boosting: state.boosting,
            up_dot: (transform.rotation * Vec3::Y).y,
            clean_landing: state.clean_landing,
            charge_rate: state.charge_rate,
        };
    }
}

/// `CarImpact` from rapier contact forces: strength from the velocity change, ground-like
/// contacts skipped (landings), weaker hits within the cooldown dropped (debounce).
pub(crate) fn report_impacts(
    mut events: MessageReader<ContactForceEvent>,
    mut tick: Local<u32>,
    mut cars: Query<(&CarSpec, &mut CarState, &Transform)>,
    mut impacts: MessageWriter<CarImpact>,
) {
    *tick = tick.wrapping_add(1);
    for ev in events.read() {
        // The manifold normal points from collider1 to collider2.
        for (me, other, sign) in [
            (ev.collider1, ev.collider2, -1.0),
            (ev.collider2, ev.collider1, 1.0),
        ] {
            let Ok((spec, mut state, transform)) = cars.get_mut(me) else {
                continue;
            };
            let normal = (ev.max_force_direction * sign).normalize_or_zero();
            if normal == Vec3::ZERO || normal.y.abs() > GROUND_NORMAL_Y {
                continue;
            }
            let dv = ev.total_force_magnitude * SIM_DT / spec.mass();
            let strength = (dv / IMPACT_FULL_DV_UPS).min(1.0);
            let recent = state.last_impact.is_some_and(|(t, s)| {
                tick.wrapping_sub(t) < IMPACT_COOLDOWN_TICKS && strength <= s * 1.5
            });
            if strength < IMPACT_MIN_STRENGTH || recent {
                continue;
            }
            state.last_impact = Some((*tick, strength));
            // ponytail: the point is where the normal leaves the chassis box, not the contact
            // manifold point; query `contact_pair` if effects need exact positions.
            impacts.write(CarImpact {
                car: me,
                other: Some(other),
                strength,
                point: spec.chassis_support(transform, -normal),
                normal,
            });
        }
    }
}
