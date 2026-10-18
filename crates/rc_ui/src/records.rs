//! `records.*` channel: best laps, best races and medals per layout from rc_game's [`Profile`].

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::{Medal, Profile, TimedRecord};

use crate::results::ticks_to_ms;

/// A Time-Trial medal.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UiMedal {
    /// Bronze.
    Bronze,
    /// Silver.
    Silver,
    /// Gold.
    Gold,
    /// Platinum.
    Platinum,
}

impl From<Medal> for UiMedal {
    fn from(medal: Medal) -> Self {
        match medal {
            Medal::Bronze => Self::Bronze,
            Medal::Silver => Self::Silver,
            Medal::Gold => Self::Gold,
            Medal::Platinum => Self::Platinum,
        }
    }
}

/// A record time and the car that set it.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiRecordTime {
    /// Time in milliseconds.
    pub ms: u32,
    /// Car id.
    pub car: String,
}

impl From<&TimedRecord> for UiRecordTime {
    fn from(record: &TimedRecord) -> Self {
        Self {
            ms: ticks_to_ms(record.ticks),
            car: record.car.0.clone(),
        }
    }
}

/// Records of one layout.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiLayoutRecord {
    /// Track id.
    pub track: String,
    /// Reverse layout.
    pub reverse: bool,
    /// Mirrored.
    pub mirror: bool,
    /// Best lap (any single-player mode).
    pub best_lap: Option<UiRecordTime>,
    /// Best race time (Time Trial).
    pub best_race: Option<UiRecordTime>,
    /// Best Time-Trial medal.
    pub medal: Option<UiMedal>,
}

/// Response of `records.get`: every layout with at least one record, in menu order.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiRecords {
    /// Layout records.
    pub layouts: Vec<UiLayoutRecord>,
}

impl UiRecords {
    /// Records of every listed track's four layouts.
    pub fn new(profile: &Profile, catalog: &Catalog) -> Self {
        let mut layouts = Vec::new();
        for track in catalog.tracks.iter().filter(|t| !t.hidden) {
            for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
                let variant = TrackVariant {
                    track: track.id.clone(),
                    reverse,
                    mirror,
                };
                let Some(record) = profile.record(&variant) else {
                    continue;
                };
                if record.best_lap.is_none() && record.best_race.is_none() && record.medal.is_none()
                {
                    continue;
                }
                layouts.push(UiLayoutRecord {
                    track: track.id.0.clone(),
                    reverse,
                    mirror,
                    best_lap: record.best_lap.as_ref().map(Into::into),
                    best_race: record.best_race.as_ref().map(Into::into),
                    medal: record.medal.map(Into::into),
                });
            }
        }
        Self { layouts }
    }
}

/// React → Bevy request `records.get`: the [`UiRecords`].
#[react_request(name = "records.get", response = UiRecords)]
pub struct GetRecords;

pub(crate) fn register(app: &mut App) {
    app.add_react_request_handler(
        |request: On<Request<GetRecords>>,
         profile: Option<Res<Profile>>,
         catalog: Option<Res<Catalog>>| match (profile, catalog) {
            (Some(profile), Some(catalog)) => request.respond(UiRecords::new(&profile, &catalog)),
            _ => request.respond_err("profile not loaded"),
        },
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn records_list_only_layouts_with_a_record() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let catalog = Catalog::load(&assets).expect("the shipped catalog loads");
        let mut profile = Profile::default();
        let variant = TrackVariant {
            track: TrackId::new("toy_town"),
            reverse: true,
            mirror: false,
        };
        profile.records.insert(
            rc_game::variant_key(&variant),
            rc_game::LayoutRecord {
                best_lap: Some(TimedRecord {
                    ticks: 3600,
                    car: CarId::new("zip"),
                }),
                medal: Some(Medal::Gold),
                ..default()
            },
        );
        let records = UiRecords::new(&profile, &catalog);
        assert_eq!(records.layouts.len(), 1);
        let r = &records.layouts[0];
        assert_eq!(
            (r.track.as_str(), r.reverse, r.mirror),
            ("toy_town", true, false)
        );
        assert_eq!(r.best_lap.as_ref().map(|t| t.ms), Some(60_000));
        assert_eq!(r.medal, Some(UiMedal::Gold));
    }
}
