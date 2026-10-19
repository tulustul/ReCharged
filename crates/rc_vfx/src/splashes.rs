//! Raindrops splashing on wet ground ahead of every local player's car (outdoor rain only). Each
//! splash sits on the ground under it (B30): near the car on the plane its wheels touch (that
//! also covers ramps and other prop surfaces), farther out on the track's surface — the banked
//! ribbon or a ground polygon — closest to that plane. Where neither lies near the plane (a jump
//! gap, the edge of a ramp, under a bridge) no splash appears instead of a floating one.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::cars::{Poses, ground_plane};
use crate::fx::Fx;
use crate::particles::{Facing, Look, Particle, Shape};
use crate::pool::Emitter;

/// Splashes per second around a local car at full rain.
const SPLASH_RATE: f32 = 90.0;
/// No splashes while the car's ground is steeper than this (normal's up component).
const MIN_SPLASH_NORMAL_Y: f32 = 0.5;
/// Splash distance from the car, u (min, max).
const SPLASH_RANGE_U: (f32, f32) = (1.5, 13.0);
/// Within this distance of the car the wheel plane is the ground (ramps, curbs), u.
const PLANE_TRUSTED_U: f32 = 3.0;
/// Farther out, the track surface must lie within this of the extrapolated wheel plane, u.
const MAX_GROUND_GAP_U: f32 = 0.6;

const SPLASH_RING: Look = Look {
    shape: Shape::Ring,
    facing: Facing::Flat,
    color: [
        LinearRgba::new(0.85, 0.9, 0.95, 0.55),
        LinearRgba::new(0.85, 0.9, 0.95, 0.0),
    ],
    size: [0.04, 0.42],
    fade_out: 0.7,
    ..Look::DEFAULT
};

const SPLASH_DROP: Look = Look {
    shape: Shape::Drop,
    facing: Facing::Velocity,
    color: [
        LinearRgba::new(0.85, 0.9, 0.95, 0.7),
        LinearRgba::new(0.85, 0.9, 0.95, 0.0),
    ],
    size: [0.03, 0.02],
    gravity: 22.0,
    stretch: 0.02,
    aspect: 1.5,
    fade_out: 0.4,
    ..Look::DEFAULT
};

/// Splash emitters of the local cars (and each car's last track position, a projection hint).
#[derive(Resource, Debug, Default)]
struct Splashes(Vec<(Entity, Emitter, Option<f32>)>);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Splashes>()
        .add_systems(Update, splashes.in_set(crate::VfxSystems::Emit));
}

/// Height of the plane through `point` with unit normal `normal` (not vertical) at (`x`, `z`).
fn plane_height((point, normal): (Vec3, Vec3), x: f32, z: f32) -> f32 {
    point.y - (normal.x * (x - point.x) + normal.z * (z - point.z)) / normal.y
}

/// The track's surface height under (`x`, `z`) closest to `guess`: the ribbon (banked, outside
/// jump gaps) or a ground polygon; `None` when none lies within [`MAX_GROUND_GAP_U`] of `guess`.
/// `hint` = the car's arc length (keeps the projection on the right level of a helix).
fn ground_height(track: &Track, x: f32, z: f32, guess: f32, hint: Option<f32>) -> Option<f32> {
    let pos = track.project(Vec3::new(x, guess, z), hint);
    let frame = track.frame_at(pos.s);
    let ribbon = (!frame.in_gap && pos.lateral.abs() <= frame.half_width)
        .then(|| (frame.position + frame.right * pos.lateral).y);
    let xz = Vec2::new(x, z);
    let polygons = track
        .ground()
        .iter()
        .filter(|g| g.contains_xz(xz))
        .map(|g| g.height);
    ribbon
        .into_iter()
        .chain(polygons)
        .map(|h| (h, (h - guess).abs()))
        .filter(|&(_, gap)| gap <= MAX_GROUND_GAP_U)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// Splashes on wet ground in front of every local player's car, none while it is airborne.
fn splashes(
    time: Res<Time>,
    weather: Option<Res<WeatherState>>,
    track: Option<Res<Track>>,
    mut fx: ResMut<Fx>,
    mut state: ResMut<Splashes>,
    poses: Poses,
    locals: Query<(Entity, &VehicleTelemetry), With<LocalPlayer>>,
) {
    let (Some(w), Some(track)) = (weather.as_deref(), track.as_deref()) else {
        return;
    };
    let raining = !w.outside_only && !w.snowing && w.precipitation > 0.05 && w.wetness > 0.2;
    if !raining {
        return;
    }
    let dt = time.delta_secs();
    let rate = SPLASH_RATE * w.precipitation * fx.density;
    state.0.retain(|(e, ..)| locals.contains(*e));
    for (car, telemetry) in &locals {
        if !state.0.iter().any(|(e, ..)| *e == car) {
            state.0.push((car, Emitter::default(), None));
        }
        let (Some(pose), Some(plane)) = (
            poses.of(car),
            ground_plane(telemetry).filter(|(_, n)| n.y > MIN_SPLASH_NORMAL_Y),
        ) else {
            continue;
        };
        let Some((_, emitter, hint)) = state.0.iter_mut().find(|(e, ..)| *e == car) else {
            continue;
        };
        *hint = Some(track.project(pose.translation, *hint).s);
        let n = emitter.count(rate, dt);
        let fwd = pose.rotation * Vec3::NEG_Z;
        let yaw = fwd.z.atan2(fwd.x);
        let flat = Quat::from_rotation_arc(Vec3::Z, plane.1);
        for _ in 0..n {
            let a = yaw + fx.rng.signed(1.6);
            let r = fx.rng.range(SPLASH_RANGE_U.0, SPLASH_RANGE_U.1);
            let (x, z) = (
                pose.translation.x + a.cos() * r,
                pose.translation.z + a.sin() * r,
            );
            let on_plane = plane_height(plane, x, z);
            let y = if r <= PLANE_TRUSTED_U {
                Some(on_plane)
            } else {
                ground_height(track, x, z, on_plane, *hint)
            };
            let Some(y) = y else {
                continue;
            };
            let at = Vec3::new(x, y, z) + plane.1 * 0.02;
            let life = fx.rng.range(0.22, 0.32);
            fx.spawn(Particle::new(SPLASH_RING, at, Vec3::ZERO, life).rotated(flat));
            for _ in 0..2 {
                let v = Vec3::new(
                    fx.rng.signed(1.0),
                    fx.rng.range(1.5, 3.0),
                    fx.rng.signed(1.0),
                );
                fx.spawn(Particle::new(SPLASH_DROP, at, v, 0.28).floored(at.y));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn track(id: &str) -> Track {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Track::load(&assets, &TrackVariant::forward(id)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn splashes_lie_on_the_ground_plane() {
        let n = Vec3::new(0.0, 0.8, 0.6);
        let plane = (Vec3::new(1.0, 2.0, 3.0), n);
        assert!((plane_height(plane, 1.0, 3.0) - 2.0).abs() < 1e-6);
        let p = Vec3::new(5.0, plane_height(plane, 5.0, -4.0), -4.0);
        assert!((p - plane.0).dot(n).abs() < 1e-5, "{p} is on the plane");
    }

    /// Toy Town's raised, banked turns and climbs: a guess off by 0.4 u (a plane extrapolated
    /// past a slope change) still finds the surface under the point; a guess far off finds
    /// nothing (no floating splash).
    #[test]
    fn far_splashes_snap_to_the_track_surface_under_them() {
        let track = track("toy_town");
        let mut checked = 0;
        for i in 0..60 {
            let s = track.length() * i as f32 / 60.0;
            let frame = track.frame_at(s);
            if frame.in_gap {
                continue;
            }
            let lateral = frame.half_width * 0.6;
            let surface = frame.position + frame.right * lateral;
            let found = ground_height(&track, surface.x, surface.z, surface.y + 0.4, Some(s));
            let h = found.unwrap_or_else(|| panic!("no ground at s {s}"));
            assert!((h - surface.y).abs() < 0.08, "s {s}: {h} vs {}", surface.y);
            assert!(
                ground_height(&track, surface.x, surface.z, surface.y + 3.0, Some(s)).is_none(),
                "s {s}: 3 u above the surface is not the ground"
            );
            checked += 1;
        }
        assert!(checked > 40, "{checked} samples");
    }
}
