//! 3D overlays per game camera (GDD §13: name tags and lock-on markers are drawn in 3D, not as
//! UI): every camera sees a tag over each opponent, in the racer's colour, on that camera's own
//! [`player_layer`] — so a player never sees their own tag and each view's tags face it — plus a
//! red reticle over cars locked by a rocket (`ItemTarget`).
//!
//! The names are rendered once per attempt into a small atlas image by an offscreen 2D camera
//! (`Text2d` rows, game font, drop shadow); each tag is a quad showing its racer's row.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite::Text2dShadow;
use bevy::text::FontSize;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::cars::{CarFx, FxVisual, Poses};
use crate::fx::{Fx, sprite};
use crate::particles::{Facing, Look, Shape, pack_tag};
use crate::weather::player_layer;

/// Atlas: one row per grid slot.
const ROW_PX: u32 = 64;
const ATLAS_W_PX: u32 = 512;
const ROWS: u32 = MAX_RACERS as u32;
/// Layer of the atlas camera and its text (no game camera sees it).
const ATLAS_LAYER: usize = 30;
/// The game's display font (rc_ui's `DISPLAY_FONT`).
const FONT: &str = "fonts/Saira-Italic-Variable.ttf";
const FONT_PX: f32 = 40.0;
/// Tag height in world units per unit of distance (≈ constant screen size), and its limits.
const TAG_PER_U: f32 = 0.05;
const TAG_MIN_U: f32 = 0.45;
const TAG_MAX_U: f32 = 2.2;
/// Tags show between these distances (u).
const TAG_NEAR_U: f32 = 2.5;
const TAG_FAR_U: f32 = 75.0;

/// The name atlas of the session.
#[derive(Resource, Debug, Default)]
struct TagAtlas {
    image: Handle<Image>,
    font: Handle<Font>,
    quad: Handle<Mesh>,
}

/// How a racer's tag looks (decorates the racer).
#[derive(Component, Debug)]
struct TagStyle {
    material: Handle<StandardMaterial>,
}

/// One camera's tag over one racer.
#[derive(Component, Debug)]
struct NameTag {
    camera: Entity,
    racer: Entity,
}

/// One camera's reticle over a rocket's locked target.
#[derive(Component, Debug)]
struct LockMarker {
    camera: Entity,
    rocket: Entity,
    age: f32,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<TagAtlas>()
        .add_systems(OnEnter(RaceSession), spawn_atlas)
        .add_systems(
            OnEnter(AppState::Unloading),
            |mut atlas: ResMut<TagAtlas>| {
                // Drop the session's atlas image and quad.
                *atlas = TagAtlas::default();
            },
        )
        .add_observer(add_racer_row)
        .add_systems(
            Update,
            (name_tags, lock_markers).in_set(crate::VfxSystems::Emit),
        );
}

fn spawn_atlas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    server: Res<AssetServer>,
    mut atlas: ResMut<TagAtlas>,
) {
    let image = images.add(Image::new_target_texture(
        ATLAS_W_PX,
        ROW_PX * ROWS,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    commands.spawn((
        Name::new("Name tag atlas camera"),
        Camera2d,
        Camera {
            order: -10,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget::from(image.clone())),
        RenderLayers::layer(ATLAS_LAYER),
        DespawnOnExit(RaceSession),
    ));
    *atlas = TagAtlas {
        image,
        font: server.load(FONT),
        quad: meshes.add(Rectangle::new(ATLAS_W_PX as f32 / ROW_PX as f32, 1.0)),
    };
}

/// Atlas row centre (2D camera space: origin at the image centre, +Y up) of grid slot `row`.
fn row_y(row: u32) -> f32 {
    (ROW_PX * ROWS) as f32 * 0.5 - ROW_PX as f32 * (row as f32 + 0.5)
}

/// UV window of a row in the atlas.
fn row_uv(row: u32) -> Affine2 {
    let h = 1.0 / ROWS as f32;
    Affine2::from_scale_angle_translation(Vec2::new(1.0, h), 0.0, Vec2::new(0.0, row as f32 * h))
}

/// Tag colour: the racer's colour, lifted toward white so dark paints stay readable.
fn tag_color(c: [f32; 3]) -> Color {
    let lift = |v: f32| v + (1.0 - v) * 0.3;
    Color::srgb(lift(c[0]), lift(c[1]), lift(c[2]))
}

fn add_racer_row(
    add: On<Add<Racer>>,
    racers: Query<&Racer>,
    atlas: Res<TagAtlas>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let Ok(racer) = racers.get(add.entity) else {
        return;
    };
    let row = (racer.index as u32).min(ROWS - 1);
    commands.spawn((
        Name::new(format!("Name tag text {}", racer.name)),
        Text2d::new(racer.name.clone()),
        TextFont {
            font: atlas.font.clone().into(),
            font_size: FontSize::Px(FONT_PX),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Text2dShadow {
            offset: Vec2::new(2.5, -2.5),
            color: Color::srgba(0.0, 0.0, 0.0, 0.85),
        },
        Transform::from_xyz(0.0, row_y(row), 0.0),
        RenderLayers::layer(ATLAS_LAYER),
        DespawnOnExit(RaceAttempt),
    ));
    let material = materials.add(StandardMaterial {
        base_color: tag_color(racer.color),
        base_color_texture: Some(atlas.image.clone()),
        uv_transform: row_uv(row),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        fog_enabled: false,
        ..default()
    });
    commands.entity(add.entity).insert(TagStyle { material });
}

/// World height of a tag seen from `distance` u.
pub fn tag_height(distance: f32) -> f32 {
    (distance * TAG_PER_U).clamp(TAG_MIN_U, TAG_MAX_U)
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn name_tags(
    mut commands: Commands,
    atlas: Res<TagAtlas>,
    poses: Poses,
    cameras: Query<(Entity, &GameCamera, &Transform), Without<FxVisual>>,
    racers: Query<(Entity, Option<&LocalPlayer>, &TagStyle, Option<&CarFx>)>,
    mut tags: Query<(Entity, &NameTag, &mut Transform, &mut Visibility), With<FxVisual>>,
    mut have: Local<Vec<(Entity, Entity)>>,
) {
    have.clear();
    for (entity, tag, mut transform, mut visibility) in &mut tags {
        let (Ok((_, _, eye)), Ok((_, _, _, car))) =
            (cameras.get(tag.camera), racers.get(tag.racer))
        else {
            commands.entity(entity).despawn();
            continue;
        };
        have.push((tag.camera, tag.racer));
        let Some(pose) = poses.of(tag.racer) else {
            continue;
        };
        let roof = car.map_or(0.8, |c| c.shape.roof());
        let at = pose.translation + Vec3::Y * (roof + 0.75);
        let d = eye.translation.distance(at);
        let shown = (TAG_NEAR_U..TAG_FAR_U).contains(&d);
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        // Screen-aligned: the quad turns with the camera, like a sign held up to it.
        transform.translation = at;
        transform.rotation = eye.rotation;
        transform.scale = Vec3::splat(tag_height(d));
    }
    for (camera, cam, _) in &cameras {
        for (racer, local, style, _) in &racers {
            let own = local.is_some_and(|l| l.0 == cam.slot);
            if own || have.contains(&(camera, racer)) {
                continue;
            }
            commands.spawn((
                Name::new("Name tag"),
                FxVisual,
                NameTag { camera, racer },
                Mesh3d(atlas.quad.clone()),
                MeshMaterial3d(style.material.clone()),
                Transform::default(),
                Visibility::Hidden,
                RenderLayers::layer(player_layer(cam.slot)),
                NotShadowCaster,
                DespawnOnExit(RaceAttempt),
            ));
        }
    }
}

const RETICLE: LinearRgba = LinearRgba::rgb(16.0, 2.2, 1.2);

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn lock_markers(
    time: Res<Time>,
    fx: Res<Fx>,
    mut commands: Commands,
    assets: Res<VfxAssets>,
    poses: Poses,
    cameras: Query<(Entity, &GameCamera), Without<FxVisual>>,
    rockets: Query<(Entity, &Projectile, &ItemTarget)>,
    locals: Query<&LocalPlayer>,
    mut markers: Query<(Entity, &mut LockMarker, &mut Transform, &mut MeshTag), With<FxVisual>>,
    mut have: Local<Vec<(Entity, Entity)>>,
) {
    let dt = time.delta_secs();
    have.clear();
    for (entity, mut marker, mut transform, mut tag) in &mut markers {
        let target = rockets
            .get(marker.rocket)
            .ok()
            .filter(|(_, p, _)| p.kind == ItemKind::SparkRocket)
            .and_then(|(_, _, t)| t.0);
        let (Some(target), true) = (target, cameras.contains(marker.camera)) else {
            commands.entity(entity).despawn();
            continue;
        };
        have.push((marker.camera, marker.rocket));
        let Some(pose) = poses.of(target) else {
            continue;
        };
        marker.age += dt;
        // Snaps in from big to tight, then pulses (steady with Reduce flashing).
        let lock_in = 1.0 + 1.5 * (1.0 - (marker.age / 0.25).min(1.0)).powi(2);
        let pulse = if fx.calm {
            1.0
        } else {
            1.0 + 0.08 * (marker.age * 14.0).sin()
        };
        transform.translation = pose.translation + pose.rotation * Vec3::Y * 0.6;
        transform.rotation = Quat::from_rotation_z(marker.age * 1.5);
        transform.scale = Vec3::splat(2.0 * lock_in * pulse);
        let k = if fx.calm {
            0.5
        } else {
            0.7 + 0.3 * (marker.age * 14.0).sin().abs()
        };
        tag.value = pack_tag(RETICLE * k, true, Shape::Reticle, Facing::Camera);
    }
    for (rocket, projectile, target) in &rockets {
        let Some(target) = target
            .0
            .filter(|_| projectile.kind == ItemKind::SparkRocket)
        else {
            continue;
        };
        let victim_slot = locals.get(target).ok().map(|l| l.0);
        for (camera, cam) in &cameras {
            // The victim is warned by its HUD arrow; everyone else sees the reticle.
            if victim_slot == Some(cam.slot) || have.contains(&(camera, rocket)) {
                continue;
            }
            let look = Look {
                shape: Shape::Reticle,
                emissive: true,
                color: [RETICLE; 2],
                ..Look::DEFAULT
            };
            commands.spawn((
                Name::new("Lock-on marker"),
                FxVisual,
                LockMarker {
                    camera,
                    rocket,
                    age: 0.0,
                },
                sprite(&assets, &look, Transform::from_scale(Vec3::splat(2.0))),
                RenderLayers::layer(player_layer(cam.slot)),
                DespawnOnExit(RaceAttempt),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_rows_tile_the_image_top_down() {
        assert_eq!(row_y(0), (ROW_PX * ROWS) as f32 / 2.0 - 32.0);
        assert_eq!(row_y(ROWS - 1), -((ROW_PX * ROWS) as f32 / 2.0) + 32.0);
        let uv = row_uv(2);
        let top = uv.transform_point2(Vec2::ZERO);
        let bottom = uv.transform_point2(Vec2::ONE);
        assert!((top.y - 2.0 / ROWS as f32).abs() < 1e-6);
        assert!((bottom.y - 3.0 / ROWS as f32).abs() < 1e-6 && bottom.x == 1.0);
    }

    #[test]
    fn tags_keep_a_readable_size() {
        assert_eq!(tag_height(1.0), TAG_MIN_U);
        assert_eq!(tag_height(500.0), TAG_MAX_U);
        assert!(tag_height(20.0) > tag_height(12.0));
        // Dark racer colours are lifted.
        let c = tag_color([0.0, 0.0, 0.0]).to_srgba();
        assert!(c.red > 0.25);
    }
}
