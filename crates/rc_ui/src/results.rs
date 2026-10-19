//! Post-race data for React: `race.results` (rc_race's `RaceResults` + rc_game's `RaceOutcome`),
//! `cup.standings` (`CupRun`) and `versus.score` (`VersusSession`), pushed when they change and
//! included in the `race.snapshot` response.

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::{CupRun, NextMedal, RaceOutcome, RacerKey, Series, VersusSession};

use crate::profile::{UiTrophy, UiUnlockedItem};
use crate::records::UiMedal;

/// Milliseconds of `ticks` simulation ticks.
pub fn ticks_to_ms(ticks: u32) -> u32 {
    (f64::from(ticks) * 1000.0 / SIM_HZ).round() as u32
}

/// One line of the results table.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiResultEntry {
    /// Final position, 1-based.
    pub rank: u8,
    /// Display name.
    pub name: String,
    /// Car id.
    pub car: String,
    /// Local player slot (`null` = AI).
    pub slot: Option<u8>,
    /// Finished normally (false = classified by progress, projected time).
    pub finished: bool,
    /// Total time in ms.
    pub time_ms: u32,
    /// Best lap in ms.
    pub best_lap_ms: Option<u32>,
    /// Points this race gave (cup: everyone; Versus: players; `null` until processed, for AI in
    /// Versus and in Quick Race / Time Trial — the PTS column shows only when some line has
    /// points).
    pub points: Option<u32>,
}

/// The next better Time-Trial medal.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiNextMedal {
    /// The medal.
    pub medal: UiMedal,
    /// Its target time in ms.
    pub target_ms: u32,
    /// How much slower the run was, ms.
    pub delta_ms: u32,
}

impl From<NextMedal> for UiNextMedal {
    fn from(next: NextMedal) -> Self {
        Self {
            medal: next.medal.into(),
            target_ms: ticks_to_ms(next.target_ticks),
            delta_ms: ticks_to_ms(next.delta_ticks),
        }
    }
}

/// What the race changed in the profile (rc_game's `RaceOutcome`).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiOutcome {
    /// New best lap on this layout.
    pub new_best_lap: bool,
    /// New best race time (Time Trial).
    pub new_best_race: bool,
    /// Medal of this run (Time Trial).
    pub medal: Option<UiMedal>,
    /// The next better medal (Time Trial).
    pub next_medal: Option<UiNextMedal>,
    /// The run became the layout's ghost.
    pub ghost_saved: bool,
    /// Older records were discarded (the track changed).
    pub records_discarded: bool,
    /// Unlocked by this result.
    pub unlocked: Vec<UiUnlockedItem>,
    /// The cup's last race: Cup end follows.
    pub cup_finished: bool,
}

/// Bevy → React event `race.results`: the race ended (sent again once rc_game processed it).
#[react_event(name = "race.results")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiResults {
    /// Entries ordered by rank.
    pub entries: Vec<UiResultEntry>,
    /// The profile summary (`null` until processed).
    pub outcome: Option<UiOutcome>,
}

impl UiResults {
    /// Results with points and summary when rc_game has processed them.
    pub fn new(results: &RaceResults, outcome: Option<&RaceOutcome>) -> Self {
        Self {
            entries: results
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| UiResultEntry {
                    rank: e.rank,
                    name: e.name.clone(),
                    car: e.car.0.clone(),
                    slot: e.slot.map(|s| s.0),
                    finished: e.finished,
                    time_ms: ticks_to_ms(e.time_ticks),
                    best_lap_ms: e.best_lap_ticks.map(ticks_to_ms),
                    points: outcome.and_then(|o| o.points.get(i).copied().flatten()),
                })
                .collect(),
            outcome: outcome.map(|o| UiOutcome {
                new_best_lap: o.new_best_lap,
                new_best_race: o.new_best_race,
                medal: o.medal.map(Into::into),
                next_medal: o.next_medal.map(Into::into),
                ghost_saved: o.ghost_saved,
                records_discarded: o.records_discarded,
                unlocked: o.unlocked.iter().map(Into::into).collect(),
                cup_finished: o.cup_finished,
            }),
        }
    }
}

/// A line of the cup standings.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiStandingRow {
    /// Display name.
    pub name: String,
    /// Car id.
    pub car: String,
    /// Local player slot (`null` = AI).
    pub slot: Option<u8>,
    /// Racer colour, sRGB 0–1.
    pub color: [f32; 3],
    /// Total points.
    pub points: u32,
    /// Points gained in the latest race (animated).
    pub gained: u32,
    /// Position in each race run so far.
    pub positions: Vec<Option<u8>>,
}

/// Bevy → React event `cup.standings`: the cup table (on every change).
#[react_event(name = "cup.standings")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq)]
pub struct UiCupStandings {
    /// Cup id.
    pub cup: String,
    /// Races with a result.
    pub played: usize,
    /// The cup is over (Cup end next).
    pub finished: bool,
    /// The player's final position once finished.
    pub final_position: Option<u8>,
    /// The player's trophy once finished on the podium.
    pub trophy: Option<UiTrophy>,
    /// Rows ordered by standing.
    pub rows: Vec<UiStandingRow>,
}

fn position_in(series: &Series, race: usize, key: &RacerKey) -> Option<u8> {
    series
        .results
        .get(race)?
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, p)| *p)
}

impl From<&CupRun> for UiCupStandings {
    fn from(run: &CupRun) -> Self {
        let series = &run.series;
        let played = series.races_played();
        let mut standings: Vec<(RacerKey, u32)> = series
            .standings()
            .into_iter()
            .map(|s| (s.key, s.points))
            .collect();
        // Before the first result everyone stands at 0, in grid order.
        for racer in &run.racers {
            let key = RacerKey::of(&racer.controller);
            if !standings.iter().any(|(k, _)| *k == key) {
                standings.push((key, 0));
            }
        }
        Self {
            cup: run.cup.0.clone(),
            played,
            finished: run.finished(),
            final_position: run.final_position,
            trophy: run.trophy().map(Into::into),
            rows: standings
                .into_iter()
                .filter_map(|(key, points)| {
                    let racer = run
                        .racers
                        .iter()
                        .find(|r| RacerKey::of(&r.controller) == key)?;
                    Some(UiStandingRow {
                        name: racer.name.clone(),
                        car: racer.car.0.clone(),
                        slot: match key {
                            RacerKey::Player(slot) => Some(slot.0),
                            RacerKey::Rival(_) => None,
                        },
                        color: racer.color,
                        points,
                        gained: played
                            .checked_sub(1)
                            .map_or(0, |last| series.race_points(last, &key)),
                        positions: (0..played)
                            .map(|race| position_in(series, race, &key))
                            .collect(),
                    })
                })
                .collect(),
        }
    }
}

/// A line of the Versus scoreboard.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiScoreRow {
    /// Slot 0–3.
    pub slot: u8,
    /// Session points.
    pub points: u32,
    /// Points gained in the latest race.
    pub gained: u32,
    /// Race wins this session.
    pub wins: u32,
    /// The AI took over this player's car.
    pub dropped: bool,
}

/// Bevy → React event `versus.score`: the session scoreboard (on every change).
#[react_event(name = "versus.score")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiVersusScore {
    /// Races run this session.
    pub races: usize,
    /// Rows ordered by points.
    pub rows: Vec<UiScoreRow>,
}

impl From<&VersusSession> for UiVersusScore {
    fn from(session: &VersusSession) -> Self {
        let series = &session.series;
        let played = series.races_played();
        Self {
            races: played,
            rows: series
                .standings()
                .into_iter()
                .filter_map(|s| {
                    let RacerKey::Player(slot) = s.key else {
                        return None;
                    };
                    let key = RacerKey::Player(slot);
                    Some(UiScoreRow {
                        slot: slot.0,
                        points: s.points,
                        gained: played
                            .checked_sub(1)
                            .map_or(0, |last| series.race_points(last, &key)),
                        wins: (0..played)
                            .filter(|&race| position_in(series, race, &key) == Some(1))
                            .count() as u32,
                        dropped: session.dropped.contains(&slot),
                    })
                })
                .collect(),
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<UiResults>()
        .add_react_event::<UiCupStandings>()
        .add_react_event::<UiVersusScore>();
}

/// Sends `race.results` when rc_race writes results or rc_game adds its summary.
pub(crate) fn push_results(
    results: Option<Res<RaceResults>>,
    outcome: Option<Res<RaceOutcome>>,
    events: ReactEvents,
) {
    let Some(results) = results else { return };
    if results.is_changed() || outcome.as_ref().is_some_and(|o| o.is_changed()) {
        events.send(&UiResults::new(&results, outcome.as_deref()));
    }
}

/// Sends `cup.standings` / `versus.score` when the series change.
pub(crate) fn push_series(
    cup: Option<Res<CupRun>>,
    versus: Option<Res<VersusSession>>,
    events: ReactEvents,
) {
    if let Some(cup) = cup.filter(|c| c.is_changed()) {
        events.send(&UiCupStandings::from(&*cup));
    }
    if let Some(versus) = versus.filter(|v| v.is_changed()) {
        events.send(&UiVersusScore::from(&*versus));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn racer(name: &str, controller: Controller) -> RacerSetup {
        RacerSetup {
            name: name.into(),
            car: CarId::new("zip"),
            controller,
            ..default()
        }
    }

    #[test]
    fn ticks_convert_to_rounded_milliseconds() {
        assert_eq!(ticks_to_ms(60), 1000);
        assert_eq!(ticks_to_ms(1), 17);
        assert_eq!(ticks_to_ms(5_000), 83_333);
    }

    #[test]
    fn cup_standings_show_points_gains_and_positions() {
        let me = Controller::Human {
            slot: PlayerSlot(0),
            autopilot: false,
        };
        let dash = Controller::Ai {
            rival: RivalId::new("dash"),
            difficulty: Difficulty::Pro,
        };
        let mut series = Series::default();
        series.record(vec![(RacerKey::of(&dash), 1), (RacerKey::of(&me), 2)]);
        series.race = 1;
        series.record(vec![(RacerKey::of(&me), 1), (RacerKey::of(&dash), 2)]);
        let run = CupRun {
            cup: CupId::new("rookie_cup"),
            series,
            racers: vec![racer("You", me), racer("Dash", dash)],
            final_position: None,
        };
        let ui = UiCupStandings::from(&run);
        assert_eq!(ui.played, 2);
        // 18 points each; the tie goes to the better result in the last race.
        assert_eq!(ui.rows[0].name, "You");
        assert_eq!((ui.rows[0].points, ui.rows[0].gained), (18, 10));
        assert_eq!(ui.rows[0].positions, [Some(2), Some(1)]);
        assert_eq!(ui.rows[0].slot, Some(0));
        assert_eq!(ui.rows[1].slot, None);
    }

    #[test]
    fn a_fresh_cup_lists_everyone_at_zero() {
        let run = CupRun {
            cup: CupId::new("rookie_cup"),
            series: Series::default(),
            racers: vec![racer(
                "You",
                Controller::Human {
                    slot: PlayerSlot(0),
                    autopilot: false,
                },
            )],
            final_position: None,
        };
        let ui = UiCupStandings::from(&run);
        assert_eq!(ui.rows.len(), 1);
        assert_eq!((ui.rows[0].points, ui.rows[0].gained), (0, 0));
        assert!(ui.rows[0].positions.is_empty());
    }

    #[test]
    fn versus_scoreboard_counts_wins_and_drops() {
        let p = |slot| RacerKey::Player(PlayerSlot(slot));
        let mut series = Series::default();
        series.record(vec![(p(1), 1), (p(0), 3)]);
        series.race = 1;
        series.record(vec![(p(1), 1), (p(0), 2)]);
        let session = VersusSession {
            series,
            dropped: vec![PlayerSlot(0)],
        };
        let ui = UiVersusScore::from(&session);
        assert_eq!(ui.races, 2);
        assert_eq!(
            (ui.rows[0].slot, ui.rows[0].points, ui.rows[0].wins),
            (1, 20, 2)
        );
        assert_eq!((ui.rows[1].slot, ui.rows[1].gained), (0, 8));
        assert!(ui.rows[1].dropped && !ui.rows[0].dropped);
    }
}
