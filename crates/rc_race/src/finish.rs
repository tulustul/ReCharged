//! The end of the race (GDD §3.2): it ends when every human has finished (with no humans: every
//! racer), or when the soft-lock guard expires. The guard starts when the winner finishes (one
//! human or none) or when the first human finishes (2+ humans) — in any case 120 s after the
//! winner — and runs 60 s, announcing the last 10 s (`RaceEndsIn`). Unfinished racers are then
//! classified by distance with a projected time. `RaceResults` is inserted before `Finished`.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::ghost::GhostRecorder;
use crate::progress::Tracker;

/// The soft-lock guard runs this long (ticks, 60 s).
const GUARD_TICKS: u32 = 3600;
/// With 2+ humans it starts at the latest this long after the winner finished (ticks, 120 s).
const GUARD_LATE_TICKS: u32 = 7200;
/// `RaceEndsIn` counts down the last seconds.
const ENDS_IN_SHOWN_SECS: u32 = 10;
/// Projected times assume at least this average pace (u/s).
const MIN_PACE_UPS: f32 = 5.0;

/// Race tick at which the soft-lock guard starts.
pub(crate) fn guard_start(
    humans: usize,
    winner: Option<u32>,
    first_human: Option<u32>,
) -> Option<u32> {
    let winner = winner?;
    if humans < 2 {
        return Some(winner);
    }
    let latest = winner + GUARD_LATE_TICKS;
    Some(first_human.map_or(latest, |h| h.min(latest)))
}

/// Total time of an unfinished racer that has covered `distance` of `race_distance` at race
/// tick `now`: the rest at its average pace so far (at least [`MIN_PACE_UPS`]). Always > `now`,
/// and more distance never projects a later time.
pub(crate) fn projected_ticks(now: u32, distance: f32, race_distance: f32) -> u32 {
    let pace = (distance.max(0.0) / now.max(1) as f32).max(MIN_PACE_UPS * SIM_DT);
    let rest = (race_distance - distance).max(0.0) / pace;
    now.saturating_add(rest.ceil().max(1.0) as u32)
}

type Entry<'a> = (
    &'a Racer,
    &'a RaceProgress,
    &'a Tracker,
    Option<&'a LocalPlayer>,
);

/// What ends the race needs to read.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Facts<'w, 's> {
    standings: Res<'w, Standings>,
    config: Option<Res<'w, RaceConfig>>,
    track: Option<Res<'w, Track>>,
    summary: Option<Res<'w, TrackSummary>>,
    recorder: Res<'w, GhostRecorder>,
    racers: Query<'w, 's, Entry<'static>>,
}

/// `SimSet::Race` (Racing, last): the soft-lock guard and the end of the race.
pub(crate) fn referee(
    facts: Facts,
    mut status: ResMut<RaceStatus>,
    mut next: ResMut<NextState<RacePhase>>,
    mut events: MessageWriter<RaceEvent>,
    mut commands: Commands,
) {
    let now = status.race_ticks;
    let racers = || facts.racers.iter();
    let humans = racers().filter(|r| r.3.is_some()).count();
    let finish = |only_humans: bool| {
        racers()
            .filter(move |r| !only_humans || r.3.is_some())
            .map(|r| r.2.finish)
    };
    let winner = finish(false).flatten().min();
    let first_human = finish(true).flatten().min();
    let everyone = finish(humans > 0).all(|f| f.is_some()) && racers().next().is_some();
    let end = guard_start(humans, winner, first_human)
        .filter(|&start| now >= start)
        .map(|start| start + GUARD_TICKS);
    status.ends_in = end.map(|e| ticks_to_secs(e.saturating_sub(now)));
    if let Some(left) = end.map(|e| e.saturating_sub(now))
        && left.is_multiple_of(SIM_HZ as u32)
        && (1..=ENDS_IN_SHOWN_SECS).contains(&(left / SIM_HZ as u32))
    {
        events.write(RaceEvent {
            racer: None,
            kind: RaceEventKind::RaceEndsIn((left / SIM_HZ as u32) as u8),
        });
    }
    if !everyone && end.is_none_or(|e| now < e) {
        return;
    }
    status.ends_in = None;
    commands.insert_resource(results(&facts, now, status.total_laps.max(1)));
    next.set_if_different(RacePhase::Finished);
    info!(
        "race over after {:.2} s ({})",
        ticks_to_secs(now),
        if everyone {
            "everyone finished"
        } else {
            "soft-lock guard"
        }
    );
}

/// Results in standings order; the Time Trial human's ghost.
fn results(facts: &Facts, now: u32, laps: u8) -> RaceResults {
    let length = facts.track.as_ref().map_or(0.0, |t| t.length());
    let race_distance = length * f32::from(laps);
    let entries: Vec<RaceResultEntry> = facts
        .standings
        .0
        .iter()
        .filter_map(|&e| facts.racers.get(e).ok())
        .enumerate()
        .map(|(i, (racer, progress, tracker, local))| RaceResultEntry {
            index: racer.index,
            name: racer.name.clone(),
            car: racer.car.clone(),
            slot: local.map(|l| l.0),
            rank: i as u8 + 1,
            finished: tracker.finish.is_some(),
            time_ticks: tracker
                .finish
                .unwrap_or_else(|| projected_ticks(now, progress.total_distance, race_distance)),
            best_lap_ticks: progress.best_lap,
        })
        .collect();
    let geometry_hash = facts.summary.as_ref().map_or(0, |s| s.geometry_hash);
    let time_trial = facts
        .config
        .as_ref()
        .is_some_and(|c| c.mode == GameMode::TimeTrial);
    let ghost = facts
        .racers
        .iter()
        .find(|r| r.3.is_some() && r.2.finish.is_some())
        .filter(|_| time_trial)
        .map(|(racer, _, tracker, _)| GhostData {
            car: racer.car.clone(),
            paint: racer.paint,
            time_ticks: tracker.finish.unwrap_or(now),
            geometry_hash,
            samples: facts.recorder.0.clone(),
        });
    RaceResults {
        entries,
        ghost,
        geometry_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_lock_guard_starts_by_number_of_humans() {
        assert_eq!(guard_start(1, None, None), None, "nobody finished");
        assert_eq!(
            guard_start(1, Some(1000), None),
            Some(1000),
            "one human: the winner"
        );
        assert_eq!(
            guard_start(0, Some(1000), None),
            Some(1000),
            "AI only: the winner"
        );
        assert_eq!(
            guard_start(2, Some(1000), Some(1500)),
            Some(1500),
            "2+: first human"
        );
        assert_eq!(
            guard_start(3, Some(1000), None),
            Some(1000 + GUARD_LATE_TICKS),
            "2+: 120 s after the winner at the latest"
        );
        assert_eq!(guard_start(2, Some(1000), Some(9000)), Some(8200));
    }

    #[test]
    fn projected_times_extend_the_average_pace() {
        // 600 u in 1200 ticks (30 u/s) of a 900 u race: 300 u more at the same pace.
        assert_eq!(projected_ticks(1200, 600.0, 900.0), 1800);
        let ahead = projected_ticks(1200, 700.0, 900.0);
        assert!(ahead < 1800 && ahead > 1200, "more distance, earlier time");
        // Standing still on the grid: the minimum pace (5 u/s) covers the race.
        assert_eq!(projected_ticks(600, -3.0, 900.0), 600 + 900 * 12 + 36);
        assert!(projected_ticks(10, 899.99, 900.0) > 10, "always after now");
    }
}
