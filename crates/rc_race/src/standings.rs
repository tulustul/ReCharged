//! Standings (`SimSet::Race`, after progress): finished racers by finish time, then the rest by
//! `total_distance`; ties keep the previous order, then the grid order (deterministic).

use std::cmp::Ordering;

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::progress::Tracker;

/// Sort key of a racer in the standings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Key {
    /// Finish tick.
    pub finish: Option<u32>,
    pub total_distance: f32,
    /// Rank of the previous tick (0 = none yet): ties keep their order.
    pub rank: u8,
    /// Grid index: the last tie-break.
    pub index: usize,
}

/// Finished racers by finish time, then the rest by distance; ties keep the previous order,
/// then the grid order.
pub(crate) fn sort_standings<T>(order: &mut [(T, Key)]) {
    order.sort_by(|(_, a), (_, b)| {
        let by_progress = match (a.finish, b.finish) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.total_distance.total_cmp(&a.total_distance),
        };
        let previous = |k: &Key| if k.rank == 0 { u8::MAX } else { k.rank };
        by_progress
            .then(previous(a).cmp(&previous(b)))
            .then(a.index.cmp(&b.index))
    });
}

/// `SimSet::Race` (after progress): ranks, `Standings`, the leader's lap and `Finished` events
/// (with the rank the finisher got).
pub(crate) fn update_standings(
    mut status: ResMut<RaceStatus>,
    mut standings: ResMut<Standings>,
    mut racers: Query<(Entity, &Racer, &mut RaceProgress, &Tracker)>,
    mut order: Local<Vec<(Entity, Key)>>,
    mut events: MessageWriter<RaceEvent>,
) {
    order.clear();
    order.extend(racers.iter().map(|(e, racer, progress, tracker)| {
        let key = Key {
            finish: tracker.finish,
            total_distance: progress.total_distance,
            rank: progress.rank,
            index: racer.index,
        };
        (e, key)
    }));
    sort_standings(&mut order);
    for (i, (car, key)) in order.iter().enumerate() {
        let rank = i as u8 + 1;
        if let Ok((.., mut progress, _)) = racers.get_mut(*car) {
            progress.rank = rank;
        }
        if let Some(time) = key.finish.filter(|&t| t == status.race_ticks && t > 0) {
            events.write(RaceEvent {
                racer: Some(*car),
                kind: RaceEventKind::Finished { rank, time },
            });
        }
    }
    if !standings.0.iter().copied().eq(order.iter().map(|o| o.0)) {
        standings.0.clear();
        standings.0.extend(order.iter().map(|o| o.0));
    }
    let leader_lap = order
        .first()
        .and_then(|(e, _)| racers.get(*e).ok())
        .map_or(0, |(_, _, p, _)| p.lap);
    if status.leader_lap != leader_lap {
        status.leader_lap = leader_lap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(finish: Option<u32>, total_distance: f32, rank: u8, index: usize) -> Key {
        Key {
            finish,
            total_distance,
            rank,
            index,
        }
    }

    #[test]
    fn standings_put_finishers_first_by_time_then_distance_and_keep_ties() {
        let mut order = vec![
            ('a', key(None, 500.0, 3, 0)),
            ('b', key(Some(900), 600.0, 2, 1)),
            ('c', key(None, 550.0, 4, 2)),
            ('d', key(Some(880), 600.0, 1, 3)),
            ('e', key(None, 550.0, 5, 4)),
            ('f', key(None, 550.0, 0, 5)),
        ];
        sort_standings(&mut order);
        let names: String = order.iter().map(|o| o.0).collect();
        assert_eq!(names, "dbcefa");
        let mut grid = vec![('x', key(None, -3.0, 0, 1)), ('y', key(None, -3.0, 0, 0))];
        sort_standings(&mut grid);
        assert_eq!(grid[0].0, 'y', "unranked ties fall back to the grid order");
        let mut same_tick = vec![
            ('p', key(Some(100), 600.0, 2, 0)),
            ('q', key(Some(100), 600.0, 1, 1)),
        ];
        sort_standings(&mut same_tick);
        assert_eq!(
            same_tick[0].0, 'q',
            "a same-tick finish keeps the previous order"
        );
    }
}
