//! Item odds by race position (GDD §6.3): the band of a rank and the keyed roulette draw.

use std::collections::HashMap;

use rc_core::prelude::*;
use serde::Deserialize;

/// Odds band of a race position (GDD §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    /// The leader.
    First,
    /// Up front: f ≤ 0.4.
    Top,
    /// f < 0.7.
    Middle,
    /// The back of the field.
    Last,
}

impl Band {
    /// Band of `rank` (1-based) among `racers`: f = (rank − 1) / (N − 1); rank 1 → First,
    /// f ≤ 0.4 → Top, f < 0.7 → Middle, else Last. Integer maths, so the 0.4 boundary is exact.
    pub fn of(rank: usize, racers: usize) -> Band {
        if rank <= 1 || racers <= 1 {
            return Band::First;
        }
        let (num, den) = (rank.min(racers) - 1, racers - 1);
        if 10 * num <= 4 * den {
            Band::Top
        } else if 10 * num < 7 * den {
            Band::Middle
        } else {
            Band::Last
        }
    }
}

/// Item weights of one band; missing items weigh 0.
pub type OddsRow = HashMap<ItemKind, u32>;

/// The odds table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Odds {
    /// Rank 1.
    pub first: OddsRow,
    /// f ≤ 0.4.
    pub top: OddsRow,
    /// f < 0.7.
    pub middle: OddsRow,
    /// The rest.
    pub last: OddsRow,
}

impl Odds {
    /// Weights of `band` in `ItemKind::ALL` order.
    pub fn weights(&self, band: Band) -> [u32; 10] {
        let row = match band {
            Band::First => &self.first,
            Band::Top => &self.top,
            Band::Middle => &self.middle,
            Band::Last => &self.last,
        };
        ItemKind::ALL.map(|k| row.get(&k).copied().unwrap_or(0))
    }

    /// The keyed roulette draw of `band` (`RngStream::ItemRoll`, key = racer index).
    pub fn roll(&self, band: Band, seed: u64, tick: u64, racer: usize) -> Option<ItemKind> {
        let weights = self.weights(band);
        rng::weighted(seed, tick, RngStream::ItemRoll, racer as u64, &weights)
            .map(|i| ItemKind::ALL[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ItemTuning;

    fn odds() -> Odds {
        ItemTuning::parse(include_str!("../../../assets/data/items.ron"))
            .expect("items.ron")
            .odds
    }

    #[test]
    fn bands_follow_the_formula_for_every_field_size() {
        use Band::*;
        let expected: [(usize, &[Band]); 7] = [
            (2, &[First, Last]),
            (3, &[First, Middle, Last]),
            (4, &[First, Top, Middle, Last]),
            (5, &[First, Top, Middle, Last, Last]),
            (6, &[First, Top, Top, Middle, Last, Last]),
            (7, &[First, Top, Top, Middle, Middle, Last, Last]),
            (8, &[First, Top, Top, Middle, Middle, Last, Last, Last]),
        ];
        for (n, bands) in expected {
            let got: Vec<Band> = (1..=n).map(|rank| Band::of(rank, n)).collect();
            assert_eq!(got, bands, "N = {n}");
            // Cross-check against the float formula of the GDD.
            for rank in 2..=n {
                let f = (rank - 1) as f64 / (n - 1) as f64;
                let gdd = if f <= 0.4 + 1e-9 {
                    Top
                } else if f < 0.7 - 1e-9 {
                    Middle
                } else {
                    Last
                };
                assert_eq!(Band::of(rank, n), gdd, "rank {rank} of {n}");
            }
        }
        assert_eq!(Band::of(1, 1), First);
    }

    #[test]
    fn roll_is_keyed_and_respects_zero_weights() {
        let odds = odds();
        for key in 0..2000 {
            let a = odds.roll(Band::First, 9, 100 + key, 3);
            assert_eq!(
                a,
                odds.roll(Band::First, 9, 100 + key, 3),
                "same keys, same item"
            );
            let a = a.expect("an item");
            assert!(
                odds.weights(Band::First)[ItemKind::ALL.iter().position(|&k| k == a).unwrap()] > 0
            );
        }
    }
}
