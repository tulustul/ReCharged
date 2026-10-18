//! Explosions and hits: rocket fireballs, Goo splatter, the EMP pulse (rings, an expanding shell,
//! radial arcs) sized by `Explosion::radius`; hit reactions (orb and leech zaps, oil splashes,
//! shield deflections) and the Storm Strike bolt. *Reduce flashing* turns every flash into a soft,
//! slower glow (GDD §10.2, §18).

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::{BubbleMaterial, VfxAssets};
use crate::cars::{CarFx, FxVisual, Poses};
use crate::fx::{Fx, FxRng, segment, zigzag};
use crate::particles::{Facing, Look, Particle, Shape};

/// Height the Storm Strike bolt comes down from, u above the car.
const STORM_HEIGHT_U: f32 = 26.0;
/// EMP shell expansion time, s.
const SHELL_S: f32 = 0.45;

const FIRE: Look = Look {
    shape: Shape::Puff,
    emissive: true,
    color: [
        LinearRgba::rgb(14.0, 8.0, 2.5),
        LinearRgba::rgb(3.0, 0.6, 0.1),
    ],
    size: [0.5, 1.8],
    fade_out: 0.6,
    drag: 4.0,
    gravity: -2.0,
    ..Look::DEFAULT
};

const SMOKE: Look = Look {
    shape: Shape::Puff,
    color: [
        LinearRgba::new(0.12, 0.11, 0.1, 0.75),
        LinearRgba::new(0.3, 0.29, 0.28, 0.0),
    ],
    size: [0.7, 2.8],
    fade_in: 0.1,
    fade_out: 0.6,
    drag: 2.5,
    gravity: -1.6,
    ..Look::DEFAULT
};

const SPARK: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Velocity,
    emissive: true,
    color: [
        LinearRgba::rgb(14.0, 9.0, 4.0),
        LinearRgba::rgb(5.0, 1.0, 0.1),
    ],
    size: [0.07, 0.03],
    fade_out: 0.4,
    drag: 1.0,
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
        LinearRgba::rgb(16.0, 12.0, 7.0),
        LinearRgba::rgb(6.0, 2.5, 0.5),
    ],
    size: [1.0, 1.6],
    fade_out: 1.0,
    ..Look::DEFAULT
};

const RING: Look = Look {
    shape: Shape::Ring,
    facing: Facing::Flat,
    emissive: true,
    color: [
        LinearRgba::rgb(8.0, 4.0, 1.0),
        LinearRgba::rgb(2.0, 0.6, 0.1),
    ],
    size: [0.3, 1.0],
    fade_out: 0.9,
    ..Look::DEFAULT
};

const CHIP: Look = Look {
    shape: Shape::Chip,
    facing: Facing::Flat,
    color: [
        LinearRgba::new(0.06, 0.05, 0.05, 1.0),
        LinearRgba::new(0.05, 0.04, 0.04, 1.0),
    ],
    size: [0.12, 0.1],
    fade_out: 0.2,
    drag: 0.5,
    gravity: 22.0,
    bounce: 0.25,
    ..Look::DEFAULT
};

const EMP: LinearRgba = LinearRgba::rgb(2.0, 5.0, 14.0);
const GOO: LinearRgba = LinearRgba::new(0.2, 0.75, 0.08, 1.0);
const BOLT: LinearRgba = LinearRgba::rgb(10.0, 12.0, 20.0);

const ARC: Look = Look {
    shape: Shape::Streak,
    facing: Facing::Axis,
    emissive: true,
    color: [EMP, LinearRgba::rgb(1.0, 2.0, 8.0)],
    size: [0.07, 0.05],
    fade_out: 0.5,
    ..Look::DEFAULT
};

/// An expanding energy shell (EMP pulse).
#[derive(Component, Debug)]
struct Shell {
    material: Handle<BubbleMaterial>,
    radius: f32,
    rim: f32,
    age: f32,
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (explosions, hits, grow_shells).in_set(crate::VfxSystems::Emit),
    );
}

/// A jagged lightning path from `from` to `to` by midpoint displacement: `2^depth + 1` points,
/// each level pushing midpoints sideways by up to `jitter` × the segment length.
pub fn bolt_path(from: Vec3, to: Vec3, depth: u32, jitter: f32, rng: &mut FxRng) -> Vec<Vec3> {
    let mut points = vec![from, to];
    let mut amount = jitter;
    for _ in 0..depth {
        let mut next = Vec::with_capacity(points.len() * 2 - 1);
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let len = d.length();
            let dir = d / len.max(1e-6);
            let off = rng.dir();
            let side = (off - dir * off.dot(dir)).normalize_or_zero();
            next.push(a);
            next.push((a + b) * 0.5 + side * len * amount * rng.range(0.3, 1.0));
        }
        next.push(*points.last().unwrap_or(&to));
        points = next;
        amount *= 0.6;
    }
    points
}

/// Draws a bolt along `points` with axial streaks.
fn draw_bolt(fx: &mut Fx, points: &[Vec3], look: Look, life: f32) {
    for w in points.windows(2) {
        segment(fx, look, w[0], w[1], Vec3::ZERO, life);
    }
}

#[allow(clippy::too_many_arguments)]
fn explosions(
    mut blasts: MessageReader<Explosion>,
    mut fx: ResMut<Fx>,
    mut commands: Commands,
    assets: Res<VfxAssets>,
    mut bubbles: ResMut<Assets<BubbleMaterial>>,
) {
    for b in blasts.read() {
        match b.kind {
            ItemKind::EmpPulse => {
                emp(&mut fx, b.position, b.radius);
                let rim = if fx.calm { 0.3 } else { 0.8 };
                let material = bubbles.add(BubbleMaterial::new(EMP * 0.35, rim));
                commands.spawn((
                    FxVisual,
                    Shell {
                        material: material.clone(),
                        radius: b.radius,
                        rim,
                        age: 0.0,
                    },
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(b.position).with_scale(Vec3::splat(0.1)),
                    NotShadowCaster,
                    DespawnOnExit(RaceAttempt),
                ));
            }
            ItemKind::GooBomb => goo(&mut fx, b.position, b.radius),
            _ => fireball(&mut fx, b.position, b.radius),
        }
    }
}

/// Rocket (and default) blast: flash, fireball, sparks, smoke and debris, sized by `radius`.
fn fireball(fx: &mut Fx, at: Vec3, radius: f32) {
    let s = (radius / 1.8).clamp(0.6, 4.0);
    let calm = fx.calm;
    // Reduce flashing: a soft glow that swells and fades instead of a flash, a dim fireball.
    let flash = if calm {
        Look {
            shape: Shape::Dot,
            color: [FLASH.color[0] * 0.12, FLASH.color[1] * 0.0],
            fade_in: 0.35,
            ..FLASH
        }
    } else {
        FLASH
    };
    fx.spawn(Particle::new(flash, at, Vec3::ZERO, if calm { 0.6 } else { 0.15 }).scaled(2.2 * s));
    let fire = if calm {
        Look {
            fade_in: 0.3,
            ..FIRE.colored(FIRE.color[0] * 0.22, FIRE.color[1] * 0.22)
        }
    } else {
        FIRE
    };
    let fire_life = if calm { 1.6 } else { 1.0 };
    for _ in 0..fx.n(16) {
        let dir = fx.rng.dir();
        let v = dir * fx.rng.range(2.0, 6.0) * s + Vec3::Y * 1.5;
        let (life, spin) = (fx.rng.range(0.3, 0.55) * fire_life, fx.rng.angle());
        fx.spawn(
            Particle::new(fire, at + dir * 0.3 * s, v, life)
                .spinning(spin, 2.0)
                .scaled(s),
        );
    }
    for _ in 0..fx.n(14) {
        let dir = fx.rng.dir();
        let v = dir * fx.rng.range(1.0, 3.0) * s + Vec3::Y * 1.0;
        let (life, spin) = (fx.rng.range(1.4, 2.2), fx.rng.angle());
        fx.spawn(
            Particle::new(SMOKE, at + dir * 0.5 * s, v, life)
                .spinning(spin, 0.6)
                .scaled(s),
        );
    }
    let spark = if calm {
        SPARK.colored(SPARK.color[0] * 0.35, SPARK.color[1] * 0.35)
    } else {
        SPARK
    };
    for _ in 0..fx.n(if calm { 16 } else { 40 }) {
        let v = fx.rng.dir() * fx.rng.range(6.0, 18.0) + Vec3::Y * 3.0;
        let life = fx.rng.range(0.35, 0.8);
        fx.spawn(Particle::new(spark, at, v, life).floored(at.y - 0.8));
    }
    for _ in 0..fx.n(10) {
        let v = fx.rng.dir() * fx.rng.range(4.0, 9.0) + Vec3::Y * 4.0;
        let (life, rot, tumble) = (
            fx.rng.range(0.7, 1.2),
            fx.rng.dir() * 3.0,
            fx.rng.dir() * 12.0,
        );
        let p = Particle::new(CHIP, at, v, life)
            .rotated(Quat::from_scaled_axis(rot))
            .tumbling(tumble)
            .floored(at.y - 0.8);
        fx.spawn(p);
    }
    let ring = Look {
        size: [0.3, 2.0 * radius],
        color: if calm {
            [RING.color[0] * 0.2, RING.color[1] * 0.1]
        } else {
            RING.color
        },
        ..RING
    };
    fx.spawn(
        Particle::new(ring, at - Vec3::Y * 0.4, Vec3::ZERO, 0.3)
            .rotated(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    );
}

/// Goo Bomb: green slime splattered across its blast radius, a green glow and smoke.
fn goo(fx: &mut Fx, at: Vec3, radius: f32) {
    let calm = fx.calm;
    let glow = Look {
        color: if calm {
            [
                LinearRgba::rgb(1.0, 3.0, 0.4),
                LinearRgba::rgb(0.2, 0.6, 0.1),
            ]
        } else {
            [
                LinearRgba::rgb(4.0, 14.0, 2.0),
                LinearRgba::rgb(1.0, 3.0, 0.3),
            ]
        },
        ..FLASH
    };
    fx.spawn(Particle::new(glow, at, Vec3::ZERO, if calm { 0.4 } else { 0.18 }).scaled(3.0));
    let blob = Look {
        shape: Shape::Chip,
        color: [GOO, LinearRgba::new(0.15, 0.6, 0.06, 1.0)],
        size: [0.28, 0.22],
        fade_out: 0.25,
        drag: 0.8,
        gravity: 20.0,
        ..Look::DEFAULT
    };
    for _ in 0..fx.n(40) {
        let dir = fx.rng.dir();
        // Fly far enough to cover the blast radius: the splatter shows who was in reach.
        let speed = fx.rng.range(0.4, 1.0) * radius * 1.6;
        let v = dir * speed + Vec3::Y * fx.rng.range(2.0, 6.0);
        let (life, spin) = (fx.rng.range(0.9, 1.4), fx.rng.angle());
        let p = Particle::new(blob, at, v, life)
            .spinning(spin, 3.0)
            .scaled(fx.rng.range(0.6, 1.5))
            .floored(at.y - 0.9);
        fx.spawn(p);
    }
    let smoke = SMOKE.colored(
        LinearRgba::new(0.3, 0.55, 0.2, 0.55),
        LinearRgba::new(0.4, 0.6, 0.3, 0.0),
    );
    for _ in 0..fx.n(10) {
        let dir = fx.rng.dir();
        let v = dir * fx.rng.range(1.0, 3.0) + Vec3::Y;
        let life = fx.rng.range(1.0, 1.6);
        fx.spawn(Particle::new(smoke, at + dir * 0.4, v, life).scaled(1.2));
    }
    let ring = Look {
        size: [0.3, 2.0 * radius],
        color: if calm {
            [
                LinearRgba::rgb(0.5, 1.5, 0.2),
                LinearRgba::rgb(0.1, 0.3, 0.05),
            ]
        } else {
            [
                LinearRgba::rgb(2.0, 7.0, 1.0),
                LinearRgba::rgb(0.5, 1.5, 0.2),
            ]
        },
        ..RING
    };
    fx.spawn(
        Particle::new(ring, at - Vec3::Y * 0.8, Vec3::ZERO, 0.4)
            .rotated(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    );
}

/// EMP: two electric rings racing out to the pulse radius and radial arcs.
fn emp(fx: &mut Fx, at: Vec3, radius: f32) {
    let k = if fx.calm { 0.12 } else { 1.0 };
    let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    for (size0, life) in [(0.5, 0.45), (0.2, 0.6)] {
        let ring = Look {
            size: [size0, 2.0 * radius],
            color: [EMP * k, EMP * 0.1 * k],
            fade_in: if fx.calm { 0.3 } else { 0.0 },
            ..RING
        };
        fx.spawn(Particle::new(ring, at, Vec3::ZERO, life).rotated(flat));
    }
    let n = fx.n(if fx.calm { 8 } else { 22 });
    for _ in 0..n {
        let a = fx.rng.angle();
        let out = Vec3::new(a.cos(), fx.rng.range(-0.15, 0.3), a.sin()).normalize();
        let from = at + out * 0.8;
        let to = at + out * radius * fx.rng.range(0.35, 0.8);
        let look = Look {
            color: [ARC.color[0] * k, ARC.color[1] * k],
            ..ARC
        };
        zigzag(fx, look, from, to, 5, 0.6, Vec3::ZERO, 0.14);
    }
}

fn grow_shells(
    time: Res<Time>,
    mut commands: Commands,
    mut bubbles: ResMut<Assets<BubbleMaterial>>,
    mut shells: Query<(Entity, &mut Shell, &mut Transform)>,
) {
    for (entity, mut shell, mut transform) in &mut shells {
        shell.age += time.delta_secs();
        let t = shell.age / SHELL_S;
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let e = 1.0 - (1.0 - t).powi(3);
        transform.scale = Vec3::splat((2.0 * shell.radius * e).max(0.1));
        // Only the rim shows (the pulse's edge, at its real radius); a lit body would wash out
        // every view the pulse passes through.
        if let Some(mut m) = bubbles.get_mut(&shell.material) {
            m.params.y = shell.rim * (1.0 - t) * (1.0 - t);
            m.params.z = 0.0;
            m.params.w = 0.0;
        }
    }
}

#[allow(clippy::type_complexity)]
fn hits(
    mut events: MessageReader<HitEvent>,
    mut fx: ResMut<Fx>,
    poses: Poses,
    cars: Query<&CarFx>,
) {
    for h in events.read() {
        let Some(pose) = poses.of(h.victim) else {
            continue;
        };
        let roof = cars.get(h.victim).map_or(0.8, |c| c.shape.roof());
        let up = pose.rotation * Vec3::Y;
        let centre = pose.translation + up * 0.45;
        // The bolt comes down whether a shield blocks it or not: the shield flashes on its own.
        if h.by == ItemKind::StormStrike {
            storm_bolt(&mut fx, pose.translation + up * roof);
        }
        if h.blocked {
            let deflect = Look {
                color: [
                    LinearRgba::rgb(2.0, 7.0, 12.0),
                    LinearRgba::rgb(0.5, 2.0, 4.0),
                ],
                ..SPARK
            };
            for _ in 0..fx.n(16) {
                let v = fx.rng.dir() * fx.rng.range(4.0, 9.0);
                let life = fx.rng.range(0.2, 0.35);
                fx.spawn(Particle::new(
                    deflect,
                    centre + v.normalize() * 1.2,
                    v,
                    life,
                ));
            }
            continue;
        }
        match h.by {
            ItemKind::ShockOrb | ItemKind::LeechCoil | ItemKind::EmpPulse => {
                let color = match h.by {
                    ItemKind::LeechCoil => LinearRgba::rgb(3.0, 11.0, 1.5),
                    ItemKind::ShockOrb => LinearRgba::rgb(8.0, 5.0, 16.0),
                    _ => EMP,
                };
                let look = Look {
                    color: [color, color * 0.4],
                    ..ARC
                };
                for _ in 0..fx.n(10) {
                    let to = centre + fx.rng.dir() * fx.rng.range(0.8, 1.5);
                    zigzag(&mut fx, look, centre, to, 4, 0.18, Vec3::ZERO, 0.1);
                }
                let glow = Look {
                    shape: Shape::Dot,
                    color: [color * if fx.calm { 0.15 } else { 0.5 }, color * 0.0],
                    ..FLASH
                };
                fx.spawn(Particle::new(glow, centre, Vec3::ZERO, 0.2).scaled(1.8));
            }
            ItemKind::OilSlick => {
                let drop = Look {
                    shape: Shape::Drop,
                    facing: Facing::Velocity,
                    color: [LinearRgba::new(0.02, 0.018, 0.015, 1.0); 2],
                    size: [0.1, 0.07],
                    gravity: 22.0,
                    stretch: 0.02,
                    aspect: 1.5,
                    fade_out: 0.2,
                    ..Look::DEFAULT
                };
                for _ in 0..fx.n(24) {
                    let a = fx.rng.angle();
                    let v = Vec3::new(a.cos() * 3.0, fx.rng.range(2.0, 5.0), a.sin() * 3.0);
                    let life = fx.rng.range(0.4, 0.7);
                    fx.spawn(
                        Particle::new(drop, pose.translation + Vec3::Y * 0.1, v, life)
                            .floored(pose.translation.y),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Storm Strike: a bolt from the sky onto the car, branches, an impact flash and sparks. With
/// *Reduce flashing*: a soft column of light that swells and fades, and a soft glow on the car.
fn storm_bolt(fx: &mut Fx, at: Vec3) {
    let top = at + Vec3::new(fx.rng.signed(3.0), STORM_HEIGHT_U, fx.rng.signed(3.0));
    if fx.calm {
        let column = Look {
            shape: Shape::Streak,
            facing: Facing::Axis,
            emissive: true,
            color: [BOLT * 0.12, BOLT * 0.0],
            size: [1.2, 1.2],
            fade_in: 0.3,
            fade_out: 0.6,
            ..Look::DEFAULT
        };
        segment(fx, column, top, at, Vec3::ZERO, 0.7);
        let glow = Look {
            shape: Shape::Dot,
            color: [BOLT * 0.15, BOLT * 0.0],
            fade_in: 0.3,
            ..FLASH
        };
        fx.spawn(Particle::new(glow, at, Vec3::ZERO, 0.7).scaled(2.5));
        return;
    }
    let main = bolt_path(top, at, 5, 0.32, &mut fx.rng);
    let width = 0.16;
    let look = Look {
        shape: Shape::Streak,
        facing: Facing::Axis,
        emissive: true,
        color: [BOLT, BOLT * 0.3],
        size: [width, width],
        fade_out: 0.6,
        ..Look::DEFAULT
    };
    draw_bolt(fx, &main, look, 0.25);
    // A wide faint glow around the channel.
    let glow = Look {
        size: [width * 6.0; 2],
        color: [BOLT * 0.08, BOLT * 0.0],
        ..look
    };
    draw_bolt(fx, &main, glow, 0.25);
    for _ in 0..2 {
        let i = 4 + (fx.rng.unit() * (main.len() - 8) as f32) as usize;
        let from = main[i];
        let to = from
            + Vec3::new(
                fx.rng.signed(4.0),
                -fx.rng.range(3.0, 7.0),
                fx.rng.signed(4.0),
            );
        let branch = bolt_path(from, to, 3, 0.35, &mut fx.rng);
        let thin = Look {
            size: [width * 0.5; 2],
            ..look
        };
        draw_bolt(fx, &branch, thin, 0.2);
    }
    let flash = Look {
        color: [BOLT * 0.8, BOLT * 0.0],
        ..FLASH
    };
    fx.spawn(Particle::new(flash, at, Vec3::ZERO, 0.18).scaled(2.5));
    let spark = SPARK.colored(BOLT, LinearRgba::rgb(2.0, 3.0, 8.0));
    for _ in 0..fx.n(26) {
        let v = fx.rng.cone(Vec3::Y, 1.3) * fx.rng.range(4.0, 11.0);
        let life = fx.rng.range(0.25, 0.5);
        fx.spawn(Particle::new(spark, at, v, life).floored(at.y - 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Peak HDR intensity of everything a blast emitted (before fades).
    fn peak(calm: bool, blast: impl Fn(&mut Fx)) -> f32 {
        let mut fx = Fx { calm, ..default() };
        blast(&mut fx);
        (0..fx.pool.slots())
            .filter_map(|i| fx.pool.get(i))
            .filter(|p| p.look.emissive)
            .map(|p| {
                let c = p.look.color[0];
                c.red.max(c.green).max(c.blue)
            })
            .fold(0.0, f32::max)
    }

    #[test]
    fn reduce_flashing_dims_every_blast() {
        type Blast = fn(&mut Fx);
        let cases: [(&str, Blast); 4] = [
            ("rocket", |fx| fireball(fx, Vec3::ZERO, 1.8)),
            ("goo", |fx| goo(fx, Vec3::ZERO, 7.0)),
            ("emp", |fx| emp(fx, Vec3::ZERO, 12.0)),
            ("storm", |fx| storm_bolt(fx, Vec3::ZERO)),
        ];
        for (name, blast) in cases {
            let (bright, soft) = (peak(false, blast), peak(true, blast));
            assert!(soft > 0.0, "{name}: still visible");
            assert!(soft <= bright * 0.4, "{name}: {soft} vs {bright}");
        }
    }

    #[test]
    fn bolt_paths_keep_their_ends_and_stay_near_the_line() {
        let mut rng = FxRng::new(3);
        let (a, b) = (Vec3::new(0.0, 20.0, 0.0), Vec3::ZERO);
        let path = bolt_path(a, b, 4, 0.3, &mut rng);
        assert_eq!(path.len(), 17);
        assert_eq!((path[0], path[16]), (a, b));
        for p in &path {
            let off = Vec2::new(p.x, p.z).length();
            // Displacements shrink each level: the sum stays well under half the length.
            assert!(off < 10.0, "{p} strays too far");
            assert!((-0.1..=20.1).contains(&p.y), "{p} overshoots the ends");
        }
        let flat = bolt_path(a, b, 0, 0.3, &mut rng);
        assert_eq!(flat, vec![a, b]);
    }
}
