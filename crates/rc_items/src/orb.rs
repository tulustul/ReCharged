//! Shock Orb (GDD §6.2): a slow plasma ball that follows the track in track coordinates
//! (`Track::frame_at`): it runs along the centreline at the launch heading's share of its speed
//! and bounces between the edges of the racing width. Any car but its owner whose centre comes
//! within 3 u is knocked sideways and up (each car once); it is spent after 3 hits or 10 s.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::data::ItemTuning;
use crate::effects::run_down;
use crate::hits::{Blow, CAR_HALF_LENGTH_U, Hits, centre, tumble};
use crate::pickups::UseItem;

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

/// Reflects a lateral position and speed off the edges at ±`limit`.
pub(crate) fn bounce(lateral: f32, across: f32, limit: f32) -> (f32, f32) {
    if lateral > limit {
        ((2.0 * limit - lateral).max(-limit), -across.abs())
    } else if lateral < -limit {
        ((-2.0 * limit - lateral).min(limit), across.abs())
    } else {
        (lateral, across)
    }
}

/// Launches the orbs of this tick's [`UseItem`]s ahead of the car, along the track in the
/// direction the car faces, at its heading (capped) to the track.
pub(crate) fn launch_orbs(
    mut commands: Commands,
    tuning: Res<ItemTuning>,
    track: Option<Res<Track>>,
    mut uses: MessageReader<UseItem>,
    cars: Query<&Transform, With<Racer>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let o = &tuning.shock_orb;
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
        let limit = (track.frame_at(s).half_width - o.edge_margin).max(0.0);
        let orb = Orb {
            s,
            lateral: at.lateral.clamp(-limit, limit),
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
    for (entity, projectile, mut orb, mut timer, mut t) in &mut orbs {
        if !run_down(&mut timer.0) || orb.hit.len() >= o.hits as usize {
            commands.entity(entity).despawn();
            continue;
        }
        orb.s = track.wrap(orb.s + orb.along * SIM_DT);
        let frame = track.frame_at(orb.s);
        let limit = (frame.half_width - o.edge_margin).max(0.0);
        (orb.lateral, orb.across) = bounce(orb.lateral + orb.across * SIM_DT, orb.across, limit);
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
        assert_eq!(bounce(1.0, 3.0, 4.0), (1.0, 3.0));
        assert_eq!(bounce(4.5, 3.0, 4.0), (3.5, -3.0));
        assert_eq!(bounce(-4.5, -3.0, 4.0), (-3.5, 3.0));
        // Far outside (a chokepoint narrowed the track): clamped inside, heading back in.
        assert_eq!(bounce(20.0, 3.0, 4.0), (-4.0, -3.0));
        assert_eq!(bounce(0.5, 1.0, 0.0), (0.0, -1.0));
    }
}
