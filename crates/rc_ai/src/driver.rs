//! `SimSet::Drivers`: the AI drivers. One system computes every AI racer's `CarControl` from its
//! track view, telemetry, the traffic and items around it (`scene`) and its brain; the decisions
//! are the pure functions of `steering`, `traffic`, `recovery`, `charge` and `items`.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{RibbonStyle, Track, TrackFrame};

use crate::brain::{AiBrain, KEYS_PER_RACER, Mistake, TrackView};
use crate::charge;
use crate::data::AiTuning;
use crate::items::{ItemView, Temper};
use crate::pace;
use crate::recovery::{Action, Sense};
use crate::scene::{
    CAR_HALF_WIDTH_U, Scene, SceneBuffers, behind_wall, narrowest_ahead, near_jump,
};
use crate::steering::{lookahead, pursuit_curvature, steer_for_curvature};
use crate::traffic::{LaneChoice, Obstacle, choose_lane, side_bounds};

/// Below this height above the centreline's surface (u) a car has fallen off an upper level
/// (rc_race's off-track depth; open floors may lie a little below the centreline, and a car
/// trapped under a ramp is caught by the recovery's progress check).
const FALLEN_HEIGHT_U: f32 = -3.0;
/// Inside a solid ribbon's width, this far below its surface (u) the car is under it (fell into
/// a gap, drove under a landing ramp): off the course.
const UNDER_RIBBON_U: f32 = 1.0;
/// Hazards are not dropped this far (u) before a jump's lock zone either (a respawned car may
/// start there).
const JUMP_NO_DROP_U: f32 = 15.0;
/// Farther than this from its planned lateral (u), the car also clears obstacles on its own
/// path back to the line.
const OFF_LINE_U: f32 = 1.5;
/// Lane changes allowed around a jump (u): a small shift for traffic, a bigger dodge around a
/// hazard on the run-up (a Spring Pad on the ramp launches the car off it) — never off the ramp.
const JUMP_LANE_U: f32 = 0.8;
const JUMP_DODGE_U: f32 = 3.0;
/// `up_dot` below this = on the side or roof (GDD §5.4).
const FLIPPED_UP_DOT: f32 = 0.3;
/// Wide-line mistakes act only in corners tighter than this radius (u).
const WIDE_LINE_RADIUS_U: f32 = 60.0;
/// Reversing out steers at least this hard (−1…1).
const REVERSE_STEER: f32 = 0.8;
/// Beyond this body slip (rad) the pursuit aims the velocity, not the nose: rc_vehicle's
/// counter-steer and stability assists catch the slide, the AI steers the path.
const SLIDE_REFERENCE_RAD: f32 = 0.14;
/// Lateral room kept between the bodies of cars alongside (u).
const ALONGSIDE_CLEARANCE_U: f32 = 0.4;

/// Ticks since the attempt began (keys the AI's RNG draws). Reset in `OnEnter(RacePhase::Setup)`.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AiClock(pub u64);

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct AiCar {
    pub(crate) entity: Entity,
    pub(crate) racer: &'static Racer,
    pub(crate) driver: &'static AiDriver,
    pub(crate) transform: &'static Transform,
    pub(crate) telemetry: &'static VehicleTelemetry,
    pub(crate) charge: &'static Charge,
    pub(crate) effects: &'static StatusEffects,
    pub(crate) slot: &'static ItemSlot,
    pub(crate) progress: &'static RaceProgress,
    pub(crate) view: &'static TrackView,
    pub(crate) catch_up: Option<&'static CatchUp>,
    pub(crate) brain: &'static mut AiBrain,
    pub(crate) control: &'static mut CarControl,
}

type HazardItem<'a> = (
    &'a Hazard,
    &'a Transform,
    Option<&'a ItemTarget>,
    Option<&'a ItemRadius>,
);
type ProjectileItem<'a> = (
    &'a Projectile,
    &'a Transform,
    Option<&'a ItemTarget>,
    Option<&'a ItemRadius>,
);

/// Every AI racer's `CarControl` for this tick.
#[allow(clippy::too_many_arguments)]
pub(crate) fn drive_ai(
    mut clock: ResMut<AiClock>,
    track: Option<Res<Track>>,
    tuning: Option<Res<AiTuning>>,
    phase: Option<Res<State<RacePhase>>>,
    config: Option<Res<RaceConfig>>,
    weather: Option<Res<WeatherState>>,
    standings: Option<Res<Standings>>,
    mut cars: Query<AiCar>,
    racers: Query<(Entity, &TrackView, &StatusEffects), With<Racer>>,
    hazards: Query<HazardItem>,
    projectiles: Query<ProjectileItem>,
    mut buffers: Local<SceneBuffers>,
) {
    clock.0 += 1;
    let (Some(track), Some(tuning)) = (track, tuning) else {
        return;
    };
    let phase = phase.map_or(RacePhase::Setup, |p| *p.get());
    let buffers = &mut *buffers;
    buffers.fill(&track, racers.iter(), hazards.iter(), projectiles.iter());
    let scene = Scene {
        track: &track,
        tuning: &tuning,
        tick: clock.0,
        seed: config.map_or(0, |c| c.seed),
        wet: weather.map_or((0.0, 0.0), |w| (w.wetness, w.snow_cover)),
        others: &buffers.others,
        hazards: &buffers.hazards,
        locked: &buffers.locked,
        goo: &buffers.goo,
        standings: standings.as_deref().map_or(&[], |s| &s.0),
    };
    for mut car in &mut cars {
        let control = drive(
            &scene,
            &mut car,
            phase,
            &mut buffers.obstacles,
            &mut buffers.alongside,
        );
        car.brain.last_throttle = control.throttle;
        *car.control = control;
    }
}

/// Where the car drives this tick.
pub(crate) struct Path {
    /// Pure-pursuit target point.
    pub(crate) target: Vec3,
    /// Planned lateral here (before the lane shift).
    pub(crate) planned: f32,
    pub(crate) lane: LaneChoice,
    /// Half the legal width of the narrowest track ahead (u).
    pub(crate) narrowest: f32,
}

/// One AI racer's control for this tick.
fn drive(
    w: &Scene,
    car: &mut AiCarItem,
    phase: RacePhase,
    obstacles: &mut Vec<Obstacle>,
    alongside: &mut Vec<f32>,
) -> CarControl {
    let brain = &mut *car.brain;
    let counting = brain.countdown.0 > 0 && brain.countdown.0 < brain.countdown.1;
    match phase {
        RacePhase::Countdown => return countdown(brain),
        RacePhase::Racing if counting => return countdown(brain),
        RacePhase::Racing | RacePhase::Finished => {}
        _ => return CarControl::default(),
    }
    let view = *car.view;
    if view.teleported {
        brain.recovery.reset();
        brain.lane = 0.0;
        brain.drift = None;
        brain.boosting = false;
    }
    let tuning = w.tuning;
    let skill = tuning.difficulty.get(car.driver.difficulty);
    let key = car.racer.index as u64 * KEYS_PER_RACER;
    let tel = car.telemetry;
    let speed = tel.forward_speed;
    let rot = car.transform.rotation;
    let (heading, up) = (rot * Vec3::NEG_Z, rot * Vec3::Y);
    let frame = w.track.frame_at(view.s);
    pace::roll_mistake(w, brain, skill, key);

    let seek = w
        .goo_target(car.entity, &view)
        .map(|o| (o.entity, o.lateral));
    let path = plan_path(
        w, brain, car.entity, &view, speed, seek, obstacles, alongside,
    );

    // Steering: pure pursuit along the velocity while sliding sideways, else the heading.
    let planar_vel = view.vel - up * view.vel.dot(up);
    let sliding = brain.drift.is_some() || tel.slip_angle.abs() > SLIDE_REFERENCE_RAD;
    let reference = match sliding && planar_vel.length() > 3.0 && speed > 0.0 {
        true => planar_vel.normalize(),
        false => heading,
    };
    let kappa = pursuit_curvature(view.pos, reference, up, path.target);
    let steer = steer_for_curvature(kappa, speed.max(0.0), brain.caps.lateral_accel);

    let sense = Sense {
        speed: speed.abs(),
        throttle: brain.last_throttle > 0.5,
        progress: view.progress,
        off_course: view.lateral.abs() > frame.half_width + frame.offroad_margin
            || view.height < FALLEN_HEIGHT_U
            || behind_wall(w.track, &view, &frame)
            || (w.track.style() != RibbonStyle::Invisible
                && view.lateral.abs() < frame.half_width
                && view.height < -UNDER_RIBBON_U),
        flipped: tel.up_dot < FLIPPED_UP_DOT,
    };
    match brain.recovery.step(&tuning.recovery, sense, SIM_DT) {
        Action::Drive => {}
        Action::Reverse => {
            let swing = -kappa.signum() * steer.abs().max(REVERSE_STEER);
            return CarControl {
                brake: 1.0,
                steer: swing,
                ..default()
            };
        }
        Action::HoldReset => {
            return CarControl {
                reset_held: true,
                ..default()
            };
        }
    }

    let was_drifting = brain.drift.is_some();
    let sense = charge::DriftSense {
        view: &view,
        tel,
        top: brain.caps.top_speed,
        steer,
        charge: car.charge.0,
        off_line: (view.lateral - path.planned - brain.lane).abs(),
        edge_room: path.narrowest - view.lateral.abs(),
    };
    let drift = charge::drift(w, brain, skill, key, sense);
    pace::adapt_grip(brain, tel);

    let (throttle, brake, brake_to, straight_secs) =
        pace::speed_plan(w, car, &view, &path, obstacles, seek.is_some());
    let brain = &mut *car.brain;
    let throttle = match (was_drifting || brain.catching) && charge::catching(tel, drift) {
        true => throttle.min(charge::CATCH_THROTTLE),
        false => throttle,
    };
    brain.catching = (was_drifting || brain.catching) && charge::catching(tel, drift);
    let top = brain.caps.top_speed;
    let straight = straight_secs >= charge::straight_secs(brain.personality, tuning);
    let boost = charge::boost(brain, car.charge.0, tel, top, brake_to, straight);

    let own = ItemView {
        shielded: car.effects.shielded(),
        boosting: tel.boosting,
        straight_secs,
        rank: car.progress.rank,
        near_jump: near_jump(w.track, view.s, JUMP_NO_DROP_U),
        ..default()
    };
    let items = w.item_view(car.entity, &view, heading, up, own);
    let usable = car.slot.item.filter(|_| car.slot.roulette <= 0.0);
    let temper = Temper {
        aggression: brain.aggression,
        hoard: (brain.personality.hoard + skill.hoard).min(1.0),
    };
    let fire = brain.items.step(
        usable,
        &items,
        temper,
        skill,
        &tuning.items,
        (w.seed, w.tick, key),
    );

    CarControl {
        throttle,
        brake,
        steer,
        drift,
        boost,
        use_item: fire.is_some(),
        look_back: fire == Some(true),
        reset_held: false,
    }
}

/// The racing line + personal offset, shifted for traffic, hazards and mistakes, never wider than
/// the narrowest track ahead nor into cars alongside; a Goo Bomb carrier aims at `seek`.
#[allow(clippy::too_many_arguments)]
fn plan_path(
    w: &Scene,
    brain: &mut AiBrain,
    me: Entity,
    view: &TrackView,
    speed: f32,
    seek: Option<(Entity, f32)>,
    obstacles: &mut Vec<Obstacle>,
    alongside: &mut Vec<f32>,
) -> Path {
    let tuning = w.tuning;
    let drifting = brain.drift.is_some();
    let edge = tuning.driving.edge_margin_u
        + if drifting {
            charge::DRIFT_EDGE_EXTRA_U
        } else {
            0.0
        };
    let bounds = |f: &TrackFrame| (f.half_width - edge).max(0.0);
    let planned = |s: f32| w.track.racing_line_offset_at(s) + brain.line_offset;
    let narrowest = narrowest_ahead(w.track, view.s, tuning.traffic.look_ahead_u, edge);
    w.obstacles(me, view, seek.map(|s| s.0), obstacles, alongside);
    let gap = 2.0 * CAR_HALF_WIDTH_U + ALONGSIDE_CLEARANCE_U;
    let (lo, hi) = side_bounds(view.lateral, (-narrowest, narrowest), alongside, gap);
    let here = planned(view.s).clamp(lo, hi);
    let lane = match seek {
        Some((_, lateral)) => LaneChoice {
            lateral: lateral.clamp(lo, hi),
            follow_speed: None,
            dodge: false,
        },
        None => choose_lane(
            here,
            view.lateral,
            CAR_HALF_WIDTH_U,
            speed,
            (lo, hi),
            obstacles,
        ),
    };
    // Well off its line (a slide, a shove, the open lawn), the car's own path back may cross a
    // prop or hazard the line does not: clear those first.
    let lane = match (view.lateral - here).abs() > OFF_LINE_U {
        true => {
            let own = choose_lane(
                view.lateral,
                view.lateral,
                CAR_HALF_WIDTH_U,
                speed,
                (-f32::MAX, f32::MAX),
                obstacles,
            );
            match own.dodge {
                true => LaneChoice {
                    lateral: own.lateral.clamp(lo.min(own.lateral), hi.max(own.lateral)),
                    ..own
                },
                false => lane,
            }
        }
        false => lane,
    };
    let d = &tuning.driving;
    let ld = lookahead(
        speed,
        d.lookahead_secs,
        d.lookahead_min_u,
        d.lookahead_max_u,
    );
    let max_lane = match (near_jump(w.track, view.s, ld), lane.dodge) {
        (true, true) => JUMP_DODGE_U,
        (true, false) => JUMP_LANE_U,
        (false, _) => f32::MAX,
    };
    let step = tuning.driving.lane_change_ups * SIM_DT;
    let want = (lane.lateral - here).clamp(-max_lane, max_lane);
    brain.lane = match lane.dodge {
        true => want,
        false => brain.lane + (want - brain.lane).clamp(-step, step),
    };
    let s_t = view.s + ld;
    let kappa_t = brain.profile.kappa_at(s_t);
    let wide = match brain.mistake {
        Some((Mistake::WideLine(u), _)) if kappa_t.abs() > 1.0 / WIDE_LINE_RADIUS_U => {
            -kappa_t.signum() * u
        }
        _ => 0.0,
    };
    let frame_t = w.track.frame_at(s_t);
    let bt = bounds(&frame_t);
    let (lo_t, hi_t) = (lo.max(-bt), hi.min(bt));
    let lateral_t = (planned(s_t) + brain.lane + wide).clamp(lo_t, hi_t.max(lo_t));
    Path {
        target: frame_t.point(lateral_t, 0.0),
        planned: here,
        lane,
        narrowest,
    }
}

/// Countdown: the throttle goes down at the planned tick (perfect start or a late reaction).
fn countdown(brain: &mut AiBrain) -> CarControl {
    brain.countdown.0 += 1;
    CarControl {
        throttle: if brain.countdown.0 >= brain.countdown.1 {
            1.0
        } else {
            0.0
        },
        ..default()
    }
}
