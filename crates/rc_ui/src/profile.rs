//! `profile.*` channels: unlocks (and NEW badges), trophies, collectibles and stats from rc_game's
//! [`Profile`]. Records: [`crate::records`].

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::{Content, GameCommand, Profile, Trophy};

use crate::content::{UiDifficulty, UiPaintSet, UiUnlockable};

/// What the player may use.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiUnlocked {
    /// Car ids.
    pub cars: Vec<String>,
    /// Track ids (forward layouts).
    pub tracks: Vec<String>,
    /// Cup ids.
    pub cups: Vec<String>,
    /// Reverse layouts.
    pub reverse: bool,
    /// Mirror mode.
    pub mirror: bool,
    /// AI difficulty options.
    pub difficulties: Vec<UiDifficulty>,
    /// Paint sets (always includes `base`).
    pub paint_sets: Vec<UiPaintSet>,
    /// The credits roll after a Grand Volt win.
    pub credits: bool,
}

/// A cup trophy.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UiTrophy {
    /// 3rd.
    Bronze,
    /// 2nd.
    Silver,
    /// 1st.
    Gold,
}

impl From<Trophy> for UiTrophy {
    fn from(trophy: Trophy) -> Self {
        match trophy {
            Trophy::Bronze => Self::Bronze,
            Trophy::Silver => Self::Silver,
            Trophy::Gold => Self::Gold,
        }
    }
}

/// The best trophy won in a cup.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiCupTrophy {
    /// Cup id.
    pub cup: String,
    /// Best trophy.
    pub trophy: UiTrophy,
}

/// Career statistics.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiStats {
    /// Races finished.
    pub races: u32,
    /// Wins.
    pub wins: u32,
    /// Podiums.
    pub podiums: u32,
    /// Distance driven in km (toy scale, like the speedometer).
    pub distance_km: f32,
    /// Items used.
    pub items_used: u32,
    /// Item hits landed.
    pub hits_landed: u32,
    /// Time played in seconds.
    pub time_played_secs: u32,
}

/// Something unlockable (NEW badges, unlock toasts).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiUnlockedItem {
    /// A car.
    Car {
        /// Car id.
        id: String,
    },
    /// A track.
    Track {
        /// Track id.
        id: String,
    },
    /// A cup.
    Cup {
        /// Cup id.
        id: String,
    },
    /// A global unlockable.
    Item {
        /// What.
        item: UiUnlockable,
    },
}

impl From<&Content> for UiUnlockedItem {
    fn from(content: &Content) -> Self {
        match content {
            Content::Car(id) => Self::Car { id: id.0.clone() },
            Content::Track(id) => Self::Track { id: id.0.clone() },
            Content::Cup(id) => Self::Cup { id: id.0.clone() },
            Content::Feature(item) => Self::Item {
                item: (*item).into(),
            },
        }
    }
}

impl From<&UiUnlockedItem> for Content {
    fn from(item: &UiUnlockedItem) -> Self {
        match item {
            UiUnlockedItem::Car { id } => Self::Car(CarId::new(id.as_str())),
            UiUnlockedItem::Track { id } => Self::Track(TrackId::new(id.as_str())),
            UiUnlockedItem::Cup { id } => Self::Cup(CupId::new(id.as_str())),
            UiUnlockedItem::Item { item } => Self::Feature((*item).into()),
        }
    }
}

/// Bevy → React event `profile.changed` (while in the menus) and the `profile.get` response.
#[react_event(name = "profile.changed")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiProfile {
    /// Unlocked content.
    pub unlocked: UiUnlocked,
    /// Unlocked but not yet viewed ("NEW" badges; `profile.seen` clears one).
    pub fresh: Vec<UiUnlockedItem>,
    /// Best trophy per cup (cups without one are absent).
    pub trophies: Vec<UiCupTrophy>,
    /// Tracks whose Golden Battery was collected.
    pub golden_batteries: Vec<String>,
    /// Time-Trial gold (or platinum) medals.
    pub gold_medals: u32,
    /// Career statistics.
    pub stats: UiStats,
}

impl UiProfile {
    /// The menu view of `profile` (unlock queries apply `--unlock-all`).
    pub fn new(profile: &Profile, catalog: &Catalog) -> Self {
        let open = |content: Content| profile.is_unlocked(catalog, &content);
        let feature = |item: Unlockable| open(Content::Feature(item));
        let stats = &profile.stats;
        Self {
            unlocked: UiUnlocked {
                cars: ids(catalog.cars.iter().map(|c| &c.id.0), |id| {
                    open(Content::Car(CarId::new(id)))
                }),
                tracks: ids(
                    catalog.tracks.iter().filter(|t| !t.hidden).map(|t| &t.id.0),
                    |id| open(Content::Track(TrackId::new(id))),
                ),
                cups: ids(catalog.cups.iter().map(|c| &c.id.0), |id| {
                    open(Content::Cup(CupId::new(id)))
                }),
                reverse: feature(Unlockable::Reverse),
                mirror: feature(Unlockable::Mirror),
                difficulties: [Difficulty::Rookie, Difficulty::Pro, Difficulty::Elite]
                    .into_iter()
                    .filter(|&d| feature(Unlockable::Difficulty(d)))
                    .map(Into::into)
                    .collect(),
                paint_sets: std::iter::once(PaintSet::Base)
                    .chain(
                        PaintSet::UNLOCKABLE
                            .into_iter()
                            .filter(|&s| feature(Unlockable::PaintSet(s))),
                    )
                    .map(Into::into)
                    .collect(),
                credits: feature(Unlockable::Credits),
            },
            fresh: profile.new_unlocks.iter().map(Into::into).collect(),
            trophies: profile
                .trophies
                .iter()
                .map(|(cup, &trophy)| UiCupTrophy {
                    cup: cup.0.clone(),
                    trophy: trophy.into(),
                })
                .collect(),
            golden_batteries: profile
                .golden_batteries
                .iter()
                .map(|t| t.0.clone())
                .collect(),
            gold_medals: profile.gold_medals(),
            stats: UiStats {
                races: stats.races,
                wins: stats.wins,
                podiums: stats.podiums,
                distance_km: (stats.distance * f64::from(SPEED_DISPLAY_KMH) / 3600.0) as f32,
                items_used: stats.items_used,
                hits_landed: stats.hits_landed,
                time_played_secs: stats.time_played_secs as u32,
            },
        }
    }
}

fn ids<'a>(all: impl Iterator<Item = &'a String>, keep: impl Fn(&str) -> bool) -> Vec<String> {
    all.filter(|id| keep(id)).cloned().collect()
}

/// Bevy → React event `profile.unlocked`: content was just unlocked (toasts; also NEW badges).
#[react_event(name = "profile.unlocked")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unlocked {
    /// The unlocks.
    pub items: Vec<UiUnlockedItem>,
}

/// React → Bevy request `profile.get`: the [`UiProfile`].
#[react_request(name = "profile.get", response = UiProfile)]
pub struct GetProfile;

/// React → Bevy message `profile.seen`: a NEW item was viewed (clears its badge).
#[react_message(name = "profile.seen")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MarkSeen {
    /// The item.
    pub item: UiUnlockedItem,
}

/// React → Bevy message `profile.reset`: erase all progress (React asked twice).
#[react_message(name = "profile.reset")]
pub struct ResetProgress;

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<Unlocked>()
        .add_react_event::<UiProfile>()
        .add_react_request_handler(
            |request: On<Request<GetProfile>>,
             profile: Option<Res<Profile>>,
             catalog: Option<Res<Catalog>>| match (profile, catalog) {
                (Some(profile), Some(catalog)) => {
                    request.respond(UiProfile::new(&profile, &catalog))
                }
                _ => request.respond_err("profile not loaded"),
            },
        )
        .add_react_handler(|seen: On<MarkSeen>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::MarkSeen((&seen.event().item).into()));
        })
        .add_react_handler(
            |_: On<ResetProgress>, mut out: MessageWriter<GameCommand>| {
                out.write(GameCommand::ResetProgress);
            },
        );
}

/// Sends `profile.changed` when the profile changes while the menus are up.
pub(crate) fn push_profile(
    state: Res<State<AppState>>,
    profile: Option<Res<Profile>>,
    catalog: Option<Res<Catalog>>,
    events: ReactEvents,
) {
    let (Some(profile), Some(catalog)) = (profile, catalog) else {
        return;
    };
    let entered_menu = state.is_changed() && *state.get() == AppState::Menu;
    if *state.get() == AppState::Menu && (profile.is_changed() || entered_menu) {
        events.send(&UiProfile::new(&profile, &catalog));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn catalog() -> Catalog {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Catalog::load(&assets).expect("the shipped catalog loads")
    }

    #[test]
    fn a_new_profile_has_the_start_content_only() {
        let profile = UiProfile::new(&Profile::default(), &catalog());
        let u = &profile.unlocked;
        assert_eq!(u.cars, ["sparky", "dumpster", "zip"]);
        assert_eq!(u.cups, ["rookie_cup"]);
        assert!(u.tracks.contains(&"toy_town".to_string()));
        assert!(!u.tracks.contains(&"maple_street".to_string()));
        assert!(
            !u.tracks.contains(&"test_oval".to_string()),
            "dev tracks are never listed"
        );
        assert!(!u.reverse && !u.mirror && !u.credits);
        assert_eq!(u.difficulties, [UiDifficulty::Rookie, UiDifficulty::Pro]);
        assert_eq!(u.paint_sets, [UiPaintSet::Base]);
    }

    #[test]
    fn unlock_all_opens_everything_but_elite_plus() {
        let catalog = catalog();
        let profile = Profile {
            unlock_all: true,
            ..default()
        };
        let u = UiProfile::new(&profile, &catalog).unlocked;
        assert_eq!(u.cars.len(), catalog.cars.len());
        assert_eq!(u.cups.len(), catalog.cups.len());
        assert!(u.reverse && u.mirror && u.credits);
        assert_eq!(u.paint_sets.len(), 5);
        assert_eq!(u.difficulties.len(), 3);
    }

    #[test]
    fn unlock_items_round_trip_to_content() {
        let content = Content::Feature(Unlockable::PaintSet(PaintSet::Neon));
        let item = UiUnlockedItem::from(&content);
        assert_eq!(Content::from(&item), content);
    }
}
