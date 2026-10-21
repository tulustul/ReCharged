//! The light baker (`docs/design/rc_editor.md` §Irradiance bake): traces the running session's own
//! meshes on the CPU (rc_render keeps their CPU copies in the editor) and produces the
//! [`IrradianceGrid`] rc_render loads.
//!
//! Per probe, rays in a fixed Fibonacci set: a ray that leaves the scene sees the environment map's
//! radiance (what Bevy's image-based light would have given); a ray that hits a surface sees it
//! lit by the session's directional lights and lamps (shadow rays for the lights that cast
//! shadows) — and, in the second pass, by the first pass's volume (one more bounce, sky light off
//! the ground). Probes inside solid geometry (rays mostly meet back faces) take their neighbours'
//! values. Direct light itself stays real-time; the volume replaces only the environment map's
//! diffuse part.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy_rapier3d::parry::query::{Ray, RayCast};
use bevy_rapier3d::parry::shape::{CompositeShapeRef, TriMesh};
use rc_render::{FACES, IrradianceGrid};

mod gpu;

pub use gpu::bake_gpu;

/// Shadow and bounce rays start this far off the surface (u).
const EPSILON_U: f32 = 0.02;
/// Probe rays (and the fill's shadow ray) start this far along their direction (u): a probe
/// exactly on a surface (the
/// lattice and the props share round coordinates) would otherwise see it or not by the sign of a
/// rounded zero — differently on the CPU and the GPU. Now it sees past it.
const RAY_START_U: f32 = 1e-3;
/// Rays reach this far (u).
const FAR_U: f32 = 2000.0;
/// A probe whose rays meet back faces this often sits inside solid geometry.
const INSIDE_SHARE: f32 = 0.3;
/// Sky table resolution (longitude × latitude).
const SKY_W: usize = 64;
const SKY_H: usize = 32;

/// A directional light: irradiance on a surface facing it (lux × colour).
#[derive(Debug, Clone, Copy)]
pub struct Sun {
    /// Unit vector toward the light.
    pub toward: Vec3,
    /// lux × linear colour.
    pub irradiance: Vec3,
    /// Casts shadows.
    pub shadows: bool,
    /// Part of the volume: the environment's fill (sky light) is off in the game where a volume
    /// exists, so its light reaches surfaces through the probes, occluded where a probe cannot
    /// see it (under a deck, under a table). Its bounce counts like any light's.
    pub in_volume: bool,
}

/// A point or spot light.
#[derive(Debug, Clone, Copy)]
pub struct Lamp {
    /// Position (u).
    pub pos: Vec3,
    /// Luminous intensity × linear colour (lumens / 4π, as Bevy shades it).
    pub candela: Vec3,
    /// 1 / range² (Bevy's smooth range cut-off).
    pub inverse_range_squared: f32,
    /// Spot: (unit direction, scale, offset) of Bevy's cone attenuation.
    pub spot: Option<(Vec3, f32, f32)>,
    /// Casts shadows.
    pub shadows: bool,
}

/// Everything the tracer needs, gathered on the main thread.
pub struct Scene {
    mesh: TriMesh,
    /// The triangles that cast shadows in the game (`None`: none do). A room's ceiling does not:
    /// the key light reaches the room through it like a skylight (rc_render's indoor model).
    shadow_mesh: Option<TriMesh>,
    normals: Vec<Vec3>,
    material_of: Vec<u32>,
    /// Per triangle: casts shadows (the shadow mesh's triangles, for the GPU's single tree).
    casts: Vec<bool>,
    albedo: Vec<Vec3>,
    double_sided: Vec<bool>,
    /// Directional lights.
    pub suns: Vec<Sun>,
    /// Point and spot lights.
    pub lamps: Vec<Lamp>,
    sky: Vec<Vec3>,
}

/// Collects triangles (world space) with their material, materials and lights into a [`Scene`].
#[derive(Default)]
pub struct SceneBuilder {
    vertices: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
    shadow_indices: Vec<[u32; 3]>,
    normals: Vec<Vec3>,
    material_of: Vec<u32>,
    casts: Vec<bool>,
    albedo: Vec<Vec3>,
    double_sided: Vec<bool>,
}

impl SceneBuilder {
    /// A material; returns its index.
    pub fn material(&mut self, albedo: Vec3, double_sided: bool) -> u32 {
        self.albedo
            .push(albedo.clamp(Vec3::ZERO, Vec3::splat(0.95)));
        self.double_sided.push(double_sided);
        (self.albedo.len() - 1) as u32
    }

    /// A triangle (counter-clockwise front face); `casts_shadows` as its mesh does in the game.
    /// Degenerate ones are dropped.
    pub fn triangle(&mut self, [a, b, c]: [Vec3; 3], material: u32, casts_shadows: bool) {
        let n = (b - a).cross(c - a);
        if !n.is_finite() || n.length_squared() < 1e-12 {
            return;
        }
        let first = self.vertices.len() as u32;
        self.vertices.extend([a, b, c]);
        self.indices.push([first, first + 1, first + 2]);
        if casts_shadows {
            self.shadow_indices.push([first, first + 1, first + 2]);
        }
        self.normals.push(n.normalize());
        self.material_of.push(material);
        self.casts.push(casts_shadows);
    }

    /// Triangles so far.
    pub fn triangles(&self) -> usize {
        self.indices.len()
    }

    /// The scene, with `sky(dir)` sampled into a table for rays that leave it.
    pub fn build(
        self,
        suns: Vec<Sun>,
        lamps: Vec<Lamp>,
        sky: impl Fn(Vec3) -> Vec3,
    ) -> Result<Scene, String> {
        if self.indices.is_empty() {
            return Err("nothing to bake: the session has no meshes".into());
        }
        let shadow_mesh = (!self.shadow_indices.is_empty())
            .then(|| TriMesh::new(self.vertices.clone(), self.shadow_indices))
            .transpose()
            .map_err(|e| format!("{e:?}"))?;
        let mesh = TriMesh::new(self.vertices, self.indices).map_err(|e| format!("{e:?}"))?;
        let sky = (0..SKY_W * SKY_H)
            .map(|i| sky(sky_direction(i % SKY_W, i / SKY_W)))
            .collect();
        Ok(Scene {
            mesh,
            shadow_mesh,
            normals: self.normals,
            material_of: self.material_of,
            casts: self.casts,
            albedo: self.albedo,
            double_sided: self.double_sided,
            suns,
            lamps,
            sky,
        })
    }
}

fn sky_direction(u: usize, v: usize) -> Vec3 {
    let lon = (u as f32 + 0.5) / SKY_W as f32 * 2.0 * PI;
    let lat = ((v as f32 + 0.5) / SKY_H as f32 - 0.5) * PI;
    Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
}

/// Where the probes go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// Volume corners (u).
    pub min: Vec3,
    #[allow(missing_docs)]
    pub max: Vec3,
    /// Probes per axis.
    pub dims: UVec3,
}

impl GridSpec {
    /// Probes over `min..max`, about `horizontal` apart in x and z and `vertical` apart in y; the
    /// box grows to whole cells.
    pub fn new(min: Vec3, max: Vec3, horizontal: f32, vertical: f32) -> Self {
        let step = Vec3::new(horizontal, vertical, horizontal);
        let dims = ((max - min) / step).ceil().max(Vec3::ONE).as_uvec3();
        let centre = (min + max) / 2.0;
        let half = dims.as_vec3() * step / 2.0;
        Self {
            min: centre - half,
            max: centre + half,
            dims,
        }
    }

    /// Probes in all.
    pub fn count(&self) -> usize {
        (self.dims.x * self.dims.y * self.dims.z) as usize
    }
}

impl Scene {
    /// The first surface along a ray: (triangle, distance).
    fn hit(&self, origin: Vec3, dir: Vec3, reach: f32) -> Option<(usize, f32)> {
        let ray = Ray::new(origin, dir);
        CompositeShapeRef(&self.mesh)
            .cast_local_ray_and_get_normal(&ray, reach, false)
            .map(|(triangle, hit)| (triangle as usize, hit.time_of_impact))
    }

    fn blocked(&self, origin: Vec3, dir: Vec3, reach: f32) -> bool {
        self.shadow_mesh
            .as_ref()
            .is_some_and(|m| m.intersects_local_ray(&Ray::new(origin, dir), reach))
    }

    fn sky(&self, dir: Vec3) -> Vec3 {
        let lon = dir.z.atan2(dir.x).rem_euclid(2.0 * PI);
        let lat = dir.y.clamp(-1.0, 1.0).asin();
        let u = ((lon / (2.0 * PI) * SKY_W as f32) as usize).min(SKY_W - 1);
        let v = (((lat / PI + 0.5) * SKY_H as f32) as usize).min(SKY_H - 1);
        self.sky[u + v * SKY_W]
    }

    /// Irradiance from the lights on a surface at `p` facing `n`.
    fn direct(&self, p: Vec3, n: Vec3) -> Vec3 {
        let from = p + n * EPSILON_U;
        let mut e = Vec3::ZERO;
        for sun in &self.suns {
            let c = n.dot(sun.toward);
            if c > 0.0 && !(sun.shadows && self.blocked(from, sun.toward, FAR_U)) {
                e += sun.irradiance * c;
            }
        }
        for lamp in &self.lamps {
            let d = lamp.pos - p;
            let dist2 = d.length_squared().max(1e-4);
            let falloff = (1.0 - (dist2 * lamp.inverse_range_squared).powi(2))
                .clamp(0.0, 1.0)
                .powi(2);
            let l = d / dist2.sqrt();
            let c = n.dot(l);
            if falloff <= 0.0 || c <= 0.0 {
                continue;
            }
            let cone = lamp.spot.map_or(1.0, |(dir, scale, offset)| {
                ((-l).dot(dir) * scale + offset).clamp(0.0, 1.0).powi(2)
            });
            if cone <= 0.0 {
                continue;
            }
            if lamp.shadows && self.blocked(from, l, dist2.sqrt() - 2.0 * EPSILON_U) {
                continue;
            }
            e += lamp.candela * (c * falloff * cone / dist2);
        }
        e
    }

    /// One probe's ambient cube; `None` if it sits inside solid geometry.
    fn probe(&self, p: Vec3, dirs: &[Vec3], bounce: Option<&IrradianceGrid>) -> Option<[Vec3; 6]> {
        let mut sum = [Vec3::ZERO; 6];
        let mut inside = 0usize;
        for &dir in dirs {
            let origin = p + dir * RAY_START_U;
            let radiance = match self.hit(origin, dir, FAR_U) {
                None => self.sky(dir),
                Some((triangle, t)) => {
                    let material = self.material_of[triangle] as usize;
                    let mut n = self.normals[triangle];
                    if n.dot(dir) > 0.0 {
                        n = -n;
                        inside += usize::from(!self.double_sided[material]);
                    }
                    let at = origin + dir * t;
                    let albedo = self.albedo[material];
                    let mut l = albedo * self.direct(at, n) / PI;
                    if let Some(grid) = bounce {
                        l += albedo * sample(grid, at + n * EPSILON_U, n);
                    }
                    l
                }
            };
            for (s, face) in sum.iter_mut().zip(FACES) {
                *s += radiance * face.dot(dir).max(0.0);
            }
        }
        if inside as f32 > INSIDE_SHARE * dirs.len() as f32 {
            return None;
        }
        // ∫ L cos dω / π ≈ Σ L cos × (4π / N) / π.
        let mut cube = sum.map(|s| s * (4.0 / dirs.len() as f32));
        // The fill's own light, where the probe sees it (E / π per face, like the rest).
        for sun in self.suns.iter().filter(|s| s.in_volume) {
            if !self.blocked(p + sun.toward * RAY_START_U, sun.toward, FAR_U) {
                for (c, face) in cube.iter_mut().zip(FACES) {
                    *c += sun.irradiance * (face.dot(sun.toward).max(0.0) / PI);
                }
            }
        }
        Some(cube)
    }
}

/// The volume's value for a surface at `p` facing `n` (trilinear, ambient-cube weights), as Bevy
/// samples it.
pub fn sample(grid: &IrradianceGrid, p: Vec3, n: Vec3) -> Vec3 {
    let cell = (grid.max - grid.min) / grid.dims.as_vec3();
    let f = ((p - grid.min) / cell - 0.5).clamp(Vec3::ZERO, (grid.dims - 1).as_vec3());
    let base = f.floor().as_uvec3().min(grid.dims - 1);
    let t = f - base.as_vec3();
    let nn = n * n;
    let faces = [
        if n.x >= 0.0 { 0 } else { 1 },
        if n.y >= 0.0 { 2 } else { 3 },
        if n.z >= 0.0 { 4 } else { 5 },
    ];
    let mut out = Vec3::ZERO;
    for corner in 0..8u32 {
        let o = UVec3::new(corner & 1, (corner >> 1) & 1, (corner >> 2) & 1);
        let q = (base + o).min(grid.dims - 1);
        let w = Vec3::select(o.as_vec3().cmpeq(Vec3::ONE), t, 1.0 - t);
        let cube = &grid.cubes[(q.x + grid.dims.x * (q.y + grid.dims.y * q.z)) as usize];
        let v = cube[faces[0]] * nn.x + cube[faces[1]] * nn.y + cube[faces[2]] * nn.z;
        out += v * w.x * w.y * w.z;
    }
    out
}

/// `count` directions spread evenly over the sphere (Fibonacci lattice).
pub fn directions(count: usize) -> Vec<Vec3> {
    let golden = PI * (3.0 - 5f32.sqrt());
    (0..count)
        .map(|i| {
            let y = 1.0 - (i as f32 + 0.5) / count as f32 * 2.0;
            let r = (1.0 - y * y).sqrt();
            let a = golden * i as f32;
            Vec3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

/// Bakes the grid: two passes of `rays` rays per probe on every core. `progress` counts probes
/// (2 × count when done).
pub fn bake(scene: &Scene, spec: &GridSpec, rays: usize, progress: &AtomicUsize) -> IrradianceGrid {
    let dirs = directions(rays);
    bake_with(spec, |grid, bounce| {
        Ok(pass(scene, grid, &dirs, bounce, progress))
    })
    .expect("the CPU pass cannot fail")
}

/// The two passes around `pass` (one cube per probe of the grid, `None` inside geometry; the
/// second gets the first's filled volume to bounce): the CPU's or the GPU's.
fn bake_with(
    spec: &GridSpec,
    mut pass: impl FnMut(
        &IrradianceGrid,
        Option<&IrradianceGrid>,
    ) -> Result<Vec<Option<[Vec3; 6]>>, String>,
) -> Result<IrradianceGrid, String> {
    let mut grid = IrradianceGrid {
        dims: spec.dims,
        min: spec.min,
        max: spec.max,
        cubes: vec![[Vec3::ZERO; 6]; spec.count()],
    };
    let first = pass(&grid, None)?;
    let inside: Vec<bool> = first.iter().map(Option::is_none).collect();
    grid.cubes = fill(&grid, first);
    let second = pass(&grid, Some(&grid))?;
    let second: Vec<Option<[Vec3; 6]>> = second
        .into_iter()
        .zip(&inside)
        .map(|(c, &inside)| if inside { None } else { c })
        .collect();
    grid.cubes = fill(&grid, second);
    Ok(grid)
}

fn pass(
    scene: &Scene,
    grid: &IrradianceGrid,
    dirs: &[Vec3],
    bounce: Option<&IrradianceGrid>,
    progress: &AtomicUsize,
) -> Vec<Option<[Vec3; 6]>> {
    let n = grid.cubes.len();
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let chunk = n.div_ceil(threads).max(1);
    let d = grid.dims;
    std::thread::scope(|s| {
        let parts: Vec<_> = (0..n)
            .step_by(chunk)
            .map(|start| {
                s.spawn(move || {
                    (start..(start + chunk).min(n))
                        .map(|i| {
                            let i = i as u32;
                            let p = grid.probe(i % d.x, (i / d.x) % d.y, i / (d.x * d.y));
                            let cube = scene.probe(p, dirs, bounce);
                            progress.fetch_add(1, Ordering::Relaxed);
                            cube
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        parts
            .into_iter()
            .flat_map(|h| h.join().expect("bake thread"))
            .collect()
    })
}

/// Probes inside geometry take, face by face, the darkest of their valid neighbours, growing
/// outward: probes sit about 2 u apart, so one inside a thin deck has neighbours on both sides,
/// and the underside must not borrow the sunlit top's light (a leak); a dark bias on a sunlit
/// surface hardly shows next to its direct light. Anything left (a grid entirely inside) takes the
/// mean of all valid probes.
fn fill(grid: &IrradianceGrid, mut cubes: Vec<Option<[Vec3; 6]>>) -> Vec<[Vec3; 6]> {
    let d = grid.dims.as_ivec3();
    let index = |p: IVec3| (p.x + d.x * (p.y + d.y * p.z)) as usize;
    loop {
        let mut next = cubes.clone();
        let mut changed = false;
        for z in 0..d.z {
            for y in 0..d.y {
                for x in 0..d.x {
                    let p = IVec3::new(x, y, z);
                    if cubes[index(p)].is_some() {
                        continue;
                    }
                    let mut darkest: Option<[Vec3; 6]> = None;
                    for step in [
                        IVec3::X,
                        IVec3::NEG_X,
                        IVec3::Y,
                        IVec3::NEG_Y,
                        IVec3::Z,
                        IVec3::NEG_Z,
                    ] {
                        let q = p + step;
                        if q.cmplt(IVec3::ZERO).any() || q.cmpge(d).any() {
                            continue;
                        }
                        if let Some(c) = cubes[index(q)] {
                            let d = darkest.get_or_insert(c);
                            for (face, v) in d.iter_mut().zip(c) {
                                if v.element_sum() < face.element_sum() {
                                    *face = v;
                                }
                            }
                        }
                    }
                    if darkest.is_some() {
                        next[index(p)] = darkest;
                        changed = true;
                    }
                }
            }
        }
        cubes = next;
        if !changed {
            break;
        }
    }
    let valid: Vec<[Vec3; 6]> = cubes.iter().flatten().copied().collect();
    let mean = if valid.is_empty() {
        [Vec3::ZERO; 6]
    } else {
        let mut m = [Vec3::ZERO; 6];
        for c in &valid {
            m.iter_mut().zip(c).for_each(|(s, v)| *s += *v);
        }
        m.map(|s| s / valid.len() as f32)
    };
    cubes.into_iter().map(|c| c.unwrap_or(mean)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed box (inward-facing walls) of `albedo` with a sun through no opening.
    fn floor_under_sky(albedo: f32) -> Scene {
        let mut b = SceneBuilder::default();
        let m = b.material(Vec3::splat(albedo), false);
        let (a, c) = (Vec3::new(-50.0, 0.0, -50.0), Vec3::new(50.0, 0.0, 50.0));
        // Ground plane facing up.
        b.triangle([a, Vec3::new(a.x, 0.0, c.z), c], m, true);
        b.triangle([a, c, Vec3::new(c.x, 0.0, a.z)], m, true);
        b.build(
            vec![Sun {
                toward: Vec3::Y,
                irradiance: Vec3::splat(1000.0),
                shadows: true,
                in_volume: false,
            }],
            Vec::new(),
            |_| Vec3::splat(10.0),
        )
        .unwrap()
    }

    #[test]
    fn a_uniform_sky_gives_its_radiance_and_the_ground_bounces_the_sun() {
        let scene = floor_under_sky(0.5);
        let dirs = directions(2048);
        let cube = scene.probe(Vec3::new(0.0, 1.0, 0.0), &dirs, None).unwrap();
        // Facing up: the sky (radiance 10) over the whole hemisphere.
        assert!((cube[2].x - 10.0).abs() < 0.3, "{:?}", cube[2]);
        // Facing down: the sunlit ground, radiance 0.5 × 1000 / π ≈ 159.
        assert!((cube[3].x - 159.15).abs() < 4.0, "{:?}", cube[3]);
        // Sideways: half sky, half ground.
        assert!(
            (cube[0].x - (10.0 + 159.15) / 2.0).abs() < 4.0,
            "{:?}",
            cube[0]
        );
    }

    #[test]
    fn probes_under_the_ground_are_filled_from_above() {
        let scene = floor_under_sky(0.5);
        let spec = GridSpec::new(
            Vec3::new(-4.0, -2.0, -4.0),
            Vec3::new(4.0, 4.0, 4.0),
            4.0,
            2.0,
        );
        assert_eq!(spec.dims, UVec3::new(2, 3, 2));
        let grid = bake(&scene, &spec, 256, &AtomicUsize::new(0));
        // Probe (0, 0, 0) (y = −1) is under the ground: it copies (0, 1, 0) above it (index 2;
        // x varies fastest).
        assert_eq!(grid.cubes[0], grid.cubes[2]);
        // The second pass adds the sky light bounced off the ground: brighter from below.
        assert!(grid.cubes[2][3].x > 159.15, "{:?}", grid.cubes[2][3]);
    }

    #[test]
    fn sampling_reads_what_bevy_would() {
        let grid = IrradianceGrid {
            dims: UVec3::new(2, 1, 1),
            min: Vec3::ZERO,
            max: Vec3::new(2.0, 1.0, 1.0),
            cubes: vec![[Vec3::splat(1.0); 6], [Vec3::splat(3.0); 6]],
        };
        assert_eq!(
            sample(&grid, Vec3::new(0.5, 0.5, 0.5), Vec3::Y),
            Vec3::splat(1.0)
        );
        assert_eq!(
            sample(&grid, Vec3::new(1.0, 0.5, 0.5), Vec3::Y),
            Vec3::splat(2.0)
        );
        assert_eq!(
            sample(&grid, Vec3::new(9.0, 0.5, 0.5), Vec3::X),
            Vec3::splat(3.0)
        );
    }
}
