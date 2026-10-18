//! The pure driving model: drive-force curve, speed-sensitive steering with the counter-steer
//! assist, spring-damper suspension, slip-based tyres with a friction ellipse, downforce. The
//! arcade assists live in `assists.rs`. No ECS here: the systems in `drive.rs` cast the wheel
//! rays and apply the results.
//!
//! Frames: car space has +Y up, −Z forward, +X right; steering `+` turns right.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::params::VehicleParams;

/// "0 → top" is measured to this fraction of the top speed (GDD §5.1 targets).
pub const TOP_SPEED_REACHED: f32 = 0.99;
/// Exponent k of the drive taper a(v) = A·(1 − v/V)^k: punchy launch, crisp arrival at top.
const DRIVE_TAPER: f32 = 0.5;
/// Share of the launch acceleration available in reverse.
const REVERSE_ACCEL_SHARE: f32 = 0.6;
/// Front share of the drive force of 4WD cars.
const AWD_FRONT_SHARE: f32 = 0.4;
/// Below this forward speed (u/s) the brake input reverses and throttle stops braking.
const REVERSE_SWITCH_UPS: f32 = 0.5;
/// Below this speed (u/s) with no pedal input the tyres hold the car (parking).
const HOLD_UPS: f32 = 0.5;
/// Engine braking / rolling resistance with no pedal input, u/s².
const COAST_DECEL: f32 = 5.0;
/// Deceleration above the effective top speed (boost ended, slow surface, downhill), u/s².
const OVERSPEED_DECEL: f32 = 7.0;
/// Speed excess (u/s) at which `OVERSPEED_DECEL` is fully applied.
const OVERSPEED_BAND_UPS: f32 = 3.0;
/// Extra rear-axle deceleration while the handbrake is held, u/s².
const HANDBRAKE_DECEL: f32 = 3.0;
/// Time constant (s) in which a gripping tyre removes its contact patch's slip velocity.
const TYRE_RELAX_SECS: f32 = 0.05;
/// Slip angle (rad) up to which a tyre delivers full grip.
const PEAK_SLIP_RAD: f32 = 8.0 * std::f32::consts::PI / 180.0;
/// Slip angle (rad) from which a tyre only delivers `SLIDE_GRIP`.
const SLIDE_SLIP_RAD: f32 = 20.0 * std::f32::consts::PI / 180.0;
/// Grip fraction of a fully sliding tyre (progressive breakaway between the two angles).
const SLIDE_GRIP: f32 = 0.78;
/// Contact patch speed (u/s) below which slip angles are not meaningful.
const SLIP_MIN_UPS: f32 = 1.0;
/// Sliding speed (u/s) of a contact patch that reports `WheelState::slip` = 1.
const SLIP_FULL_UPS: f32 = 6.0;
/// Tyre forces act this fraction of the way from the contact patch up to the centre of mass:
/// less body roll and no cornering rollovers (toy physics; flips come from jumps and hits).
const TYRE_FORCE_HEIGHT: f32 = 0.7;
/// Full-lock steering asks for this multiple of the tyres' lateral grip at any speed.
const STEER_OVERDRIVE: f32 = 1.15;
/// Smallest steering limit at very high speed, rad.
const MIN_STEER_RAD: f32 = 2.0 * std::f32::consts::PI / 180.0;
/// Rear-axle slip angle (rad) below which the counter-steer assist stays out of normal cornering.
const COUNTER_STEER_DEADZONE_RAD: f32 = 4.0 * std::f32::consts::PI / 180.0;
/// Forward speed (u/s) above which the counter-steer assist works.
const COUNTER_STEER_MIN_UPS: f32 = 2.0;
/// The progressive bump stop acts over this last fraction of the bump travel.
const BUMP_STOP_RANGE: f32 = 0.3;
/// Bump-stop stiffness and damping as multiples of the spring and damper rates.
const BUMP_STOP_STIFFNESS: f32 = 6.0;
const BUMP_STOP_DAMPING: f32 = 3.0;
/// Rebound (extension) damping as a multiple of the compression damping: the shock soaks up
/// the energy a hard landing stored in the spring instead of throwing the car back up.
const REBOUND_DAMPING: f32 = 2.5;
/// Digressive compression damping: linear up to this shaft speed (u/s), above it only this share
/// of the slope (high-speed blow-off), so ramp lips and curbs compress the spring instead of
/// kicking the car into the air.
const DAMPER_KNEE_UPS: f32 = 2.0;
const DAMPER_HIGH_SPEED_SLOPE: f32 = 0.3;
/// Upper bound of one wheel's suspension force in units of the car's weight.
const MAX_WHEEL_LOAD_G: f32 = 8.0;
/// The chassis collider uses this fraction of the body's length and width.
const CHASSIS_FOOTPRINT: f32 = 0.96;

/// Constants derived from a car's params once (spawn, hot reload).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct CarSpec {
    /// The car's tuning.
    pub params: VehicleParams,
    /// Drive acceleration at standstill, u/s².
    pub launch: f32,
    /// Principal inertia about the car's X (pitch), Y (yaw), Z (roll) axes.
    pub inertia: Vec3,
    /// Centre of mass in car space.
    pub com: Vec3,
    /// Wheels FL, FR, RL, RR.
    pub wheels: [WheelSpec; 4],
}

/// One wheel's suspension and tyre constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelSpec {
    /// Wheel centre at full bump in car space (the ray starts here).
    pub hardpoint: Vec3,
    /// Radius in u.
    pub radius: f32,
    /// Total travel (bump + droop) in u.
    pub travel: f32,
    /// Extension (hardpoint → wheel centre) at rest, in u.
    pub rest: f32,
    /// Extension at which the spring is unloaded, in u.
    pub free: f32,
    /// Spring rate per u.
    pub stiffness: f32,
    /// Damper rate per u/s.
    pub damping: f32,
    /// Steered front wheel.
    pub front: bool,
    /// Driven wheel.
    pub driven: bool,
    /// Lateral friction coefficient on asphalt.
    pub grip: f32,
}

impl CarSpec {
    /// Derives the constants of `params`.
    pub fn new(params: &VehicleParams) -> Self {
        let p = params;
        let m = p.mass;
        let omega = std::f32::consts::TAU * p.suspension.frequency;
        let wheel_mass = m / 4.0;
        let stiffness = wheel_mass * omega * omega;
        let wheels = p.wheel_layout().map(|(centre, radius, _)| {
            let front = centre.z < 0.0;
            WheelSpec {
                hardpoint: centre + Vec3::Y * p.suspension.bump,
                radius,
                travel: p.suspension.bump + p.suspension.droop,
                rest: p.suspension.bump,
                free: p.suspension.bump + GRAVITY / (omega * omega),
                stiffness,
                damping: 2.0 * p.suspension.damping * (stiffness * wheel_mass).sqrt(),
                front,
                driven: !front || p.drive == Drivetrain::Awd,
                grip: if front {
                    p.tyres.grip
                } else {
                    p.tyres.rear_grip
                },
            }
        });
        let (l, w, h) = (p.body.length, p.body.width, p.body.height);
        Self {
            params: p.clone(),
            launch: launch_accel(p.top_speed, p.accel_time),
            inertia: Vec3::new(h * h + l * l, w * w + l * l, w * w + h * h) * (m / 12.0),
            com: Vec3::Y * p.com_height,
            wheels,
        }
    }

    /// Mass.
    pub fn mass(&self) -> f32 {
        self.params.mass
    }

    /// Chassis collider box in car space: (centre, half extents), inside the body envelope and
    /// raised so the suspension can travel.
    pub fn chassis_box(&self) -> (Vec3, Vec3) {
        let p = &self.params;
        let bottom = p.collider_bottom();
        let half = Vec3::new(
            p.body.width * CHASSIS_FOOTPRINT,
            p.body.height - bottom,
            p.body.length * CHASSIS_FOOTPRINT,
        ) / 2.0;
        (Vec3::Y * (bottom + p.body.height) / 2.0, half)
    }

    /// The point where a ray from the chassis centre along `dir` (world) leaves the chassis box
    /// (impact positions).
    pub fn chassis_support(&self, transform: &Transform, dir: Vec3) -> Vec3 {
        let (centre, half) = self.chassis_box();
        let local = transform.rotation.inverse() * dir.normalize_or(Vec3::Y);
        let t = (half / local.abs().max(Vec3::splat(1e-6))).min_element();
        transform.translation + transform.rotation * (centre + local * t)
    }
}

/// Standstill drive acceleration so that a(v) = A·(1 − v/V)^k reaches `TOP_SPEED_REACHED`·V
/// after `secs` (closed form of the integral of dv / a(v)).
pub(crate) fn launch_accel(top_speed: f32, secs: f32) -> f32 {
    let k = DRIVE_TAPER;
    top_speed * (1.0 - (1.0 - TOP_SPEED_REACHED).powf(1.0 - k)) / ((1.0 - k) * secs)
}

/// Drive acceleration fraction 0–1 at speed `v` toward the top speed `top`.
pub(crate) fn drive_taper(v: f32, top: f32) -> f32 {
    if top <= 0.0 || v >= top {
        0.0
    } else {
        (1.0 - v.max(0.0) / top).powf(DRIVE_TAPER)
    }
}

/// Speed-sensitive steering limit (rad): full lock at low speed, at speed the angle at which
/// the front tyres reach `STEER_OVERDRIVE` × their grip (kinematic radius L / tan δ).
pub(crate) fn steer_limit(speed: f32, wheelbase: f32, lateral_accel: f32, lock: f32) -> f32 {
    let v2 = speed * speed;
    if v2 < 1e-3 {
        return lock;
    }
    (wheelbase * lateral_accel / v2)
        .atan()
        .clamp(MIN_STEER_RAD.min(lock), lock)
}

/// Grip fraction of a tyre at `slip` (|slip angle| in rad): 1 up to the peak, then a linear
/// fall to `SLIDE_GRIP` (progressive breakaway).
pub(crate) fn slip_grip(slip: f32) -> f32 {
    let t = ((slip - PEAK_SLIP_RAD) / (SLIDE_SLIP_RAD - PEAK_SLIP_RAD)).clamp(0.0, 1.0);
    1.0 + (SLIDE_GRIP - 1.0) * t
}

/// Scales a (longitudinal, lateral) tyre force into the friction ellipse of the two caps.
pub(crate) fn friction_ellipse(long: f32, lat: f32, cap_long: f32, cap_lat: f32) -> (f32, f32) {
    if cap_long <= 0.0 || cap_lat <= 0.0 {
        return (0.0, 0.0);
    }
    let usage = (long / cap_long).powi(2) + (lat / cap_lat).powi(2);
    if usage <= 1.0 {
        (long, lat)
    } else {
        let s = usage.sqrt().recip();
        (long * s, lat * s)
    }
}

/// Suspension force (≥ 0) at `extension` (hardpoint → wheel centre, u) while compressing at
/// `rate` (u/s, > 0 = compressing).
pub(crate) fn suspension_force(w: &WheelSpec, extension: f32, rate: f32, max: f32) -> f32 {
    let damping = if rate < 0.0 {
        w.damping * REBOUND_DAMPING
    } else {
        w.damping
    };
    // Compression blows off at high shaft speed; rebound stays firm (no pogo after landings).
    let shaft = match rate > DAMPER_KNEE_UPS {
        true => DAMPER_KNEE_UPS + DAMPER_HIGH_SPEED_SLOPE * (rate - DAMPER_KNEE_UPS),
        false => rate,
    };
    let mut f = w.stiffness * (w.free - extension) + damping * shaft;
    let stop = BUMP_STOP_RANGE * w.rest;
    if extension < stop {
        let depth = (stop - extension) / stop.max(1e-3);
        f += w.stiffness * BUMP_STOP_STIFFNESS * (stop - extension)
            + w.damping * BUMP_STOP_DAMPING * shaft.max(0.0) * depth.min(1.0);
    }
    f.clamp(0.0, max)
}

/// Kinematic state at the start of a tick (world space).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Kinematics {
    /// Body origin (ground-contact centre).
    pub pos: Vec3,
    /// Orientation.
    pub rot: Quat,
    /// Linear velocity.
    pub lin: Vec3,
    /// Angular velocity.
    pub ang: Vec3,
}

impl Kinematics {
    fn com(&self, spec: &CarSpec) -> Vec3 {
        self.pos + self.rot * spec.com
    }

    fn velocity_at(&self, com: Vec3, p: Vec3) -> Vec3 {
        self.lin + self.ang.cross(p - com)
    }
}

/// A wheel ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Distance from the hardpoint along the ray (−up).
    pub distance: f32,
    /// Hit point.
    pub point: Vec3,
    /// Surface normal.
    pub normal: Vec3,
    /// Surface × weather grip multiplier.
    pub grip: f32,
}

/// What the car is asked to do this tick (control resolved against effects and phase).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Command {
    /// Throttle 0–1.
    pub throttle: f32,
    /// Brake / reverse 0–1.
    pub brake: f32,
    /// Steering −1…1.
    pub steer: f32,
    /// Handbrake held.
    pub handbrake: bool,
    /// Held by the race (Intro / Countdown): brakes on, no drive.
    pub held: bool,
    /// Drive-force multiplier (power loss × boost force).
    pub drive_mul: f32,
    /// Effective top speed in u/s (surface, effects, boost, catch-up; capped).
    pub top_speed: f32,
    /// Grip multiplier of status effects.
    pub grip_mul: f32,
    /// Rear lateral grip multiplier (handbrake, ramped).
    pub rear_grip_mul: f32,
}

/// One wheel's result.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelOut {
    /// Touching the ground.
    pub grounded: bool,
    /// Extension (hardpoint → wheel centre), u.
    pub extension: f32,
    /// Steering angle, rad.
    pub steer: f32,
    /// Suspension load.
    pub load: f32,
    /// Contact patch speed along the wheel, u/s.
    pub long_speed: f32,
    /// Slip 0–1 for effects.
    pub slip: f32,
    /// The drive force was cut by the friction limit (wheelspin).
    pub spinning: bool,
}

/// The forces of one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepOut {
    /// Total force at the centre of mass (world).
    pub force: Vec3,
    /// Total torque about the centre of mass (world).
    pub torque: Vec3,
    /// Per-wheel results.
    pub wheels: [WheelOut; 4],
    /// Wheels on the ground.
    pub grounded: u8,
}

/// Longitudinal request of the whole car: (drive force along +forward, resisting force
/// magnitude opposing motion, rear-only resisting force, hold the car).
fn longitudinal(spec: &CarSpec, cmd: &Command, v_fwd: f32) -> (f32, f32, f32, bool) {
    let p = &spec.params;
    let m = p.mass;
    if cmd.held {
        return (0.0, 0.0, 0.0, true);
    }
    let (mut drive, mut resist) = (0.0, 0.0);
    let pedal = cmd.throttle.max(cmd.brake);
    if pedal < 0.05 {
        if v_fwd.abs() < HOLD_UPS {
            return (0.0, 0.0, 0.0, true);
        }
        resist += m * COAST_DECEL;
    } else if cmd.throttle >= cmd.brake {
        if v_fwd < -REVERSE_SWITCH_UPS {
            resist += m * p.brake_decel * cmd.throttle;
        } else {
            drive = m * spec.launch * drive_taper(v_fwd, cmd.top_speed) * cmd.throttle;
        }
    } else if v_fwd > REVERSE_SWITCH_UPS {
        resist += m * p.brake_decel * cmd.brake;
    } else {
        let reverse = spec.launch * REVERSE_ACCEL_SHARE;
        drive = -m * reverse * drive_taper(-v_fwd, p.reverse_speed) * cmd.brake;
    }
    drive *= cmd.drive_mul;
    if v_fwd > cmd.top_speed {
        resist += m * OVERSPEED_DECEL * ((v_fwd - cmd.top_speed) / OVERSPEED_BAND_UPS).min(1.0);
    }
    let handbrake = if cmd.handbrake && v_fwd.abs() > 2.0 {
        m * HANDBRAKE_DECEL
    } else {
        0.0
    };
    (drive, resist, handbrake, false)
}

/// Front-wheel steering angle (rad): player input within the speed-sensitive limit plus the
/// counter-steer assist that points the wheels along the rear axle's slide.
pub(crate) fn steer_angle(spec: &CarSpec, kin: &Kinematics, steer: f32) -> f32 {
    let p = &spec.params;
    let lock = p.steering.lock_deg.to_radians();
    let fwd = kin.rot * Vec3::NEG_Z;
    let right = kin.rot * Vec3::X;
    let v_fwd = kin.lin.dot(fwd);
    let lateral = p.tyres.grip * GRAVITY * STEER_OVERDRIVE;
    let input = steer.clamp(-1.0, 1.0) * steer_limit(v_fwd.abs(), p.wheelbase, lateral, lock);
    let mut assist = 0.0;
    if v_fwd > COUNTER_STEER_MIN_UPS {
        let com = kin.com(spec);
        let rear = kin.velocity_at(com, kin.pos + kin.rot * Vec3::Z * (p.wheelbase / 2.0));
        let slip = rear.dot(right).atan2(rear.dot(fwd));
        let excess = slip.abs() - COUNTER_STEER_DEADZONE_RAD;
        if excess > 0.0 {
            assist = slip.signum() * excess * p.steering.counter_steer;
        }
    }
    (input + assist).clamp(-lock, lock)
}

/// Ground forces of one tick: suspension, drive/brake and lateral tyre forces, downforce.
pub(crate) fn step(
    spec: &CarSpec,
    kin: &Kinematics,
    contacts: &[Option<Contact>; 4],
    cmd: &Command,
) -> StepOut {
    let p = &spec.params;
    let m = p.mass;
    let wheel_mass = m / 4.0;
    let up = kin.rot * Vec3::Y;
    let fwd = kin.rot * Vec3::NEG_Z;
    let com = kin.com(spec);
    let v_fwd = kin.lin.dot(fwd);
    let delta = steer_angle(spec, kin, cmd.steer);
    let mut out = StepOut::default();

    let max_load = m * GRAVITY * MAX_WHEEL_LOAD_G;
    for (i, (w, c)) in spec.wheels.iter().zip(contacts).enumerate() {
        let wo = &mut out.wheels[i];
        wo.steer = if w.front { delta } else { 0.0 };
        wo.extension = w.travel;
        let Some(c) = c else { continue };
        let extension = c.distance - w.radius;
        let hardpoint = kin.pos + kin.rot * w.hardpoint;
        let rate = -kin.velocity_at(com, hardpoint).dot(up);
        wo.grounded = true;
        wo.extension = extension.clamp(0.0, w.travel);
        wo.load = suspension_force(w, extension, rate, max_load);
        out.grounded += 1;
    }
    if out.grounded == 0 {
        return out;
    }

    let (drive, resist, handbrake, hold) = longitudinal(spec, cmd, v_fwd);
    let shares = drive_shares(spec, &out.wheels);
    let total_load: f32 = out.wheels.iter().map(|w| w.load).sum::<f32>().max(1e-3);
    let rear_load: f32 = out.wheels[2..]
        .iter()
        .map(|w| w.load)
        .sum::<f32>()
        .max(1e-3);
    let lift = up * (TYRE_FORCE_HEIGHT * p.com_height);

    for (i, (w, c)) in spec.wheels.iter().zip(contacts).enumerate() {
        let Some(c) = c else { continue };
        let wo = &mut out.wheels[i];
        let n = c.normal;
        let dir = Quat::from_axis_angle(up, -wo.steer) * fwd;
        let along = (dir - n * dir.dot(n)).normalize_or_zero();
        let side = along.cross(n);
        let v = kin.velocity_at(com, c.point);
        let (v_long, v_lat) = (v.dot(along), v.dot(side));
        let grip = c.grip * cmd.grip_mul;
        let rear_mul = if w.front { 1.0 } else { cmd.rear_grip_mul };
        let slip = if v_long.hypot(v_lat) > SLIP_MIN_UPS {
            v_lat.abs().atan2(v_long.abs())
        } else {
            0.0
        };
        let cap_lat = w.grip * grip * rear_mul * wo.load * slip_grip(slip);
        let cap_long = p.tyres.traction * grip * wo.load;
        let lat = (-v_lat * wheel_mass / TYRE_RELAX_SECS).clamp(-cap_lat, cap_lat);
        let mut long = drive * shares[i];
        if hold {
            long = -v_long * wheel_mass / TYRE_RELAX_SECS;
        } else {
            let mut r = resist * wo.load / total_load;
            if !w.front {
                r += handbrake * wo.load / rear_load;
            }
            long -= v_long.signum() * r.min(v_long.abs() * wheel_mass / TYRE_RELAX_SECS);
        }
        let (long_f, lat_f) = friction_ellipse(long, lat, cap_long, cap_lat);
        wo.spinning = drive.abs() > 0.0 && long_f.abs() < 0.95 * long.abs();
        wo.long_speed = v_long;
        let sliding = (v_lat.abs() - cap_lat.max(1e-3) * TYRE_RELAX_SECS / wheel_mass).max(0.0);
        wo.slip = (sliding / SLIP_FULL_UPS).clamp(0.0, 1.0);
        let tyre = along * long_f + side * lat_f;
        let suspension = n * wo.load;
        out.force += suspension + tyre;
        out.torque += (c.point - com).cross(suspension) + (c.point + lift - com).cross(tyre);
    }
    if out.grounded >= 2 && p.downforce > 0.0 {
        let k = (v_fwd.max(0.0) / p.top_speed).powi(2);
        out.force -= up * (m * GRAVITY * p.downforce * k);
    }
    out
}

/// Drive-force share of each wheel: 4WD splits front/rear (all to one axle when the other is
/// airborne), every axle splits equally between its grounded wheels (locked differentials).
fn drive_shares(spec: &CarSpec, wheels: &[WheelOut; 4]) -> [f32; 4] {
    let on = |a: usize, b: usize| u8::from(wheels[a].grounded) + u8::from(wheels[b].grounded);
    let (front_on, rear_on) = (on(0, 1), on(2, 3));
    let awd = spec.params.drive == Drivetrain::Awd;
    let (mut front, mut rear) = if awd {
        (AWD_FRONT_SHARE, 1.0 - AWD_FRONT_SHARE)
    } else {
        (0.0, 1.0)
    };
    if awd && front_on == 0 {
        (front, rear) = (0.0, 1.0);
    } else if awd && rear_on == 0 {
        (front, rear) = (1.0, 0.0);
    }
    let mut shares = [0.0; 4];
    for (i, w) in wheels.iter().enumerate() {
        if w.grounded {
            shares[i] = match i < 2 {
                true => front / f32::from(front_on.max(1)),
                false => rear / f32::from(rear_on.max(1)),
            };
        }
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> CarSpec {
        let p: VehicleParams =
            ron::from_str(include_str!("../../../assets/data/vehicles/hornet.ron"))
                .expect("parses");
        CarSpec::new(&p)
    }

    #[test]
    fn launch_accel_reaches_99_percent_in_accel_time() {
        // Integrate dv/dt = A·taper(v) with a fine step and compare to the closed form.
        let (top, secs) = (30.0, 3.0);
        let a = launch_accel(top, secs);
        let (mut v, mut t, dt) = (0.0_f32, 0.0_f32, 1e-4_f32);
        while v < TOP_SPEED_REACHED * top {
            v += a * drive_taper(v, top) * dt;
            t += dt;
        }
        assert!((t - secs).abs() < 0.01, "t = {t}");
    }

    #[test]
    fn drive_taper_ends_at_top_speed() {
        assert_eq!(drive_taper(0.0, 30.0), 1.0);
        assert!(drive_taper(15.0, 30.0) > 0.7);
        assert_eq!(drive_taper(30.0, 30.0), 0.0);
        assert_eq!(drive_taper(40.0, 30.0), 0.0);
        assert_eq!(drive_taper(-3.0, 30.0), 1.0);
    }

    #[test]
    fn steering_is_speed_sensitive() {
        let lock = 30f32.to_radians();
        assert_eq!(steer_limit(0.0, 1.3, 35.0, lock), lock);
        assert_eq!(steer_limit(2.0, 1.3, 35.0, lock), lock);
        let mid = steer_limit(15.0, 1.3, 35.0, lock);
        let top = steer_limit(30.0, 1.3, 35.0, lock);
        assert!(
            mid < lock && top < mid && top >= MIN_STEER_RAD,
            "{mid} {top}"
        );
        // At the limit the kinematic lateral acceleration v²·tan(δ)/L equals the request.
        assert!((15.0f32.powi(2) * mid.tan() / 1.3 - 35.0).abs() < 0.5);
    }

    #[test]
    fn slip_grip_breaks_away_progressively() {
        assert_eq!(slip_grip(0.0), 1.0);
        assert_eq!(slip_grip(PEAK_SLIP_RAD), 1.0);
        let mid = slip_grip((PEAK_SLIP_RAD + SLIDE_SLIP_RAD) / 2.0);
        assert!(mid < 1.0 && mid > SLIDE_GRIP);
        assert!((slip_grip(1.0) - SLIDE_GRIP).abs() < 1e-6);
    }

    #[test]
    fn friction_ellipse_keeps_direction_and_caps() {
        assert_eq!(friction_ellipse(1.0, 1.0, 10.0, 10.0), (1.0, 1.0));
        let (l, s) = friction_ellipse(10.0, 10.0, 10.0, 10.0);
        assert!(((l / 10.0).powi(2) + (s / 10.0).powi(2) - 1.0).abs() < 1e-5);
        assert!((l - s).abs() < 1e-6);
        assert_eq!(friction_ellipse(5.0, 5.0, 0.0, 10.0), (0.0, 0.0));
    }

    #[test]
    fn suspension_holds_a_quarter_of_the_weight_at_rest() {
        let s = spec();
        let w = s.wheels[0];
        let f = suspension_force(&w, w.rest, 0.0, f32::INFINITY);
        assert!((f - s.mass() * GRAVITY / 4.0).abs() < 1e-3, "{f}");
        assert_eq!(suspension_force(&w, w.free + 1.0, 0.0, f32::INFINITY), 0.0);
        assert!(
            suspension_force(&w, w.rest, 1.0, f32::INFINITY) > f,
            "damper adds load"
        );
        assert!(
            suspension_force(&w, 0.0, 0.0, f32::INFINITY) > 2.0 * f,
            "bump stop"
        );
    }

    #[test]
    fn drive_shares_follow_the_drivetrain() {
        let mut s = spec();
        let grounded = WheelOut {
            grounded: true,
            ..default()
        };
        let all = [grounded; 4];
        s.params.drive = Drivetrain::Rwd;
        assert_eq!(drive_shares(&s, &all), [0.0, 0.0, 0.5, 0.5]);
        s.params.drive = Drivetrain::Awd;
        let shares = drive_shares(&s, &all);
        assert!((shares.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        let mut front_air = all;
        front_air[0].grounded = false;
        front_air[1].grounded = false;
        assert_eq!(drive_shares(&s, &front_air), [0.0, 0.0, 0.5, 0.5]);
        let mut one_rear = all;
        one_rear[3].grounded = false;
        assert!((drive_shares(&s, &one_rear)[2] - (1.0 - AWD_FRONT_SHARE)).abs() < 1e-6);
    }

    #[test]
    fn longitudinal_modes() {
        let s = spec();
        let cmd = Command {
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            handbrake: false,
            held: false,
            drive_mul: 1.0,
            top_speed: s.params.top_speed,
            grip_mul: 1.0,
            rear_grip_mul: 1.0,
        };
        assert!(longitudinal(&s, &cmd, 0.0).3, "no input at rest holds");
        let coast = longitudinal(&s, &cmd, 10.0);
        assert!(coast.1 > 0.0 && !coast.3);
        let gas = longitudinal(
            &s,
            &Command {
                throttle: 1.0,
                ..cmd
            },
            0.0,
        );
        assert!((gas.0 - s.mass() * s.launch).abs() < 1e-3);
        let brake = longitudinal(&s, &Command { brake: 1.0, ..cmd }, 10.0);
        assert_eq!(brake.0, 0.0);
        assert!((brake.1 - s.mass() * s.params.brake_decel).abs() < 1e-3);
        let reverse = longitudinal(&s, &Command { brake: 1.0, ..cmd }, 0.0);
        assert!(reverse.0 < 0.0);
        let cut = longitudinal(
            &s,
            &Command {
                throttle: 1.0,
                drive_mul: 0.0,
                ..cmd
            },
            0.0,
        );
        assert_eq!(cut.0, 0.0, "power loss cuts the drive");
        let held = longitudinal(
            &s,
            &Command {
                throttle: 1.0,
                held: true,
                ..cmd
            },
            0.0,
        );
        assert!(held.3 && held.0 == 0.0);
        let over = longitudinal(
            &s,
            &Command {
                throttle: 1.0,
                ..cmd
            },
            s.params.top_speed + 5.0,
        );
        assert!(over.0 == 0.0 && over.1 > 0.0);
    }
}
