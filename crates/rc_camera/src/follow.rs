//! Following a racer (GDD §11): chase / far chase / hood, look-back, spring-damped follow with
//! velocity look-ahead, obstruction pull-in, Hor+ FOV with speed/boost widening, and the views the
//! director picks (intro fly-by, finish orbit, spectate). The pose maths live in [`crate::pose`].

use std::f32::consts::PI;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::{QueryFilter, ReadRapierContext};
use rc_core::prelude::*;

use crate::layout::{MAX_2P_HFOV_DEG, REFERENCE_ASPECT, hor_plus_vfov};
use crate::pose::{
    Boom, CHASE, FAR, chase_points, damp, damp_angle, flyby_pose, heading, hood_pose, look,
    orbit_pose, widen_target,
};
use crate::shake;

/// Finish orbit angular speed, rad/s.
const ORBIT_RATE: f32 = 0.35;
/// Length of the fly-by in seconds (GDD §3.2 "≈ 3 s").
pub(crate) const INTRO_SECS: f32 = 3.0;
/// Positional lag of the followed point: x/z (keeps the car planted) and y (soaks up bumps), s.
const FOCUS_SMOOTH_XZ_S: f32 = 0.03;
const FOCUS_SMOOTH_Y_S: f32 = 0.15;
/// Obstruction: gap kept to the hit surface, shortest boom (u), boom regrow smooth time (s).
const PULL_MARGIN_U: f32 = 0.3;
const MIN_BOOM_U: f32 = 1.2;
const BOOM_RELEASE_S: f32 = 0.35;
/// Smooth time of the FOV widening, s.
const WIDEN_SMOOTH_S: f32 = 0.35;

/// What a game camera shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum View {
    /// Its own racer in the selected mode.
    #[default]
    Follow,
    /// Orbiting its own (finished) racer.
    Orbit,
    /// Chasing another racer after finishing.
    Spectate(Entity),
}

/// Follow state of one game camera.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct Rig {
    /// Selected mode (cycled by the Camera action).
    pub mode: CameraMode,
    /// The slot's own racer.
    pub target: Option<Entity>,
    /// What is shown.
    pub view: View,
    /// Jump to the next pose instead of easing (rebind, teleport, look-back toggle).
    pub snap: bool,
    /// Shake trauma 0–1 ([`shake`]).
    pub trauma: f32,
    yaw: f32,
    yaw_vel: f32,
    focus: Vec3,
    focus_vel: Vec3,
    boom: f32,
    boom_vel: f32,
    widen: f32,
    widen_vel: f32,
    look_back: bool,
    orbit: f32,
    /// Shake noise clock, s.
    pub clock: f32,
}

impl Rig {
    /// A rig in `mode` that snaps to its first target.
    pub fn new(mode: CameraMode) -> Self {
        Self {
            mode,
            target: None,
            view: View::Follow,
            snap: true,
            trauma: 0.0,
            yaw: 0.0,
            yaw_vel: 0.0,
            focus: Vec3::ZERO,
            focus_vel: Vec3::ZERO,
            boom: 1.0,
            boom_vel: 0.0,
            widen: 0.0,
            widen_vel: 0.0,
            look_back: false,
            orbit: 0.0,
            clock: 0.0,
        }
    }

    /// The racer on screen: the spectated one, else the slot's own.
    pub fn followed(&self) -> Option<Entity> {
        match self.view {
            View::Spectate(other) => Some(other),
            View::Follow | View::Orbit => self.target,
        }
    }
}

/// One target racer as the camera sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Subject {
    pub pose: Transform,
    pub speed: f32,
    pub boosting: bool,
    pub look_back: bool,
}

/// Camera inputs that do not depend on the camera.
pub(crate) struct Frame {
    pub dt: f32,
    pub phase: Option<RacePhase>,
    pub intro_t: f32,
}

/// Advances `rig` by one frame and returns the camera pose (before obstruction and shake).
pub(crate) fn step(
    rig: &mut Rig,
    subject: Subject,
    frame: &Frame,
) -> (Transform, Option<(Vec3, f32)>) {
    let dt = frame.dt;
    let car = subject.pose;
    let car_yaw = heading(car.rotation).unwrap_or(rig.yaw);
    let look_back = subject.look_back && rig.view == View::Follow;
    if look_back != rig.look_back {
        rig.look_back = look_back;
        rig.snap = true;
    }
    if rig.snap {
        rig.yaw = car_yaw;
        rig.focus = car.translation;
        rig.yaw_vel = 0.0;
        rig.focus_vel = Vec3::ZERO;
        rig.boom = 1.0;
        rig.boom_vel = 0.0;
    }
    let yaw_smooth_s = boom_of(rig).yaw_smooth_s;
    rig.yaw = damp_angle(rig.yaw, car_yaw, &mut rig.yaw_vel, yaw_smooth_s, dt);
    let mut v = rig.focus_vel;
    rig.focus = Vec3::new(
        damp(
            rig.focus.x,
            car.translation.x,
            &mut v.x,
            FOCUS_SMOOTH_XZ_S,
            dt,
        ),
        damp(
            rig.focus.y,
            car.translation.y,
            &mut v.y,
            FOCUS_SMOOTH_Y_S,
            dt,
        ),
        damp(
            rig.focus.z,
            car.translation.z,
            &mut v.z,
            FOCUS_SMOOTH_XZ_S,
            dt,
        ),
    );
    rig.focus_vel = v;
    rig.snap = false;

    if frame.phase == Some(RacePhase::Intro) && rig.view == View::Follow {
        return (flyby_pose(car.translation, car_yaw, frame.intro_t), None);
    }
    match (rig.view, rig.mode) {
        (View::Orbit, _) => {
            rig.orbit += ORBIT_RATE * dt;
            (orbit_pose(car.translation, car_yaw + rig.orbit), None)
        }
        (View::Follow, CameraMode::Hood) => (hood_pose(car, look_back), None),
        _ => {
            let yaw = rig.yaw + if look_back { PI } else { 0.0 };
            let (pivot, eye, look_at) = chase_points(rig.focus, yaw, subject.speed, boom_of(rig));
            let pose = look(eye, look_at);
            (pose, Some((pivot, eye.distance(pivot))))
        }
    }
}

fn boom_of(rig: &Rig) -> Boom {
    match (rig.view, rig.mode) {
        (View::Follow, CameraMode::FarChase) => FAR,
        _ => CHASE,
    }
}

/// Boom length after obstruction: pulls in at once to `hit` (distance along the boom of length
/// `full`), regrows smoothly once clear. The rig keeps the fraction, so mode changes and snaps
/// start from a full boom.
pub(crate) fn pull_in(rig: &mut Rig, full: f32, hit: Option<f32>, dt: f32) -> f32 {
    let full = full.max(f32::EPSILON);
    let allowed = hit.map_or(1.0, |d| {
        ((d - PULL_MARGIN_U).max(MIN_BOOM_U) / full).min(1.0)
    });
    rig.boom = if allowed < rig.boom {
        rig.boom_vel = 0.0;
        allowed
    } else {
        damp(rig.boom, allowed, &mut rig.boom_vel, BOOM_RELEASE_S, dt)
    };
    rig.boom * full
}

/// What a camera reads from the racer it follows.
type SubjectData = (
    &'static Transform,
    Option<&'static PoseHistory>,
    Option<&'static Visual>,
    Option<&'static VehicleTelemetry>,
    Option<&'static CarControl>,
);

/// Racers as targets: proxy pose when rc_render made one, else the interpolated sim pose.
#[derive(SystemParam)]
pub(crate) struct Targets<'w, 's> {
    racers: Query<'w, 's, SubjectData, (With<Racer>, Without<GameCamera>)>,
    proxies: Query<'w, 's, &'static Transform, (Without<Racer>, Without<GameCamera>)>,
    fixed: Res<'w, Time<Fixed>>,
}

impl Targets<'_, '_> {
    fn subject(&self, entity: Entity) -> Option<Subject> {
        let (transform, history, visual, telemetry, control) = self.racers.get(entity).ok()?;
        let alpha = self.fixed.overstep_fraction();
        let pose = visual
            .and_then(|v| self.proxies.get(v.get()).ok().copied())
            .unwrap_or_else(|| history.map_or(*transform, |h| h.interpolate(alpha)));
        Some(Subject {
            pose,
            speed: telemetry.map_or(0.0, |t| t.speed),
            boosting: telemetry.is_some_and(|t| t.boosting),
            look_back: control.is_some_and(|c| c.look_back),
        })
    }
}

/// Frame-wide inputs of the follow system.
#[derive(SystemParam)]
pub(crate) struct Context<'w, 's> {
    time: Res<'w, Time>,
    pause: Option<Res<'w, State<PauseState>>>,
    phase: Option<Res<'w, State<RacePhase>>>,
    settings: Option<Res<'w, Settings>>,
    intro: Res<'w, crate::director::IntroClock>,
    window: Option<Single<'w, 's, &'static Window, With<PrimaryWindow>>>,
}

/// The game cameras' follow state, pose and lens.
type GameCameras<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Rig,
        &'static mut Transform,
        &'static mut Projection,
        &'static Camera,
    ),
    With<GameCamera>,
>;

/// `CameraSet`: poses every game camera and sets its lens.
pub(crate) fn follow_racers(
    ctx: Context,
    targets: Targets,
    rapier: ReadRapierContext,
    mut cameras: GameCameras,
) {
    let paused = ctx.pause.is_some_and(|p| *p.get() == PauseState::Paused);
    let frame = Frame {
        dt: if paused { 0.0 } else { ctx.time.delta_secs() },
        phase: ctx.phase.map(|p| *p.get()),
        intro_t: ctx.intro.0 / INTRO_SECS,
    };
    let (fov_setting, shake_on, split) = ctx.settings.map_or((90.0, true, default()), |s| {
        (
            s.graphics.fov_deg,
            s.gameplay.camera_shake,
            s.gameplay.split,
        )
    });
    let top_bottom_2p = cameras.iter().count() == 2 && split == SplitOrientation::Horizontal;
    let physics = rapier.single().ok();
    let window_size = ctx.window.map(|w| w.physical_size().as_vec2());
    for (mut rig, mut transform, mut projection, camera) in &mut cameras {
        let Some(subject) = rig.followed().and_then(|e| targets.subject(e)) else {
            continue; // no racer yet: hold the pose
        };
        let (mut pose, boom) = step(&mut rig, subject, &frame);
        if let Some((pivot, full)) = boom {
            let dir = (pose.translation - pivot).normalize_or_zero();
            let hit = physics.as_ref().and_then(|c| {
                c.cast_ray(
                    pivot,
                    dir,
                    full,
                    true,
                    QueryFilter::only_fixed().exclude_sensors(),
                )
                .map(|(_, toi)| toi)
            });
            pose.translation = pivot + dir * pull_in(&mut rig, full, hit, frame.dt);
        }
        pose.rotation *= shake::offset(&mut rig, frame.dt, shake_on);
        *transform = pose;

        let target = widen_target(subject.speed, subject.boosting);
        rig.widen = damp(
            rig.widen,
            target,
            &mut rig.widen_vel,
            WIDEN_SMOOTH_S,
            frame.dt,
        );
        let size = camera
            .viewport
            .as_ref()
            .map(|v| v.physical_size.as_vec2())
            .or(window_size)
            .unwrap_or(Vec2::new(16.0, 9.0));
        let aspect = if size.y > 0.0 {
            size.x / size.y
        } else {
            REFERENCE_ASPECT
        };
        let clamp = top_bottom_2p.then_some(MAX_2P_HFOV_DEG);
        let fov = hor_plus_vfov(fov_setting + rig.widen, aspect, clamp);
        if let Projection::Perspective(p) = projection.as_mut()
            && (p.fov - fov).abs() > 1e-4
        {
            p.fov = fov;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pose::HOOD_OFFSET;

    fn subject_at(pose: Transform) -> Subject {
        Subject {
            pose,
            speed: 0.0,
            boosting: false,
            look_back: false,
        }
    }

    fn frame(dt: f32) -> Frame {
        Frame {
            dt,
            phase: Some(RacePhase::Racing),
            intro_t: 1.0,
        }
    }

    #[test]
    fn follow_lags_on_turns_then_settles_behind_the_car() {
        let mut rig = Rig::new(CameraMode::Chase);
        let car = Transform::default();
        step(&mut rig, subject_at(car), &frame(1.0 / 60.0));
        let turned = car.with_rotation(Quat::from_rotation_y(1.0));
        let (first, _) = step(&mut rig, subject_at(turned), &frame(1.0 / 60.0));
        assert!(
            heading(first.rotation).expect("level") < 0.5,
            "lags the turn"
        );
        let mut pose = first;
        for _ in 0..120 {
            pose = step(&mut rig, subject_at(turned), &frame(1.0 / 60.0)).0;
        }
        let behind = turned.translation + turned.rotation * Vec3::Z;
        assert!(pose.translation.distance(behind) < CHASE.distance + 1.0);
        assert!((heading(pose.rotation).expect("level") - 1.0).abs() < 0.05);
    }

    #[test]
    fn look_back_flips_to_the_front_at_once() {
        let mut rig = Rig::new(CameraMode::Chase);
        let car = Transform::default();
        step(&mut rig, subject_at(car), &frame(1.0 / 60.0));
        let back = Subject {
            look_back: true,
            ..subject_at(car)
        };
        let (pose, _) = step(&mut rig, back, &frame(1.0 / 60.0));
        assert!(
            pose.translation.z < -4.0,
            "camera in front of the car: {}",
            pose.translation
        );
        assert!(
            (pose.forward().z - 1.0).abs() < 0.3,
            "looking backward (+Z)"
        );
        let (hood, _) = step(&mut Rig::new(CameraMode::Hood), back, &frame(1.0 / 60.0));
        assert!(hood.forward().z > 0.9);
    }

    #[test]
    fn far_chase_and_hood_frame_differently() {
        let car = subject_at(Transform::default());
        let mut far = Rig::new(CameraMode::FarChase);
        let (far_pose, boom) = step(&mut far, car, &frame(1.0 / 60.0));
        assert!(far_pose.translation.z > CHASE.distance + 2.0);
        assert!(boom.is_some(), "chase modes are obstruction-checked");
        let (hood, boom) = step(&mut Rig::new(CameraMode::Hood), car, &frame(1.0 / 60.0));
        assert_eq!(hood.translation, HOOD_OFFSET);
        assert!(boom.is_none());
    }

    #[test]
    fn obstruction_pulls_in_at_once_and_regrows_smoothly() {
        let mut rig = Rig::new(CameraMode::Chase);
        let full = 5.0;
        assert_eq!(pull_in(&mut rig, full, None, 0.016), full);
        let pulled = pull_in(&mut rig, full, Some(2.0), 0.016);
        assert!((pulled - (2.0 - PULL_MARGIN_U)).abs() < 1e-6, "{pulled}");
        assert_eq!(
            pull_in(&mut rig, full, Some(0.1), 0.016),
            MIN_BOOM_U,
            "never inside the car"
        );
        let regrow = pull_in(&mut rig, full, None, 0.016);
        assert!(regrow > MIN_BOOM_U && regrow < full, "eases out: {regrow}");
        for _ in 0..120 {
            pull_in(&mut rig, full, None, 0.016);
        }
        assert!((pull_in(&mut rig, full, None, 0.016) - full).abs() < 0.01);
    }
}
