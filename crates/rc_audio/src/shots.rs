//! One-shots: the shared player ([`Shots`]: positional or centred, a time-based limiter per cue,
//! round-robin variants, a pitch rotation for world sounds — musical cues stay in tune), the
//! [`Pending`] queue for delayed ones, impacts / landings / respawns, race events (local
//! players' HUD cues play centred) and UI cues. Stopped kira instances are pruned periodically
//! (the plugin never does). Every started one-shot and stinger is logged at debug level
//! ([`CueLog`]: cue, bus, volume; at most one line per cue per second).

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::bank::Bank;
use crate::buses::{SfxBus, UiBus};
use crate::output::audio_on;
use crate::spatial::{Ears, REACH_CAR, Reach, Spot, spot};

/// At most this many starts of one cue per [`LIMIT_WINDOW_S`].
const LIMIT_STARTS: usize = 4;
const LIMIT_WINDOW_S: f64 = 0.25;
/// Quieter than this after attenuation is not worth a voice, dB.
const AUDIBLE_DB: f32 = -50.0;
/// Pitch rotation of world one-shots.
const PITCHES: [f64; 5] = [1.0, 0.96, 1.04, 0.98, 1.02];
/// Impacts below this strength are ignored; from `IMPACT_HARD` on they are hard.
const IMPACT_MIN: f32 = 0.05;
const IMPACT_HARD: f32 = 0.45;
/// Quietest soft impact / hard impact / landing, dB (the strongest are 0 dB).
const SOFT_QUIET_DB: f32 = -12.0;
const HARD_QUIET_DB: f32 = -4.0;
const LANDING_QUIET_DB: f32 = -14.0;
/// A flip in place (reset tap) sounds like a small landing, dB.
const FLIP_DB: f32 = -8.0;
/// Soft-lock countdown beeps, dB.
const ENDS_IN_DB: f32 = -6.0;
/// The cue log writes at most one line per cue per this many seconds (repeats are counted).
const LOG_EVERY_S: f64 = 1.0;

const SOFT_IMPACTS: [&str; 3] = ["impact_soft_1", "impact_soft_2", "impact_soft_3"];
const HARD_IMPACTS: [&str; 3] = ["impact_hard_1", "impact_hard_2", "impact_hard_3"];
const LANDINGS: [&str; 2] = ["landing_thump_1", "landing_thump_2"];

/// Impact variants and level for a `CarImpact` strength.
pub(crate) fn impact_cue(strength: f32) -> Option<(&'static [&'static str], f32)> {
    let s = strength.clamp(0.0, 1.0);
    if s < IMPACT_MIN {
        None
    } else if s < IMPACT_HARD {
        Some((&SOFT_IMPACTS, SOFT_QUIET_DB * (1.0 - s / IMPACT_HARD)))
    } else {
        let t = (s - IMPACT_HARD) / (1.0 - IMPACT_HARD);
        Some((&HARD_IMPACTS, HARD_QUIET_DB * (1.0 - t)))
    }
}

/// Landing level for a `CarLanded` strength.
pub(crate) fn landing_db(strength: f32) -> Option<f32> {
    let s = strength.clamp(0.0, 1.0);
    (s >= IMPACT_MIN).then_some(LANDING_QUIET_DB * (1.0 - s))
}

/// Race-event cue: (stem, dB, local players only). Perfect starts sound via the boost edge.
pub(crate) fn race_cue(kind: RaceEventKind) -> Option<(&'static str, f32, bool)> {
    Some(match kind {
        RaceEventKind::CountdownTick(_) => ("countdown_beep", 0.0, false),
        RaceEventKind::Go => ("countdown_go", 0.0, false),
        RaceEventKind::RaceEndsIn(_) => ("countdown_beep", ENDS_IN_DB, false),
        RaceEventKind::LapCompleted { .. } => ("lap_chime", 0.0, true),
        RaceEventKind::FinalLap => ("final_lap_fanfare", 0.0, true),
        RaceEventKind::Finished { .. } => ("finish_horn", 0.0, true),
        RaceEventKind::WrongWay(true) => ("wrong_way_buzzer", 0.0, true),
        RaceEventKind::GoldenBattery => ("golden_battery_jingle", 0.0, true),
        _ => return None,
    })
}

/// Start times per cue (limiter), variant and pitch counters.
#[derive(Resource, Default)]
pub(crate) struct Limiter {
    starts: HashMap<&'static str, VecDeque<f64>>,
    variants: HashMap<&'static str, usize>,
    pitch: usize,
}

/// Debug log of started one-shots and stingers, so every cue a run triggered can be shown from
/// its log (`RUST_LOG=rc_audio=debug`): at most one line per cue per [`LOG_EVERY_S`], counting
/// the starts it stands for.
#[derive(Resource, Default)]
pub(crate) struct CueLog(HashMap<&'static str, (f64, u32)>);

impl CueLog {
    /// Records a start of `cue` at real-time second `now`; `Some(starts since the last line)`
    /// when a line is due.
    fn note(&mut self, cue: &'static str, now: f64) -> Option<u32> {
        let (last, starts) = self.0.entry(cue).or_insert((f64::NEG_INFINITY, 0));
        *starts += 1;
        (now - *last >= LOG_EVERY_S).then(|| {
            *last = now;
            std::mem::take(starts)
        })
    }

    /// Logs a start of `cue` on `bus` at `db` (rate-limited).
    pub(crate) fn started(&mut self, cue: &'static str, bus: &str, db: f32, now: f64) {
        if let Some(starts) = self.note(cue, now) {
            let repeats = if starts > 1 {
                format!(" (×{starts} since the last line)")
            } else {
                String::new()
            };
            debug!("audio cue {cue}: {bus} bus, {db:.1} dB{repeats}");
        }
    }
}

/// Plays SFX and UI one-shots.
#[derive(SystemParam)]
pub(crate) struct Shots<'w> {
    bank: Res<'w, Bank>,
    sfx: Res<'w, AudioChannel<SfxBus>>,
    ui: Res<'w, AudioChannel<UiBus>>,
    ears: Res<'w, Ears>,
    time: Res<'w, Time<Real>>,
    limiter: ResMut<'w, Limiter>,
    log: ResMut<'w, CueLog>,
}

impl Shots<'_> {
    /// Real-time seconds (scheduling).
    pub(crate) fn now(&self) -> f64 {
        self.time.elapsed_secs_f64()
    }

    /// A one-shot at a world position (car reach), or centred and unattenuated for `None`.
    pub(crate) fn play(&mut self, stem: &'static str, db: f32, at: Option<Vec3>) {
        self.play_reach(stem, db, at, REACH_CAR);
    }

    /// [`Self::play`] with an explicit reach.
    pub(crate) fn play_reach(
        &mut self,
        stem: &'static str,
        db: f32,
        at: Option<Vec3>,
        reach: Reach,
    ) {
        let place = at.map_or(Spot::CENTRE, |p| spot(&self.ears.0, p, reach));
        let now = self.now();
        let starts = self.limiter.starts.entry(stem).or_default();
        while starts.front().is_some_and(|&t| now - t > LIMIT_WINDOW_S) {
            starts.pop_front();
        }
        if db + place.db < AUDIBLE_DB || starts.len() >= LIMIT_STARTS {
            return;
        }
        let Some(source) = self.bank.sfx(stem) else {
            return;
        };
        starts.push_back(now);
        self.log.started(stem, "Sfx", db + place.db, now);
        let mut cmd = self.sfx.play(source);
        cmd.with_volume(Decibels(db + place.db))
            .with_panning(place.pan);
        if at.is_some() {
            self.limiter.pitch = (self.limiter.pitch + 1) % PITCHES.len();
            cmd.with_playback_rate(PITCHES[self.limiter.pitch]);
        }
    }

    /// A UI cue (UI bus: never paused, never attenuated).
    pub(crate) fn play_ui(&mut self, stem: &'static str) {
        if let Some(source) = self.bank.sfx(stem) {
            let now = self.now();
            self.log.started(stem, "Ui", 0.0, now);
            self.ui.play(source);
        }
    }

    /// The next of a group of variants (round-robin: never the same twice in a row).
    pub(crate) fn variant(&mut self, stems: &'static [&'static str]) -> &'static str {
        let n = self.limiter.variants.entry(stems[0]).or_default();
        *n = (*n + 1) % stems.len();
        stems[*n]
    }
}

/// One-shots due later: (real-time second, stem, dB, position).
#[derive(Resource, Default)]
pub(crate) struct Pending(Vec<(f64, &'static str, f32, Option<Vec3>)>);

impl Pending {
    /// Plays `stem` at real-time second `at` (see [`Shots::play`]).
    pub(crate) fn push(&mut self, at: f64, stem: &'static str, db: f32, pos: Option<Vec3>) {
        self.0.push((at, stem, db, pos));
    }
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Limiter>()
        .init_resource::<CueLog>()
        .init_resource::<Pending>()
        .add_systems(
            Update,
            (ui_shots, car_shots, race_shots, play_pending)
                .chain()
                .in_set(Step::Shots)
                .run_if(audio_on),
        )
        .add_systems(
            Update,
            prune_stopped
                .in_set(Step::Shots)
                .run_if(audio_on.and_then(on_timer(Duration::from_secs(2)))),
        )
        .add_systems(OnEnter(RacePhase::Setup), |mut p: ResMut<Pending>| {
            p.0.clear()
        })
        .add_systems(OnEnter(AppState::Unloading), |mut p: ResMut<Pending>| {
            p.0.clear()
        });
}

fn ui_shots(mut events: MessageReader<UiSound>, mut shots: Shots) {
    for &UiSound(cue) in events.read() {
        shots.play_ui(cue.stem());
    }
}

fn car_shots(
    mut impacts: MessageReader<CarImpact>,
    mut landings: MessageReader<CarLanded>,
    mut respawns: MessageReader<CarRespawned>,
    cars: Query<&Transform>,
    mut shots: Shots,
) {
    for i in impacts.read() {
        if let Some((group, db)) = impact_cue(i.strength) {
            let stem = shots.variant(group);
            shots.play(stem, db, Some(i.point));
        }
    }
    let at = |car| cars.get(car).ok().map(|t| t.translation);
    for l in landings.read() {
        if let Some(db) = landing_db(l.strength) {
            let stem = shots.variant(&LANDINGS);
            shots.play(stem, db, at(l.car));
        }
    }
    for r in respawns.read() {
        match r.kind {
            RespawnKind::ToTrack => shots.play("respawn_materialise", 0.0, at(r.car)),
            RespawnKind::FlipInPlace => {
                let stem = shots.variant(&LANDINGS);
                shots.play(stem, FLIP_DB, at(r.car));
            }
        }
    }
}

/// HUD cues of local players (or global ones). On one racer's frame, a finish beats the
/// final-lap fanfare, which beats the lap chime.
fn race_shots(
    mut events: MessageReader<RaceEvent>,
    locals: Query<(), With<LocalPlayer>>,
    mut shots: Shots,
) {
    let events: Vec<RaceEvent> = events.read().copied().collect();
    let racers_with = |pred: fn(&RaceEventKind) -> bool| -> HashSet<Option<Entity>> {
        events
            .iter()
            .filter(|e| pred(&e.kind))
            .map(|e| e.racer)
            .collect()
    };
    let finished = racers_with(|k| matches!(k, RaceEventKind::Finished { .. }));
    let final_lap = racers_with(|k| matches!(k, RaceEventKind::FinalLap));
    for e in &events {
        let Some((stem, db, local_only)) = race_cue(e.kind) else {
            continue;
        };
        let superseded = match e.kind {
            RaceEventKind::LapCompleted { .. } => {
                finished.contains(&e.racer) || final_lap.contains(&e.racer)
            }
            RaceEventKind::FinalLap => finished.contains(&e.racer),
            _ => false,
        };
        if superseded || (local_only && e.racer.is_some_and(|r| !locals.contains(r))) {
            continue;
        }
        shots.play(stem, db, None);
    }
}

fn play_pending(mut pending: ResMut<Pending>, mut shots: Shots) {
    let now = shots.now();
    pending.0.retain(|&(at, stem, db, pos)| {
        if at > now {
            return true;
        }
        shots.play(stem, db, pos);
        false
    });
}

/// bevy_kira_audio never removes stopped instances from `Assets<AudioInstance>`.
fn prune_stopped(mut instances: ResMut<Assets<AudioInstance>>) {
    let stopped: Vec<_> = instances
        .iter()
        .filter(|(_, i)| i.state() == PlaybackState::Stopped)
        .map(|(id, _)| id)
        .collect();
    for id in stopped {
        instances.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impacts_are_soft_or_hard_by_strength_and_louder_when_stronger() {
        assert_eq!(impact_cue(0.01), None);
        let (soft, quiet) = impact_cue(0.1).unwrap();
        let (_, louder) = impact_cue(0.4).unwrap();
        assert_eq!(soft, SOFT_IMPACTS);
        assert!(quiet < louder);
        let (hard, db) = impact_cue(1.0).unwrap();
        assert_eq!((hard, db), (&HARD_IMPACTS[..], 0.0));
        assert_eq!(impact_cue(0.45).unwrap().0, HARD_IMPACTS);
        assert_eq!(landing_db(0.0), None);
        assert_eq!(landing_db(1.0), Some(0.0));
    }

    #[test]
    fn the_cue_log_writes_one_line_per_cue_per_second_and_counts_repeats() {
        let mut log = CueLog::default();
        assert_eq!(log.note("lap_chime", 10.0), Some(1), "first start");
        assert_eq!(log.note("lap_chime", 10.2), None);
        assert_eq!(log.note("impact_soft_1", 10.3), Some(1), "per cue");
        assert_eq!(log.note("lap_chime", 10.5), None);
        assert_eq!(
            log.note("lap_chime", 11.0),
            Some(3),
            "the line stands for 3 starts"
        );
        assert_eq!(log.note("lap_chime", 12.5), Some(1));
    }

    #[test]
    fn race_events_map_to_hud_cues() {
        let known: HashSet<&str> = crate::bank::SFX.into_iter().collect();
        assert_eq!(
            race_cue(RaceEventKind::Go),
            Some(("countdown_go", 0.0, false))
        );
        assert_eq!(
            race_cue(RaceEventKind::FinalLap).map(|c| c.0),
            Some("final_lap_fanfare")
        );
        assert_eq!(race_cue(RaceEventKind::WrongWay(false)), None);
        assert_eq!(race_cue(RaceEventKind::PerfectStart), None);
        for kind in [
            RaceEventKind::CountdownTick(3),
            RaceEventKind::RaceEndsIn(5),
            RaceEventKind::GoldenBattery,
            RaceEventKind::Finished { rank: 1, time: 0 },
            RaceEventKind::LapCompleted {
                lap: 1,
                time: 0,
                best: true,
            },
        ] {
            let (stem, _, _) = race_cue(kind).unwrap();
            assert!(known.contains(stem), "{stem}");
        }
    }
}
