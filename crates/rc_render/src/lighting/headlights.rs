//! Headlights (GDD §10.1): at Night every local player's car gets two shadowless spot lights,
//! children of its visual proxy. (Lens emissives belong to the car visuals.)

use bevy::prelude::*;
use rc_core::prelude::*;

use super::ActiveEnvironment;

/// Spot intensity (lumens), range (u) and cone (rad) of one headlight: lights the road ≈ 25 u
/// ahead of a 2 u car.
const HEADLIGHT_LUMENS: f32 = 40_000.0;
const HEADLIGHT_RANGE_U: f32 = 45.0;
const HEADLIGHT_OUTER: f32 = 0.55;
const HEADLIGHT_INNER: f32 = 0.2;
/// Mount point in car space (x = ± half spacing, y up, z forward is −) and aim.
const MOUNT: Vec3 = Vec3::new(0.3, 0.38, -0.95);
const AIM: Vec3 = Vec3::new(0.04, -0.13, -1.0);
/// Warm white LED.
const HEADLIGHT_COLOR: Color = Color::srgb(1.0, 0.94, 0.84);

/// The proxy already carries headlights.
#[derive(Component, Debug, Clone, Copy)]
struct Headlights;

pub(super) fn build(app: &mut App) {
    app.add_systems(Update, add_headlights.in_set(VisualSet));
}

fn add_headlights(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    proxies: Query<(Entity, &VisualOf), Without<Headlights>>,
    locals: Query<(), With<LocalPlayer>>,
) {
    if env.is_none_or(|e| e.time != TimeOfDay::Night) {
        return;
    }
    for (proxy, of) in &proxies {
        if locals.get(of.0).is_err() {
            continue;
        }
        commands
            .entity(proxy)
            .insert(Headlights)
            .with_children(|car| {
                for side in [-1.0, 1.0] {
                    let mount = MOUNT * Vec3::new(side, 1.0, 1.0);
                    let aim = AIM * Vec3::new(side, 1.0, 1.0);
                    car.spawn((
                        Name::new("Headlight"),
                        SpotLight {
                            color: HEADLIGHT_COLOR,
                            intensity: HEADLIGHT_LUMENS,
                            range: HEADLIGHT_RANGE_U,
                            outer_angle: HEADLIGHT_OUTER,
                            inner_angle: HEADLIGHT_INNER,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(mount).looking_to(aim, Vec3::Y),
                    ));
                }
            });
    }
}
