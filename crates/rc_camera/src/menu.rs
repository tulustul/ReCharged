//! Menu showroom camera (GDD §11, §14): a slow drift around the car on the turntable at
//! [`SHOWROOM_FOCUS`], wide or close per `MenuBackdrop::focus`. It hosts the menu UI.

use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;
use rc_core::prelude::*;

use crate::pose::damp;
use crate::rig::BACKDROP;

/// Framing per backdrop focus: radius, height of the eye and of the look-at point, u.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Framing {
    radius: f32,
    height: f32,
    look_height: f32,
}

const WIDE: Framing = Framing {
    radius: 9.5,
    height: 3.2,
    look_height: 0.6,
};
const CLOSE: Framing = Framing {
    radius: 4.4,
    height: 1.3,
    look_height: 0.45,
};
/// Drift: centre angle around the car (0 = in front of it, on −Z), swing (rad) and rate (rad/s).
const DRIFT_CENTRE: f32 = 0.6;
const DRIFT_SWING: f32 = 0.45;
const DRIFT_RATE: f32 = 0.12;
/// Smooth time of a wide ↔ close change, s.
const REFRAME_S: f32 = 0.6;

/// Showroom drift state.
#[derive(Component, Debug, Clone, Default)]
pub(crate) struct Drift {
    clock: f32,
    framing: Option<Framing>,
    velocity: Vec3,
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Menu), spawn_menu_camera);
}

fn spawn_menu_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Menu camera"),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
        showroom_pose(WIDE, DRIFT_CENTRE),
        Drift::default(),
        MenuCamera,
        UiHost,
        IsDefaultUiCamera,
        DespawnOnExit(AppState::Menu),
    ));
}

/// The showroom eye at `angle` around the car.
fn showroom_pose(f: Framing, angle: f32) -> Transform {
    let eye = SHOWROOM_FOCUS + Quat::from_rotation_y(angle) * Vec3::new(0.0, f.height, -f.radius);
    Transform::from_translation(eye).looking_at(SHOWROOM_FOCUS + Vec3::Y * f.look_height, Vec3::Y)
}

pub(crate) fn drift(
    time: Res<Time>,
    backdrop: Option<Res<MenuBackdrop>>,
    mut cameras: Query<(&mut Drift, &mut Transform), With<MenuCamera>>,
) {
    let goal = match backdrop.map(|b| b.focus) {
        Some(BackdropFocus::Car) => CLOSE,
        _ => WIDE,
    };
    let dt = time.delta_secs();
    for (mut state, mut transform) in &mut cameras {
        state.clock += dt;
        let current = state.framing.unwrap_or(goal);
        let mut v = state.velocity;
        let framing = Framing {
            radius: damp(current.radius, goal.radius, &mut v.x, REFRAME_S, dt),
            height: damp(current.height, goal.height, &mut v.y, REFRAME_S, dt),
            look_height: damp(
                current.look_height,
                goal.look_height,
                &mut v.z,
                REFRAME_S,
                dt,
            ),
        };
        state.velocity = v;
        state.framing = Some(framing);
        let angle = DRIFT_CENTRE + DRIFT_SWING * (state.clock * DRIFT_RATE).sin();
        *transform = showroom_pose(framing, angle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_showroom_eye_frames_the_car_from_the_front() {
        for f in [WIDE, CLOSE] {
            let pose = showroom_pose(f, 0.0);
            assert!(pose.translation.z < 0.0, "in front of a −Z-facing car");
            let to_car = (SHOWROOM_FOCUS - pose.translation).normalize();
            assert!(pose.forward().dot(to_car) > 0.95, "looks at the car");
        }
    }
}
