//! Points series (pure): cup runs and the Versus scoreboard (GDD §3.1, §3.3).
//!
//! A [`Series`] stores each finished race's positions. Recording a race replaces any earlier
//! result of the same race, and a restart removes it, so restarting reverts that race's points.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::profile::Trophy;

/// Championship points by finishing position (8 racers, GDD §3.1).
pub const POINTS: [u32; MAX_RACERS] = [10, 8, 6, 5, 4, 3, 2, 1];

/// Points for a 1-based finishing position (0 outside the table).
pub fn points_for(position: u8) -> u32 {
    usize::from(position)
        .checked_sub(1)
        .and_then(|i| POINTS.get(i))
        .copied()
        .unwrap_or(0)
}

/// Who scores in a series: a local player or a rival.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RacerKey {
    /// A local player (slot 0 in cups).
    Player(PlayerSlot),
    /// An AI rival.
    Rival(RivalId),
}

impl RacerKey {
    /// The key of a grid entry.
    pub fn of(controller: &Controller) -> Self {
        match controller {
            Controller::Human { slot, .. } => Self::Player(*slot),
            Controller::Ai { rival, .. } => Self::Rival(rival.clone()),
        }
    }
}

/// One line of the series standings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// Who.
    pub key: RacerKey,
    /// Total points.
    pub points: u32,
    /// 1-based position in the standings.
    pub position: u8,
}

/// Races of a points series.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Series {
    /// Index of the current race (0-based).
    pub race: usize,
    /// Positions of every scored racer, per finished race (`results[race]`).
    pub results: Vec<Vec<(RacerKey, u8)>>,
}

impl Series {
    /// Stores the current race's positions, replacing an earlier result of the same race.
    pub fn record(&mut self, positions: Vec<(RacerKey, u8)>) {
        self.results.truncate(self.race);
        self.results.push(positions);
    }

    /// Forgets the current race's result (restart).
    pub fn revert(&mut self) {
        self.results.truncate(self.race);
    }

    /// The current race has a result.
    pub fn race_done(&self) -> bool {
        self.results.len() > self.race
    }

    /// Races with a result.
    pub fn races_played(&self) -> usize {
        self.results.len()
    }

    /// Total points of `key`.
    pub fn points(&self, key: &RacerKey) -> u32 {
        self.results
            .iter()
            .filter_map(|race| position_in(race, key))
            .map(points_for)
            .sum()
    }

    /// Points `key` scored in race `race`.
    pub fn race_points(&self, race: usize, key: &RacerKey) -> u32 {
        self.results
            .get(race)
            .and_then(|r| position_in(r, key))
            .map_or(0, points_for)
    }

    /// Standings by points; ties go to the better result in the last race, then the race before,
    /// and so on (GDD §3.1). Racers missing from a race count as last in it.
    pub fn standings(&self) -> Vec<Standing> {
        let mut keys: Vec<&RacerKey> = Vec::new();
        for (key, _) in self.results.iter().flatten() {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let mut rows: Vec<(&RacerKey, u32)> = keys.iter().map(|k| (*k, self.points(k))).collect();
        rows.sort_by(|(a, pa), (b, pb)| {
            pb.cmp(pa).then_with(|| {
                let latest_first = self.results.iter().rev();
                let ranks = |key| latest_first.clone().map(move |r| position_in(r, key));
                ranks(a)
                    .map(|r| r.unwrap_or(u8::MAX))
                    .cmp(ranks(b).map(|r| r.unwrap_or(u8::MAX)))
            })
        });
        rows.into_iter()
            .enumerate()
            .map(|(i, (key, points))| Standing {
                key: key.clone(),
                points,
                position: i as u8 + 1,
            })
            .collect()
    }

    /// 1-based standings position of `key`.
    pub fn position(&self, key: &RacerKey) -> Option<u8> {
        self.standings()
            .into_iter()
            .find(|s| &s.key == key)
            .map(|s| s.position)
    }
}

fn position_in(race: &[(RacerKey, u8)], key: &RacerKey) -> Option<u8> {
    race.iter().find(|(k, _)| k == key).map(|(_, p)| *p)
}

/// A championship in progress (GDD §3.1). Lives from the cup's `StartRace` until the cup is quit
/// unfinished; a finished cup stays (for the Cup end screen) until the next `StartRace`. Never
/// saved: a cup in progress does not survive an app restart.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CupRun {
    /// The cup.
    pub cup: CupId,
    /// Points series; `series.race` is the index of the current cup race.
    pub series: Series,
    /// The field, the same in every race (names, cars, colours, controllers).
    pub racers: Vec<RacerSetup>,
    /// The player's final position once the last race has a result.
    pub final_position: Option<u8>,
}

impl CupRun {
    /// The last race has a result.
    pub fn finished(&self) -> bool {
        self.final_position.is_some()
    }

    /// The player's trophy (`None` while running or off the podium).
    pub fn trophy(&self) -> Option<Trophy> {
        self.final_position.and_then(Trophy::for_position)
    }
}

/// The Versus scoreboard (GDD §3.3, UI-22): championship points per player across the races of a
/// lobby. Lives from the first Versus `StartRace` until `CloseLobby` or a non-Versus race.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct VersusSession {
    /// Points series of the players (`RacerKey::Player`); positions are overall race positions.
    pub series: Series,
    /// Players whose car the AI took over (`GameCommand::DropPlayer`) for this session.
    pub dropped: Vec<PlayerSlot>,
}

impl VersusSession {
    /// Total points of a player.
    pub fn points(&self, slot: PlayerSlot) -> u32 {
        self.series.points(&RacerKey::Player(slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rival(id: &str) -> RacerKey {
        RacerKey::Rival(id.into())
    }

    const P1: RacerKey = RacerKey::Player(PlayerSlot(0));

    #[test]
    fn points_follow_the_table() {
        let got: Vec<u32> = (0..=9).map(points_for).collect();
        assert_eq!(got, [0, 10, 8, 6, 5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn standings_sum_points_and_break_ties_by_the_latest_race() {
        let mut s = Series::default();
        s.record(vec![(P1, 1), (rival("dash"), 2), (rival("nova"), 3)]);
        s.race = 1;
        s.record(vec![(P1, 2), (rival("dash"), 1), (rival("nova"), 3)]);
        // P1 and Dash both have 18: Dash won the latest race.
        let st = s.standings();
        assert_eq!(st[0].key, rival("dash"));
        assert_eq!((st[0].points, st[1].points), (18, 18));
        assert_eq!(st[1].key, P1);
        assert_eq!(
            (st[2].key.clone(), st[2].points, st[2].position),
            (rival("nova"), 12, 3)
        );

        // Both missing from the latest race (equal there): the race before decides.
        let mut t = Series::default();
        t.record(vec![(P1, 1), (rival("dash"), 2)]);
        t.race = 1;
        t.record(vec![(P1, 2), (rival("dash"), 1)]);
        t.race = 2;
        t.record(vec![(rival("nova"), 1)]);
        assert_eq!((t.points(&P1), t.points(&rival("dash"))), (18, 18));
        assert_eq!(t.position(&rival("dash")), Some(1), "won the race before");
        assert_eq!(
            (t.position(&P1), t.position(&rival("nova"))),
            (Some(2), Some(3))
        );
    }

    #[test]
    fn a_missing_racer_counts_as_last() {
        let mut s = Series::default();
        s.record(vec![(P1, 1)]);
        s.race = 1;
        s.record(vec![(P1, 8), (RacerKey::Player(PlayerSlot(1)), 1)]);
        let st = s.standings();
        assert_eq!((st[0].key.clone(), st[0].points), (P1, 11));
        assert_eq!(st[1].points, 10);
    }

    #[test]
    fn restart_reverts_the_race_and_rerecording_replaces_it() {
        let mut s = Series::default();
        s.record(vec![(P1, 1)]);
        assert!(s.race_done());
        s.race = 1;
        assert!(!s.race_done());
        s.record(vec![(P1, 8)]);
        assert_eq!(s.points(&P1), 11);
        s.revert();
        assert_eq!(
            (s.points(&P1), s.races_played()),
            (10, 1),
            "restart reverts the race"
        );
        s.record(vec![(P1, 2)]);
        s.record(vec![(P1, 3)]);
        assert_eq!(s.points(&P1), 16, "a race is scored once");
        assert_eq!(s.race_points(1, &P1), 6);
    }

    #[test]
    fn cup_trophy_follows_the_final_position() {
        let mut run = CupRun {
            cup: "rookie_cup".into(),
            series: Series::default(),
            racers: Vec::new(),
            final_position: None,
        };
        assert!(!run.finished() && run.trophy().is_none());
        run.final_position = Some(2);
        assert_eq!(run.trophy(), Some(Trophy::Silver));
        run.final_position = Some(4);
        assert!(run.finished() && run.trophy().is_none());
    }
}
