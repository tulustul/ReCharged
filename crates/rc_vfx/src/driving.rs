//! Driving effects from telemetry, every frame per racer: drift sparks coloured and sized by the
//! charge gain (GDD §5.3), wheel debris by surface and weather (dust, grass, mud, water spray,
//! snow powder, tyre smoke) and the boost flame with electric arcs.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::cars::{CarFx, Eyes, Poses, SKID_SLIP, car_velocity};
use crate::fx::{Fx, zigzag};
use crate::particles::{Facing, Look, Particle, Shape};
use crate::pool::Emitter;

/// Drift charge rate that maps to full spark size/colour (GDD §5.3: drift peaks at 22 %/s).
pub const DRIFT_RATE_FULL: f32 = 0.22;

pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (sparks_and_debris, boost_flames).in_set(crate::VfxSystems::Emit),
    );
}

/// Drift spark colour for a charge-rate fraction `f` (0 = just charging, 1 = peak): amber →
/// electric cyan → violet, brighter as the gain grows.
pub fn spark_color(f: f32) -> LinearRgba {
    let f = f.clamp(0.0, 1.25);
    let amber = Vec3::new(1.0, 0.5, 0.12);
    let cyan = Vec3::new(0.3, 0.85, 1.0);
    let violet = Vec3::new(0.85, 0.42, 1.0);
    let c = if f < 0.5 {
        amber.lerp(cyan, f / 0.5)
    } else {
        cyan.lerp(violet, ((f - 0.5) / 0.5).min(1.0))
    };
    let intensity = 6.0 + 10.0 * f.min(1.0);
    LinearRgba::rgb(c.x * intensity, c.y * intensity, c.z * intensity)
}

/// Drift spark width (u) for a charge-rate fraction.
pub fn spark_size(f: f32) -> f32 {
    0.035 + 0.05 * f.clamp(0.0, 1.25)
}

const SPARK: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Velocity,
    emissive: true,
    color: [
        LinearRgba::rgb(8.0, 5.0, 2.0),
        LinearRgba::rgb(3.0, 0.6, 0.1),
    ],
    size: [0.05, 0.02],
    fade_out: 0.35,
    drag: 1.5,
    gravity: 18.0,
    stretch: 0.028,
    aspect: 2.5,
    bounce: 0.35,
    ..Look::DEFAULT
};

const HOT_SPOT: Look = Look {
    shape: Shape::Core,
    emissive: true,
    size: [0.3, 0.15],
    fade_out: 0.8,
    ..Look::DEFAULT
};

const DUST: Look = Look {
    shape: Shape::Puff,
    color: [
        LinearRgba::new(0.42, 0.31, 0.2, 0.55),
        LinearRgba::new(0.5, 0.4, 0.3, 0.0),
    ],
    size: [0.4, 2.2],
    fade_in: 0.15,
    fade_out: 0.7,
    drag: 2.2,
    gravity: -0.3,
    ..Look::DEFAULT
};

const SMOKE: Look = Look {
    color: [
        LinearRgba::new(0.8, 0.8, 0.82, 0.2),
        LinearRgba::new(0.85, 0.85, 0.88, 0.0),
    ],
    size: [0.4, 2.0],
    ..DUST
};

const CHIP: Look = Look {
    shape: Shape::Chip,
    facing: Facing::Flat,
    color: [
        LinearRgba::new(0.22, 0.42, 0.1, 1.0),
        LinearRgba::new(0.25, 0.38, 0.12, 1.0),
    ],
    size: [0.14, 0.12],
    fade_out: 0.25,
    drag: 0.8,
    gravity: 22.0,
    bounce: 0.2,
    ..Look::DEFAULT
};

const SPRAY: Look = Look {
    shape: Shape::Drop,
    facing: Facing::Velocity,
    color: [
        LinearRgba::new(0.85, 0.9, 0.95, 0.65),
        LinearRgba::new(0.85, 0.9, 0.97, 0.0),
    ],
    size: [0.045, 0.03],
    fade_out: 0.4,
    drag: 0.6,
    gravity: 24.0,
    stretch: 0.012,
    aspect: 1.3,
    ..Look::DEFAULT
};

/// White water thrown up in sheets (short, bright puffs).
const SPLASH: Look = Look {
    color: [
        LinearRgba::new(0.9, 0.94, 0.98, 0.35),
        LinearRgba::new(0.9, 0.94, 0.98, 0.0),
    ],
    size: [0.25, 0.8],
    fade_in: 0.05,
    fade_out: 0.8,
    drag: 2.5,
    gravity: 6.0,
    ..DUST
};

/// What a wheel throws up on a surface (weather included).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Debris {
    /// Nothing visible.
    None,
    /// Tyre smoke, only while sliding (hard surfaces).
    Smoke,
    /// Dust clouds and pebbles (dirt, sand).
    Dust,
    /// Grass bits and a little dust.
    Grass,
    /// Heavy mud blobs (wet dirt).
    Mud,
    /// Water spray and mist (puddles, wet roads).
    Water,
    /// Snow powder.
    Snow,
}

/// Debris of a wheel on `surface` with outdoor `wetness` / `snow` cover (0–1).
pub fn debris_for(surface: SurfaceKind, wetness: f32, snow: f32) -> Debris {
    use SurfaceKind::*;
    match surface {
        Water => Debris::Water,
        _ if snow > 0.4 => Debris::Snow,
        Dirt if wetness > 0.35 => Debris::Mud,
        Dirt => Debris::Dust,
        Grass => Debris::Grass,
        Asphalt | Stone | Tile | Wood | ToyTrack if wetness > 0.35 => Debris::Water,
        Asphalt | Stone | Tile | Wood | ToyTrack | Carpet | Cardboard | Ice => Debris::Smoke,
        Oil => Debris::None,
    }
}

/// Particles per second a wheel throws for `debris` at `speed` (u/s) and `slip` (0–1).
pub fn debris_rate(debris: Debris, speed: f32, slip: f32) -> f32 {
    let moving = (speed - 2.0).max(0.0);
    match debris {
        Debris::None => 0.0,
        Debris::Smoke => {
            if slip > SKID_SLIP {
                12.0 + 40.0 * slip
            } else {
                0.0
            }
        }
        Debris::Dust => moving * 1.4 + 30.0 * slip,
        Debris::Grass => moving * 1.1 + 20.0 * slip,
        Debris::Mud => moving * 1.2 + 25.0 * slip,
        Debris::Water => moving * 2.2 + 20.0 * slip,
        Debris::Snow => moving * 1.5 + 30.0 * slip,
    }
}

#[allow(clippy::type_complexity)]
fn sparks_and_debris(
    time: Res<Time>,
    weather: Option<Res<WeatherState>>,
    mut fx: ResMut<Fx>,
    eyes: Eyes,
    poses: Poses,
    mut cars: Query<(Entity, &VehicleTelemetry, &mut CarFx)>,
) {
    let dt = time.delta_secs();
    let (wet, snow) = weather.map_or((0.0, 0.0), |w| (w.wetness, w.snow_cover));
    for (entity, t, mut car) in &mut cars {
        let Some(pose) = poses.of(entity) else {
            continue;
        };
        let lod = eyes.lod(pose.translation) * fx.density;
        let vel = car_velocity(&pose, t);
        let up = pose.rotation * Vec3::Y;
        let right = pose.rotation * Vec3::X;
        let back = pose.rotation * Vec3::Z;

        // Drift sparks: only while the drift charges the battery, so sparks teach the mechanic.
        let target = if t.drifting {
            (t.charge_rate / DRIFT_RATE_FULL).clamp(0.0, 1.25)
        } else {
            0.0
        };
        car.charge += (target - car.charge) * (1.0 - (-dt * 10.0).exp());
        let f = car.charge;
        for (k, wheel) in [2, 3].into_iter().enumerate() {
            let w = &t.wheels[wheel];
            let rate = if t.drifting && t.charge_rate > 0.005 && w.grounded {
                (70.0 + 200.0 * f) * lod
            } else {
                0.0
            };
            let n = car.sparks[k].count(rate, dt);
            if n == 0 {
                continue;
            }
            let at = pose.transform_point(car.shape.contact(wheel, w.offset));
            let color = spark_color(f);
            let cool = LinearRgba::rgb(color.red * 0.3, color.green * 0.2, color.blue * 0.3);
            let size = spark_size(f);
            let look = Look {
                size: [size, size * 0.4],
                ..SPARK.colored(color, cool)
            };
            let side = -t.slip_angle.signum();
            for _ in 0..n {
                let v = vel * 0.55
                    + back * fx.rng.range(1.5, 5.0)
                    + right * side * fx.rng.range(0.5, 4.0)
                    + up * fx.rng.range(1.0, 4.5)
                    + fx.rng.dir() * 1.2;
                let life = fx.rng.range(0.18, 0.5);
                let p = Particle::new(look, at + fx.rng.dir() * 0.05, v, life).floored(at.y);
                fx.spawn(p);
            }
            let glow = Look {
                size: [0.45 + 0.45 * f, 0.2],
                ..HOT_SPOT.colored(color, color)
            };
            fx.spawn(Particle::new(glow, at + up * 0.05, vel, dt * 2.0));
        }

        // Wheel debris by surface.
        for (i, w) in t.wheels.iter().enumerate() {
            let Some(surface) = w.surface.filter(|_| w.grounded) else {
                continue;
            };
            let debris = debris_for(surface, wet, snow);
            let front = if i < 2 { 0.4 } else { 1.0 };
            let rate = debris_rate(debris, t.speed, w.slip) * front * lod;
            let n = car.debris[i].count(rate, dt);
            if n == 0 {
                continue;
            }
            let at = pose.transform_point(car.shape.contact(i, w.offset));
            for _ in 0..n {
                throw_debris(&mut fx, debris, at, vel, back, up, t.speed);
            }
        }
    }
}

/// One debris particle from a contact patch at `at` of a car moving at `vel`.
fn throw_debris(
    fx: &mut Fx,
    debris: Debris,
    at: Vec3,
    vel: Vec3,
    back: Vec3,
    up: Vec3,
    speed: f32,
) {
    let kick = (speed / 25.0).clamp(0.3, 1.4);
    let rng = &mut fx.rng;
    let jitter = Vec3::new(rng.signed(0.15), 0.0, rng.signed(0.15));
    let (p, extra) = match debris {
        Debris::None => return,
        Debris::Smoke | Debris::Dust | Debris::Snow => {
            let look = match debris {
                Debris::Smoke => SMOKE,
                Debris::Dust => DUST,
                _ => DUST.colored(
                    LinearRgba::new(0.92, 0.94, 0.98, 0.6),
                    LinearRgba::new(0.95, 0.96, 1.0, 0.0),
                ),
            };
            let v = vel * 0.15 + back * rng.range(0.5, 2.0) * kick + up * rng.range(0.4, 1.4);
            let p = Particle::new(look, at + jitter + up * 0.12, v, rng.range(0.8, 1.5))
                .spinning(rng.angle(), rng.signed(1.2))
                .scaled(rng.range(0.7, 1.2));
            // Dirt also spits pebbles.
            let pebble = (debris == Debris::Dust && rng.unit() < 0.25).then(|| {
                let look = CHIP.colored(
                    LinearRgba::new(0.3, 0.22, 0.14, 1.0),
                    LinearRgba::new(0.28, 0.2, 0.12, 1.0),
                );
                let v = vel * 0.3 + back * rng.range(2.0, 5.0) * kick + up * rng.range(2.5, 5.0);
                Particle::new(look, at + up * 0.05, v, rng.range(0.5, 0.9))
                    .scaled(rng.range(0.4, 0.8))
                    .rotated(Quat::from_scaled_axis(rng.dir() * 3.0))
                    .tumbling(rng.dir() * 12.0)
                    .floored(at.y)
            });
            (p, pebble)
        }
        Debris::Grass | Debris::Mud => {
            let look = if debris == Debris::Mud {
                CHIP.colored(
                    LinearRgba::new(0.2, 0.13, 0.07, 1.0),
                    LinearRgba::new(0.17, 0.11, 0.06, 1.0),
                )
            } else if rng.unit() < 0.7 {
                CHIP
            } else {
                CHIP.colored(
                    LinearRgba::new(0.36, 0.3, 0.14, 1.0),
                    LinearRgba::new(0.33, 0.27, 0.12, 1.0),
                )
            };
            let v = vel * 0.3 + back * rng.range(2.0, 5.0) * kick + up * rng.range(2.5, 6.0);
            let p = Particle::new(look, at + jitter + up * 0.05, v, rng.range(0.5, 0.9))
                .scaled(rng.range(0.6, 1.3))
                .rotated(Quat::from_scaled_axis(rng.dir() * 3.0))
                .tumbling(rng.dir() * 14.0)
                .floored(at.y);
            let dust = (debris == Debris::Grass && rng.unit() < 0.2).then(|| {
                let look = DUST.colored(
                    LinearRgba::new(0.45, 0.5, 0.3, 0.25),
                    LinearRgba::new(0.5, 0.55, 0.35, 0.0),
                );
                Particle::new(
                    look,
                    at + up * 0.1,
                    vel * 0.1 + up * 0.6,
                    rng.range(0.6, 1.0),
                )
            });
            (p, dust)
        }
        Debris::Water => {
            let side = Vec3::new(rng.signed(1.0), 0.0, rng.signed(1.0)) * 2.5;
            let v =
                vel * 0.35 + back * rng.range(1.0, 3.5) * kick + up * rng.range(2.0, 5.0) + side;
            let p = Particle::new(SPRAY, at + jitter + up * 0.05, v, rng.range(0.35, 0.6))
                .floored(at.y - 0.02);
            let sheet = (rng.unit() < 0.35).then(|| {
                let v = vel * 0.4 + back * rng.range(0.5, 2.0) * kick + up * rng.range(1.0, 2.5);
                Particle::new(SPLASH, at + up * 0.1, v, rng.range(0.35, 0.55))
                    .spinning(rng.angle(), rng.signed(2.0))
            });
            (p, sheet)
        }
    };
    fx.spawn(p);
    if let Some(e) = extra {
        fx.spawn(e);
    }
}

const FLAME: Look = Look {
    shape: Shape::Core,
    facing: Facing::Velocity,
    emissive: true,
    color: [
        LinearRgba::rgb(6.0, 11.0, 16.0),
        LinearRgba::rgb(1.5, 1.5, 9.0),
    ],
    size: [0.3, 0.06],
    fade_out: 0.6,
    drag: 2.0,
    stretch: 0.03,
    aspect: 1.4,
    ..Look::DEFAULT
};

const ARC: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Axis,
    emissive: true,
    color: [
        LinearRgba::rgb(7.0, 10.0, 14.0),
        LinearRgba::rgb(4.0, 6.0, 12.0),
    ],
    size: [0.035, 0.03],
    fade_out: 0.5,
    ..Look::DEFAULT
};

/// Warm palette of the Turbo Cell's flame (charge boosts burn electric blue).
const TURBO: [LinearRgba; 2] = [
    LinearRgba::rgb(12.0, 8.0, 3.0),
    LinearRgba::rgb(6.0, 1.5, 0.3),
];

fn boost_flames(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    mut cars: Query<(Entity, &VehicleTelemetry, &StatusEffects, &mut CarFx)>,
) {
    let dt = time.delta_secs();
    for (entity, t, effects, mut car) in &mut cars {
        let started = t.boosting && !car.was_boosting;
        car.was_boosting = t.boosting;
        if !t.boosting {
            car.flame = Emitter::default();
            continue;
        }
        let Some(pose) = poses.of(entity) else {
            continue;
        };
        let turbo = effects.boost().is_some_and(|b| b.freeze_charge);
        let colors = if turbo { TURBO } else { FLAME.color };
        let vel = car_velocity(&pose, t);
        let back = pose.rotation * Vec3::Z;
        let nozzle = pose.transform_point(car.shape.exhaust());
        let look = Look {
            color: colors,
            ..FLAME
        };
        let n = car.flame.count(170.0 * fx.density, dt);
        for _ in 0..n {
            let v = vel + back * fx.rng.range(8.0, 13.0) + fx.rng.dir() * 0.7;
            let p = Particle::new(
                look,
                nozzle + fx.rng.dir() * 0.05,
                v,
                fx.rng.range(0.12, 0.24),
            );
            fx.spawn(p);
        }
        // The jet core: a flickering cone glued to the nozzle (lives one frame).
        let jet = Look {
            shape: Shape::Core,
            facing: Facing::Axis,
            size: [0.26, 0.26],
            aspect: fx.rng.range(4.5, 6.5),
            fade_out: 0.0,
            ..look
        };
        let len = 0.26 * jet.aspect;
        let core = Particle::new(jet, nozzle + back * len * 0.45, vel, dt * 1.5)
            .rotated(Quat::from_rotation_arc(Vec3::Y, back));
        fx.spawn(core);
        // Electric arcs crawling over the rear.
        let n = car.arcs.count(16.0 * fx.density, dt);
        for _ in 0..n {
            let from = nozzle + fx.rng.dir() * 0.25;
            let to = from + back * fx.rng.range(0.1, 0.4) + fx.rng.dir() * 0.35;
            let arc = Look {
                color: if turbo {
                    [colors[0], colors[0]]
                } else {
                    ARC.color
                },
                ..ARC
            };
            zigzag(&mut fx, arc, from, to, 3, 0.12, vel, 0.07);
        }
        if started && !fx.calm {
            let flash = Look {
                size: [1.5, 0.6],
                fade_out: 1.0,
                ..HOT_SPOT.colored(colors[0], colors[1])
            };
            fx.spawn(Particle::new(flash, nozzle, vel, 0.14));
            let ring = Look {
                shape: Shape::Ring,
                facing: Facing::Flat,
                size: [0.3, 1.8],
                fade_out: 1.0,
                ..HOT_SPOT.colored(colors[0], colors[1])
            };
            fx.spawn(Particle::new(ring, nozzle, vel * 0.8, 0.25).rotated(pose.rotation));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spark_colour_and_size_follow_the_charge_rate() {
        let low = spark_color(0.0);
        let mid = spark_color(0.5);
        let high = spark_color(1.0);
        // Amber at the start, electric cyan in the middle, violet at the peak.
        assert!(low.red > low.blue * 3.0, "{low:?}");
        assert!(mid.blue > mid.red * 2.0, "{mid:?}");
        assert!(high.red > mid.red && high.blue > high.green, "{high:?}");
        let peak = |c: LinearRgba| c.red.max(c.green).max(c.blue);
        assert!(
            peak(low) < peak(mid) && peak(mid) < peak(high),
            "brighter with the gain"
        );
        assert!(spark_size(0.0) < spark_size(0.5) && spark_size(0.5) < spark_size(1.0));
        assert_eq!(spark_size(5.0), spark_size(1.25), "clamped");
    }

    #[test]
    fn surfaces_throw_the_right_debris() {
        use SurfaceKind::*;
        assert_eq!(debris_for(Dirt, 0.0, 0.0), Debris::Dust);
        assert_eq!(debris_for(Dirt, 0.8, 0.0), Debris::Mud);
        assert_eq!(debris_for(Grass, 0.0, 0.0), Debris::Grass);
        assert_eq!(debris_for(Water, 0.0, 1.0), Debris::Water);
        assert_eq!(
            debris_for(Asphalt, 0.9, 0.0),
            Debris::Water,
            "wet road spray"
        );
        assert_eq!(debris_for(Asphalt, 0.0, 0.9), Debris::Snow);
        assert_eq!(debris_for(Carpet, 0.0, 0.0), Debris::Smoke);
        // Hard surfaces only smoke while sliding; soft ones throw stuff whenever moving.
        assert_eq!(debris_rate(Debris::Smoke, 30.0, 0.1), 0.0);
        assert!(debris_rate(Debris::Smoke, 30.0, 0.8) > 0.0);
        assert!(debris_rate(Debris::Dust, 30.0, 0.0) > debris_rate(Debris::Dust, 10.0, 0.0));
        assert_eq!(debris_rate(Debris::Dust, 1.0, 0.0), 0.0, "standing still");
    }
}
