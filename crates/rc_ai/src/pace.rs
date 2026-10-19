//! Pace (pure-ish helpers of the driver): the speed plan → throttle and brake, difficulty
//! mistakes, the cornering adaptation from measured slides.

use rc_core::prelude::*;

use crate::brain::{AiBrain, Mistake, TrackView};
use crate::charge;
use crate::data::DifficultyProfile;
use crate::driver::{AiCarItem, Path};
use crate::profile::brake_point_speed;
use crate::scene::{CAR_HALF_WIDTH_U, Scene, jump_takeoff_ahead, near_jump, nearest_car_ahead};
use crate::steering::pedals;
use crate::traffic::{Obstacle, rear_end_limit};

/// Mistakes are rolled every this many ticks.
const MISTAKE_PERIOD_TICKS: u64 = 30;
/// Late-braking overspeed share and wide-line offset (u) ranges, mistake length (s).
const LATE_BRAKING: (f32, f32) = (0.08, 0.16);
const WIDE_LINE_U: (f32, f32) = (1.0, 2.2);
const MISTAKE_SECS: (f32, f32) = (1.0, 2.0);
/// The pedals act on what the car meets this far ahead (s × speed): one tick of control lag
/// plus the brakes' build-up.
pub(crate) const PEDAL_LEAD_SECS: f32 = 0.1;
/// Body slip beyond this (rad) outside a drift counts as a slide: the corner plan adapts down.
const SLIDE_RAD: f32 = 0.2;
/// Cornering adaptation: lost per second of sliding, regained per second, its floor.
const ADAPT_DROP_PER_SEC: f32 = 0.4;
const ADAPT_RISE_PER_SEC: f32 = 0.02;
const ADAPT_MIN: f32 = 0.8;
/// Behind a slower car, close in to this gap (u, centre to centre) plus this headway (s) at the
/// own speed, at this rate (u/s per u of extra gap).
const FOLLOW_GAP_U: f32 = 3.0;
const FOLLOW_HEADWAY_SECS: f32 = 0.25;
const FOLLOW_CLOSE_RATE: f32 = 1.0;
/// An inside lane never plans below this share of the racing line's cornering (sqrt taken).
const MIN_LANE_SHARE: f32 = 0.5;
/// Horizon of the straight-road scan for boosts and Turbo Cells (s at the current speed).
const STRAIGHT_SCAN_SECS: f32 = 3.0;
/// The straight scan starts from at least this speed (u/s), so a standing car scans too.
const MIN_SCAN_UPS: f32 = 5.0;
/// On a jump's run-up (the lip within this many u) the throttle aims at no less than this share
/// of the jump's reference takeoff speed, whatever the difficulty: rc_track guarantees landings
/// from 0.8× of it, a slower car drops into the gap.
pub(crate) const JUMP_RUNUP_U: f32 = 45.0;
pub(crate) const JUMP_TAKEOFF_SHARE: f32 = 0.9;
/// Lateral room kept between the bodies of cars that overlap (u, as the driver's alongside gap).
const OVERLAP_CLEARANCE_U: f32 = 0.4;
/// A Goo Bomb carrier chasing a car to bump drives this much over its plan (closing 10 u on a
/// car at 25 u/s takes ≈ 4 s of the 6 s fuse).
const GOO_CHASE_SCALE: f32 = 1.1;

/// Throttle and brake: the speed plan scaled by difficulty, braked into early enough; the car's
/// current top speed (surface, catch-up, boost) caps the throttle only. Also returns the speed
/// the brakes aim at and the seconds of straight road ahead.
pub(crate) fn speed_plan(
    w: &Scene,
    car: &AiCarItem,
    view: &TrackView,
    path: &Path,
    obstacles: &[Obstacle],
    seeking: bool,
) -> (f32, f32, f32, f32) {
    let brain = &*car.brain;
    let tel = car.telemetry;
    let speed = tel.forward_speed;
    let d = &w.tuning.driving;
    let skill = w.tuning.difficulty.get(car.driver.difficulty);
    let mut scale = skill.speed_scale;
    if let Some((Mistake::LateBraking(x), _)) = brain.mistake {
        scale *= 1.0 + x;
    }
    if seeking {
        scale *= GOO_CHASE_SCALE;
    }
    if car.progress.finished {
        scale *= d.finished_speed_scale;
    }
    let boost_top = match tel.boosting {
        true => car.effects.boost().map_or(charge::CHARGE_BOOST_TOP, |b| {
            b.top_speed.max(charge::CHARGE_BOOST_TOP)
        }),
        false => 1.0,
    };
    let cap_mul = (1.0 + car.catch_up.map_or(0.0, |c| c.0)) * car.effects.top_speed_mul();
    let grip = brain.adapt * car.effects.grip_mul();
    let (wet, snow) = w.wet;
    let length = w.track.length();
    let corner_at = |s: f32| {
        let props = brain.surface_at(length, s).props();
        let weather = props.with_weather(wet, snow);
        // An inside lane turns tighter than the racing line: κ' = κ / (1 − κ·d).
        let kappa = brain.kappa_at(length, s);
        let inside = brain.lane * kappa.signum();
        let lane = match inside > 0.0 {
            true => (1.0 - kappa.abs() * inside).max(MIN_LANE_SHARE).sqrt(),
            false => 1.0,
        };
        scale * lane * brain.corner_at(length, s) * (grip * weather.grip / props.grip).sqrt()
    };
    let here = brain
        .surface_at(length, view.s)
        .props()
        .with_weather(wet, snow);
    let cap = scale * brain.caps.top_speed * here.top_speed * cap_mul;
    let lead = view.s + speed.max(0.0) * PEDAL_LEAD_SECS;
    let spacing = brain.profile.spacing();
    let mut brake_to = brake_point_speed(speed.max(0.0), d.brake_decel, spacing, |x| {
        corner_at(lead + x)
    });
    // Traffic: never run into a slower car — unless bumping it on purpose with a Goo Bomb, or on
    // a jump's run-up, where too little speed drops the car into the gap.
    if !seeking && !near_jump(w.track, view.s, 0.0) {
        let gap = FOLLOW_GAP_U + FOLLOW_HEADWAY_SECS * speed.max(0.0);
        if let (Some(follow), Some(ahead)) = (path.lane.follow_speed, nearest_car_ahead(obstacles))
        {
            brake_to = brake_to.min(follow + (ahead - gap) * FOLLOW_CLOSE_RATE);
        }
        let overlap = 2.0 * CAR_HALF_WIDTH_U + OVERLAP_CLEARANCE_U;
        if let Some(limit) = rear_end_limit(
            view.lateral,
            speed,
            obstacles,
            overlap,
            gap,
            FOLLOW_CLOSE_RATE,
        ) {
            brake_to = brake_to.min(limit);
        }
    }
    let mut target = brake_to.min(cap * boost_top);
    if let Some(takeoff) = jump_takeoff_ahead(w.track, view.s, JUMP_RUNUP_U) {
        let own_top = brain.caps.top_speed * here.top_speed * cap_mul * boost_top;
        target = target.max((JUMP_TAKEOFF_SHARE * takeoff).min(own_top).min(brake_to));
    }
    let (throttle, brake) = pedals(speed, target, brake_to);
    // Seconds of straight road ahead (the plan stays above the boosted cap), driving it at full
    // throttle with the car's acceleration (Accel stars).
    let (mut v, mut x, mut secs) = (speed.max(MIN_SCAN_UPS), 0.0, 0.0);
    while secs < STRAIGHT_SCAN_SECS && corner_at(view.s + x) >= cap * charge::CHARGE_BOOST_TOP {
        let dt = spacing / v;
        secs += dt;
        x += spacing;
        v = (v + brain.caps.accel_at(v, cap) * dt).min(cap.max(MIN_SCAN_UPS));
    }
    (throttle, brake, brake_to, secs)
}

/// Mistakes (GDD §8.2): rolled every half second, more often for less consistent rivals.
pub(crate) fn roll_mistake(w: &Scene, brain: &mut AiBrain, skill: &DifficultyProfile, key: u64) {
    if let Some((m, left)) = brain.mistake {
        brain.mistake = (left > SIM_DT).then_some((m, left - SIM_DT));
        return;
    }
    if !w.tick.is_multiple_of(MISTAKE_PERIOD_TICKS) {
        return;
    }
    let per_check = MISTAKE_PERIOD_TICKS as f32 * SIM_DT / 60.0;
    let p = skill.mistakes_per_min * (1.5 - brain.consistency) * per_check;
    if !rng::chance(w.seed, w.tick, RngStream::AiMistakes, key, p) {
        return;
    }
    let pick = |n: u64, (lo, hi): (f32, f32)| {
        rng::range(w.seed, w.tick, RngStream::AiMistakes, key + n, lo, hi)
    };
    let kind = match rng::chance(w.seed, w.tick, RngStream::AiMistakes, key + 1, 0.5) {
        true => Mistake::LateBraking(pick(2, LATE_BRAKING)),
        false => Mistake::WideLine(pick(2, WIDE_LINE_U)),
    };
    brain.mistake = Some((kind, pick(3, MISTAKE_SECS)));
}

/// Slides (outside a drift) lower the corner plan a little; clean driving earns it back.
pub(crate) fn adapt_grip(brain: &mut AiBrain, tel: &VehicleTelemetry) {
    let sliding =
        brain.drift.is_none() && tel.grounded_wheels >= 3 && tel.slip_angle.abs() > SLIDE_RAD;
    brain.adapt = match sliding {
        true => (brain.adapt - ADAPT_DROP_PER_SEC * SIM_DT).max(ADAPT_MIN),
        false => (brain.adapt + ADAPT_RISE_PER_SEC * SIM_DT).min(1.0),
    };
}
