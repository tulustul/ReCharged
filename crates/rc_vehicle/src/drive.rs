//! `SimSet::Vehicle`: every car's forces for the coming rapier step — wheel rays → model →
//! `ExternalForce`; boost resolution; air control; auto-right; wind on airborne cars.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::assists;
use crate::charge;
use crate::model::{self, CarSpec, Command, Contact, Kinematics, StepOut};
use crate::state::CarState;

/// Rear grip ramp of the handbrake, per second (down: grab the slide, up: smooth catch).
const HANDBRAKE_GRIP_DROP_RATE: f32 = 8.0;
const HANDBRAKE_GRIP_RISE_RATE: f32 = 3.0;
/// Fore/aft offset of a wheel's outer probe rays as a fraction of its radius.
const WHEEL_PROBE: f32 = 0.7;
/// A wheel ray hit counts as ground only on surfaces facing the car's up axis this much.
const MIN_GROUND_NORMAL_DOT: f32 = 0.5;
/// Auto-right (GDD §5.4): on its side/roof (`up_dot` below this) …
const AUTO_RIGHT_UP_DOT: f32 = 0.3;
/// … slower than this (u/s) …
const AUTO_RIGHT_MAX_UPS: f32 = 2.0;
/// … for this long (s) → hop and roll upright.
const AUTO_RIGHT_DELAY_SECS: f32 = 1.0;
/// Vertical speed of the auto-right hop, u/s.
const AUTO_RIGHT_HOP_UPS: f32 = 6.0;
/// Longest auto-right manoeuvre, s; it ends early once upright (`up_dot` above the second) and
/// calm (angular speed below the third, rad/s).
const AUTO_RIGHT_SECS: f32 = 1.5;
const AUTO_RIGHT_DONE_UP_DOT: f32 = 0.97;
const AUTO_RIGHT_DONE_RATE: f32 = 1.0;
/// The handbrake engages a drift above this forward speed, u/s, once steering passes the second.
const DRIFT_ENGAGE_UPS: f32 = 6.0;
const DRIFT_ENGAGE_STEER: f32 = 0.2;
/// The stability assist works above this forward speed, u/s.
const STABILITY_MIN_UPS: f32 = 4.0;
/// Toy shock absorber: a landing closing faster than the bump travel can absorb at this
/// deceleration (u/s²) loses the excess at touchdown instead of slamming the chassis into the
/// ground (rapier would bounce it back up).
const LANDING_ABSORB_DECEL: f32 = 110.0;
/// The wheel rays look ahead by this many ticks of the current fall, so fast landings are seen.
const LANDING_LOOKAHEAD_TICKS: f32 = 1.5;
/// Deceleration (u/s²) of a car scraping along on its body (roof, side): the low chassis
/// friction keeps walls slippery, this lets a flipped car come to rest and auto-right.
const SCRAPE_DECEL: f32 = 12.0;
/// Acceleration of airborne cars per u/s of wind (storm gusts push them slightly), 1/s.
const WIND_PUSH: f32 = 0.15;
/// Spin-down of free (undriven, airborne) wheels, 1/s; spin-up of driven ones, 1/s.
const WHEEL_SPIN_DECAY: f32 = 1.0;
const WHEEL_SPIN_UP: f32 = 6.0;

/// Collision groups of the wheel rays: track and static props only.
fn wheel_ray_groups() -> CollisionGroups {
    CollisionGroups::new(
        Group::from_bits_truncate(layers::CAR),
        Group::from_bits_truncate(layers::TRACK | layers::PROP),
    )
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct DriveItem {
    entity: Entity,
    spec: &'static CarSpec,
    state: &'static mut CarState,
    control: &'static CarControl,
    effects: &'static StatusEffects,
    catch_up: Option<&'static CatchUp>,
    charge: &'static Charge,
    transform: &'static Transform,
    velocity: &'static mut Velocity,
    force: &'static mut ExternalForce,
    impulse: &'static mut ExternalImpulse,
    touching: &'static CollidingEntities,
}

/// Forces of every car for this tick.
pub(crate) fn drive_cars(
    rapier: ReadRapierContext,
    phase: Option<Res<State<RacePhase>>>,
    weather: Option<Res<WeatherState>>,
    surfaces: Query<&Surface>,
    mut cars: Query<DriveItem>,
) -> Result {
    let ctx = rapier.single()?;
    let held = phase.is_some_and(|p| matches!(p.get(), RacePhase::Intro | RacePhase::Countdown));
    let (wet, snow, wind) = weather.map_or((0.0, 0.0, Vec3::ZERO), |w| {
        (w.wetness, w.snow_cover, w.wind)
    });
    for mut car in &mut cars {
        let kin = Kinematics {
            pos: car.transform.translation,
            rot: car.transform.rotation,
            lin: car.velocity.linear,
            ang: car.velocity.angular,
        };
        let probe = probe_wheels(&ctx, car.entity, car.spec, &kin, &surfaces, (wet, snow));
        let item = &mut car;
        // Only real landings (no wheel down last tick): ramp entries are the suspension's job.
        if probe.ground_normal != Vec3::ZERO && item.state.grounded == 0 {
            absorb_landing(item, probe.ground_normal.normalize());
        }
        let cmd = command(item, probe.surface_top, held);
        let mut out = model::step(item.spec, &kin, &probe.contacts, &cmd);
        slip_assist(item, &kin, &cmd, &probe.contacts, &mut out);
        air_and_recovery(item, &kin, &cmd, &mut out, wind);
        record_wheels(item, &kin, &out, &probe, &cmd);
        item.force.force = out.force;
        item.force.torque = out.torque;
    }
    Ok(())
}

/// What the wheel rays found this tick.
struct Probe {
    contacts: [Option<Contact>; 4],
    kinds: [Option<SurfaceKind>; 4],
    /// Mean top-speed factor of the surfaces under the wheels (1 when airborne).
    surface_top: f32,
    /// Sum of the ground normals seen, including within the landing look-ahead.
    ground_normal: Vec3,
}

/// Casts every wheel: three rays per wheel approximate the round tyre fore and aft (the ground
/// point the wheel circle touches first wins: smooth over ramp lips, curbs and steps); the rays
/// look ahead by the current fall so fast landings are seen in time.
fn probe_wheels(
    ctx: &RapierContext,
    entity: Entity,
    spec: &CarSpec,
    kin: &Kinematics,
    surfaces: &Query<&Surface>,
    (wet, snow): (f32, f32),
) -> Probe {
    // ponytail: wheels only ride on fixed track/prop colliders (the chassis handles dynamic
    // props and cars); include dynamic bodies with reaction impulses to drive over other cars.
    let filter = QueryFilter::exclude_dynamic()
        .exclude_sensors()
        .exclude_rigid_body(entity)
        .groups(wheel_ray_groups());
    let up = kin.rot * Vec3::Y;
    let fwd = kin.rot * Vec3::NEG_Z;
    let lookahead = (-kin.lin.dot(up)).max(0.0) * SIM_DT * LANDING_LOOKAHEAD_TICKS;
    let mut probe = Probe {
        contacts: [None; 4],
        kinds: [None; 4],
        surface_top: 1.0,
        ground_normal: Vec3::ZERO,
    };
    let mut top = (0.0, 0.0);
    for (i, w) in spec.wheels.iter().enumerate() {
        let hardpoint = kin.pos + kin.rot * w.hardpoint;
        let mut best: Option<(f32, Entity, RayIntersection)> = None;
        for k in [-1.0, 0.0, 1.0] {
            let offset = k * WHEEL_PROBE * w.radius;
            let drop = (w.radius * w.radius - offset * offset).sqrt();
            let max = w.travel + drop + lookahead;
            let origin = hardpoint + fwd * offset;
            if let Some((e, hit)) = ctx.cast_ray_and_get_normal(origin, -up, max, true, filter)
                && hit.normal.dot(up) >= MIN_GROUND_NORMAL_DOT
                && best.is_none_or(|(ext, ..)| hit.time_of_impact - drop < ext)
            {
                best = Some((hit.time_of_impact - drop, e, hit));
            }
        }
        let Some((extension, hit_entity, hit)) = best else {
            continue;
        };
        probe.ground_normal += hit.normal;
        if extension > w.travel {
            continue;
        }
        let kind = surfaces
            .get(hit_entity)
            .map_or(SurfaceKind::default(), |s| s.0);
        let props = kind.props().with_weather(wet, snow);
        probe.contacts[i] = Some(Contact {
            distance: extension + w.radius,
            point: hit.point,
            normal: hit.normal,
            grip: props.grip,
        });
        probe.kinds[i] = Some(kind);
        top = (top.0 + props.top_speed, top.1 + 1.0);
    }
    if top.1 > 0.0 {
        probe.surface_top = top.0 / top.1;
    }
    probe
}

/// Toy shock absorber: removes the closing speed into the ground beyond what the bump travel
/// can absorb (see `LANDING_ABSORB_DECEL`).
fn absorb_landing(car: &mut DriveItemItem<'_, '_>, normal: Vec3) {
    let absorbable = (2.0 * LANDING_ABSORB_DECEL * car.spec.params.suspension.bump).sqrt();
    let closing = -car.velocity.linear.dot(normal);
    // The landing's strength is judged from the speed before the absorber took its share.
    car.state.closing_speed = car.state.closing_speed.max(closing);
    if closing > absorbable {
        car.velocity.linear += normal * (closing - absorbable);
    }
}

/// Resolves control, effects, boost and race phase into this tick's [`Command`].
fn command(car: &mut DriveItemItem<'_, '_>, surface_top: f32, held: bool) -> Command {
    let control = car.control;
    let effects = car.effects;
    let (boost_top, boost_force) = charge::resolve_boost(
        &mut car.state,
        effects,
        control.boost && !held,
        car.charge.0,
    );
    let base = car.spec.params.top_speed;
    let catch_up = car.catch_up.map_or(0.0, |c| c.0.clamp(-0.06, 0.06));
    let top = base * surface_top * effects.top_speed_mul() * boost_top * (1.0 + catch_up);
    car.state.top_speed = top.min(base * TOP_SPEED_CAP);

    let handbrake = control.drift && !held;
    let target = if handbrake {
        car.spec.params.tyres.drift_grip
    } else {
        1.0
    };
    let rate = if target < car.state.rear_grip {
        HANDBRAKE_GRIP_DROP_RATE
    } else {
        HANDBRAKE_GRIP_RISE_RATE
    };
    let g = &mut car.state.rear_grip;
    *g += (target - *g).clamp(-rate * SIM_DT, rate * SIM_DT);

    Command {
        throttle: control.throttle.clamp(0.0, 1.0),
        brake: control.brake.clamp(0.0, 1.0),
        steer: control.steer.clamp(-1.0, 1.0),
        handbrake,
        held,
        drive_mul: effects.power_mul().max(0.0) * boost_force,
        top_speed: car.state.top_speed,
        grip_mul: effects.grip_mul().max(0.0),
        rear_grip_mul: car.state.rear_grip,
    }
}

/// Arcade drift and stability: the handbrake engages a drift in the steering direction whose
/// slip angle is held by a yaw servo; otherwise slides beyond a dead zone are caught. Both scale
/// with the tyres' grip, so ice and oil still let the car spin.
fn slip_assist(
    car: &mut DriveItemItem<'_, '_>,
    kin: &Kinematics,
    cmd: &Command,
    contacts: &[Option<Contact>; 4],
    out: &mut StepOut,
) {
    let spec = car.spec;
    let state = &mut car.state;
    let (grounded, grip) = contacts
        .iter()
        .flatten()
        .fold((0.0, 0.0), |(n, g), c| (n + 1.0, g + c.grip));
    let up = kin.rot * Vec3::Y;
    let v_fwd = kin.lin.dot(kin.rot * Vec3::NEG_Z);
    if grounded < 2.0 || cmd.held {
        state.drift_dir = 0.0;
        return;
    }
    let grip = grip / grounded * cmd.grip_mul;
    let planar = |v: Vec3| v - up * v.dot(up);
    let (a, b) = (planar(state.pre_velocity), planar(kin.lin));
    let path_rate = match a.length() > 1.0 && b.length() > 1.0 {
        true => a.cross(b).dot(up).atan2(a.dot(b)) / SIM_DT,
        false => 0.0,
    };
    let (target, strength) = if cmd.handbrake && v_fwd > DRIFT_ENGAGE_UPS {
        if state.drift_dir == 0.0 && cmd.steer.abs() > DRIFT_ENGAGE_STEER {
            state.drift_dir = -cmd.steer.signum();
        }
        match state.drift_dir {
            0.0 => (None, spec.params.steering.stability * grip),
            dir => {
                out.force += assists::drift_turn_force(spec, kin, dir, cmd.steer, grip);
                (Some(assists::drift_target(spec, dir, cmd.steer)), grip)
            }
        }
    } else {
        state.drift_dir = 0.0;
        // Forward only: reversing has a slip angle near 180° and must not be "caught".
        if v_fwd < STABILITY_MIN_UPS {
            return;
        }
        (None, spec.params.steering.stability * grip)
    };
    out.torque += assists::slip_assist_torque(spec, kin, path_rate, target, strength);
}

/// Air control, auto-right and wind (added to the ground forces of `out`).
fn air_and_recovery(
    car: &mut DriveItemItem<'_, '_>,
    kin: &Kinematics,
    cmd: &Command,
    out: &mut StepOut,
    wind: Vec3,
) {
    let spec = car.spec;
    let state = &mut car.state;
    let up_dot = (kin.rot * Vec3::Y).y;
    if state.righting > 0.0 {
        state.righting -= SIM_DT;
        if up_dot > AUTO_RIGHT_DONE_UP_DOT && kin.ang.length() < AUTO_RIGHT_DONE_RATE {
            state.righting = 0.0;
        }
        out.torque += assists::righting_torque(spec, kin);
    } else if out.grounded == 0 && !cmd.held {
        out.torque += assists::air_torque(spec, kin, cmd);
    }
    if out.grounded == 0 {
        out.force += wind * (spec.mass() * WIND_PUSH);
    }
    if out.grounded < 2 && !car.touching.is_empty() {
        let planar = kin.lin.with_y(0.0);
        let decel = planar.length().min(SCRAPE_DECEL * SIM_DT) / SIM_DT;
        out.force -= planar.normalize_or_zero() * (spec.mass() * decel);
    }
    let resting_wrong = up_dot < AUTO_RIGHT_UP_DOT
        && kin.lin.length() < AUTO_RIGHT_MAX_UPS
        && out.grounded < 3
        && state.righting <= 0.0;
    state.flip_timer = if resting_wrong {
        state.flip_timer + SIM_DT
    } else {
        0.0
    };
    if state.flip_timer >= AUTO_RIGHT_DELAY_SECS {
        state.flip_timer = 0.0;
        state.righting = AUTO_RIGHT_SECS;
        car.impulse.impulse += Vec3::Y * (spec.mass() * AUTO_RIGHT_HOP_UPS);
    }
}

/// Stores the wheel results (visual spin, steer, suspension, surface, slip) and engine values.
fn record_wheels(
    car: &mut DriveItemItem<'_, '_>,
    kin: &Kinematics,
    out: &StepOut,
    probe: &Probe,
    cmd: &Command,
) {
    let spec = car.spec;
    let state = &mut car.state;
    let (contacts, kinds) = (&probe.contacts, &probe.kinds);
    state.grounded = out.grounded;
    // Start-of-tick velocity (before the landing absorber): landing strength, path rate.
    state.pre_velocity = kin.lin;
    let mut driven_speed = (0.0_f32, 0.0_f32);
    let top = state.top_speed;
    for (i, (w, o)) in spec.wheels.iter().zip(&out.wheels).enumerate() {
        let r = &mut state.wheels[i];
        r.grounded = o.grounded;
        r.extension = o.extension;
        r.steer = o.steer;
        r.surface = kinds[i];
        r.normal = contacts[i].map_or(Vec3::Y, |c| c.normal);
        r.contact = contacts[i].map_or(Vec3::ZERO, |c| c.point);
        r.slip = if o.spinning { o.slip.max(0.7) } else { o.slip };
        let free_spin = cmd.throttle * cmd.drive_mul * top / w.radius;
        r.spin_rate = match (o.grounded, w.driven) {
            (true, _) if !o.spinning => o.long_speed / w.radius,
            (true, _) => o.long_speed.max(0.0) / w.radius + 0.5 * free_spin,
            (false, true) if !cmd.held => {
                r.spin_rate + (free_spin - r.spin_rate) * (WHEEL_SPIN_UP * SIM_DT).min(1.0)
            }
            _ => r.spin_rate * (1.0 - WHEEL_SPIN_DECAY * SIM_DT),
        };
        r.spin = (r.spin + r.spin_rate * SIM_DT).rem_euclid(std::f32::consts::TAU);
        if w.driven {
            driven_speed = (
                driven_speed.0 + r.spin_rate.abs() * w.radius,
                driven_speed.1 + 1.0,
            );
        }
    }
    state.rpm = match cmd.held {
        true => cmd.throttle,
        false => (driven_speed.0 / driven_speed.1.max(1.0) / state.top_speed.max(1.0)).min(1.0),
    };
    state.throttle_load = cmd.throttle.max(cmd.brake) * cmd.drive_mul.min(1.0);
    state.surface = dominant_surface(kinds);
}

/// The surface under most wheels (ties: the first in wheel order).
fn dominant_surface(kinds: &[Option<SurfaceKind>; 4]) -> Option<SurfaceKind> {
    let count = |k: SurfaceKind| kinds.iter().filter(|x| **x == Some(k)).count();
    kinds
        .iter()
        .enumerate()
        .filter_map(|(i, k)| k.map(|k| (i, k)))
        .max_by_key(|&(i, k)| (count(k), std::cmp::Reverse(i)))
        .map(|(_, k)| k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_surface_is_the_majority() {
        use SurfaceKind::*;
        assert_eq!(dominant_surface(&[None; 4]), None);
        assert_eq!(
            dominant_surface(&[Some(Ice), Some(Grass), Some(Grass), None]),
            Some(Grass)
        );
        assert_eq!(dominant_surface(&[Some(Ice), None, None, None]), Some(Ice));
    }
}
