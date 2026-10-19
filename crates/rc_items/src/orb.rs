//! Shock Orb (GDD §6.2): a slow plasma ball that follows the track in track coordinates
//! (`Track::frame_at`): it runs along the centreline at the launch heading's share of its speed
//! and bounces off the barriers — the track's wall colliders, found by a ray across the track
//! at the orb's height — or, on a side without one, off the edge of the racing width. Any car
//! but its owner whose centre comes within 3 u is knocked sideways and up (each car once); it is
//! spent after 3 hits or 10 s.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_track::{Track, TrackFrame};

use crate::data::ItemTuning;
use crate::effects::run_down;
use crate::hits::{Blow, CAR_HALF_LENGTH_U, Hits, centre, track_filter, tumble};
use crate::pickups::UseItem;

/// Barriers are looked for this far beyond the racing width (u): the widest edge style (an
/// Asphalt curb, 0.9 u) plus room.
const BARRIER_REACH_U: f32 = 1.5;

/// An orb in track coordinates.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct Orb {
    /// Arc length (u).
    pub s: f32,
    /// Lateral offset (u, + = right).
    pub lateral: f32,
    /// Speed along `s` (u/s; negative when fired against the travel direction).
    pub along: f32,
    /// Lateral speed (u/s).
    pub across: f32,
    /// Cars it already knocked.
    pub hit: Vec<Entity>,
}

/// Reflects a lateral position and speed off the edges at `lo` (left, ≤ 0) and `hi` (right).
pub(crate) fn bounce(lateral: f32, across: f32, (lo, hi): (f32, f32)) -> (f32, f32) {
    if lateral > hi {
        ((2.0 * hi - lateral).max(lo), -across.abs())
    } else if lateral < lo {
        ((2.0 * lo - lateral).min(hi), across.abs())
    } else {
        (lateral, across)
    }
}

/// The lateral range (left ≤ 0, right ≥ 0) an orb's centre `height` above the surface at
/// `frame` bounces within: `margin` inside each barrier a ray across the track finds, else inside
/// the racing width's edge.
pub(crate) fn edges(
    ctx: Option<&RapierContext>,
    frame: &TrackFrame,
    height: f32,
    margin: f32,
) -> (f32, f32) {
    let side = |sign: f32| {
        let from = frame.point(0.0, height);
        let reach = frame.half_width + BARRIER_REACH_U;
        let wall = ctx.and_then(|c| {
            c.cast_ray(from, frame.right * sign, reach, true, track_filter())
                .map(|(_, toi)| toi)
        });
        (wall.unwrap_or(frame.half_width) - margin).max(0.0)
    };
    (-side(-1.0), side(1.0))
}

/// Launches the orbs of this tick's [`UseItem`]s ahead of the car, along the track in the
/// direction the car faces, at its heading (capped) to the track.
#[allow(clippy::too_many_arguments)]
pub(crate) fn launch_orbs(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    track: Option<Res<Track>>,
    rapier: ReadRapierContext,
    mut uses: MessageReader<UseItem>,
    cars: Query<&Transform, With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let o = &tuning.shock_orb;
    let ctx = rapier.single().ok();
    for u in uses.read().filter(|u| u.kind == ItemKind::ShockOrb) {
        let (Some(track), Ok(t)) = (track.as_deref(), cars.get(u.racer)) else {
            continue;
        };
        let at = track.project(t.translation, None);
        let frame = track.frame_at(at.s);
        let fwd = *t.forward();
        let sign = if fwd.dot(frame.forward) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let max = o.max_angle_deg.to_radians();
        let angle = fwd.dot(frame.right).atan2(fwd.dot(frame.forward).abs());
        let angle = angle.clamp(-max, max);
        let s = track.wrap(at.s + sign * (CAR_HALF_LENGTH_U + 0.5));
        let (lo, hi) = edges(ctx.as_ref(), &track.frame_at(s), o.height, o.edge_margin);
        let orb = Orb {
            s,
            lateral: at.lateral.clamp(lo, hi),
            along: sign * o.speed * angle.cos(),
            across: o.speed * angle.sin(),
            hit: Vec::new(),
        };
        let pose = track.frame_at(s);
        let entity = commands
            .spawn((
                Name::new("shock orb"),
                Projectile {
                    kind: ItemKind::ShockOrb,
                    owner: u.racer,
                },
                Transform::from_translation(pose.point(orb.lateral, o.height))
                    .with_rotation(pose.rotation()),
                orb,
                ItemTimer(o.life_secs),
                ItemRadius(o.radius),
                Interpolated,
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        teleported.write(Teleported { entity });
    }
}

/// Moves the orbs along the track, bounces them, knocks cars and retires spent orbs.
#[allow(clippy::type_complexity)]
pub(crate) fn roll_orbs(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    track: Option<Res<Track>>,
    rapier: ReadRapierContext,
    mut orbs: Query<(
        Entity,
        &Projectile,
        &mut Orb,
        &mut ItemTimer,
        &mut Transform,
    )>,
    cars: Query<(Entity, &Transform), (With<Racer>, Without<Orb>)>,
    mut hits: Hits,
) {
    let Some(track) = track else { return };
    let o = &tuning.shock_orb;
    let ctx = rapier.single().ok();
    for (entity, projectile, mut orb, mut timer, mut t) in &mut orbs {
        if !run_down(&mut timer.0) || orb.hit.len() >= o.hits as usize {
            commands.entity(entity).despawn();
            continue;
        }
        orb.s = track.wrap(orb.s + orb.along * SIM_DT);
        let frame = track.frame_at(orb.s);
        let limits = edges(ctx.as_ref(), &frame, o.height, o.edge_margin);
        (orb.lateral, orb.across) = bounce(orb.lateral + orb.across * SIM_DT, orb.across, limits);
        let pos = frame.point(orb.lateral, o.height);
        *t = Transform::from_translation(pos).with_rotation(frame.rotation());
        for (car, c) in &cars {
            let to = centre(c) - pos;
            if car == projectile.owner
                || orb.hit.contains(&car)
                || orb.hit.len() >= o.hits as usize
                || to.length() > o.radius
            {
                continue;
            }
            orb.hit.push(car);
            let side = if to.dot(frame.right) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let blow = Blow::impulse(tumble(&o.knock, frame.right * side));
            hits.land(car, projectile.owner, ItemKind::ShockOrb, entity, blow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounce_reflects_off_both_edges() {
        let lane = (-4.0, 4.0);
        assert_eq!(bounce(1.0, 3.0, lane), (1.0, 3.0));
        assert_eq!(bounce(4.5, 3.0, lane), (3.5, -3.0));
        assert_eq!(bounce(-4.5, -3.0, lane), (-3.5, 3.0));
        // Far outside (a chokepoint narrowed the track): clamped inside, heading back in.
        assert_eq!(bounce(20.0, 3.0, lane), (-4.0, -3.0));
        assert_eq!(bounce(0.5, 1.0, (0.0, 0.0)), (0.0, -1.0));
        // A barrier farther out on one side (an Asphalt curb) than the open edge on the other.
        assert_eq!(bounce(5.75, 2.0, (-4.0, 5.25)), (4.75, -2.0));
        assert_eq!(bounce(-4.5, -2.0, (-4.0, 5.25)), (-3.5, 2.0));
    }
}
