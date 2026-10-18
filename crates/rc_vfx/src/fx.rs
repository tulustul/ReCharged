//! The effects context and the particle slots: [`Fx`] (pool, density, *Reduce flashing*, RNG)
//! is what every emitter writes to; [`Slots`] are the pooled entities that show the particles.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::particles::{Look, MAX_PARTICLES, Particle, pack_tag};
use crate::pool::Pool;

/// A small presentation-only PRNG (SplitMix64). Effects need no determinism and must not
/// consume the gameplay streams of `rc_core::rng`.
#[derive(Debug, Clone)]
pub struct FxRng(u64);

impl FxRng {
    /// Seeded generator.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [a, b).
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    /// Uniform in [−r, r).
    pub fn signed(&mut self, r: f32) -> f32 {
        self.range(-r, r)
    }

    /// Random unit vector.
    pub fn dir(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let a = self.range(0.0, std::f32::consts::TAU);
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * a.cos(), z, r * a.sin())
    }

    /// Random unit vector within `angle` rad of `axis` (unit).
    pub fn cone(&mut self, axis: Vec3, angle: f32) -> Vec3 {
        let cos = self.range(angle.cos(), 1.0);
        let a = self.range(0.0, std::f32::consts::TAU);
        let sin = (1.0 - cos * cos).max(0.0).sqrt();
        let local = Vec3::new(sin * a.cos(), cos, sin * a.sin());
        Quat::from_rotation_arc(Vec3::Y, axis) * local
    }

    /// Random angle in [0, τ).
    pub fn angle(&mut self) -> f32 {
        self.range(0.0, std::f32::consts::TAU)
    }
}

/// Particle density settings → (emission multiplier, particle budget). Split-screen with 3–4
/// views drops one step (GDD §15: split-screen lowers the effective tier).
pub fn density(setting: ParticleDensity, views: usize) -> (f32, usize) {
    let step: usize = match setting {
        ParticleDensity::Low => 0,
        ParticleDensity::Medium => 1,
        ParticleDensity::High => 2,
    };
    let step = if views >= 3 {
        step.saturating_sub(1)
    } else {
        step
    };
    let rate = [0.35, 0.65, 1.0][step];
    (rate, (MAX_PARTICLES as f32 * rate) as usize)
}

/// The effects context every emitter writes to: the particle pool, the density multiplier, the
/// *Reduce flashing* flag and the effects RNG.
#[derive(Resource, Debug)]
pub struct Fx {
    /// The particles.
    pub pool: Pool,
    /// Emission multiplier from the density setting (0.35–1).
    pub density: f32,
    /// *Reduce flashing* (GDD §18): flashes become soft glows.
    pub calm: bool,
    /// Effects randomness.
    pub rng: FxRng,
    /// Local views (game cameras) this frame.
    pub views: usize,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            pool: Pool::new(MAX_PARTICLES),
            density: 1.0,
            calm: false,
            rng: FxRng::new(0x5EED_F00D),
            views: 1,
        }
    }
}

impl Fx {
    /// Emits one particle.
    pub fn spawn(&mut self, p: Particle) {
        self.pool.emit(p);
    }

    /// `n` scaled by the density setting (at least one when `n > 0`).
    pub fn n(&self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            ((n as f32 * self.density).round() as u32).max(1)
        }
    }
}

/// Slot entities of the pool (index = slot) and which of them are currently shown.
#[derive(Resource, Debug, Default)]
pub struct Slots {
    entities: Vec<Entity>,
    shown: Vec<bool>,
}

#[cfg(test)]
impl Slots {
    /// Slot entities of this session.
    pub fn len(&self) -> usize {
        self.entities.len()
    }
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Fx>()
        .init_resource::<Slots>()
        .add_systems(OnEnter(RaceSession), spawn_slots)
        .add_systems(OnEnter(RacePhase::Setup), clear_pool)
        .add_systems(OnEnter(AppState::Unloading), forget_slots)
        .add_systems(
            Update,
            (apply_settings, step_pool, sync_slots)
                .chain()
                .in_set(crate::VfxSystems::Simulate),
        );
}

/// The pool's slot entities, hidden until used (session-scoped).
fn spawn_slots(mut commands: Commands, assets: Res<VfxAssets>, mut slots: ResMut<Slots>) {
    let n = MAX_PARTICLES;
    slots.entities = (0..n)
        .map(|_| {
            commands
                .spawn((
                    Mesh3d(assets.particle_mesh.clone()),
                    MeshMaterial3d(assets.particle_material.clone()),
                    MeshTag::new(0),
                    Transform::default(),
                    Visibility::Hidden,
                    NotShadowCaster,
                    DespawnOnExit(RaceSession),
                ))
                .id()
        })
        .collect();
    slots.shown = vec![false; n];
}

fn clear_pool(mut fx: ResMut<Fx>) {
    fx.pool.clear();
}

/// The slot entities despawn with the session: drop their ids and every particle.
fn forget_slots(mut fx: ResMut<Fx>, mut slots: ResMut<Slots>) {
    fx.pool.clear();
    slots.entities.clear();
    slots.shown.clear();
}

/// Density and *Reduce flashing* from `Settings`, view count from the game cameras.
fn apply_settings(
    settings: Option<Res<Settings>>,
    cameras: Query<(), With<GameCamera>>,
    mut fx: ResMut<Fx>,
) {
    let views = cameras.iter().count().max(1);
    let (setting, calm) = settings.map_or((ParticleDensity::High, false), |s| {
        (s.graphics.particles, s.accessibility.reduce_flashing)
    });
    let (rate, budget) = density(setting, views);
    fx.density = rate;
    fx.calm = calm;
    fx.views = views;
    fx.pool.set_limit(budget);
}

fn step_pool(time: Res<Time>, mut fx: ResMut<Fx>) {
    // A long hitch (window drag, load) must not fling particles through the world.
    let dt = time.delta_secs().min(0.1);
    fx.pool.step(dt);
}

/// Copies live particles to their slot entities; hides slots that died.
pub(crate) fn sync_slots(
    fx: Res<Fx>,
    mut slots: ResMut<Slots>,
    mut q: Query<(&mut Transform, &mut MeshTag, &mut Visibility)>,
) {
    let Slots { entities, shown } = &mut *slots;
    for (i, &entity) in entities.iter().enumerate() {
        let live = fx.pool.get(i);
        if live.is_none() && !shown[i] {
            continue;
        }
        let Ok((mut transform, mut tag, mut visibility)) = q.get_mut(entity) else {
            continue;
        };
        match live {
            Some(p) => {
                *transform = p.transform();
                tag.value = p.tag();
                if !shown[i] {
                    *visibility = Visibility::Inherited;
                    shown[i] = true;
                }
            }
            None => {
                *visibility = Visibility::Hidden;
                shown[i] = false;
            }
        }
    }
}

/// A particle-material quad that is not pooled: halos, icons and markers that live as long as
/// the thing they decorate (the transform's scale is the sprite's size). Its look is fixed at
/// spawn; rewrite its `MeshTag` to animate it.
pub(crate) fn sprite(assets: &VfxAssets, look: &Look, transform: Transform) -> impl Bundle {
    (
        Mesh3d(assets.particle_mesh.clone()),
        MeshMaterial3d(assets.particle_material.clone()),
        MeshTag::new(pack_tag(
            look.color[0],
            look.emissive,
            look.shape,
            look.facing,
        )),
        transform,
        NotShadowCaster,
        // Sprites ride moving parents and their AABB is the unit cube: never cull them early.
        NoFrustumCulling,
    )
}

/// An electric zigzag from `from` to `to`: `segments` axial streaks with jittered joints.
#[allow(clippy::too_many_arguments)]
pub(crate) fn zigzag(
    fx: &mut Fx,
    look: Look,
    from: Vec3,
    to: Vec3,
    segments: usize,
    jitter: f32,
    vel: Vec3,
    life: f32,
) {
    let mut prev = from;
    for k in 1..=segments {
        let t = k as f32 / segments as f32;
        let next = if k == segments {
            to
        } else {
            from.lerp(to, t) + fx.rng.dir() * jitter
        };
        segment(fx, look, prev, next, vel, life);
        prev = next;
    }
}

/// One axial streak between two points.
pub(crate) fn segment(fx: &mut Fx, look: Look, a: Vec3, b: Vec3, vel: Vec3, life: f32) {
    let d = b - a;
    let len = d.length();
    if len < 1e-4 {
        return;
    }
    let width = look.size[0].max(1e-4);
    let look = Look {
        aspect: len / width + 1.0,
        ..look
    };
    let p = Particle::new(look, (a + b) * 0.5, vel, life)
        .rotated(Quat::from_rotation_arc(Vec3::Y, d / len));
    fx.spawn(p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_scales_rate_and_budget() {
        assert_eq!(density(ParticleDensity::High, 1), (1.0, MAX_PARTICLES));
        let (low, low_budget) = density(ParticleDensity::Low, 1);
        assert!(low < 0.5 && low_budget < MAX_PARTICLES / 2);
        assert_eq!(
            density(ParticleDensity::High, 4),
            density(ParticleDensity::Medium, 1),
            "4-way split drops one step"
        );
        assert_eq!(
            density(ParticleDensity::Low, 4),
            density(ParticleDensity::Low, 1)
        );
    }

    #[test]
    fn rng_cone_stays_inside_its_angle() {
        let mut rng = FxRng::new(7);
        for _ in 0..500 {
            let d = rng.cone(Vec3::X, 0.3);
            assert!(d.dot(Vec3::X) >= 0.3f32.cos() - 1e-4);
            assert!((d.length() - 1.0).abs() < 1e-4);
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
        }
    }
}
