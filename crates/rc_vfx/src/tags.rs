//! 3D overlays per game camera (GDD §13: name tags and lock-on markers are drawn in 3D, not as
//! UI): every camera sees a tag over each opponent, in the racer's colour, on that camera's own
//! [`player_layer`] — so a player never sees their own tag and each view's tags face it — plus a
//! red reticle over cars locked by a rocket (`ItemTarget`).
//!
//! The names are rendered once per attempt into a small atlas image by an offscreen 2D camera
//! (`Text2d` rows, game font, drop shadow); each tag is a quad showing its racer's row. With
//! *Colour-blind markers* on, each row also shows the racer's marker left of the name (rc_core
//! [`RacerMarker`]: a shape from a generated strip for rivals, the player number for humans —
//! the same mapping the HUD's minimap and standings use). Tags fade out closer than
//! [`TAG_FADE_U`] so a car alongside is not covered by its name.

mod markers;

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite::Text2dShadow;
use bevy::text::{FontSize, TextLayoutInfo};
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::cars::{CarFx, FxVisual, Poses};
use crate::fx::{Fx, sprite};
use crate::particles::{Facing, Look, Shape, pack_tag};
use crate::weather::player_layer;
use markers::{marker_rect, marker_strip};

/// Atlas: one row per grid slot.
const ROW_PX: u32 = 64;
const ATLAS_W_PX: u32 = 512;
const ROWS: u32 = MAX_RACERS as u32;
/// Layer of the atlas camera and its text (no game camera sees it).
const ATLAS_LAYER: usize = 30;
/// The game's display font (rc_ui's `DISPLAY_FONT`).
const FONT: &str = "fonts/Saira-Italic-Variable.ttf";
const FONT_PX: f32 = 40.0;
/// Tag height in world units per unit of distance (≈ constant screen size), and its limits: the
/// floor applies only where tags are fading out, the cap shrinks far tags (less clutter).
const TAG_PER_U: f32 = 0.05;
const TAG_MIN_U: f32 = 0.3;
const TAG_MAX_U: f32 = 1.6;
/// Tags fade out from this distance (u) down to [`TAG_NEAR_U`], where they are hidden…
pub const TAG_FADE_U: f32 = 6.0;
const TAG_NEAR_U: f32 = 3.0;
/// …and show up to this distance (u).
const TAG_FAR_U: f32 = 75.0;
/// Fade steps: a tag's material changes only when its alpha crosses one.
const FADE_STEPS: f32 = 16.0;
/// A marker's size in the atlas and its gap to the name (px).
const MARKER_PX: f32 = 40.0;
const MARKER_GAP_PX: f32 = 12.0;
/// The name text's drop shadow (also under the markers).
const SHADOW_OFFSET: Vec2 = Vec2::new(2.5, -2.5);
const SHADOW_ALPHA: f32 = 0.85;

/// The name atlas of the session.
#[derive(Resource, Debug, Default)]
struct TagAtlas {
    image: Handle<Image>,
    font: Handle<Font>,
    quad: Handle<Mesh>,
    /// The marker shapes, white on transparent.
    markers: Handle<Image>,
}

/// How a racer's tag looks (decorates the racer).
#[derive(Component, Debug)]
struct TagStyle {
    material: Handle<StandardMaterial>,
}

/// One camera's tag over one racer (its own material copy, so it can fade on its own).
#[derive(Component, Debug)]
struct NameTag {
    camera: Entity,
    racer: Entity,
    alpha: f32,
}

/// A row's colour-blind marker (or its shadow), placed left of the row's name text.
#[derive(Component, Debug)]
struct RowMarker {
    text: Entity,
    shadow: bool,
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
        )
        // Not paused with the race: the setting may change in the pause menu.
        .add_systems(Update, (place_markers, show_markers).in_set(VisualSet));
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
        markers: images.add(marker_strip()),
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
    settings: Option<Res<Settings>>,
    config: Option<Res<RaceConfig>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let Ok(racer) = racers.get(add.entity) else {
        return;
    };
    let row = (racer.index as u32).min(ROWS - 1);
    let font = TextFont {
        font: atlas.font.clone().into(),
        font_size: FontSize::Px(FONT_PX),
        ..default()
    };
    let shadow = Text2dShadow {
        offset: SHADOW_OFFSET,
        color: Color::srgba(0.0, 0.0, 0.0, SHADOW_ALPHA),
    };
    let text = commands
        .spawn((
            Name::new(format!("Name tag text {}", racer.name)),
            Text2d::new(racer.name.clone()),
            font.clone(),
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Center),
            shadow,
            Transform::from_xyz(0.0, row_y(row), 0.0),
            RenderLayers::layer(ATLAS_LAYER),
            DespawnOnExit(RaceAttempt),
        ))
        .id();
    let shown = settings.is_some_and(|s| s.accessibility.colorblind_markers);
    let visibility = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let at = Transform::from_xyz(0.0, row_y(row), 0.0);
    let scope = (RenderLayers::layer(ATLAS_LAYER), DespawnOnExit(RaceAttempt));
    // Human or rival by the grid (rc_race adds `LocalPlayer` after this observer runs); rc_ui
    // reads the same `RaceConfig`, so tags and HUD always show the same marker.
    let slot = config
        .as_ref()
        .and_then(|c| c.racers.get(racer.index))
        .and_then(|r| match r.controller {
            Controller::Human { slot, .. } => Some(slot),
            Controller::Ai { .. } => None,
        });
    match RacerMarker::of(racer.index, slot) {
        RacerMarker::Shape(shape) => {
            for (shadow, color) in [
                (true, Color::srgba(0.0, 0.0, 0.0, SHADOW_ALPHA)),
                (false, Color::WHITE),
            ] {
                let sprite = Sprite {
                    image: atlas.markers.clone(),
                    rect: Some(marker_rect(shape)),
                    custom_size: Some(Vec2::splat(MARKER_PX)),
                    color,
                    ..default()
                };
                let below = if shadow {
                    SHADOW_OFFSET.extend(-0.1) * Vec3::new(0.0, 1.0, 1.0)
                } else {
                    Vec3::ZERO
                };
                commands.spawn((
                    Name::new("Name tag marker"),
                    sprite,
                    RowMarker { text, shadow },
                    at.with_translation(at.translation + below),
                    visibility,
                    scope.clone(),
                ));
            }
        }
        // Humans are P1–P4; split-screen names already are (then nothing is added).
        RacerMarker::Player(slot) => {
            let label = format!("P{}", slot.0 + 1);
            if label != racer.name {
                commands.spawn((
                    Name::new("Name tag marker"),
                    Text2d::new(label),
                    font,
                    TextColor(Color::WHITE),
                    shadow,
                    RowMarker {
                        text,
                        shadow: false,
                    },
                    at,
                    visibility,
                    scope,
                ));
            }
        }
    }
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

/// Opacity of a tag seen from `distance` u: fades out closer than [`TAG_FADE_U`] (stepped, so
/// its material changes only now and then), 0 beyond the far limit.
pub fn tag_alpha(distance: f32) -> f32 {
    if distance >= TAG_FAR_U {
        return 0.0;
    }
    let t = ((distance - TAG_NEAR_U) / (TAG_FADE_U - TAG_NEAR_U)).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t) * FADE_STEPS).round() / FADE_STEPS
}

/// Keeps each marker just left of its name (the name is centred, its width known once laid out).
fn place_markers(texts: Query<&TextLayoutInfo>, mut markers: Query<(&RowMarker, &mut Transform)>) {
    for (marker, mut transform) in &mut markers {
        let Ok(layout) = texts.get(marker.text) else {
            continue;
        };
        let width = layout.size.x / layout.scale_factor.max(1e-3);
        let offset = if marker.shadow { SHADOW_OFFSET.x } else { 0.0 };
        let x = -(width + MARKER_PX) * 0.5 - MARKER_GAP_PX + offset;
        if (transform.translation.x - x).abs() > 0.5 {
            transform.translation.x = x;
        }
    }
}

/// Shows the markers while *Colour-blind markers* is on (toggled live, also from the pause menu).
fn show_markers(
    settings: Option<Res<Settings>>,
    mut markers: Query<&mut Visibility, With<RowMarker>>,
) {
    let Some(settings) = settings.filter(|s| s.is_changed()) else {
        return;
    };
    let shown = if settings.accessibility.colorblind_markers {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut markers {
        visibility.set_if_neq(shown);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn name_tags(
    mut commands: Commands,
    atlas: Res<TagAtlas>,
    poses: Poses,
    cameras: Query<(Entity, &GameCamera, &Transform), Without<FxVisual>>,
    racers: Query<(Entity, Option<&LocalPlayer>, &TagStyle, Option<&CarFx>)>,
    mut tags: Query<
        (
            Entity,
            &mut NameTag,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<FxVisual>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut have: Local<Vec<(Entity, Entity)>>,
) {
    have.clear();
    for (entity, mut tag, mut transform, mut visibility, material) in &mut tags {
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
        let alpha = tag_alpha(d);
        visibility.set_if_neq(if alpha > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if alpha > 0.0 && alpha != tag.alpha {
            tag.alpha = alpha;
            if let Some(mut m) = materials.get_mut(&material.0) {
                m.base_color.set_alpha(alpha);
            }
        }
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
            // Each camera's tag fades on its own: a copy of the racer's material.
            let material = materials
                .get(&style.material)
                .cloned()
                .map_or_else(|| style.material.clone(), |m| materials.add(m));
            commands.spawn((
                Name::new("Name tag"),
                FxVisual,
                NameTag {
                    camera,
                    racer,
                    alpha: 1.0,
                },
                Mesh3d(atlas.quad.clone()),
                MeshMaterial3d(material),
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
        // Constant on screen from where the fade ends to the cap: never bigger up close.
        assert!((tag_height(TAG_FADE_U) - TAG_FADE_U * TAG_PER_U).abs() < 1e-6);
        // Dark racer colours are lifted.
        let c = tag_color([0.0, 0.0, 0.0]).to_srgba();
        assert!(c.red > 0.25);
    }

    #[test]
    fn near_tags_fade_out() {
        assert_eq!(tag_alpha(TAG_FADE_U + 1.0), 1.0);
        assert_eq!(tag_alpha(TAG_NEAR_U), 0.0, "hidden right next to the car");
        let mid = tag_alpha((TAG_FADE_U + TAG_NEAR_U) / 2.0);
        assert!(mid > 0.3 && mid < 0.7, "{mid}");
        assert!(tag_alpha(4.0) < tag_alpha(5.0));
        assert_eq!(tag_alpha(TAG_FAR_U), 0.0);
        let steps = (tag_alpha(4.3) * FADE_STEPS).fract();
        assert!(steps.abs() < 1e-5, "stepped");
    }
}
