//! Sky (GDD §10.1): sun and moon directions by time of day, the physically based atmosphere for
//! outdoor environments, a star field behind it (the atmosphere composites over the skybox, so
//! stars fade out by themselves in daylight) and the cloud layer ([`clouds`]).

pub(crate) mod clouds;

use bevy::asset::RenderAssetUsages;
use bevy::light::Atmosphere;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// Star field cubemap face size (px).
const STAR_FACE: u32 = 512;
/// Number of stars.
const STAR_COUNT: usize = 5000;

pub(crate) fn build(app: &mut App) {
    clouds::build(app);
}

/// Unit vector from the scene toward a sun at `elevation_deg` above the horizon and
/// `azimuth_deg` clockwise from north (−Z) toward east (+X).
pub fn sun_direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (el, az) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos()).normalize()
}

/// Transform of a directional light shining from `toward_light` (it lights along −Z).
pub fn light_transform(toward_light: Vec3) -> Transform {
    let up = if toward_light.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    Transform::default().looking_to(-toward_light, up)
}

/// The planet with an Earth-like atmosphere (one per outdoor session or menu).
pub(crate) fn atmosphere(
    media: &mut Assets<ScatteringMedium>,
    ground_albedo: [f32; 3],
) -> Atmosphere {
    let mut a = Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256)));
    a.ground_albedo = Vec3::from_array(ground_albedo);
    a
}

/// Sky colours (linear, relative: 1 ≈ a noon sky near the horizon; the environment's
/// `env_light` gives the absolute scale in cd/m²).
const DAY_ZENITH: [f32; 3] = [0.16, 0.32, 0.8];
const DAY_HORIZON: [f32; 3] = [0.72, 0.82, 0.96];
const DUSK_ZENITH: [f32; 3] = [0.16, 0.2, 0.42];
const DUSK_HORIZON: [f32; 3] = [1.0, 0.56, 0.3];
const NIGHT_ZENITH: [f32; 3] = [0.012, 0.02, 0.05];
const NIGHT_HORIZON: [f32; 3] = [0.03, 0.04, 0.07];

/// Overcast sky: a flat grey (relative to the clear sky's brightness).
const OVERCAST: [f32; 3] = [0.5, 0.52, 0.55];

/// Radiance of the sky toward `dir` with the key light (sun or moon) toward `sun`: a zenith to
/// horizon gradient that warms and darkens as the sun sinks, a glow around the sun, the ground
/// (`ground` albedo lit by the sky) below the horizon, all blended toward a flat grey by
/// `overcast` (0–1). Used for outdoor image-based lighting (the visible sky is Bevy's
/// atmosphere plus the cloud layer).
pub(crate) fn sky_radiance(
    dir: Vec3,
    sun: Vec3,
    ground: [f32; 3],
    daylight: bool,
    overcast: f32,
) -> [f32; 3] {
    let clear = clear_sky(dir, sun, ground, daylight);
    let grey: [f32; 3] = {
        let day = if daylight {
            ((sun.y + 0.05) / 0.25).clamp(0.0, 1.0)
        } else {
            0.04
        };
        let below = if dir.y < 0.0 {
            ground.iter().sum::<f32>() / 3.0 * 1.6
        } else {
            1.0
        };
        std::array::from_fn(|i| OVERCAST[i] * day * below)
    };
    let t = overcast.clamp(0.0, 1.0);
    std::array::from_fn(|i| clear[i] + (grey[i] - clear[i]) * t)
}

fn clear_sky(dir: Vec3, sun: Vec3, ground: [f32; 3], daylight: bool) -> [f32; 3] {
    let smooth = |e0: f32, e1: f32, x: f32| {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let lerp3 = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
        std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
    };
    let day = if daylight {
        smooth(-0.05, 0.2, sun.y)
    } else {
        0.0
    };
    let low = 1.0 - smooth(0.05, 0.45, sun.y);
    let zenith = lerp3(NIGHT_ZENITH, lerp3(DAY_ZENITH, DUSK_ZENITH, low), day);
    let horizon = lerp3(NIGHT_HORIZON, lerp3(DAY_HORIZON, DUSK_HORIZON, low), day);
    if dir.y < 0.0 {
        let sky_avg = lerp3(horizon, zenith, 0.4);
        let below = lerp3(horizon, sky_avg, smooth(0.0, 0.15, -dir.y));
        return std::array::from_fn(|i| below[i] * ground[i] * 1.6);
    }
    let mut c = lerp3(horizon, zenith, dir.y.sqrt());
    // Sun glow (forward scattering), warmer near the horizon.
    let cos = dir.dot(sun).max(0.0);
    let glow = (cos.powi(6) * 0.6 + cos.powi(64) * 3.0) * day;
    let tint = lerp3([1.0, 0.95, 0.88], [1.0, 0.6, 0.32], low);
    for (ch, t) in c.iter_mut().zip(tint) {
        *ch += glow * t;
    }
    c
}

/// Writes a cubemap by evaluating `f(world direction)` per texel. Face order and orientation are
/// wgpu's (+X, −X, +Y, −Y, +Z, −Z); Bevy samples cubemaps with z negated (left-handed), so the
/// texel direction is mapped back to Bevy's right-handed world before calling `f`.
pub(crate) fn cubemap_texels(size: u32, mut f: impl FnMut(Vec3) -> [f32; 4]) -> Vec<[f32; 4]> {
    let mut out = Vec::with_capacity((size * size * 6) as usize);
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let s = 2.0 * (x as f32 + 0.5) / size as f32 - 1.0;
                let t = 2.0 * (y as f32 + 0.5) / size as f32 - 1.0;
                out.push(f(face_direction(face, s, t)));
            }
        }
    }
    out
}

/// World direction of cubemap texel (s, t ∈ −1..1, t down) on `face`.
pub(crate) fn face_direction(face: u32, s: f32, t: f32) -> Vec3 {
    let cube = match face {
        0 => Vec3::new(1.0, -t, -s),
        1 => Vec3::new(-1.0, -t, s),
        2 => Vec3::new(s, 1.0, t),
        3 => Vec3::new(s, -1.0, -t),
        4 => Vec3::new(s, -t, 1.0),
        _ => Vec3::new(-s, -t, -1.0),
    };
    Vec3::new(cube.x, cube.y, -cube.z).normalize()
}

/// Inverse of [`face_direction`]: face and texel of a world direction.
pub(crate) fn direction_texel(dir: Vec3, size: u32) -> (u32, u32, u32) {
    let c = Vec3::new(dir.x, dir.y, -dir.z);
    let a = c.abs();
    let (face, s, t, m) = if a.x >= a.y && a.x >= a.z {
        if c.x > 0.0 {
            (0, -c.z, -c.y, a.x)
        } else {
            (1, c.z, -c.y, a.x)
        }
    } else if a.y >= a.z {
        if c.y > 0.0 {
            (2, c.x, c.z, a.y)
        } else {
            (3, c.x, -c.z, a.y)
        }
    } else if c.z > 0.0 {
        (4, c.x, -c.y, a.z)
    } else {
        (5, -c.x, -c.y, a.z)
    };
    let px = |v: f32| (((v / m + 1.0) * 0.5 * size as f32) as u32).min(size - 1);
    (face, px(s), px(t))
}

/// Packs texels into an Rgba16Float cube image.
pub(crate) fn cube_image(size: u32, texels: &[[f32; 4]]) -> Image {
    let data = texels
        .iter()
        .flat_map(|c| c.map(f16_bits))
        .flat_map(u16::to_le_bytes)
        .collect();
    cube_with(size, data, TextureFormat::Rgba16Float)
}

fn cube_with(size: u32, data: Vec<u8>, format: TextureFormat) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image
}

/// IEEE half-float bits of a non-negative value (truncating; tiny values flush to zero).
fn f16_bits(v: f32) -> u16 {
    let x = v.clamp(0.0, 65504.0);
    if x < 6.2e-5 {
        return 0;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = (bits >> 13) & 0x3ff;
    (((exp as u32) << 10) | mantissa) as u16
}

/// Star field: a sparse cubemap of tinted points, brighter toward the zenith band of the
/// Milky Way, seeded deterministically.
pub(crate) fn star_cubemap() -> Image {
    let size = STAR_FACE;
    let mut data = vec![0u8; (size * size * 6 * 4) as usize];
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        // SplitMix64: deterministic, no RNG dependency.
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f32 / (1u64 << 53) as f32
    };
    let band = Vec3::new(0.35, 0.55, -0.76).normalize();
    for _ in 0..STAR_COUNT {
        let z = next() * 2.0 - 1.0;
        let phi = next() * std::f32::consts::TAU;
        let r = (1.0 - z * z).sqrt();
        let dir = Vec3::new(r * phi.cos(), z, r * phi.sin());
        // Denser near a great-circle band (a hint of the Milky Way).
        let near_band = 1.0 - dir.dot(band).abs();
        if next() > 0.35 + 0.65 * near_band.powi(3) {
            continue;
        }
        let magnitude = next().powf(6.0);
        let warmth = next();
        let tint = [0.85 + 0.15 * warmth, 0.9, 1.0 - 0.15 * warmth];
        let (face, x, y) = direction_texel(dir, size);
        let i = ((face * size * size + y * size + x) * 4) as usize;
        for (c, t) in tint.iter().enumerate() {
            let v = (0.08 + 0.92 * magnitude) * t;
            data[i + c] = data[i + c].saturating_add((v * 255.0) as u8);
        }
        data[i + 3] = 255;
    }
    cube_with(size, data, TextureFormat::Rgba8UnormSrgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_direction_follows_compass_and_elevation() {
        let east_morning = sun_direction(15.0, 90.0);
        assert!(east_morning.x > 0.9 && east_morning.y > 0.2);
        let noon = sun_direction(60.0, 180.0);
        assert!(noon.z > 0.0, "south is +Z");
        assert!((noon.y - 60f32.to_radians().sin()).abs() < 1e-5);
        let night = sun_direction(-20.0, 270.0);
        assert!(night.y < 0.0 && night.x < 0.0);
        assert!((sun_direction(33.0, 47.0).length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lights_shine_away_from_the_sun() {
        let dir = sun_direction(30.0, 120.0);
        let t = light_transform(dir);
        assert!(t.forward().dot(-dir) > 0.999);
        let zenith = light_transform(Vec3::Y);
        assert!(
            zenith.forward().dot(Vec3::NEG_Y) > 0.999,
            "no NaN straight up"
        );
    }

    #[test]
    fn cubemap_faces_round_trip_directions() {
        let size = 64;
        for face in 0..6 {
            for (x, y) in [(0, 0), (10, 50), (63, 63), (32, 7)] {
                let s = 2.0 * (x as f32 + 0.5) / size as f32 - 1.0;
                let t = 2.0 * (y as f32 + 0.5) / size as f32 - 1.0;
                let dir = face_direction(face, s, t);
                assert_eq!(direction_texel(dir, size), (face, x, y));
            }
        }
        // +Y face is "up" in Bevy's world too; −Z (forward) maps to wgpu's +Z face.
        assert_eq!(direction_texel(Vec3::Y, size).0, 2);
        assert_eq!(direction_texel(Vec3::NEG_Z, size).0, 4);
    }

    #[test]
    fn the_sky_is_bluer_at_noon_warmer_at_sunset_dark_at_night() {
        let ground = [0.3, 0.3, 0.3];
        let noon = sun_direction(60.0, 180.0);
        let zenith = sky_radiance(Vec3::Y, noon, ground, true, 0.0);
        assert!(zenith[2] > zenith[0] * 2.0, "blue zenith {zenith:?}");
        let sunset = sun_direction(4.0, 270.0);
        let toward_sun = sky_radiance(sun_direction(5.0, 270.0), sunset, ground, true, 0.0);
        assert!(toward_sun[0] > toward_sun[2], "warm glow {toward_sun:?}");
        let night = sky_radiance(Vec3::Y, sun_direction(40.0, 200.0), ground, false, 0.0);
        assert!(night.iter().all(|c| *c < 0.1), "{night:?}");
        let below = sky_radiance(Vec3::NEG_Y, noon, ground, true, 0.0);
        assert!(below[1] < zenith[1] + 1.0 && below[1] > 0.0);
        let grey = sky_radiance(Vec3::Y, noon, ground, true, 1.0);
        assert!((grey[2] - grey[0]).abs() < 0.1, "overcast is grey {grey:?}");
    }

    #[test]
    fn half_floats_encode_common_values() {
        assert_eq!(f16_bits(0.0), 0);
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(2.0), 0x4000);
        assert_eq!(f16_bits(0.5), 0x3800);
        assert_eq!(f16_bits(-3.0), 0);
        assert_eq!(f16_bits(1e9), 0x7bff, "clamped to the largest finite half");
    }
}
