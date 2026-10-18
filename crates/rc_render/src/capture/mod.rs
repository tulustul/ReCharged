//! Capture tools (ARCHITECTURE §8) on the race started by `--quick-race <track>`:
//! - `--capture-previews`: the track's preview card (`TrackInfo.preview`, e.g.
//!   `assets/ui/tracks/toy_town.png`, 1280×720) from an elevated three-quarter view, forward
//!   layout only;
//! - `--track-tour <track>`: a screenshot every 25 u along the racing line into
//!   `<user data>/captures/tour_<track>_<variant>/NNN.png`.
//!
//! A capture camera (a `QualityCamera`, so it renders exactly like a game view) renders into a
//! 1280×720 image (no HUD, independent of the window size); the app quits once the last image is
//! written.

use std::path::PathBuf;

use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use rc_core::prelude::*;
use rc_track::{Track, variant_name};

use crate::lighting::ActiveEnvironment;
use crate::quality::QualityCamera;

/// Wall-clock seconds the capture camera renders its first pose before the first shot
/// (pipelines compile, assets, exposure and TAA settle).
const SETTLE_SECS: f32 = 4.0;
/// Seconds the camera holds each pose before its shot (TAA converges, shadows update).
const DWELL_SECS: f32 = 0.6;
/// Longest wait for the last images to reach the disk before quitting anyway (s).
const FLUSH_TIMEOUT_SECS: f32 = 15.0;
/// Distance between tour shots along the lap (u).
const TOUR_STEP_U: f32 = 25.0;
/// Tour eye height above the racing line, and how far ahead it looks (u).
const TOUR_EYE_U: f32 = 1.6;
const TOUR_AHEAD_U: f32 = 8.0;
/// Tour poses keep this far (arc length, u) from item rows: closer, the eye sits inside the
/// Volt Cubes' translucent volumes and the shot is washed out.
const TOUR_ITEM_CLEARANCE_U: f32 = 3.0;
/// Vertical field of view of tour shots: the game's default 90° horizontal at 16:9.
const TOUR_FOV_DEG: f32 = 58.7;
/// Size of every capture (px).
const CAPTURE_SIZE: UVec2 = UVec2::new(1280, 720);
/// Vertical field of view of previews.
const PREVIEW_FOV_DEG: f32 = 50.0;
/// Preview camera pitch (degrees down) when the room allows it.
const PREVIEW_PITCH_DEG: f32 = 32.0;
/// The track outline fills the preview up to this fraction of the frame (NDC).
const PREVIEW_FILL: f32 = 0.8;
/// The preview looks across the track's long axis, turned along it by this much (tan).
const PREVIEW_TURN: f32 = 0.6;
/// Indoors the preview camera stays this far below the ceiling and inside the walls (u).
const ROOM_CLEARANCE_U: f32 = 1.5;
/// Indoors the preview aims this far below the track centre per unit of horizontal distance,
/// so the low camera shows the floor rather than the ceiling.
const INDOOR_AIM_DROP: f32 = 0.25;
/// An indoor preview that overfills the frame by more than this switches to a cutaway.
const CUTAWAY_SLACK: f32 = 1.1;
/// Highest cutaway eye, relative to the ceiling height.
const CUTAWAY_HEIGHT: f32 = 2.2;

/// The camera of the capture tools.
#[derive(Component, Debug, Clone, Copy)]
struct CaptureCamera;

/// Room ceiling and lamp fixtures: hidden for cutaway previews of low rooms (they cast no
/// shadows, so the lighting stays the same).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Overhead;

/// One image to take.
#[derive(Debug, Clone)]
struct Shot {
    pose: Transform,
    fov_deg: f32,
    path: PathBuf,
}

/// The capture plan of the session and its progress.
#[derive(Resource, Debug)]
struct CaptureRun {
    shots: Vec<Shot>,
    image: Handle<Image>,
    next: usize,
    posed: bool,
    wait: f32,
    /// Images written so far (screenshots arrive a few frames after they are requested).
    saved: usize,
    /// Frame times (s) after the settle time, for the closing performance line.
    frames: Vec<f32>,
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::InRace), plan_captures)
        .add_systems(OnEnter(AppState::Unloading), |mut commands: Commands| {
            commands.remove_resource::<CaptureRun>();
        })
        .add_systems(
            Update,
            run_captures
                .in_set(VisualSet)
                .run_if(resource_exists::<CaptureRun>.and_then(in_state(AppState::InRace))),
        );
}

/// Largest |NDC| coordinate of `points` seen from `pose` through the preview lens.
fn frame_extent(pose: &Transform, points: &[Vec3]) -> f32 {
    let view = pose.compute_affine().inverse();
    let ty = (PREVIEW_FOV_DEG.to_radians() * 0.5).tan();
    let tx = ty * CAPTURE_SIZE.x as f32 / CAPTURE_SIZE.y as f32;
    points
        .iter()
        .map(|&p| {
            let v = view.transform_point3(p);
            let depth = (-v.z).max(1e-3);
            (v.x / (depth * tx)).abs().max((v.y / (depth * ty)).abs())
        })
        .fold(0.0, f32::max)
}

/// Where the preview camera stands: across the track's long axis on the start line's side,
/// pitched down at the centre, as close as framing the whole outline allows; indoors kept inside
/// the walls and under the ceiling (above it with `cutaway`: the ceiling is hidden).
pub(crate) fn preview_pose(
    outline: &[Vec2],
    start: Vec3,
    bounds: (Vec3, Vec3),
    ceiling: Option<f32>,
    cutaway: bool,
) -> Transform {
    let lo = outline.iter().copied().fold(Vec2::MAX, Vec2::min);
    let hi = outline.iter().copied().fold(Vec2::MIN, Vec2::max);
    let centre = (lo + hi) * 0.5;
    let size = hi - lo;
    let (across, along) = if size.x >= size.y {
        (Vec2::Y, Vec2::X)
    } else {
        (Vec2::X, Vec2::Y)
    };
    let side = start.xz() - centre;
    let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
    let dir = (across * sign(side.dot(across)) + along * sign(side.dot(along)) * PREVIEW_TURN)
        .normalize();
    let target = Vec3::new(centre.x, 0.0, centre.y);
    let pitch = PREVIEW_PITCH_DEG.to_radians();
    let pose_at = |d: f32| {
        let flat = dir * d * pitch.cos();
        let mut eye = target + Vec3::new(flat.x, d * pitch.sin(), flat.y);
        let mut aim = target;
        if let Some(ceiling) = ceiling {
            let (min, max) = bounds;
            let c = ROOM_CLEARANCE_U;
            eye.x = eye.x.clamp(min.x + c, max.x - c);
            eye.z = eye.z.clamp(min.z + c, max.z - c);
            // A cutaway looks in from just above the hidden ceiling, close enough that the
            // walls frame the view (higher, the room floats in the void).
            let top = if cutaway {
                ceiling * CUTAWAY_HEIGHT
            } else {
                ceiling - c
            };
            eye.y = eye.y.min(top);
            aim.y -= INDOOR_AIM_DROP * eye.xz().distance(target.xz());
        }
        Transform::from_translation(eye).looking_at(aim, Vec3::Y)
    };
    let points: Vec<Vec3> = outline.iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect();
    // The view shrinks with distance: bisect for the nearest distance that frames it all.
    let (mut near, mut far) = (1.0_f32, 5000.0_f32);
    for _ in 0..40 {
        let mid = (near + far) * 0.5;
        if frame_extent(&pose_at(mid), &points) > PREVIEW_FILL {
            near = mid;
        } else {
            far = mid;
        }
    }
    pose_at(far)
}

/// Arc length of the tour shot planned at `s`: moved forward past jump gaps and item rows.
fn tour_s(track: &Track, s: f32) -> f32 {
    let len = track.length();
    let near_items = |s: f32| {
        track.item_rows().iter().any(|row| {
            let d = (s - row.s).rem_euclid(len);
            d.min(len - d) < TOUR_ITEM_CLEARANCE_U
        })
    };
    let mut s = s;
    while (track.frame_at(s).in_gap || near_items(s)) && s < len * 2.0 {
        s += 1.0;
    }
    s
}

/// Tour camera at arc length `s` (see [`tour_s`]): above the racing line, looking along it.
fn tour_pose(track: &Track, s: f32) -> Transform {
    let s = tour_s(track, s);
    let eye = track.racing_line_at(s) + track.frame_at(s).up * TOUR_EYE_U;
    let ahead = track.racing_line_at(s + TOUR_AHEAD_U) + Vec3::Y * (TOUR_EYE_U * 0.4);
    Transform::from_translation(eye).looking_at(ahead, Vec3::Y)
}

/// `OnEnter(InRace)`: plan the requested captures and spawn the capture camera.
#[allow(clippy::too_many_arguments)]
fn plan_captures(
    mut commands: Commands,
    automation: Option<Res<AutomationConfig>>,
    track: Option<Res<Track>>,
    catalog: Option<Res<Catalog>>,
    env: Option<Res<ActiveEnvironment>>,
    paths: Option<Res<UserPaths>>,
    mut images: ResMut<Assets<Image>>,
    existing: Option<Res<CaptureRun>>,
    overhead: Query<Entity, With<Overhead>>,
) {
    let (Some(automation), Some(track), Some(paths)) = (automation, track, paths) else {
        return;
    };
    if existing.is_some() || !(automation.capture_previews || automation.track_tour.is_some()) {
        return;
    }
    let variant = track.variant().clone();
    let mut shots = Vec::new();
    if automation.capture_previews {
        let preview = catalog
            .as_ref()
            .and_then(|c| c.track(&variant.track))
            .map(|t| t.preview.clone());
        match preview {
            Some(file) if !variant.reverse && !variant.mirror => {
                let ceiling = env
                    .as_ref()
                    .and_then(|e| e.def.room.as_ref())
                    .map(|r| r.height);
                let outline = track.outline_2d();
                let start = track.frame_at(0.0).position;
                let mut pose = preview_pose(&outline, start, track.bounds(), ceiling, false);
                // A low room cannot frame its track from under the ceiling: look in from above.
                let points: Vec<Vec3> = outline.iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect();
                if ceiling.is_some() && frame_extent(&pose, &points) > PREVIEW_FILL * CUTAWAY_SLACK
                {
                    pose = preview_pose(&outline, start, track.bounds(), ceiling, true);
                    for entity in &overhead {
                        commands.entity(entity).insert(Visibility::Hidden);
                    }
                    info!("capture: cutaway preview (ceiling hidden)");
                }
                info!(
                    "capture: preview from {:.1} looking {:.2}",
                    pose.translation,
                    pose.forward().as_vec3()
                );
                shots.push(Shot {
                    pose,
                    fov_deg: PREVIEW_FOV_DEG,
                    path: paths.assets_dir.join(file),
                });
            }
            Some(_) => warn!(
                "--capture-previews: previews show the forward layout; skipped for {}",
                variant_name(&variant)
            ),
            None => warn!(
                "--capture-previews: track {} is not in the catalog",
                variant.track
            ),
        }
    }
    if let Some(tour) = &automation.track_tour {
        if *tour != variant.track {
            warn!(
                "--track-tour {tour}: the race runs on {}; touring that",
                variant.track
            );
        }
        let name = format!(
            "tour_{}_{}",
            variant.track,
            variant_name(&variant).replace(' ', "_")
        );
        let dir = paths.data_dir.join("captures").join(name);
        let count = (track.length() / TOUR_STEP_U).floor().max(1.0) as usize;
        shots.extend((0..count).map(|i| Shot {
            pose: tour_pose(&track, i as f32 * TOUR_STEP_U),
            fov_deg: TOUR_FOV_DEG,
            path: dir.join(format!("{i:03}.png")),
        }));
    }
    if shots.is_empty() {
        return;
    }
    info!("capture: {} shots planned", shots.len());
    let image = images.add(Image::new_target_texture(
        CAPTURE_SIZE.x,
        CAPTURE_SIZE.y,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    commands.spawn((
        Name::new("capture camera"),
        CaptureCamera,
        Camera3d::default(),
        Camera::default(),
        shots[0].pose,
        Projection::from(PerspectiveProjection {
            fov: shots[0].fov_deg.to_radians(),
            ..default()
        }),
        RenderTarget::Image(ImageRenderTarget::from(image.clone())),
        QualityCamera,
        DespawnOnExit(RaceSession),
    ));
    commands.insert_resource(CaptureRun {
        shots,
        image,
        next: 0,
        posed: true,
        wait: SETTLE_SECS,
        saved: 0,
        frames: Vec::new(),
    });
}

/// Poses the capture camera, waits, shoots; quits after the last image.
fn run_captures(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut run: ResMut<CaptureRun>,
    mut camera: Query<(&mut Transform, &mut Projection), With<CaptureCamera>>,
    mut exit: MessageWriter<AppExit>,
) {
    run.wait -= time.delta_secs();
    if run.next > 0 || run.wait <= 0.0 {
        run.frames.push(time.delta_secs());
    }
    if run.wait > 0.0 {
        return;
    }
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Some(shot) = run.shots.get(run.next).cloned() else {
        // Every shot is requested: quit once all of them are on disk.
        if run.saved < run.shots.len() && run.wait > -FLUSH_TIMEOUT_SECS {
            return;
        }
        if run.saved < run.shots.len() {
            error!(
                "capture: only {} of {} images were written",
                run.saved,
                run.shots.len()
            );
        }
        let (p50, p95, max) = frame_stats(&mut run.frames);
        info!(
            "capture: done; frame time p50 {:.1} ms, p95 {:.1} ms, max {:.1} ms over {} frames",
            p50 * 1000.0,
            p95 * 1000.0,
            max * 1000.0,
            run.frames.len()
        );
        exit.write(AppExit::Success);
        commands.remove_resource::<CaptureRun>();
        return;
    };
    if !run.posed {
        *transform = shot.pose;
        *projection = Projection::from(PerspectiveProjection {
            fov: shot.fov_deg.to_radians(),
            ..default()
        });
        run.posed = true;
        run.wait = DWELL_SECS;
        return;
    }
    if let Some(dir) = shot.path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        error!("capture: {}: {e}", dir.display());
    }
    info!("capture -> {}", shot.path.display());
    commands
        .spawn(Screenshot::image(run.image.clone()))
        .observe(save_to_disk(shot.path))
        .observe(|_: On<ScreenshotCaptured>, mut run: ResMut<CaptureRun>| run.saved += 1);
    run.next += 1;
    run.posed = false;
    run.wait = 0.0;
}

/// (median, 95th percentile, maximum) of frame times; zeros when empty.
fn frame_stats(frames: &mut [f32]) -> (f32, f32, f32) {
    if frames.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    frames.sort_by(f32::total_cmp);
    let at = |q: f32| frames[((frames.len() - 1) as f32 * q).round() as usize];
    (at(0.5), at(0.95), at(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tour shots never stand in a jump gap or inside an item row's cubes.
    #[test]
    fn tour_shots_keep_clear_of_gaps_and_item_rows() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let variant = rc_core::TrackVariant::forward("toy_town");
        let track = Track::load(&assets, &variant).unwrap_or_else(|e| panic!("{e}"));
        assert!(!track.item_rows().is_empty(), "toy_town has item rows");
        let len = track.length();
        for i in 0..(len / TOUR_STEP_U) as usize {
            let s = tour_s(&track, i as f32 * TOUR_STEP_U);
            assert!(!track.frame_at(s).in_gap, "shot {i} in a gap");
            for row in track.item_rows() {
                let d = (s - row.s).rem_euclid(len);
                assert!(
                    d.min(len - d) >= TOUR_ITEM_CLEARANCE_U,
                    "shot {i} at an item row"
                );
            }
        }
    }

    #[test]
    fn frame_stats_pick_percentiles() {
        let mut frames: Vec<f32> = (1..=100).map(|i| i as f32 / 1000.0).collect();
        let (p50, p95, max) = frame_stats(&mut frames);
        assert!((p50 - 0.050).abs() < 0.0015 && (p95 - 0.095).abs() < 0.0015 && max == 0.1);
        assert_eq!(frame_stats(&mut []), (0.0, 0.0, 0.0));
    }

    fn square(r: f32) -> Vec<Vec2> {
        vec![
            Vec2::new(-r, -r),
            Vec2::new(r, -r),
            Vec2::new(r, r),
            Vec2::new(-r, r),
        ]
    }

    #[test]
    fn outdoor_previews_frame_the_whole_track_tightly() {
        let oval: Vec<Vec2> = (0..64)
            .map(|i| {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                Vec2::new(25.0 * a.cos(), 70.0 * a.sin())
            })
            .collect();
        let pose = preview_pose(
            &oval,
            Vec3::new(-25.0, 0.3, 10.0),
            (Vec3::splat(-500.0), Vec3::splat(500.0)),
            None,
            false,
        );
        let points: Vec<Vec3> = oval.iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect();
        let extent = frame_extent(&pose, &points);
        assert!(
            (PREVIEW_FILL * 0.97..=PREVIEW_FILL + 1e-3).contains(&extent),
            "fills the frame: {extent}"
        );
        assert!(
            pose.translation.x < 0.0,
            "looks across the long axis from the start's side"
        );
    }

    #[test]
    fn outdoor_previews_look_down_at_the_track_from_the_start_line_side() {
        let pose = preview_pose(
            &square(50.0),
            Vec3::new(10.0, 0.3, 50.0),
            (Vec3::splat(-200.0), Vec3::splat(200.0)),
            None,
            false,
        );
        let eye = pose.translation;
        assert!(eye.x > 0.0 && eye.z > 0.0, "on the start's side: {eye}");
        let look = pose.forward();
        assert!(look.y < -0.4, "pitched down: {look:?}");
        let to_centre = (Vec3::ZERO - eye).normalize();
        assert!(look.dot(to_centre) > 0.999, "aimed at the centre");
    }

    #[test]
    fn indoor_previews_stay_inside_the_room() {
        let bounds = (Vec3::new(-60.0, -10.0, -50.0), Vec3::new(60.0, 40.0, 50.0));
        let outline = square(45.0);
        let start = Vec3::new(-10.0, 0.3, -40.0);
        let pose = preview_pose(&outline, start, bounds, Some(16.0), false);
        let eye = pose.translation;
        assert!(
            eye.y <= 16.0 - ROOM_CLEARANCE_U + 1e-4,
            "under the ceiling: {eye}"
        );
        assert!(
            eye.x >= -60.0 + ROOM_CLEARANCE_U - 1e-4 && eye.z >= -50.0 + ROOM_CLEARANCE_U - 1e-4
        );
        assert!(pose.forward().y < 0.0, "still looking down");
        // The cutaway (ceiling hidden) rises above the ceiling, not far, still inside the walls.
        let cut = preview_pose(&outline, start, bounds, Some(16.0), true).translation;
        assert!(
            cut.y > 16.0 && cut.y <= 16.0 * CUTAWAY_HEIGHT + 1e-4,
            "{cut}"
        );
        assert!(
            cut.x >= -60.0 + ROOM_CLEARANCE_U - 1e-4 && cut.z >= -50.0 + ROOM_CLEARANCE_U - 1e-4
        );
    }
}
