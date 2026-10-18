//! Shared meshes and materials of the effects, created once at startup (never per frame): the
//! particle quad + [`ParticleMaterial`], the fresnel [`BubbleMaterial`] (shields, EMP shells) and
//! the primitives and PBR materials of item and pickup visuals.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;

use crate::particles::MAX_INTENSITY;

/// The particle shader (`assets/shaders/vfx/particle.wesl`).
const PARTICLE_SHADER: &str = "shaders/vfx/particle.wesl";
/// The fresnel bubble shader (`assets/shaders/vfx/bubble.wesl`).
const BUBBLE_SHADER: &str = "shaders/vfx/bubble.wesl";
/// Side of the generated oil-sheen texture, px.
const OIL_TEXTURE_PX: u32 = 128;

/// The particle material: `StandardMaterial` lighting with a billboarding vertex stage and
/// procedural shapes decoded from each instance's `MeshTag` (see [`crate::particles`]).
pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleExtension>;

/// Shader parameters of the particle material.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct ParticleExtension {
    /// x: intensity of a full emissive level byte ([`MAX_INTENSITY`]).
    #[uniform(100)]
    pub params: Vec4,
}

impl MaterialExtension for ParticleExtension {
    fn vertex_shader() -> ShaderRef {
        PARTICLE_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        PARTICLE_SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// An additive energy shell: bright fresnel rim, faint body, moving bands (Shield Bubble, EMP).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct BubbleMaterial {
    /// HDR colour of the rim.
    #[uniform(0)]
    pub color: LinearRgba,
    /// x: rim exponent, y: rim intensity, z: body intensity, w: hit flash 0–1.
    #[uniform(1)]
    pub params: Vec4,
}

impl BubbleMaterial {
    /// A shell of `color` with rim `intensity`.
    pub fn new(color: LinearRgba, intensity: f32) -> Self {
        Self {
            color,
            params: Vec4::new(2.6, intensity, intensity * 0.06, 0.0),
        }
    }
}

impl Material for BubbleMaterial {
    fn fragment_shader() -> ShaderRef {
        BUBBLE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

/// Handles of every shared effects asset.
#[derive(Resource, Debug, Clone, Default)]
pub struct VfxAssets {
    /// One quad for every particle and sprite (corners in UV, AABB = unit cube).
    pub particle_mesh: Handle<Mesh>,
    /// The one particle material (premultiplied alpha: additive glows and lit puffs in one batch).
    pub particle_material: Handle<ParticleMaterial>,
    /// Unit cube.
    pub cube: Handle<Mesh>,
    /// Unit-diameter sphere.
    pub sphere: Handle<Mesh>,
    /// Unit cylinder (diameter 1, height 1, along Y).
    pub cylinder: Handle<Mesh>,
    /// Unit cone (base diameter 1, height 1, tip at +Y).
    pub cone: Handle<Mesh>,
    /// Coil ring of the Spring Pad (outer diameter 1).
    pub torus: Handle<Mesh>,
    /// Unit square in the XZ plane facing +Y (ground decals).
    pub decal: Handle<Mesh>,
    /// Translucent glowing glass of the Volt Cubes.
    pub cube_glass: Handle<StandardMaterial>,
    /// Glossy white plastic (rocket body).
    pub white: Handle<StandardMaterial>,
    /// Glossy red plastic (rocket nose, goo light housing).
    pub red: Handle<StandardMaterial>,
    /// Satin black plastic (fins, nozzles, battery band, pad base).
    pub black: Handle<StandardMaterial>,
    /// Polished gold (Golden Battery).
    pub gold: Handle<StandardMaterial>,
    /// Chrome (battery terminal, spring coil).
    pub chrome: Handle<StandardMaterial>,
    /// Shiny green slime (Goo Bomb).
    pub goo: Handle<StandardMaterial>,
    /// Yellow plastic with a warm glow (Spring Pad plate).
    pub spring_plate: Handle<StandardMaterial>,
    /// Glowing plasma core (Shock Orb).
    pub plasma: Handle<StandardMaterial>,
    /// Dark oil with a rainbow sheen (Oil Slick).
    pub oil: Handle<StandardMaterial>,
    /// Cyan shield shell (template; shields clone it to flash on hits).
    pub shield: BubbleMaterial,
    /// Faint violet field showing a Shock Orb's knock radius.
    pub orb_field: Handle<BubbleMaterial>,
}

pub(crate) fn build(app: &mut App) {
    app.add_plugins((
        MaterialPlugin::<ParticleMaterial>::default(),
        MaterialPlugin::<BubbleMaterial>::default(),
    ))
    .add_systems(Startup, create_assets);
}

fn create_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut bubbles: ResMut<Assets<BubbleMaterial>>,
) {
    let mut pbr = |base: [f32; 3], rough: f32, metal: f32, glow: LinearRgba| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(base[0], base[1], base[2]),
            emissive: glow,
            perceptual_roughness: rough,
            metallic: metal,
            ..default()
        })
    };
    let none = LinearRgba::BLACK;
    let white = pbr([0.92, 0.92, 0.9], 0.3, 0.0, none);
    let red = pbr([0.85, 0.12, 0.1], 0.3, 0.0, none);
    let black = pbr([0.08, 0.08, 0.09], 0.55, 0.0, none);
    // A warm glow keeps the gold golden where there is little to reflect.
    let gold = pbr(
        [1.0, 0.78, 0.34],
        0.22,
        1.0,
        LinearRgba::rgb(0.5, 0.32, 0.05),
    );
    let chrome = pbr([0.96, 0.96, 0.97], 0.08, 1.0, none);
    let oil_texture = images.add(oil_image());
    commands.insert_resource(VfxAssets {
        particle_mesh: meshes.add(particle_quad()),
        particle_material: particle_materials.add(ParticleMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.95,
                reflectance: 0.2,
                alpha_mode: AlphaMode::Premultiplied,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: ParticleExtension {
                params: Vec4::new(MAX_INTENSITY, 0.0, 0.0, 0.0),
            },
        }),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        sphere: meshes.add(Sphere::new(0.5).mesh().uv(32, 16)),
        cylinder: meshes.add(Cylinder::new(0.5, 1.0)),
        cone: meshes.add(Cone {
            radius: 0.5,
            height: 1.0,
        }),
        torus: meshes.add(Torus {
            minor_radius: 0.07,
            major_radius: 0.43,
        }),
        decal: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        cube_glass: materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.85, 1.0, 0.28),
            emissive: LinearRgba::rgb(0.25, 1.1, 1.8),
            perceptual_roughness: 0.12,
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        white,
        red,
        black,
        gold,
        chrome,
        goo: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.85, 0.12),
            emissive: LinearRgba::rgb(0.05, 0.25, 0.02),
            perceptual_roughness: 0.1,
            reflectance: 0.7,
            ..default()
        }),
        spring_plate: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.75, 0.1),
            emissive: LinearRgba::rgb(0.6, 0.35, 0.02),
            perceptual_roughness: 0.3,
            ..default()
        }),
        plasma: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(5.0, 3.0, 12.0),
            unlit: true,
            ..default()
        }),
        oil: materials.add(StandardMaterial {
            base_color_texture: Some(oil_texture),
            perceptual_roughness: 0.06,
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        shield: BubbleMaterial::new(LinearRgba::rgb(0.4, 1.3, 2.4), 2.2),
        orb_field: bubbles.add(BubbleMaterial::new(LinearRgba::rgb(0.5, 0.3, 1.2), 0.3)),
    });
}

/// The particle quad: UVs carry the corners; positions only span the unit cube so the AABB
/// (frustum culling) covers the quad in every orientation the shader gives it.
pub(crate) fn particle_quad() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, 0.5],
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343) ^ (y as u32).wrapping_mul(0xD816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

fn value_noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * ux;
    let b = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * ux;
    a + (b - a) * uy
}

/// Oil puddle texture: an irregular blob (alpha) of near-black oil with thin-film rainbow swirls.
pub(crate) fn oil_image() -> Image {
    let n = OIL_TEXTURE_PX;
    let mut data = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let r = (u * u + v * v).sqrt();
            let a = v.atan2(u);
            let rim = 0.8
                + 0.08 * (a * 3.0 + 0.7).sin()
                + 0.05 * (a * 7.0 + 2.1).sin()
                + 0.04 * (a * 11.0).sin();
            let alpha = ((rim - r) / 0.05).clamp(0.0, 1.0) * 0.96;
            // Thin film: hue follows a swirl of noise; strongest in bands.
            let swirl = value_noise(u * 3.0 + 4.0, v * 3.0 + 7.0) * 1.6 + r * 1.2;
            let band = (swirl * std::f32::consts::TAU).sin() * 0.5 + 0.5;
            let sheen = band.powf(4.0) * 0.5;
            let hue = swirl.fract() * std::f32::consts::TAU;
            let film = [
                0.5 + 0.5 * hue.cos(),
                0.5 + 0.5 * (hue - 2.1).cos(),
                0.5 + 0.5 * (hue - 4.2).cos(),
            ];
            let base = [0.02, 0.018, 0.015];
            for c in 0..3 {
                // Mostly black oil; the thin-film colours only glint in narrow bands.
                let lin = base[c] + film[c] * sheen * 0.09;
                data.push((lin.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8);
            }
            data.push((alpha * 255.0).round() as u8);
        }
    }
    Image::new(
        Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oil_texture_is_a_soft_blob() {
        let img = oil_image();
        let data = img.data.as_ref().expect("pixels");
        let n = OIL_TEXTURE_PX as usize;
        let alpha = |x: usize, y: usize| data[(y * n + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0, "corners are clear");
        assert!(alpha(n / 2, n / 2) > 200, "the centre is oil");
        let px = |x: usize, y: usize| &data[(y * n + x) * 4..(y * n + x) * 4 + 3];
        assert!(px(n / 2, n / 2).iter().all(|&c| c < 160), "oil is dark");
    }
}
