//! Progress (`SimSet::Race`): track projection with continuity hints, ordered checkpoint gates,
//! laps in ticks, wrong way and distances (ranks: `standings.rs`).
//!
//! - A racer is on lap 0 until it first crosses the start line (the grid is behind it), so
//!   `total_distance` ≤ 0 there and grows continuously across the line.
//! - A checkpoint counts when the projected `s` passes it **forward**, in order, with the car
//!   inside its gate (`position ± (half width + off-road margin)`, near the gate plane and the
//!   surface). A missed gate freezes the racer's distance at it until it is passed properly;
//!   passing its `s` outside the gate sends `MissedCheckpoint` once per pass (re-armed when the
//!   car is back behind the gate, so a car jittering on the gate line reports it once).
//! - Lap 1 is timed from GO (the run-up from the grid is part of it), so the lap times add up to
//!   the race time.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{Track, TrackFrame, TrackPos};

use crate::ghost::GhostRun;
use crate::lost::{Lost, REJOIN_AHEAD_U};
use crate::recovery::OFF_TRACK_DEPTH_U;
use crate::spawn::autopilot;

/// A crossing counts only this close to the gate plane along the track (u): a car moves < 1 u
/// per tick, so this rejects projection jumps.
const GATE_DEPTH_U: f32 = 4.0;
/// …at most this far above the surface (u): generous, so cars flying over a gate after a jump
/// (boosted cars reach ~5 u) still count — the hinted projection already keeps them on this
/// stretch of track…
const GATE_ABOVE_U: f32 = 12.0;
/// …and at most this far below it (u): never from a lower level passing under the gate.
const GATE_BELOW_U: f32 = 3.0;
/// Wrong way: moving against the track faster than this (u/s, GDD §7.3)…
const WRONG_WAY_SPEED_UPS: f32 = 5.0;
/// …for this long (ticks, 2 s) turns the warning on…
const WRONG_WAY_TICKS: u32 = 120;
/// …and moving forward along the track for this long (ticks, 0.5 s) turns it off.
const RIGHT_WAY_TICKS: u32 = 30;
/// Back in the corridor after a trip outside it, the gates passed out there count if the car
/// drove at least this share of the track it gained: running wide drives more than the
/// centreline, cutting an infield far less.
const CUT_SHARE: f32 = 0.8;

/// Wrong-way warning with hysteresis.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct WrongWay {
    on: bool,
    ticks: u32,
}

impl WrongWay {
    /// One tick at `track_speed` (u/s along the travel direction); returns the new state when it
    /// flips.
    pub(crate) fn step(&mut self, track_speed: f32) -> Option<bool> {
        let (toward, needed) = if self.on {
            (track_speed > 0.0, RIGHT_WAY_TICKS)
        } else {
            (track_speed < -WRONG_WAY_SPEED_UPS, WRONG_WAY_TICKS)
        };
        self.ticks = if toward { self.ticks + 1 } else { 0 };
        if self.ticks < needed {
            return None;
        }
        self.on = !self.on;
        self.ticks = 0;
        Some(self.on)
    }
}

/// rc_race's private per-racer bookkeeping.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct Tracker {
    /// Projected `s` of the latest tick (the next projection's hint); `None` before the first
    /// projection.
    pub s: Option<f32>,
    /// Lateral offset of the latest projection (u, + = right).
    pub lateral: f32,
    /// Height above the surface of the latest projection (u).
    pub height: f32,
    /// Race tick the current lap started.
    pub lap_start: u32,
    /// Race tick of the finish.
    pub finish: Option<u32>,
    /// Checkpoints validated so far (index into the ghost's splits).
    pub crossings: usize,
    /// Wrong-way warning.
    pub wrong_way: WrongWay,
    /// Lost warning.
    pub lost: Lost,
    /// Position of the latest tick (the lost rule's speed).
    pub pos: Option<Vec3>,
    /// Outside the legal corridor since arc length `.0`, having driven `.1` u out there.
    pub excursion: Option<(f32, f32)>,
    /// The gate whose missed pass was reported; re-armed once the car is back behind it (so a
    /// car jittering on the gate line outside the gate reports it once).
    pub missed: Option<usize>,
}

/// What a tick of motion did (events for the HUD, audio and the ghost splits).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Happening {
    /// The `ordinal`-th checkpoint crossing of the race, at fractional race tick `time`.
    Checkpoint { ordinal: usize, time: f32 },
    /// A lap was completed.
    Lap { lap: u8, time: u32, best: bool },
    /// The final lap started.
    FinalLap,
    /// The wrong-way warning turned on or off.
    WrongWay(bool),
    /// The next gate's `s` was passed outside the gate.
    MissedCheckpoint,
}

/// The world as one racer sees it this tick.
pub(crate) struct Tick<'a> {
    pub track: &'a Track,
    pub pos: Vec3,
    /// Teleported since the last tick: project globally, no crossings, no speed.
    pub teleported: bool,
    /// The race is on (`RacePhase::Racing`); otherwise only the position is tracked.
    pub racing: bool,
    /// Race tick (1 = the first tick after GO).
    pub now: u32,
    /// Laps of the race (≥ 1).
    pub laps: u8,
}

/// Fraction of the move from `s0` (`d` u forward) at which the checkpoint at `c` is passed, if it
/// is passed forward in this move.
pub(crate) fn crossed(c: f32, s0: f32, d: f32, length: f32) -> Option<f32> {
    let ahead = (c - s0).rem_euclid(length);
    (d > 0.0 && ahead > 0.0 && ahead <= d).then(|| ahead / d)
}

/// Taking another part of the track for the car's position needs it this close above that part's
/// surface (u): a car, not a helix level or a bridge above…
const REJOIN_ABOVE_U: f32 = 2.0;
/// …and, while the hinted point is still inside its corridor, that part this much closer (u).
const REJOIN_CLOSER_U: f32 = 3.0;

/// `tp` lies outside the legal corridor (beside it, or fallen below it).
fn outside_corridor(track: &Track, tp: &TrackPos) -> bool {
    let f = track.frame_at(tp.s);
    tp.lateral.abs() > f.half_width + f.offroad_margin || tp.height < -OFF_TRACK_DEPTH_U
}

/// The car's track position: the closest point near the previous one (`hint`) — unless the car
/// is on another part of the track (inside its corridor, on its surface) that is clearly closer,
/// or the hinted point leaves it outside the corridor while that part does not. Off the track the
/// nearest point can slide onto a neighbouring part (a hairpin's other leg) and the hinted search
/// would never find the way back; a jump forward gains nothing (the gates in between are missed:
/// driven through outside them, or a cut).
fn rejoined(track: &Track, pos: Vec3, hint: Option<f32>) -> TrackPos {
    let tp = track.project(pos, hint);
    let outside = outside_corridor(track, &tp);
    if hint.is_none() || (!outside && tp.lateral.abs() <= REJOIN_CLOSER_U) {
        return tp;
    }
    let global = track.project(pos, None);
    let on_it = (-OFF_TRACK_DEPTH_U..=REJOIN_ABOVE_U).contains(&global.height)
        && !outside_corridor(track, &global);
    let closer = global.lateral.abs() + REJOIN_CLOSER_U < tp.lateral.abs();
    if on_it && (outside || closer) {
        global
    } else {
        tp
    }
}

/// `pos` is inside checkpoint gate `c`: across the legal corridor, near its plane and surface.
pub(crate) fn in_gate(c: &TrackFrame, pos: Vec3) -> bool {
    let d = pos - c.position;
    d.dot(c.right).abs() <= c.half_width + c.offroad_margin
        && d.dot(c.forward).abs() <= GATE_DEPTH_U
        && (-GATE_BELOW_U..=GATE_ABOVE_U).contains(&d.dot(c.up))
}

impl Tracker {
    /// One tick of a racer: projection, then (racing, unfinished) wrong way, checkpoints, laps
    /// and the finish, then the distances.
    pub(crate) fn step(
        &mut self,
        progress: &mut RaceProgress,
        tick: &Tick,
        mut emit: impl FnMut(Happening),
    ) {
        let track = tick.track;
        let hint = self.s.filter(|_| !tick.teleported);
        let tp = rejoined(track, tick.pos, hint);
        (self.s, self.lateral, self.height) = (Some(tp.s), tp.lateral, tp.height);
        let last_pos = self.pos.replace(tick.pos).filter(|_| hint.is_some());
        if progress.finished {
            return;
        }
        progress.rejoin = track.frame_at(tp.s + REJOIN_AHEAD_U).position;
        let cps = track.checkpoints();
        let length = track.length();
        if !tick.racing || hint.is_none() {
            self.lost = Lost::default();
            progress.lost = false;
            self.excursion = None;
        }
        if let (true, Some(s0)) = (tick.racing, hint) {
            let d = track.delta_s(s0, tp.s);
            if let Some(on) = self.wrong_way.step(d / SIM_DT) {
                progress.wrong_way = on;
                emit(Happening::WrongWay(on));
            }
            let outside = outside_corridor(track, &tp);
            let speed = last_pos.map_or(0.0, |p| p.distance(tick.pos) / SIM_DT);
            if progress.wrong_way {
                self.lost = Lost::default();
            }
            progress.lost = !progress.wrong_way && self.lost.step(outside, speed, d / SIM_DT);
            if self
                .missed
                .is_some_and(|m| track.delta_s(tp.s, cps[m].s) > GATE_DEPTH_U)
            {
                self.missed = None;
            }
            if outside {
                let trip = self.excursion.get_or_insert((s0, 0.0));
                trip.1 += last_pos.map_or(0.0, |p| p.distance(tick.pos));
            }
            self.pass_gates(progress, tick, (s0, d), false, &mut emit);
            let trip = if outside { None } else { self.excursion.take() };
            if let (Some((left_at, driven)), Some(m)) = (trip, self.missed) {
                // Back on the track after a trip past the missed gate: running round the outside
                // keeps the gates passed out there; a cut does not, and only now is there a way
                // to "go back" to.
                let gained = track.delta_s(left_at, tp.s);
                if crossed(cps[m].s, left_at, gained, length).is_some() {
                    if driven >= CUT_SHARE * gained {
                        self.missed = None;
                        self.pass_gates(progress, tick, (left_at, gained), true, &mut emit);
                    } else {
                        emit(Happening::MissedCheckpoint);
                    }
                }
            }
        }
        // Along the lap from the start line: never past the next gate still to pass.
        let along = if progress.finished {
            length
        } else {
            let last = cps[progress.last_valid_checkpoint].s;
            let upper = match progress.next_checkpoint {
                0 => length,
                next => cps[next].s,
            };
            (last + track.delta_s(last, tp.s)).min(upper)
        };
        progress.distance = if progress.lap == 0 {
            along - length
        } else {
            along
        };
        progress.total_distance = f32::from(progress.lap.max(1) - 1) * length + progress.distance;
    }

    /// Validates the gates passed moving `span.1` u forward from arc length `span.0`, in order,
    /// up to the first one passed outside its gate (missed). `lenient`: no gate check — a trip
    /// outside the corridor being judged when it ends.
    fn pass_gates(
        &mut self,
        progress: &mut RaceProgress,
        tick: &Tick,
        (from, d): (f32, f32),
        lenient: bool,
        emit: &mut impl FnMut(Happening),
    ) {
        let cps = tick.track.checkpoints();
        while !progress.finished {
            let next = (progress.last_valid_checkpoint + 1) % cps.len();
            let Some(frac) = crossed(cps[next].s, from, d, tick.track.length()) else {
                break;
            };
            if !lenient && !in_gate(&cps[next], tick.pos) {
                // Inside the corridor (above or below the gate) it is missed now; outside it, when
                // the trip ends.
                if self.missed.replace(next) != Some(next) && self.excursion.is_none() {
                    emit(Happening::MissedCheckpoint);
                }
                break;
            }
            self.missed = None;
            progress.last_valid_checkpoint = next;
            progress.next_checkpoint = (next + 1) % cps.len();
            let time = tick.now.saturating_sub(1) as f32 + frac;
            emit(Happening::Checkpoint {
                ordinal: self.crossings,
                time,
            });
            self.crossings += 1;
            if next == 0 && self.cross_line(progress, tick, emit) {
                break;
            }
        }
    }

    /// The start line was validated: lap 1 starts, or a lap is complete (and maybe the race).
    /// Returns `true` when the racer finished.
    fn cross_line(
        &mut self,
        progress: &mut RaceProgress,
        tick: &Tick,
        emit: &mut impl FnMut(Happening),
    ) -> bool {
        if progress.lap == 0 {
            progress.lap = 1;
            if tick.laps == 1 {
                emit(Happening::FinalLap);
            }
            return false;
        }
        let time = tick.now - self.lap_start;
        self.lap_start = tick.now;
        let best = progress.best_lap.is_some_and(|b| time < b);
        progress.best_lap = Some(progress.best_lap.map_or(time, |b| b.min(time)));
        progress.lap_times.push(time);
        emit(Happening::Lap {
            lap: progress.lap,
            time,
            best,
        });
        if progress.lap >= tick.laps {
            progress.finished = true;
            progress.lost = false;
            self.finish = Some(tick.now);
            return true;
        }
        progress.lap += 1;
        if progress.lap == tick.laps {
            emit(Happening::FinalLap);
        }
        false
    }
}

type ProgressItem<'a> = (
    Entity,
    &'a Transform,
    &'a mut RaceProgress,
    &'a mut Tracker,
    Has<LocalPlayer>,
    Has<AiDriver>,
);

/// `SimSet::Race` (Intro, Countdown, Racing): every racer's progress and its events. A human
/// who finishes is handed to the AI (Re-Volt style), so the car keeps driving its cool-down lap
/// while the player spectates or waits.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_progress(
    track: Option<Res<Track>>,
    (status, phase, config): (
        Res<RaceStatus>,
        Res<State<RacePhase>>,
        Option<Res<RaceConfig>>,
    ),
    mut teleported: MessageReader<Teleported>,
    mut racers: Query<ProgressItem>,
    ghost: Option<Single<&GhostRun>>,
    mut jumped: Local<Vec<Entity>>,
    mut events: MessageWriter<RaceEvent>,
    mut commands: Commands,
) {
    let Some(track) = track else {
        warn_once!("rc_race: no Track in a race; progress is not tracked");
        return;
    };
    jumped.clear();
    jumped.extend(teleported.read().map(|t| t.entity));
    let time_trial = config.is_some_and(|c| c.mode == GameMode::TimeTrial);
    let splits = ghost.as_deref().map_or(&[][..], |g| &g.splits[..]);
    for (car, transform, mut progress, mut tracker, human, driven) in &mut racers {
        let was_finished = progress.finished;
        let tick = Tick {
            track: &track,
            pos: transform.translation,
            teleported: jumped.contains(&car),
            racing: *phase.get() == RacePhase::Racing,
            now: status.race_ticks,
            laps: status.total_laps.max(1),
        };
        let mut send = |kind| {
            events.write(RaceEvent {
                racer: Some(car),
                kind,
            });
        };
        tracker.step(&mut progress, &tick, |happening| match happening {
            Happening::Checkpoint { ordinal, time } if time_trial && human => {
                if let Some(ghost_time) = splits.get(ordinal) {
                    send(RaceEventKind::CheckpointSplit((time - ghost_time) * SIM_DT));
                }
            }
            Happening::Lap { lap, time, best } => {
                send(RaceEventKind::LapCompleted { lap, time, best });
            }
            Happening::FinalLap => send(RaceEventKind::FinalLap),
            Happening::WrongWay(on) => send(RaceEventKind::WrongWay(on)),
            Happening::MissedCheckpoint => send(RaceEventKind::MissedCheckpoint),
            Happening::Checkpoint { .. } => {}
        });
        if human && !driven && progress.finished && !was_finished {
            info!("rc_race: the AI drives the finished human's car");
            commands.entity(car).insert(autopilot());
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn gate_counts_cars_flying_over_it_but_not_from_below() {
        let gate = TrackFrame {
            s: 0.0,
            position: Vec3::ZERO,
            forward: Vec3::NEG_Z,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 5.0,
            bank: 0.0,
            surface: SurfaceKind::Asphalt,
            offroad_margin: 6.0,
            in_gap: false,
        };
        assert!(in_gate(&gate, Vec3::new(1.0, 0.5, 0.0)), "on the surface");
        assert!(
            in_gate(&gate, Vec3::new(1.0, 6.0, 0.0)),
            "boosted jump over the gate"
        );
        assert!(!in_gate(&gate, Vec3::new(1.0, 15.0, 0.0)), "far above");
        assert!(
            !in_gate(&gate, Vec3::new(1.0, -4.0, 0.0)),
            "lower level under the gate"
        );
    }

    pub(crate) fn track(reverse: bool) -> Track {
        let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let variant = TrackVariant {
            reverse,
            ..TrackVariant::forward("test_oval")
        };
        Track::load(&assets, &variant).expect("test_oval loads")
    }

    /// A car driving the centreline from grid slot 0 at `speed` u/s.
    struct Drive<'a> {
        track: &'a Track,
        progress: RaceProgress,
        tracker: Tracker,
        s: f32,
        now: u32,
        events: Vec<Happening>,
    }

    impl<'a> Drive<'a> {
        fn new(track: &'a Track) -> Self {
            let mut d = Drive {
                track,
                progress: RaceProgress {
                    last_valid_checkpoint: track.checkpoints().len() - 1,
                    ..default()
                },
                tracker: Tracker::default(),
                s: track.project(track.grid()[0].translation, None).s,
                now: 0,
                events: Vec::new(),
            };
            d.tick_at(d.s, 0.0, false);
            d
        }

        fn tick_at(&mut self, s: f32, lateral: f32, racing: bool) {
            let pos = self.track.frame_at(s).point(lateral, 0.3);
            let tick = Tick {
                track: self.track,
                pos,
                teleported: false,
                racing,
                now: self.now,
                laps: 2,
            };
            let events = &mut self.events;
            self.tracker
                .step(&mut self.progress, &tick, |h| events.push(h));
        }

        /// Drives `secs` at `speed` (negative = backwards) with a lateral offset.
        fn drive(&mut self, secs: f32, speed: f32, lateral: f32) {
            for _ in 0..secs_to_ticks(secs) {
                self.now += 1;
                self.s = self.track.wrap(self.s + speed * SIM_DT);
                self.tick_at(self.s, lateral, true);
            }
        }

        fn laps(&self) -> usize {
            let lap = |h: &&Happening| matches!(h, Happening::Lap { .. });
            self.events.iter().filter(lap).count()
        }

        fn missed(&self) -> usize {
            let missed = |h: &&Happening| **h == Happening::MissedCheckpoint;
            self.events.iter().filter(missed).count()
        }
    }

    #[test]
    fn grid_is_behind_the_line_and_distance_is_continuous_across_it() {
        for reverse in [false, true] {
            let track = track(reverse);
            let mut car = Drive::new(&track);
            assert_eq!(car.progress.lap, 0);
            assert!(car.progress.total_distance < 0.0, "grid total ≤ 0");
            let mut last = car.progress.total_distance;
            for _ in 0..120 {
                car.drive(1.0 / 60.0, 30.0, 0.0);
                let d = car.progress.total_distance;
                assert!((d - last - 0.5).abs() < 0.05, "continuous: {last} → {d}");
                last = d;
            }
            assert_eq!(car.progress.lap, 1, "crossed the line (reverse: {reverse})");
            assert!(car.progress.total_distance > 0.0);
            assert_eq!(car.laps(), 0, "the first crossing starts lap 1");
        }
    }

    #[test]
    fn laps_count_after_all_checkpoints_and_time_from_go() {
        for reverse in [false, true] {
            let track = track(reverse);
            let mut car = Drive::new(&track);
            let lap_secs = track.length() / 30.0;
            car.drive(lap_secs + 1.0, 30.0, 0.0);
            assert_eq!(car.progress.lap, 2, "lap 2 after one full lap");
            assert_eq!(car.laps(), 1);
            assert!(car.events.contains(&Happening::FinalLap));
            let n = track.checkpoints().len();
            assert_eq!(car.tracker.crossings, n + 1, "start line + n gates");
            car.drive(lap_secs, 30.0, 0.0);
            assert!(car.progress.finished);
            let total: u32 = car.progress.lap_times.iter().sum();
            assert_eq!(
                Some(total),
                car.tracker.finish,
                "lap times add up to the race time"
            );
            assert_eq!(car.progress.total_distance, 2.0 * track.length());
            let frozen = car.progress.clone();
            car.drive(2.0, 30.0, 0.0);
            assert_eq!(car.progress, frozen, "finished progress is frozen");
        }
    }

    /// The first corner: its start, span and the lateral sign of its outside (+ = right).
    fn corner(track: &Track) -> (f32, f32, f32) {
        let t = track.turns()[0];
        (t.start_s, t.span, if t.right { -1.0 } else { 1.0 })
    }

    /// Drives `secs` at `speed` along the track, easing the lateral offset from `from` to `to`.
    fn swing(car: &mut Drive, secs: f32, speed: f32, from: f32, to: f32) {
        let n = secs_to_ticks(secs);
        for k in 1..=n {
            car.now += 1;
            car.s = car.track.wrap(car.s + speed * SIM_DT);
            car.tick_at(car.s, from + (to - from) * k as f32 / n as f32, true);
        }
    }

    #[test]
    fn running_wide_past_a_gate_and_back_keeps_the_progress() {
        for reverse in [false, true] {
            let track = track(reverse);
            let mut car = Drive::new(&track);
            let (start, span, out) = corner(&track);
            let gate = track
                .checkpoints()
                .iter()
                .find(|c| track.delta_s(start, c.s) > 0.0 && track.delta_s(start, c.s) < span)
                .copied()
                .expect("a gate in the first corner");
            let wide = out * (gate.half_width + gate.offroad_margin + 4.0);
            car.drive(
                track
                    .delta_s(car.s, start - 10.0)
                    .rem_euclid(track.length())
                    / 30.0,
                30.0,
                0.0,
            );
            // Out over the open floor round the outside of the corner, past its gate, and back.
            swing(&mut car, 0.5, 20.0, 0.0, wide);
            swing(&mut car, (span + 10.0) / 20.0, 20.0, wide, wide);
            assert!(car.progress.lost, "outside long enough to be warned");
            let before = car.progress.total_distance;
            swing(&mut car, 0.6, 20.0, wide, 0.0);
            car.drive(1.0, 30.0, 0.0);
            assert!(
                car.progress.total_distance > before + 30.0,
                "progress goes on (reverse: {reverse}): {before} → {}",
                car.progress.total_distance
            );
            assert_eq!(car.missed(), 0, "nothing to go back for");
        }
    }

    #[test]
    fn cutting_the_infield_past_a_gate_still_misses_it() {
        let track = track(false);
        let mut car = Drive::new(&track);
        let gate = cut_first_corner(&mut car);
        car.drive(0.5, 30.0, 0.0);
        assert_eq!(
            car.progress.next_checkpoint, gate,
            "the gate is still to pass"
        );
        assert_eq!(
            car.missed(),
            1,
            "told to go back once it is on the track again"
        );
    }

    #[test]
    fn a_missed_gate_blocks_the_lap_until_passed() {
        let track = track(false);
        let mut car = Drive::new(&track);
        let gate = cut_first_corner(&mut car);
        assert_eq!(car.progress.next_checkpoint, gate, "gate missed");
        let frozen = car.progress.total_distance;
        car.drive(1.0, 30.0, 0.0);
        assert_eq!(
            car.progress.total_distance, frozen,
            "no distance past a missed gate"
        );
        // A full extra lap later the car passes it properly.
        car.drive(track.length() / 30.0, 30.0, 0.0);
        assert!(car.progress.next_checkpoint != gate || car.progress.lap > 1);
        assert_eq!(car.laps(), 0, "the first lap still needs the missed gate");
        assert_eq!(car.missed(), 1, "one notice for the one missed pass");
    }

    #[test]
    fn a_missed_gate_is_reported_once_per_pass() {
        let track = track(false);
        let mut car = Drive::new(&track);
        let gate = cut_first_corner(&mut car);
        assert_eq!(car.missed(), 1);
        // Another trip off the track ahead of the missed gate: not about it, no new notice.
        let wide = track.frame_at(car.s).half_width + track.frame_at(car.s).offroad_margin + 3.0;
        swing(&mut car, 0.4, 20.0, 0.0, wide);
        swing(&mut car, 0.4, 20.0, wide, 0.0);
        assert_eq!(car.missed(), 1, "one notice per missed pass");
        assert_eq!(car.progress.next_checkpoint, gate, "still to pass");
        // Back behind the gate and through it: validated, no notice.
        let back = track.delta_s(track.checkpoints()[gate].s, car.s) + 6.0;
        car.drive(back / 10.0, -10.0, 0.0);
        car.drive(1.2, 10.0, 0.0);
        assert_eq!(car.missed(), 1);
        assert_ne!(car.progress.next_checkpoint, gate, "the gate counted");
    }

    /// Cuts the first corner's infield past its gate and comes back on the track; returns the
    /// missed gate.
    fn cut_first_corner(car: &mut Drive) -> usize {
        let track = car.track;
        let (start, span, out) = corner(track);
        let gate = (0..track.checkpoints().len())
            .find(|&i| {
                let d = track.delta_s(start, track.checkpoints()[i].s);
                d > 0.0 && d < span
            })
            .expect("a gate in the first corner");
        let c = track.checkpoints()[gate];
        let inside = -out * (c.half_width + c.offroad_margin + 4.0);
        car.drive(
            track.delta_s(car.s, start).rem_euclid(track.length()) / 30.0,
            30.0,
            0.0,
        );
        swing(car, 0.3, 30.0, 0.0, inside);
        swing(car, (span - 12.0) / 30.0, 30.0, inside, inside);
        swing(car, 0.1, 30.0, inside, 0.0);
        gate
    }

    #[test]
    fn driving_backwards_validates_nothing_and_warns() {
        let track = track(false);
        let mut car = Drive::new(&track);
        car.drive(2.0, 30.0, 0.0);
        let validated = car.progress.last_valid_checkpoint;
        let before = car.progress.total_distance;
        car.drive(1.9, -10.0, 0.0);
        assert!(!car.progress.wrong_way, "not before 2 s");
        car.drive(0.2, -10.0, 0.0);
        assert!(car.progress.wrong_way);
        assert!(car.events.contains(&Happening::WrongWay(true)));
        assert!(car.progress.total_distance < before);
        assert_eq!(car.progress.last_valid_checkpoint, validated);
        assert!(!car.progress.lost, "backwards is wrong way, not lost");
        car.drive(0.4, 10.0, 0.0);
        assert!(car.progress.wrong_way, "hysteresis: still on");
        car.drive(0.2, 10.0, 0.0);
        assert!(!car.progress.wrong_way);
    }

    #[test]
    fn leaving_the_corridor_is_lost_and_points_back_ahead() {
        let track = track(false);
        let mut car = Drive::new(&track);
        car.drive(2.0, 30.0, 0.0);
        assert!(!car.progress.lost);
        let f = track.frame_at(car.s);
        let wide = f.half_width + f.offroad_margin + 3.0;
        car.drive(0.4, 30.0, wide);
        assert!(!car.progress.lost, "not before 0.5 s outside");
        car.drive(0.2, 30.0, wide);
        assert!(car.progress.lost);
        let ahead = track.frame_at(car.s + 10.0).position;
        assert!(
            car.progress.rejoin.distance(ahead) < 0.5,
            "the centreline ahead"
        );
        car.drive(0.4, 30.0, 0.0);
        assert!(car.progress.lost, "hysteresis: still on");
        car.drive(0.2, 30.0, 0.0);
        assert!(!car.progress.lost, "found again");
    }

    #[test]
    fn wrong_way_needs_two_seconds_above_five_ups() {
        let mut w = WrongWay::default();
        for _ in 0..200 {
            assert_eq!(w.step(-4.9), None, "slow reversing never warns");
        }
        for _ in 0..119 {
            assert_eq!(w.step(-6.0), None);
        }
        assert_eq!(w.step(-6.0), Some(true));
        assert_eq!(w.step(0.0), None, "standing still keeps the warning");
        for _ in 0..29 {
            assert_eq!(w.step(1.0), None);
        }
        assert_eq!(w.step(1.0), Some(false));
        let mut w = WrongWay::default();
        for _ in 0..100 {
            w.step(-6.0);
        }
        w.step(0.0);
        for _ in 0..100 {
            assert_eq!(w.step(-6.0), None, "an interruption restarts the 2 s");
        }
    }

    #[test]
    fn crossings_wrap_at_the_start_line() {
        let l = 300.0;
        assert_eq!(crossed(0.0, 299.0, 2.0, l), Some(0.5));
        assert_eq!(crossed(0.0, 0.0, 1.0, l), None, "already on the line");
        assert_eq!(crossed(0.0, 299.0, -2.0, l), None, "backwards");
        assert_eq!(crossed(50.0, 49.0, 1.0, l), Some(1.0));
        assert_eq!(crossed(50.0, 48.0, 1.0, l), None);
    }

    /// From every 10 u of every track, both sides: 5 u out past the corridor's edge, 20 u along out
    /// there, 5 u back and 10 u on the track — progress is back on the car's own part of the track.
    #[test]
    fn progress_finds_the_car_again_after_a_trip_off_the_track() {
        let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for id in [
            "toy_town",
            "sofa_circuit",
            "backyard_bash",
            "aisle_rush",
            "wrench_works",
            "maple_street",
        ] {
            let t = Track::load(&assets, &TrackVariant::forward(id)).expect("loads");
            for side in [-1.0_f32, 1.0] {
                let mut start = 0.0;
                while start < t.length() {
                    let (mut hint, mut s) = (Some(start), start);
                    for k in 0..80 {
                        let f = t.frame_at(s);
                        let out = side * (f.half_width + f.offroad_margin + 3.0);
                        let lateral = match k {
                            0..10 => out * k as f32 / 10.0,
                            10..50 => out,
                            50..60 => out * (60 - k) as f32 / 10.0,
                            _ => 0.0,
                        };
                        hint = Some(rejoined(&t, f.point(lateral, 0.3), hint).s);
                        s += 0.5;
                    }
                    let off = t.delta_s(s - 0.5, hint.unwrap_or_default()).abs();
                    assert!(
                        off < 2.0,
                        "{id}: lost {off:.0} u after a trip from s = {start} (side {side})"
                    );
                    start += 10.0;
                }
            }
        }
    }
}
