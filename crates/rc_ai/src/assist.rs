//! Steering assist (GDD §15, `SimSet::Assist`): for local players without the AI and with
//! `Settings.accessibility.steering_assist` on, blends `CarControl::steer` toward the steer that
//! follows the racing line; after a respawn or when the car points away from the track (a crash,
//! a spin), a stronger pull turns it back for a moment. Hands off the steering, it also brakes
//! gently before corners taken too fast (auto-brake: the throttle lifts, the brake goes to at most
//! [`MAX_AUTO_BRAKE`]), so a throttle-only player gets round. It only ever lowers the speed, and
//! the player's own brake or steering takes over.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::brain::{REFERENCE_STARS, TrackView};
use crate::data::AiTuning;
use crate::pace::{JUMP_RUNUP_U, JUMP_TAKEOFF_SHARE, PEDAL_LEAD_SECS};
use crate::profile::{CarCaps, SpeedProfile, brake_point_speed};
use crate::scene::jump_takeoff_ahead;
use crate::steering::{angle_to, lookahead, pursuit_curvature, steer_for_curvature};

/// The player's own steering keeps this much priority: the blend share shrinks by it × |steer|.
const HUMAN_PRIORITY: f32 = 0.6;
/// With no steering input the assist steers the line almost fully — a throttle-only player
/// (the kid-friendly path) understeered off every corner at 40 % — and hands control back as the
/// input grows to [`HANDS_ON_INPUT`].
const HANDS_OFF_STRENGTH: f32 = 0.9;
const HANDS_ON_INPUT: f32 = 0.25;
/// Pointing farther than this (rad) from the track direction means the car is turned away
/// (crash, spin): the recovery pull starts.
const TURNED_AWAY_RAD: f32 = 0.8;
/// Moving backward faster than this (u/s) the player is reversing out: steering acts mirrored,
/// so the assist stands down.
const REVERSING_UPS: f32 = 1.0;
/// The auto-brake never brakes harder than this.
pub const MAX_AUTO_BRAKE: f32 = 0.4;
/// Over the plan by up to this much (u/s), the throttle lifts; beyond, the brake comes on,
/// reaching [`MAX_AUTO_BRAKE`] this much further over.
const LIFT_RAMP_UPS: f32 = 1.0;
const BRAKE_RAMP_UPS: f32 = 2.0;
/// Deceleration the auto-brake's plan counts on (u/s²): the 0.4 brake of a 40 u/s² car, less a
/// margin.
const AUTO_BRAKE_DECEL: f32 = 14.0;
/// Share of the car's cornering grip the plan uses (the AI's: Rookie 0.82 … Elite 0.96); the
/// assisted line is followed less tightly than the AI's.
const PLAN_CORNER_GRIP: f32 = 0.85;
/// Braking above this pedal level, the player keeps the pedals.
const PLAYER_BRAKE: f32 = 0.05;

/// Assist state of a local player's car.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct AssistState {
    /// Seconds of recovery pull left.
    pub recovering: f32,
    /// The auto-brake's speed plan (built on the first assisted tick of the race).
    pub plan: Option<SpeedProfile>,
}

/// Decorates local players' racers with an [`AssistState`].
pub(crate) fn add_assist_state(add: On<Add<LocalPlayer>>, mut commands: Commands) {
    commands.entity(add.entity).insert(AssistState::default());
}

/// The player's `human` steer blended toward `assist` with `strength` (0–1, the share once the
/// player steers); hands off it steers almost fully, and the more the player steers, the less it
/// does.
pub fn blend(human: f32, assist: f32, strength: f32) -> f32 {
    let hands = (human.abs() / HANDS_ON_INPUT).min(1.0);
    let share = strength.max(HANDS_OFF_STRENGTH * (1.0 - hands));
    let w = share * (1.0 - HUMAN_PRIORITY * human.abs().min(1.0));
    (human + w * (assist - human)).clamp(-1.0, 1.0)
}

/// The player's `throttle` and `brake` with the auto-brake, `over` u/s above the plan, the player
/// steering `steer`: hands off, the throttle lifts and the brake comes on gently; the player's
/// own brake, or steering, takes over. Neither pedal ever pushes the car faster.
pub fn auto_pedals(throttle: f32, brake: f32, steer: f32, over: f32) -> (f32, f32) {
    let ease = 1.0 - (steer.abs() / HANDS_ON_INPUT).min(1.0);
    if brake > PLAYER_BRAKE || over <= 0.0 || ease <= 0.0 {
        return (throttle, brake);
    }
    let lift = (over / LIFT_RAMP_UPS).min(1.0) * ease;
    let auto = MAX_AUTO_BRAKE * ((over - LIFT_RAMP_UPS) / BRAKE_RAMP_UPS).clamp(0.0, 1.0) * ease;
    (throttle * (1.0 - lift), brake.max(auto))
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
    weather: Option<Res<WeatherState>>,
    mut humans: Query<AssistItem, (With<LocalPlayer>, Without<AiDriver>)>,
) {
    let on = settings.is_some_and(|s| s.accessibility.steering_assist);
    let racing = phase.is_some_and(|p| *p.get() == RacePhase::Racing);
    let (Some(track), Some(tuning), true, true) = (track, tuning, on, racing) else {
        return;
    };
    let (a, d) = (&tuning.assist, &tuning.driving);
    let (wet, snow) = weather.map_or((0.0, 0.0), |w| (w.wetness, w.snow_cover));
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
        let stars = catalog
            .as_deref()
            .and_then(|c| c.car(&racer.car))
            .map_or(REFERENCE_STARS, |c| c.stars);
        let caps = CarCaps::from_stars(stars, &d.lateral_accel_by_handling);
        let assist = steer_for_curvature(kappa, speed, caps.lateral_accel);
        let strength = match state.recovering > 0.0 {
            true => a.recovery_strength,
            false => a.strength,
        };
        let steer = blend(control.steer, assist, strength);
        // Auto-brake: the speed the car can still brake from (gently) into the corners ahead.
        let plan = state.plan.get_or_insert_with(|| {
            SpeedProfile::build(&track, &caps, PLAN_CORNER_GRIP, AUTO_BRAKE_DECEL)
        });
        let lead = view.s + speed * PEDAL_LEAD_SECS;
        let corner = |x: f32| {
            let props = plan.surface_at(lead + x).props();
            plan.corner_at(lead + x) * (props.with_weather(wet, snow).grip / props.grip).sqrt()
        };
        let mut brake_to = brake_point_speed(speed, AUTO_BRAKE_DECEL, plan.spacing(), corner);
        if let Some(takeoff) = jump_takeoff_ahead(&track, view.s, JUMP_RUNUP_U) {
            // A jump's run-up: too little speed drops the car into the gap.
            brake_to = brake_to.max(JUMP_TAKEOFF_SHARE * takeoff);
        }
        let (throttle, brake) = auto_pedals(
            control.throttle,
            control.brake,
            control.steer,
            speed - brake_to,
        );
        if (steer, throttle, brake) != (control.steer, control.throttle, control.brake) {
            control.steer = steer;
            control.throttle = throttle;
            control.brake = brake;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blend_leans_toward_the_line_and_yields_to_the_player() {
        assert!(
            (blend(0.0, 1.0, 0.4) - HANDS_OFF_STRENGTH).abs() < 1e-6,
            "hands off: (almost) the line steer"
        );
        let light = blend(0.3, 1.0, 0.4);
        assert!(
            (light - (0.3 + 0.4 * (1.0 - 0.6 * 0.3) * 0.7)).abs() < 1e-5,
            "steering hands control back: {light}"
        );
        let firm = blend(-1.0, 1.0, 0.4);
        assert!(firm < -0.6, "full opposite lock is mostly kept: {firm}");
        assert_eq!(blend(0.5, 0.5, 0.4), 0.5, "agreeing input is unchanged");
        assert_eq!(blend(0.3, 0.3, 0.0), 0.3, "strength 0 = off");
        assert!(blend(1.0, 1.0, 1.0) <= 1.0);
    }

    #[test]
    fn the_auto_brake_only_slows_and_yields_to_the_player() {
        assert_eq!(
            auto_pedals(1.0, 0.0, 0.0, -3.0),
            (1.0, 0.0),
            "under the plan: untouched"
        );
        let (t, b) = auto_pedals(1.0, 0.0, 0.0, 0.5);
        assert!(
            t > 0.0 && t < 1.0 && b == 0.0,
            "just over: the throttle lifts"
        );
        assert_eq!(
            auto_pedals(1.0, 0.0, 0.0, 10.0),
            (0.0, MAX_AUTO_BRAKE),
            "gentle brake"
        );
        assert_eq!(
            auto_pedals(1.0, 0.8, 0.0, 10.0),
            (1.0, 0.8),
            "the player's brake wins"
        );
        assert_eq!(
            auto_pedals(1.0, 0.0, 0.5, 10.0),
            (1.0, 0.0),
            "steering hands it back"
        );
        let (t, b) = auto_pedals(1.0, 0.0, 0.1, 10.0);
        assert!(
            t > 0.0 && b > 0.0 && b < MAX_AUTO_BRAKE,
            "light steering: partly"
        );
    }
}
