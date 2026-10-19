//! Reset & respawn decisions (`SimSet::Race`, Racing; GDD §5.4). rc_race is the sole decider:
//! it sends `RespawnCar` (rc_vehicle teleports) plus 1.5 s of ghosting (`ApplyEffect`, rc_items).
//!
//! | Trigger | Respawn |
//! |---|---|
//! | Reset tapped (released before 0.4 s) while `up_dot` < 0.3 | flip in place |
//! | Reset held 0.4 s | to the track |
//! | below the kill height | to the track |
//! | off-track > 3 s (> 2 s with steering assist) | to the track |
//! | stuck: < 1 u/s with throttle held for 4 s | to the track |
//!
//! At most one respawn per 2 s: a trigger during the cooldown waits for it to end (a tap is
//! dropped). To the track = `Track::respawn_before` (never past the next gate still to pass).

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::progress::Tracker;

/// Holding Reset this long respawns (ticks, 0.4 s).
const HOLD_TICKS: u32 = 24;
/// Minimum time between two respawns of a car (ticks, 2 s).
const COOLDOWN_TICKS: u32 = 120;
/// Off-track longer than this respawns (ticks, 3 s)…
const OFF_TRACK_TICKS: u32 = 180;
/// …or than this with steering assist (ticks, 2 s, GDD §15).
const OFF_TRACK_ASSIST_TICKS: u32 = 120;
/// Stuck this long respawns (ticks, 4 s).
const STUCK_TICKS: u32 = 240;
/// Stuck this long shows the "Hold Reset" hint (ticks, 3 s, GDD §13).
pub(crate) const STUCK_HINT_TICKS: u32 = 180;
/// Stuck: slower than this (u/s)…
const STUCK_SPEED_UPS: f32 = 1.0;
/// …with at least this much throttle.
const STUCK_THROTTLE: f32 = 0.5;
/// On its side or roof below this `up_dot`.
const UPSIDE_DOWN_UP_DOT: f32 = 0.3;
/// A flipped car is put back this much higher (u).
const FLIP_LIFT_U: f32 = 0.5;
/// More than this below the surface it follows counts as off the track (fell off, u).
pub(crate) const OFF_TRACK_DEPTH_U: f32 = 3.0;
/// Ghosting after every respawn (GDD §5.4).
const RESPAWN_GHOST: Effect = Effect::Ghost { secs: 1.5 };

/// What the rules see of a car this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Signals {
    pub reset_held: bool,
    pub upside_down: bool,
    pub below_kill: bool,
    pub off_track: bool,
    pub stuck: bool,
    /// Steering assist shortens the off-track limit.
    pub assist: bool,
}

/// Per-car reset button and auto-respawn timers.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Recovery {
    /// Ticks the reset button has been held (0 = released).
    held: u32,
    /// This press already respawned the car.
    hold_fired: bool,
    /// Ticks off the track.
    pub off_track: u32,
    /// Ticks stuck.
    pub stuck: u32,
    /// Ticks until the next respawn may happen.
    cooldown: u32,
    /// Why this tick's respawn happened by itself (`None`: none, or the player asked).
    pub auto: Option<RespawnCause>,
}

impl Recovery {
    /// One tick of the decision table; returns the respawn to perform now.
    pub(crate) fn step(&mut self, s: Signals) -> Option<RespawnKind> {
        self.cooldown = self.cooldown.saturating_sub(1);
        self.off_track = if s.off_track { self.off_track + 1 } else { 0 };
        self.stuck = if s.stuck { self.stuck + 1 } else { 0 };
        let ready = self.cooldown == 0;
        let mut kind = None;
        if s.reset_held {
            self.held += 1;
            if self.held >= HOLD_TICKS && !self.hold_fired && ready {
                self.hold_fired = true;
                kind = Some(RespawnKind::ToTrack);
            }
        } else {
            let tap = (1..HOLD_TICKS).contains(&self.held) && !self.hold_fired;
            if tap && s.upside_down && ready {
                kind = Some(RespawnKind::FlipInPlace);
            }
            (self.held, self.hold_fired) = (0, false);
        }
        let cause = if s.below_kill {
            Some(RespawnCause::Fell)
        } else if self.off_track > off_track_limit(s.assist) {
            Some(RespawnCause::OffTrack)
        } else if self.stuck >= STUCK_TICKS {
            Some(RespawnCause::Stuck)
        } else {
            None
        };
        self.auto = None;
        if let (Some(cause), true) = (cause, ready) {
            self.auto = kind.is_none().then_some(cause);
            kind = Some(RespawnKind::ToTrack);
        }
        if kind.is_some() {
            (self.cooldown, self.off_track, self.stuck) = (COOLDOWN_TICKS, 0, 0);
        }
        kind
    }
}

/// Ticks off the track before the automatic respawn.
fn off_track_limit(assist: bool) -> u32 {
    if assist {
        OFF_TRACK_ASSIST_TICKS
    } else {
        OFF_TRACK_TICKS
    }
}

/// Upright at the car's position, raised, keeping its heading (`fallback` when it points straight
/// up or down).
pub(crate) fn flip_pose(car: &Transform, fallback: Vec3) -> Transform {
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).try_normalize();
    let heading = flat(*car.forward())
        .or_else(|| flat(fallback))
        .unwrap_or(Vec3::NEG_Z);
    // Yaw that turns −Z (Bevy forward) onto the heading.
    let yaw = (-heading.x).atan2(-heading.z);
    Transform::from_translation(car.translation + Vec3::Y * FLIP_LIFT_U)
        .with_rotation(Quat::from_rotation_y(yaw))
}

type RecoveryItem<'a> = (
    Entity,
    &'a Transform,
    (&'a CarControl, &'a VehicleTelemetry, &'a Tracker),
    (&'a mut RaceProgress, &'a mut Recovery),
    (Has<LocalPlayer>, Has<AiDriver>),
);

/// `SimSet::Race` (Racing): reset button and auto-respawn decisions.
pub(crate) fn decide_respawns(
    track: Option<Res<Track>>,
    settings: Option<Res<Settings>>,
    mut racers: Query<RecoveryItem>,
    mut respawns: MessageWriter<RespawnCar>,
    mut effects: MessageWriter<ApplyEffect>,
    mut events: MessageWriter<RaceEvent>,
) {
    let Some(track) = track else {
        return;
    };
    let assist = settings.is_some_and(|s| s.accessibility.steering_assist);
    for (car, transform, (control, telemetry, tracker), (mut progress, mut recovery), who) in
        &mut racers
    {
        let Some(s) = tracker.s else {
            continue;
        };
        let frame = track.frame_at(s);
        let outside = tracker.lateral.abs() > frame.half_width + frame.offroad_margin
            || tracker.height < -OFF_TRACK_DEPTH_U;
        let signals = Signals {
            reset_held: control.reset_held,
            upside_down: telemetry.up_dot < UPSIDE_DOWN_UP_DOT,
            below_kill: transform.translation.y < track.kill_height(),
            off_track: telemetry.grounded_wheels == 0 || outside,
            stuck: telemetry.speed < STUCK_SPEED_UPS && control.throttle >= STUCK_THROTTLE,
            assist: assist && who == (true, false),
        };
        let decision = recovery.step(signals);
        let off_time = ticks_to_secs(recovery.off_track);
        if progress.off_track_time != off_time {
            progress.off_track_time = off_time;
        }
        // Only leaving the corridor counts down on the HUD (a jump's air time would flicker it).
        let respawn_in = (outside && recovery.off_track > 0).then(|| {
            ticks_to_secs(off_track_limit(signals.assist).saturating_sub(recovery.off_track))
        });
        if progress.respawn_in != respawn_in {
            progress.respawn_in = respawn_in;
        }
        if let Some(cause) = recovery.auto {
            events.write(RaceEvent {
                racer: Some(car),
                kind: RaceEventKind::AutoRespawn(cause),
            });
        }
        let Some(kind) = decision else {
            continue;
        };
        let pose = match kind {
            RespawnKind::FlipInPlace => flip_pose(transform, frame.forward),
            RespawnKind::ToTrack => track.respawn_before(s, progress.last_valid_checkpoint),
        };
        debug!("rc_race: respawn {car} ({kind:?})");
        respawns.write(RespawnCar {
            car,
            transform: pose,
            kind,
        });
        effects.write(ApplyEffect {
            car,
            effect: RESPAWN_GHOST,
            source: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(r: &mut Recovery, s: Signals, ticks: u32) -> Vec<(u32, RespawnKind)> {
        (0..ticks)
            .filter_map(|t| r.step(s).map(|k| (t, k)))
            .collect()
    }

    const HELD: Signals = Signals {
        reset_held: true,
        upside_down: false,
        below_kill: false,
        off_track: false,
        stuck: false,
        assist: false,
    };
    const IDLE: Signals = Signals {
        reset_held: false,
        ..HELD
    };

    #[test]
    fn a_tap_flips_only_upside_down_cars() {
        let mut r = Recovery::default();
        run(&mut r, HELD, 5);
        assert_eq!(r.step(IDLE), None, "upright: a tap does nothing");
        let upside_down = Signals {
            upside_down: true,
            ..IDLE
        };
        run(&mut r, HELD, 5);
        assert_eq!(r.step(upside_down), Some(RespawnKind::FlipInPlace));
    }

    #[test]
    fn holding_respawns_once_after_point_four_seconds() {
        let mut r = Recovery::default();
        let mut causes = Vec::new();
        for _ in 0..30 {
            r.step(HELD);
            causes.extend(r.auto);
        }
        assert!(causes.is_empty(), "the player asked: no automatic cause");
        let mut r = Recovery::default();
        assert_eq!(run(&mut r, HELD, 200), [(23, RespawnKind::ToTrack)]);
        let released = Signals {
            upside_down: true,
            ..IDLE
        };
        assert_eq!(r.step(released), None, "releasing a hold is not a tap");
    }

    #[test]
    fn the_cooldown_delays_triggers_and_drops_taps() {
        let mut r = Recovery::default();
        let kill = Signals {
            below_kill: true,
            ..IDLE
        };
        assert_eq!(r.step(kill), Some(RespawnKind::ToTrack));
        assert_eq!(r.auto, Some(RespawnCause::Fell));
        // Still below the kill height (the teleport is pending): waits for the 2 s cooldown.
        assert_eq!(run(&mut r, kill, 200), [(119, RespawnKind::ToTrack)]);
        run(&mut r, HELD, 5);
        let tap = Signals {
            upside_down: true,
            ..IDLE
        };
        assert_eq!(r.step(tap), None, "a tap during the cooldown is dropped");
        // A hold during the cooldown fires as soon as it ends.
        let fired = run(&mut r, HELD, 200);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].1, RespawnKind::ToTrack);
    }

    #[test]
    fn off_track_and_stuck_timers() {
        let off = Signals {
            off_track: true,
            ..IDLE
        };
        let mut r = Recovery::default();
        assert_eq!(run(&mut r, off, 181)[0].0, 180, "> 3 s");
        assert_eq!(r.auto, Some(RespawnCause::OffTrack));
        let assisted = Signals {
            assist: true,
            ..off
        };
        assert_eq!(
            run(&mut Recovery::default(), assisted, 400)[0].0,
            120,
            "> 2 s"
        );
        let stuck = Signals {
            stuck: true,
            ..IDLE
        };
        let mut r = Recovery::default();
        assert_eq!(run(&mut r, stuck, 239), []);
        assert_eq!(r.stuck, 239);
        assert_eq!(r.step(stuck), Some(RespawnKind::ToTrack), "4 s");
        assert_eq!(r.auto, Some(RespawnCause::Stuck));
        assert_eq!(r.stuck, 0, "timers restart after a respawn");
        r.step(IDLE);
        assert_eq!(r.auto, None, "only the respawn's tick carries a cause");
        let mut r = Recovery::default();
        run(&mut r, off, 150);
        r.step(IDLE);
        assert_eq!(r.off_track, 0, "back on track resets the timer");
    }

    #[test]
    fn flips_keep_the_heading_and_stand_upright() {
        let heading = Vec3::new(1.0, 0.0, 1.0).normalize();
        let on_roof = Transform::from_xyz(3.0, 1.0, -2.0).looking_to(heading, Vec3::NEG_Y);
        let flipped = flip_pose(&on_roof, Vec3::NEG_Z);
        assert!((flipped.translation - Vec3::new(3.0, 1.5, -2.0)).length() < 1e-5);
        assert!(flipped.up().dot(Vec3::Y) > 0.9999, "upright");
        assert!(flipped.forward().dot(heading) > 0.9999, "same heading");
        let nose_down = Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::NEG_Y, Vec3::X);
        let flipped = flip_pose(&nose_down, Vec3::X);
        assert!(
            flipped.forward().dot(Vec3::X) > 0.9999,
            "falls back to the track"
        );
    }
}
