//! Race results → profile (pure): stats, records with geometry hashes, medals (GDD §9).

use rc_core::prelude::*;

use crate::medals::MedalTargets;
use crate::profile::{Medal, Profile, TimedRecord, variant_key};

/// The next better medal of a Time Trial run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextMedal {
    /// The medal.
    pub medal: Medal,
    /// Its target time in ticks.
    pub target_ticks: u32,
    /// How many ticks the run was slower than the target.
    pub delta_ticks: u32,
}

/// What a result changed in the profile's records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecordsUpdate {
    pub new_best_lap: bool,
    pub new_best_race: bool,
    pub medal: Option<Medal>,
    pub next_medal: Option<NextMedal>,
    /// Records of this layout were set on other geometry and were discarded.
    pub discarded: bool,
}

/// Discards the times of layout `key` if they were set on other geometry (the medal stays);
/// returns whether there was anything to discard.
pub(crate) fn discard_stale(profile: &mut Profile, key: &str, geometry_hash: u64) -> bool {
    let Some(record) = profile.records.get_mut(key) else {
        return false;
    };
    if record.geometry_hash == geometry_hash {
        return false;
    }
    let had_times = record.best_lap.is_some() || record.best_race.is_some();
    record.best_lap = None;
    record.best_race = None;
    record.geometry_hash = geometry_hash;
    had_times
}

/// Number of human racers of a grid.
pub(crate) fn humans(config: &RaceConfig) -> usize {
    let human = |r: &&RacerSetup| matches!(r.controller, Controller::Human { .. });
    config.racers.iter().filter(human).count()
}

/// A single-player race (records, Golden Batteries): one human, not Versus.
pub(crate) fn single_player(config: &RaceConfig) -> bool {
    config.mode != GameMode::Versus && humans(config) == 1
}

/// Applies the result of the profile owner (slot 0) to stats and, in single-player modes, to the
/// layout's records (best lap any mode, best race and medal Time Trial only). `distance` = how far
/// the owner drove (u).
pub(crate) fn apply_results(
    profile: &mut Profile,
    config: &RaceConfig,
    results: &RaceResults,
    targets: Option<MedalTargets>,
    distance: f32,
) -> RecordsUpdate {
    let mut update = RecordsUpdate::default();
    let Some(entry) = results
        .entries
        .iter()
        .find(|e| e.slot == Some(PlayerSlot(0)))
    else {
        return update;
    };
    let stats = &mut profile.stats;
    stats.races += 1;
    stats.wins += u32::from(entry.rank == 1);
    stats.podiums += u32::from((1..=3).contains(&entry.rank));
    stats.distance += f64::from(distance.max(0.0));
    if !single_player(config) {
        return update;
    }

    let key = variant_key(&config.variant);
    update.discarded = discard_stale(profile, &key, results.geometry_hash);
    let record = profile.records.entry(key).or_default();
    record.geometry_hash = results.geometry_hash;
    let timed = |ticks| TimedRecord {
        ticks,
        car: entry.car.clone(),
    };
    if let Some(lap) = entry.best_lap_ticks
        && record.best_lap.as_ref().is_none_or(|b| lap < b.ticks)
    {
        record.best_lap = Some(timed(lap));
        update.new_best_lap = true;
    }
    if config.mode != GameMode::TimeTrial || !entry.finished {
        return update;
    }
    let time = entry.time_ticks;
    if record.best_race.as_ref().is_none_or(|b| time < b.ticks) {
        record.best_race = Some(timed(time));
        update.new_best_race = true;
    }
    if let Some(targets) = targets {
        update.medal = targets.medal_for(time);
        record.medal = record.medal.max(update.medal);
        update.next_medal = targets.next(update.medal).map(|(medal, target)| NextMedal {
            medal,
            target_ticks: target,
            delta_ticks: time.saturating_sub(target),
        });
    }
    update
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::medals::MedalTable;

    fn entry(slot: Option<u8>, rank: u8, time: u32, lap: u32) -> RaceResultEntry {
        RaceResultEntry {
            index: usize::from(rank) - 1,
            name: "x".into(),
            car: "zip".into(),
            slot: slot.map(PlayerSlot),
            rank,
            finished: true,
            time_ticks: time,
            best_lap_ticks: Some(lap),
        }
    }

    fn race(mode: GameMode, humans: u8) -> RaceConfig {
        let human = |slot| RacerSetup {
            controller: Controller::Human {
                slot: PlayerSlot(slot),
                autopilot: false,
            },
            ..Default::default()
        };
        RaceConfig {
            mode,
            variant: TrackVariant::forward("toy_town"),
            racers: (0..humans).map(human).collect(),
            ..Default::default()
        }
    }

    fn results(entries: Vec<RaceResultEntry>, hash: u64) -> RaceResults {
        RaceResults {
            entries,
            ghost: None,
            geometry_hash: hash,
        }
    }

    fn targets() -> Option<MedalTargets> {
        let table = MedalTable::parse(r#"(reference_secs: {"toy_town": 100.0})"#).ok()?;
        table.targets(&TrackVariant::forward("toy_town"))
    }

    #[test]
    fn time_trial_sets_records_medals_and_the_next_medal() {
        let mut p = Profile::default();
        let tt = race(GameMode::TimeTrial, 1);
        let u = apply_results(
            &mut p,
            &tt,
            &results(vec![entry(Some(0), 1, 6000, 1900)], 7),
            targets(),
            1950.0,
        );
        assert!(u.new_best_lap && u.new_best_race && !u.discarded);
        assert_eq!(u.medal, Some(Medal::Silver));
        let next = u.next_medal.expect("gold is next");
        assert_eq!(
            (next.medal, next.target_ticks, next.delta_ticks),
            (Medal::Gold, 5880, 120)
        );
        let rec = p.record(&tt.variant).expect("record").clone();
        assert_eq!(
            (rec.geometry_hash, rec.best_race.map(|r| r.ticks)),
            (7, Some(6000))
        );
        assert_eq!(
            (p.stats.races, p.stats.wins, p.stats.distance),
            (1, 1, 1950.0)
        );

        // Slower run: no new records; the best medal stays.
        let u = apply_results(
            &mut p,
            &tt,
            &results(vec![entry(Some(0), 1, 6500, 2000)], 7),
            targets(),
            0.0,
        );
        assert!(!u.new_best_lap && !u.new_best_race);
        assert_eq!(u.medal, Some(Medal::Bronze));
        assert_eq!(
            p.record(&tt.variant).and_then(|r| r.medal),
            Some(Medal::Silver)
        );
    }

    #[test]
    fn a_changed_track_discards_times_but_keeps_the_medal() {
        let mut p = Profile::default();
        let tt = race(GameMode::TimeTrial, 1);
        apply_results(
            &mut p,
            &tt,
            &results(vec![entry(Some(0), 1, 5000, 1600)], 1),
            targets(),
            0.0,
        );
        let u = apply_results(
            &mut p,
            &tt,
            &results(vec![entry(Some(0), 1, 6500, 2100)], 2),
            targets(),
            0.0,
        );
        assert!(u.discarded && u.new_best_race && u.new_best_lap);
        let rec = p.record(&tt.variant).expect("record");
        assert_eq!(
            (rec.geometry_hash, rec.best_race.as_ref().map(|r| r.ticks)),
            (2, Some(6500))
        );
        assert_eq!(rec.medal, Some(Medal::Platinum), "earned medals stay");
        assert!(
            !discard_stale(&mut p, "toy_town", 2),
            "same geometry: nothing to discard"
        );
        assert!(!discard_stale(&mut p, "sofa_circuit", 2), "no record");
    }

    #[test]
    fn quick_race_records_only_the_best_lap_and_versus_none() {
        let mut p = Profile::default();
        let quick = race(GameMode::QuickRace, 1);
        let r = results(
            vec![entry(None, 1, 5000, 1500), entry(Some(0), 2, 5100, 1600)],
            3,
        );
        let u = apply_results(&mut p, &quick, &r, targets(), 0.0);
        assert!(u.new_best_lap && !u.new_best_race && u.medal.is_none());
        let rec = p.record(&quick.variant).expect("record");
        assert_eq!(
            (
                rec.best_lap.as_ref().map(|l| l.ticks),
                rec.best_race.as_ref()
            ),
            (Some(1600), None)
        );
        assert_eq!((p.stats.wins, p.stats.podiums), (0, 1));

        let mut v = Profile::default();
        let versus = race(GameMode::Versus, 2);
        let r = results(
            vec![entry(Some(1), 1, 5000, 1500), entry(Some(0), 2, 5100, 1400)],
            3,
        );
        let u = apply_results(&mut v, &versus, &r, targets(), 0.0);
        assert_eq!(u, RecordsUpdate::default());
        assert!(
            v.records.is_empty() && v.stats.races == 1,
            "stats count, records do not"
        );
    }

    #[test]
    fn an_unfinished_time_trial_sets_no_race_record() {
        let mut p = Profile::default();
        let mut e = entry(Some(0), 1, 4000, 1500);
        e.finished = false;
        let u = apply_results(
            &mut p,
            &race(GameMode::TimeTrial, 1),
            &results(vec![e], 0),
            targets(),
            0.0,
        );
        assert!(u.new_best_lap && !u.new_best_race && u.medal.is_none());
    }
}
