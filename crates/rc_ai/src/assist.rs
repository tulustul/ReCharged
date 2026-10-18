//! Steering assist (GDD §15, `SimSet::Assist`): for local players without the AI and with
//! `Settings.accessibility.steering_assist` on, blends `CarControl::steer` — and nothing else —
//! toward the steer that follows the racing line; after a respawn or when the car points away
//! from the track (a crash, a spin), a stronger pull turns it back for a moment.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::brain::TrackView;
use crate::data::AiTuning;
use crate::profile::CarCaps;
use crate::steering::{angle_to, lookahead, pursuit_curvature, steer_for_curvature};

/// The player's own steering keeps this much priority: the blend share shrinks by it × |steer|.
const HUMAN_PRIORITY: f32 = 0.6;
/// Pointing farther than this (rad) from the track direction means the car is turned away
/// (crash, spin): the recovery pull starts.
const TURNED_AWAY_RAD: f32 = 0.8;
/// Cornering of a car missing from the catalog (u/s², the 3★ value).
const DEFAULT_LATERAL: f32 = 30.5;
/// Moving backward faster than this (u/s) the player is reversing out: steering acts mirrored,
/// so the assist stands down.
const REVERSING_UPS: f32 = 1.0;

/// Assist state of a local player's car.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct AssistState {
    /// Seconds of recovery pull left.
    pub recovering: f32,
}

/// Decorates local players' racers with an [`AssistState`].
pub(crate) fn add_assist_state(add: On<Add<LocalPlayer>>, mut commands: Commands) {
    commands.entity(add.entity).insert(AssistState::default());
}

/// The player's `human` steer blended toward `assist` with `strength` (0–1); the more the player
/// steers, the less the assist does.
pub fn blend(human: f32, assist: f32, strength: f32) -> f32 {
    let w = strength * (1.0 - HUMAN_PRIORITY * human.abs().min(1.0));
    (human + w * (assist - human)).clamp(-1.0, 1.0)
}

type AssistItem<'a> = (
    &'a Racer,
    &'a Transform,
    &'a TrackView,
    &'a VehicleTelemetry,
    &'a mut CarControl,
    &'a mut AssistState,
);

/// `SimSet::Assist`: assisted players' steering.
pub(crate) fn steering_assist(
    settings: Option<Res<Settings>>,
    track: Option<Res<Track>>,
    tuning: Option<Res<AiTuning>>,
    catalog: Option<Res<Catalog>>,
    phase: Option<Res<State<RacePhase>>>,
    mut humans: Query<AssistItem, (With<LocalPlayer>, Without<AiDriver>)>,
) {
    let on = settings.is_some_and(|s| s.accessibility.steering_assist);
    let racing = phase.is_some_and(|p| *p.get() == RacePhase::Racing);
    let (Some(track), Some(tuning), true, true) = (track, tuning, on, racing) else {
        return;
    };
    let (a, d) = (&tuning.assist, &tuning.driving);
    for (racer, transform, view, tel, mut control, mut state) in &mut humans {
        if tel.forward_speed < -REVERSING_UPS {
            continue;
        }
        let (heading, up) = (
            transform.rotation * Vec3::NEG_Z,
            transform.rotation * Vec3::Y,
        );
        let forward = track.frame_at(view.s).forward;
        let turned_away = angle_to(heading, forward, up).abs() > TURNED_AWAY_RAD;
        state.recovering = if view.teleported || turned_away {
            a.recovery_secs
        } else {
            (state.recovering - SIM_DT).max(0.0)
        };
        let speed = tel.forward_speed.max(0.0);
        let s_t = view.s
            + lookahead(
                speed,
                d.lookahead_secs,
                d.lookahead_min_u,
                d.lookahead_max_u,
            );
        let target = track.racing_line_at(s_t);
        let kappa = pursuit_curvature(view.pos, heading, up, target);
        let lateral = catalog
            .as_deref()
            .and_then(|c| c.car(&racer.car))
            .map_or(DEFAULT_LATERAL, |c| {
                CarCaps::from_stars(c.stars, &d.lateral_accel_by_handling).lateral_accel
            });
        let assist = steer_for_curvature(kappa, speed, lateral);
        let strength = match state.recovering > 0.0 {
            true => a.recovery_strength,
            false => a.strength,
        };
        let steer = blend(control.steer, assist, strength);
        if steer != control.steer {
            control.steer = steer;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blend_leans_toward_the_line_and_yields_to_the_player() {
        assert!(
            (blend(0.0, 1.0, 0.4) - 0.4).abs() < 1e-6,
            "hands off: 40 % of the line steer"
        );
        let firm = blend(-1.0, 1.0, 0.4);
        assert!(firm < -0.6, "full opposite lock is mostly kept: {firm}");
        assert_eq!(blend(0.5, 0.5, 0.4), 0.5, "agreeing input is unchanged");
        assert_eq!(blend(0.3, 0.3, 0.0), 0.3, "strength 0 = off");
        assert!(blend(1.0, 1.0, 1.0) <= 1.0);
    }
}
