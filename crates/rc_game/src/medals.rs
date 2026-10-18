//! Time Trial medal targets (`assets/data/medals.ron`, GDD §9): one reference time per layout
//! (Elite AI in a Phantom, items and catch-up off, 3 laps); each medal's target is the reference
//! × [`Medal::factor`].

use std::collections::BTreeMap;
use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use serde::Deserialize;

use crate::profile::{Medal, variant_key};

/// Path of the medal file relative to the assets root.
pub const MEDALS_FILE: &str = "data/medals.ron";

#[derive(Deserialize)]
struct MedalsFile {
    reference_secs: BTreeMap<String, f32>,
}

/// Medal reference times per layout. Written by rc_game in `Boot`; read by rc_ui.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MedalTable {
    reference_secs: BTreeMap<String, f32>,
}

/// Target times of one layout, in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MedalTargets {
    /// Best first: Platinum, Gold, Silver, Bronze (see [`Medal::BEST_FIRST`]).
    pub ticks: [u32; 4],
}

impl MedalTargets {
    /// Target of one medal.
    pub fn target(&self, medal: Medal) -> u32 {
        let i = Medal::BEST_FIRST.iter().position(|m| *m == medal);
        self.ticks[i.unwrap_or(3)]
    }

    /// The best medal a race time earns.
    pub fn medal_for(&self, time_ticks: u32) -> Option<Medal> {
        Medal::BEST_FIRST
            .into_iter()
            .find(|&m| time_ticks <= self.target(m))
    }

    /// The next better medal than `medal` (`None` = Bronze is next) and its target.
    pub fn next(&self, medal: Option<Medal>) -> Option<(Medal, u32)> {
        let better = |m: &Medal| medal.is_none_or(|have| *m > have);
        let next = Medal::BEST_FIRST.into_iter().rev().find(better)?;
        Some((next, self.target(next)))
    }
}

impl MedalTable {
    /// Reads `<assets_dir>/data/medals.ron`.
    pub fn load(assets_dir: &Path) -> Result<Self, String> {
        let path = assets_dir.join(MEDALS_FILE);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Parses and validates medal RON (every reference time positive and finite).
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: MedalsFile = ron::from_str(text).map_err(|e| e.to_string())?;
        if let Some((key, secs)) = file
            .reference_secs
            .iter()
            .find(|(_, s)| !s.is_finite() || **s <= 0.0)
        {
            return Err(format!(
                "reference_secs[\"{key}\"]: {secs} is not a positive time"
            ));
        }
        Ok(Self {
            reference_secs: file.reference_secs,
        })
    }

    /// Targets of a layout (`None` = not calibrated: no medals there).
    pub fn targets(&self, variant: &TrackVariant) -> Option<MedalTargets> {
        let reference = *self.reference_secs.get(&variant_key(variant))?;
        Some(MedalTargets {
            ticks: Medal::BEST_FIRST.map(|m| secs_to_ticks(reference * m.factor())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::catalog;

    fn table() -> MedalTable {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        MedalTable::load(&assets).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn medal_file_covers_every_layout() {
        let (table, catalog) = (table(), catalog());
        for track in &catalog.tracks {
            for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
                let v = TrackVariant {
                    track: track.id.clone(),
                    reverse,
                    mirror,
                };
                assert!(
                    table.targets(&v).is_some(),
                    "{} has no medal targets",
                    variant_key(&v)
                );
            }
        }
    }

    #[test]
    fn medals_scale_the_reference_time() {
        let table = MedalTable::parse(r#"(reference_secs: {"toy_town": 100.0})"#).expect("parses");
        let t = table
            .targets(&TrackVariant::forward("toy_town"))
            .expect("targets");
        assert_eq!(t.ticks, [5700, 5880, 6300, 6900]);
        assert_eq!(t.medal_for(5700), Some(Medal::Platinum));
        assert_eq!(t.medal_for(5701), Some(Medal::Gold));
        assert_eq!(t.medal_for(6900), Some(Medal::Bronze));
        assert_eq!(t.medal_for(6901), None);
        assert_eq!(t.next(None), Some((Medal::Bronze, 6900)));
        assert_eq!(t.next(Some(Medal::Silver)), Some((Medal::Gold, 5880)));
        assert_eq!(t.next(Some(Medal::Platinum)), None);
        assert!(
            table
                .targets(&TrackVariant::forward("sofa_circuit"))
                .is_none()
        );
    }

    #[test]
    fn bad_medal_files_are_rejected() {
        assert!(MedalTable::parse(r#"(reference_secs: {"a": 0.0})"#).is_err());
        assert!(MedalTable::parse("(reference: {})").is_err());
    }
}
