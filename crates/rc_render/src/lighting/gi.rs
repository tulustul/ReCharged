//! Baked indirect light (`docs/design/rc_editor.md` §Irradiance bake): the session's
//! `IrradianceVolume` from `assets/maps/<track>/<normal|mirror>_<time>.irv`, written by the
//! track editor's baker. Inside it, Bevy takes diffuse indirect light from the volume instead of
//! the camera's environment map (whose specular stays); without a file the look is unchanged.
//!
//! The volume holds one ambient cube per probe: for a surface facing ±X/±Y/±Z, the irradiance
//! arriving over that hemisphere divided by π (so it multiplies the diffuse colour directly, like
//! Bevy's environment maps). Weather scales it like the environment map (`Look::env_light`).
//!
//! File: `RCIV`, version, probe counts (x, y, z), the volume's min and max corner (probes sit at
//! voxel centres), then the RGB9E5 texels in Bevy's layout — a 3D texture of (x, 2y, 3z): rows
//! `[0, y)` hold the +axis faces, `[y, 2y)` the −axis faces; slabs `[0, z)`, `[z, 2z)`, `[2z, 3z)`
//! the X, Y and Z axes (the order `irradiance_volume.wesl` samples).

use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::light::{IrradianceVolume, LightProbe};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rc_core::prelude::*;
use rc_track::Track;

use super::ActiveEnvironment;
use crate::weather_fx::Look;

const MAGIC: &[u8; 4] = b"RCIV";
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 3 * 4 + 6 * 4;

/// Faces of an ambient cube: +X, −X, +Y, −Y, +Z, −Z.
pub const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// A grid of ambient cubes (probe `x` varies fastest, then `y`, then `z`).
#[derive(Debug, Clone, PartialEq)]
pub struct IrradianceGrid {
    /// Probes along x, y, z.
    pub dims: UVec3,
    /// The volume's corners (u); probe (i, j, k) sits at the centre of voxel (i, j, k).
    pub min: Vec3,
    #[allow(missing_docs)]
    pub max: Vec3,
    /// One cube per probe, faces in [`FACES`] order (linear, cd/m²-like: irradiance / π).
    pub cubes: Vec<[Vec3; 6]>,
}

impl IrradianceGrid {
    /// Where probe (x, y, z) sits.
    pub fn probe(&self, x: u32, y: u32, z: u32) -> Vec3 {
        let cell = (self.max - self.min) / self.dims.as_vec3();
        self.min + (UVec3::new(x, y, z).as_vec3() + 0.5) * cell
    }

    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + self.dims.x * (y + self.dims.y * z)) as usize
    }

    /// The RGB9E5 texels of the 3D texture.
    pub fn texels(&self) -> Vec<u8> {
        let UVec3 {
            x: rx,
            y: ry,
            z: rz,
        } = self.dims;
        let mut out = vec![0u8; (rx * 2 * ry * 3 * rz * 4) as usize];
        for z in 0..rz {
            for y in 0..ry {
                for x in 0..rx {
                    let cube = &self.cubes[self.index(x, y, z)];
                    for (face, value) in cube.iter().enumerate() {
                        let (axis, negative) = ((face / 2) as u32, face % 2 == 1);
                        let (ty, tz) = (y + if negative { ry } else { 0 }, z + axis * rz);
                        let at = (x + rx * (ty + 2 * ry * tz)) as usize * 4;
                        out[at..at + 4].copy_from_slice(&rgb9e5(*value).to_le_bytes());
                    }
                }
            }
        }
        out
    }

    /// The `.irv` file contents.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.cubes.len() * 24);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        for v in self.dims.to_array() {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in self.min.to_array().into_iter().chain(self.max.to_array()) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend(self.texels());
        out
    }

    /// Reads an `.irv` file's contents back (values as stored, i.e. RGB9E5-rounded).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < HEADER_BYTES || &bytes[..4] != MAGIC {
            return Err("not an irradiance volume (.irv) file".into());
        }
        let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        if word(4) != VERSION {
            return Err(format!(
                "irradiance volume version {} (want {VERSION})",
                word(4)
            ));
        }
        let dims = UVec3::new(word(8), word(12), word(16));
        let float = |i: usize| f32::from_bits(word(i));
        let min = Vec3::new(float(20), float(24), float(28));
        let max = Vec3::new(float(32), float(36), float(40));
        let count = dims.x as usize * dims.y as usize * dims.z as usize;
        if count == 0 || bytes.len() != HEADER_BYTES + count * 24 || min.cmpge(max).any() {
            return Err(format!("irradiance volume {dims} does not match its size"));
        }
        let mut grid = Self {
            dims,
            min,
            max,
            cubes: vec![[Vec3::ZERO; 6]; count],
        };
        let texels = &bytes[HEADER_BYTES..];
        let UVec3 {
            x: rx,
            y: ry,
            z: rz,
        } = dims;
        for z in 0..rz {
            for y in 0..ry {
                for x in 0..rx {
                    let i = grid.index(x, y, z);
                    for face in 0..6 {
                        let (axis, negative) = ((face / 2) as u32, face % 2 == 1);
                        let (ty, tz) = (y + if negative { ry } else { 0 }, z + axis * rz);
                        let at = (x + rx * (ty + 2 * ry * tz)) as usize * 4;
                        let bits = u32::from_le_bytes(texels[at..at + 4].try_into().expect("4"));
                        grid.cubes[i][face] = from_rgb9e5(bits);
                    }
                }
            }
        }
        Ok(grid)
    }

    /// The 3D texture Bevy samples.
    pub fn image(&self) -> Image {
        Image::new(
            Extent3d {
                width: self.dims.x,
                height: 2 * self.dims.y,
                depth_or_array_layers: 3 * self.dims.z,
            },
            TextureDimension::D3,
            self.texels(),
            TextureFormat::Rgb9e5Ufloat,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

/// The baked volume of a layout at a time of day (reverse layouts share the forward bake: only
/// `dir`-filtered features differ).
pub fn volume_file(assets_dir: &Path, track: &str, mirror: bool, time: TimeOfDay) -> PathBuf {
    let variant = if mirror { "mirror" } else { "normal" };
    let time = format!("{time:?}").to_lowercase();
    rc_track::map_dir(assets_dir, track).join(format!("{variant}_{time}.irv"))
}

/// Packs a linear colour into RGB9E5 (shared exponent; negatives clamp to 0).
pub fn rgb9e5(c: Vec3) -> u32 {
    const MANTISSA: i32 = 9;
    const BIAS: i32 = 15;
    const MAX: f32 = 65408.0; // (2^9 − 1) / 2^9 × 2^16
    let c = c.clamp(Vec3::ZERO, Vec3::splat(MAX));
    let top = c.max_element();
    if top <= 0.0 {
        return 0;
    }
    let mut exp = (top.log2().floor() as i32).max(-BIAS - 1) + 1 + BIAS;
    let mut scale = 2f32.powi(exp - BIAS - MANTISSA);
    if (top / scale).round() as i32 == 1 << MANTISSA {
        exp += 1;
        scale *= 2.0;
    }
    let m = (c / scale).round().as_uvec3().min(UVec3::splat(511));
    m.x | (m.y << 9) | (m.z << 18) | ((exp as u32) << 27)
}

/// Unpacks [`rgb9e5`].
pub fn from_rgb9e5(bits: u32) -> Vec3 {
    let scale = 2f32.powi((bits >> 27) as i32 - 15 - 9);
    Vec3::new(
        (bits & 511) as f32,
        ((bits >> 9) & 511) as f32,
        ((bits >> 18) & 511) as f32,
    ) * scale
}

/// Whether a session's baked volume is used (a diagnostic switch: the track editor's probes
/// toggle). Off = exactly the unbaked look: environment-map diffuse, the fill light on.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BakedLight {
    /// Use the volume.
    pub enabled: bool,
}

impl Default for BakedLight {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// The session's baked volume (its `IrradianceVolume` is present while [`BakedLight`] is on).
#[derive(Component, Debug)]
pub struct BakedGi {
    /// `env_light` of the look the bake was made in (clear weather): weather scales by the ratio.
    clear_env_light: f32,
    voxels: Handle<Image>,
}

/// `Loading`: spawn the session's volume when its track appears (if one was baked).
pub(super) fn spawn_volume(
    mut commands: Commands,
    track: Option<Res<Track>>,
    env: Option<Res<ActiveEnvironment>>,
    paths: Option<Res<UserPaths>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(track), Some(env), Some(paths)) = (track, env, paths) else {
        return;
    };
    if !track.is_added() {
        return;
    }
    let v = track.variant();
    let path = volume_file(&paths.assets_dir, v.track.as_str(), v.mirror, env.time);
    let Ok(bytes) = std::fs::read(&path) else {
        return; // not baked: the environment map lights everything, as before
    };
    let grid = match IrradianceGrid::from_bytes(&bytes) {
        Ok(grid) => grid,
        Err(e) => {
            warn!("{}: {e}; lit without it", path.display());
            return;
        }
    };
    info!(
        "baked light: {} ({} probes)",
        path.display(),
        grid.cubes.len()
    );
    commands.spawn((
        Name::new("Baked indirect light"),
        BakedGi {
            clear_env_light: env.def.look(env.time).env_light.max(1e-6),
            voxels: images.add(grid.image()),
        },
        LightProbe::default(),
        Transform::from_translation((grid.min + grid.max) / 2.0).with_scale(grid.max - grid.min),
        DespawnOnExit(RaceSession),
    ));
}

/// Adds or removes the volume as [`BakedLight`] says.
pub(super) fn switch_volume(
    mut commands: Commands,
    setting: Res<BakedLight>,
    volumes: Query<(Entity, &BakedGi, Has<IrradianceVolume>)>,
) {
    for (entity, baked, on) in &volumes {
        if setting.enabled && !on {
            commands.entity(entity).insert(IrradianceVolume {
                voxels: baked.voxels.clone(),
                intensity: 1.0,
                affects_lightmapped_meshes: true,
            });
        } else if !setting.enabled && on {
            commands.entity(entity).remove::<IrradianceVolume>();
        }
    }
}

/// Weather dims or brightens the bounce like it does the environment map.
pub(super) fn follow_weather(
    look: Option<Res<Look>>,
    mut volumes: Query<(&BakedGi, &mut IrradianceVolume)>,
) {
    let Some(look) = look else {
        return;
    };
    for (baked, mut volume) in &mut volumes {
        let intensity = look.env_light / baked.clear_env_light;
        if (volume.intensity - intensity).abs() > 1e-4 {
            volume.intensity = intensity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb9e5_round_trips_within_its_precision() {
        for c in [
            Vec3::ZERO,
            Vec3::new(1.0, 0.5, 0.25),
            Vec3::new(1200.0, 3.0, 0.0),
            Vec3::new(0.001, 0.002, 0.0005),
        ] {
            let back = from_rgb9e5(rgb9e5(c));
            let tolerance = c.max_element() / 256.0 + 1e-6;
            assert!((back - c).abs().max_element() <= tolerance, "{c} -> {back}");
        }
        assert_eq!(from_rgb9e5(rgb9e5(Vec3::splat(-1.0))), Vec3::ZERO);
    }

    #[test]
    fn a_grid_round_trips_and_lays_faces_out_as_bevy_samples_them() {
        let dims = UVec3::new(3, 2, 2);
        let n = 12;
        let grid = IrradianceGrid {
            dims,
            min: Vec3::new(-3.0, 0.0, -2.0),
            max: Vec3::new(3.0, 4.0, 2.0),
            cubes: (0..n)
                .map(|i| std::array::from_fn(|f| Vec3::splat((i * 6 + f) as f32 + 1.0)))
                .collect(),
        };
        assert_eq!(grid.probe(0, 0, 0), Vec3::new(-2.0, 1.0, -1.0));
        let back = IrradianceGrid::from_bytes(&grid.to_bytes()).unwrap();
        assert_eq!(back, grid);
        // Probe (1, 0, 1), −Y face: texel (1, 0 + ry, 1 + rz).
        let texels = grid.texels();
        let at = (1 + 3 * (2 + 2 * 2 * 3)) * 4;
        let value = from_rgb9e5(u32::from_le_bytes(texels[at..at + 4].try_into().unwrap()));
        assert_eq!(value, grid.cubes[grid.index(1, 0, 1)][3]);
        assert!(IrradianceGrid::from_bytes(b"nope").is_err());
    }
}
