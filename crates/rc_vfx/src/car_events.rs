//! Car effects driven by messages and status: landing dust (and the clean-landing charge ring),
//! impact sparks, the respawn materialise, finish confetti, and the electric crackle of a car
//! whose power is knocked out (EMP, Storm Strike).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::cars::{CarFx, Poses, car_velocity};
use crate::fx::Fx;
use crate::fx::{segment, zigzag};
use crate::particles::{Facing, Look, Particle, Shape};
use crate::pool::Emitter;

/// Landings and impacts weaker than this make no effect.
const MIN_STRENGTH: f32 = 0.12;
/// Confetti pieces of a finish at full density.
const CONFETTI: u32 = 240;

const PUFF: Look = Look {
    shape: Shape::Puff,
    color: [
        LinearRgba::new(0.62, 0.6, 0.57, 0.5),
        LinearRgba::new(0.68, 0.66, 0.63, 0.0),
    ],
    size: [0.4, 1.6],
    fade_in: 0.08,
    fade_out: 0.7,
    drag: 3.5,
    gravity: -0.4,
    ..Look::DEFAULT
};

const IMPACT_SPARK: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Velocity,
    emissive: true,
    color: [
        LinearRgba::rgb(14.0, 10.0, 6.0),
        LinearRgba::rgb(5.0, 1.2, 0.2),
    ],
    size: [0.06, 0.03],
    fade_out: 0.4,
    drag: 1.2,
    gravity: 18.0,
    stretch: 0.03,
    aspect: 2.5,
    bounce: 0.3,
    ..Look::DEFAULT
};

const FLASH: Look = Look {
    shape: Shape::Core,
    emissive: true,
    color: [
        LinearRgba::rgb(10.0, 8.0, 5.0),
        LinearRgba::rgb(4.0, 2.0, 0.6),
    ],
    size: [0.8, 1.2],
    fade_out: 1.0,
    ..Look::DEFAULT
};

/// Charge-cyan glow of good things (clean landing, respawn).
const CHARGE: LinearRgba = LinearRgba::rgb(2.0, 7.0, 10.0);

const RING: Look = Look {
    shape: Shape::Ring,
    facing: Facing::Flat,
    emissive: true,
    color: [CHARGE, LinearRgba::rgb(0.5, 2.0, 4.0)],
    size: [0.6, 3.2],
    fade_out: 0.8,
    ..Look::DEFAULT
};

const TWINKLE: Look = Look {
    shape: Shape::Star,
    emissive: true,
    color: [
        LinearRgba::rgb(4.0, 9.0, 12.0),
        LinearRgba::rgb(1.0, 3.0, 6.0),
    ],
    size: [0.22, 0.05],
    fade_in: 0.15,
    fade_out: 0.5,
    drag: 2.0,
    ..Look::DEFAULT
};

const CONFETTO: Look = Look {
    shape: Shape::Square,
    facing: Facing::Flat,
    color: [LinearRgba::WHITE, LinearRgba::WHITE],
    size: [0.13, 0.13],
    fade_out: 0.15,
    drag: 2.4,
    gravity: 5.0,
    ..Look::DEFAULT
};

/// Toy palette of the confetti (linear).
const CONFETTI_COLORS: [LinearRgba; 6] = [
    LinearRgba::new(0.85, 0.06, 0.05, 1.0),
    LinearRgba::new(0.05, 0.25, 0.85, 1.0),
    LinearRgba::new(0.9, 0.6, 0.03, 1.0),
    LinearRgba::new(0.05, 0.6, 0.15, 1.0),
    LinearRgba::new(0.9, 0.9, 0.9, 1.0),
    LinearRgba::new(0.75, 0.08, 0.6, 1.0),
];

const STUN_ARC: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Axis,
    emissive: true,
    color: [
        LinearRgba::rgb(7.0, 10.0, 18.0),
        LinearRgba::rgb(3.0, 5.0, 14.0),
    ],
    size: [0.055, 0.04],
    fade_out: 0.5,
    ..Look::DEFAULT
};

/// Crackle emitter of a power-less car.
#[derive(Component, Debug, Default)]
pub(crate) struct Stunned(Emitter);

pub(crate) fn build(app: &mut App) {
    app.add_observer(|add: On<Add<CarFx>>, mut commands: Commands| {
        commands.entity(add.entity).insert(Stunned::default());
    })
    .add_systems(
        Update,
        (landings, impacts, respawns, confetti, stun_crackle).in_set(crate::VfxSystems::Emit),
    );
}

/// Puff colour of the ground a car landed on (linear RGB).
fn ground_dust(surface: Option<SurfaceKind>) -> Option<LinearRgba> {
    use SurfaceKind::*;
    Some(match surface? {
        Dirt => LinearRgba::new(0.42, 0.31, 0.2, 0.6),
        Grass => LinearRgba::new(0.42, 0.45, 0.28, 0.4),
        Water => LinearRgba::new(0.9, 0.94, 0.98, 0.6),
        Carpet | Cardboard => LinearRgba::new(0.6, 0.57, 0.52, 0.35),
        Ice => LinearRgba::new(0.85, 0.9, 0.95, 0.4),
        Oil => return None,
        _ => LinearRgba::new(0.62, 0.6, 0.57, 0.4),
    })
}

fn landings(
    mut landed: MessageReader<CarLanded>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    cars: Query<&VehicleTelemetry>,
) {
    for msg in landed.read() {
        if msg.strength < MIN_STRENGTH {
            continue;
        }
        let (Some(pose), Ok(t)) = (poses.of(msg.car), cars.get(msg.car)) else {
            continue;
        };
        let at = pose.translation;
        let up = pose.rotation * Vec3::Y;
        if let Some(color) = ground_dust(t.surface) {
            let end = LinearRgba {
                alpha: 0.0,
                ..color
            };
            let look = PUFF.colored(color, end);
            let n = fx.n(6 + (msg.strength * 22.0) as u32);
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU + fx.rng.signed(0.3);
                let out = pose.rotation * Vec3::new(a.cos(), 0.0, a.sin());
                let v = out * fx.rng.range(3.0, 7.0) * (0.5 + msg.strength)
                    + up * fx.rng.range(0.3, 1.2);
                let p = Particle::new(look, at + out * 0.6 + up * 0.1, v, fx.rng.range(0.6, 1.1))
                    .spinning(fx.rng.angle(), fx.rng.signed(1.0))
                    .scaled(0.6 + msg.strength * 0.6);
                fx.spawn(p);
            }
        }
        // A clean landing earned charge: say so with a ring of light.
        if msg.clean {
            let ring = Look {
                emissive: true,
                color: if fx.calm {
                    [CHARGE * 0.35, CHARGE * 0.1]
                } else {
                    RING.color
                },
                ..RING
            };
            let flat = Quat::from_rotation_arc(Vec3::Z, up);
            fx.spawn(Particle::new(ring, at + up * 0.05, Vec3::ZERO, 0.45).rotated(flat));
        }
    }
}

fn impacts(mut hits: MessageReader<CarImpact>, mut fx: ResMut<Fx>) {
    for msg in hits.read() {
        if msg.strength < MIN_STRENGTH {
            continue;
        }
        let n = fx.n(8 + (msg.strength * 36.0) as u32);
        let normal = msg.normal.normalize_or(Vec3::Y);
        for _ in 0..n {
            let dir = fx.rng.cone(normal, 1.2);
            let v = dir * fx.rng.range(4.0, 13.0) * (0.6 + msg.strength * 0.6);
            let life = fx.rng.range(0.2, 0.45);
            let p = Particle::new(IMPACT_SPARK, msg.point, v, life).floored(msg.point.y - 0.6);
            fx.spawn(p);
        }
        // A soft glow replaces the flash with *Reduce flashing* and for light knocks.
        let strong = msg.strength > 0.35;
        let flash = Look {
            color: if fx.calm || !strong {
                [FLASH.color[0] * 0.25, FLASH.color[1] * 0.1]
            } else {
                FLASH.color
            },
            ..FLASH
        };
        fx.spawn(
            Particle::new(flash, msg.point + normal * 0.1, Vec3::ZERO, 0.12)
                .scaled(0.6 + msg.strength),
        );
    }
}

fn respawns(
    mut done: MessageReader<CarRespawned>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    cars: Query<&CarFx>,
) {
    for msg in done.read() {
        let (Some(pose), Ok(car)) = (poses.of(msg.car), cars.get(msg.car)) else {
            continue;
        };
        let at = pose.translation;
        let up = pose.rotation * Vec3::Y;
        let half = car.shape.half_length();
        if msg.kind == RespawnKind::ToTrack {
            // Hologram materialise: a scan ring rising through the car, sparks converging on it.
            let flat = Quat::from_rotation_arc(Vec3::Z, up);
            let scan = Look {
                size: [half * 2.6, half * 1.8],
                color: [CHARGE * 1.2, CHARGE * 0.4],
                ..RING
            };
            fx.spawn(Particle::new(scan, at, up * 2.0, 0.6).rotated(flat));
            let beam = Look {
                shape: Shape::Streak,
                facing: Facing::Axis,
                color: [CHARGE * 0.35, CHARGE * 0.0],
                size: [half * 1.6, half * 1.6],
                aspect: 2.0,
                fade_in: 0.15,
                ..RING
            };
            fx.spawn(
                Particle::new(beam, at + up * half * 1.4, Vec3::ZERO, 0.7)
                    .rotated(Quat::from_rotation_arc(Vec3::Y, up)),
            );
        }
        let n = fx.n(if msg.kind == RespawnKind::ToTrack {
            36
        } else {
            12
        });
        for _ in 0..n {
            let from = at + up * fx.rng.range(0.1, 1.2) + fx.rng.dir() * half * 1.5;
            let v = (at + up * 0.5 - from) * 1.6;
            let (life, scale) = (fx.rng.range(0.45, 0.7), fx.rng.range(0.7, 1.4));
            fx.spawn(Particle::new(TWINKLE, from, v, life).scaled(scale));
        }
    }
}

fn confetti(
    mut events: MessageReader<RaceEvent>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    locals: Query<(), With<LocalPlayer>>,
) {
    for ev in events.read() {
        let (Some(racer), RaceEventKind::Finished { rank, .. }) = (ev.racer, ev.kind) else {
            continue;
        };
        if !locals.contains(racer) {
            continue;
        }
        let Some(pose) = poses.of(racer) else {
            continue;
        };
        let at = pose.translation + Vec3::Y * 1.5;
        let winner = rank == 1;
        let n = fx.n(if winner { CONFETTI } else { CONFETTI * 2 / 3 });
        for k in 0..n {
            let color = if winner && k % 4 == 0 {
                LinearRgba::new(1.0, 0.7, 0.15, 1.0)
            } else {
                CONFETTI_COLORS[k as usize % CONFETTI_COLORS.len()]
            };
            let look = CONFETTO.colored(color, color);
            let v = Vec3::new(
                fx.rng.signed(4.0),
                fx.rng.range(5.0, 10.0),
                fx.rng.signed(4.0),
            );
            let p = Particle::new(look, at + fx.rng.dir() * 0.6, v, fx.rng.range(2.6, 3.8))
                .rotated(Quat::from_scaled_axis(fx.rng.dir() * 3.0))
                .tumbling(fx.rng.dir() * fx.rng.range(6.0, 14.0))
                .scaled(fx.rng.range(0.7, 1.3));
            fx.spawn(p);
        }
        if winner {
            let gold = Look {
                color: [
                    LinearRgba::rgb(12.0, 8.0, 2.0),
                    LinearRgba::rgb(4.0, 2.0, 0.3),
                ],
                size: [0.35, 0.05],
                gravity: 3.0,
                ..TWINKLE
            };
            for _ in 0..fx.n(40) {
                let v = fx.rng.dir() * fx.rng.range(2.0, 6.0) + Vec3::Y * 3.0;
                let life = fx.rng.range(0.8, 1.6);
                fx.spawn(Particle::new(gold, at, v, life));
            }
        }
    }
}

/// Power knocked out (EMP, Storm Strike: `PowerLoss(0)`): blue arcs crawl over the body.
fn stun_crackle(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    mut cars: Query<(
        Entity,
        &StatusEffects,
        &VehicleTelemetry,
        &CarFx,
        &mut Stunned,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, effects, t, car, mut stunned) in &mut cars {
        let rate = if effects.power_mul() < 0.05 {
            40.0 * fx.density
        } else {
            0.0
        };
        let n = stunned.0.count(rate, dt);
        if n == 0 {
            continue;
        }
        let Some(pose) = poses.of(entity) else {
            continue;
        };
        let vel = car_velocity(&pose, t);
        let half = car.shape.half_length();
        for _ in 0..n {
            let local = Vec3::new(
                fx.rng.signed(0.6),
                fx.rng.range(0.2, car.shape.roof()),
                fx.rng.signed(half),
            );
            let from = pose.transform_point(local);
            let to = from + fx.rng.dir() * fx.rng.range(0.4, 0.9);
            zigzag(&mut fx, STUN_ARC, from, to, 4, 0.12, vel, 0.08);
        }
        if fx.rng.unit() < 0.3 {
            let a = pose.transform_point(Vec3::new(0.0, car.shape.roof(), 0.0));
            segment(&mut fx, STUN_ARC, a, a + Vec3::Y * 0.4, vel, 0.05);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ground_but_oil_raises_dust() {
        for s in SurfaceKind::ALL {
            let dust = ground_dust(Some(s));
            assert_eq!(dust.is_none(), s == SurfaceKind::Oil, "{s:?}");
            if let Some(c) = dust {
                assert!(c.alpha > 0.0 && c.alpha <= 1.0);
            }
        }
        assert!(ground_dust(None).is_none(), "airborne: no surface, no dust");
    }
}
