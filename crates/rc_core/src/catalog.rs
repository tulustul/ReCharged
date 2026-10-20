//! Identifiers and content metadata (ARCHITECTURE §4.1): cars, paints, tracks, cups, rivals,
//! unlocks. Loaded once in `Boot` from `assets/data/catalog.ron` by rc_game.
//!
//! [`Catalog::load`] is pure (std::fs + ron) and validates every cross-reference, including the
//! rule that every cup race uses content unlocked no later than the cup itself.

mod load;

use std::fmt;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::weather::{TimeOfDay, WeatherKind, WeatherSetup};

pub use load::CatalogError;

/// Path of the catalog file relative to the assets root.
pub const CATALOG_FILE: &str = "data/catalog.ron";

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Creates an id from anything string-like.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The id as a string slice (also the data/asset file stem).
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self(id.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// Car id, e.g. `"sparky"` (matches `vehicles/<id>.ron` and `models/cars/<id>.glb`).
    CarId
);
string_id!(
    /// Track id, e.g. `"toy_town"` (matches `tracks/<id>.ron`).
    TrackId
);
string_id!(
    /// Cup id, e.g. `"rookie_cup"`.
    CupId
);
string_id!(
    /// Rival id, e.g. `"dash"`.
    RivalId
);

/// Index of a paint within its car's paint list.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PaintId(pub u8);

/// Local player slot, `0..=3` (P1–P4).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PlayerSlot(pub u8);

impl PlayerSlot {
    /// Slot as an array index.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The slot's colour (sRGB 0–1), see [`crate::SLOT_COLORS`].
    pub fn color(self) -> [f32; 3] {
        crate::SLOT_COLORS[self.index() % crate::MAX_LOCAL_PLAYERS]
    }
}

/// Car class (GDD §4.1). Rivals drive the class of the cup or of the best human car.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum CarClass {
    /// Starter cars.
    #[default]
    Rookie,
    /// Unlocked by the Rookie Cup.
    Pro,
    /// Unlocked by the Pro and Elite Cups.
    Elite,
    /// Bolt; counts as Elite for rival car choice.
    Secret,
}

/// AI difficulty (GDD §8.2). `ElitePlus` exists only in the Grand Volt Cup.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Difficulty {
    /// Speed scale 0.86, frequent small mistakes.
    Rookie,
    /// Speed scale 0.94.
    #[default]
    Pro,
    /// Speed scale 1.00.
    Elite,
    /// Speed scale 1.02, smarter item hoarding.
    ElitePlus,
}

/// Driven wheels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Drivetrain {
    /// Rear-wheel drive.
    #[default]
    Rwd,
    /// Four-wheel drive (4WD).
    Awd,
}

/// Menu star ratings, 1–5 each; physics implements them (GDD §5.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CarStars {
    /// Top speed.
    pub speed: u8,
    /// Acceleration (0 → top time).
    pub accel: u8,
    /// Handling.
    pub handling: u8,
    /// Weight: pushes others, resists knock-back; less air control, costlier landings.
    pub weight: u8,
}

/// Paint set a paint belongs to; every non-base set adds one paint to every car (GDD §9).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum PaintSet {
    /// Always available (4 per car).
    #[default]
    Base,
    /// 3 Time-Trial gold medals.
    Neon,
    /// 6 Time-Trial gold medals.
    Carbon,
    /// 12 Time-Trial gold medals.
    Gold,
    /// Grand Volt Cup gold.
    Chrome,
}

impl PaintSet {
    /// The unlockable sets (everything but `Base`).
    pub const UNLOCKABLE: [PaintSet; 4] = [Self::Neon, Self::Carbon, Self::Gold, Self::Chrome];
}

/// One paint of a car (applied to the model's `paint` / `paint_accent` materials).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintInfo {
    /// Index within the car's paints.
    pub id: PaintId,
    /// Display name.
    pub name: String,
    /// Base colour, sRGB 0–1.
    pub base: [f32; 3],
    /// Accent colour, sRGB 0–1.
    pub accent: [f32; 3],
    /// PBR metallic, 0–1.
    pub metallic: f32,
    /// PBR perceptual roughness, 0–1.
    pub roughness: f32,
    /// Set this paint belongs to.
    pub set: PaintSet,
}

/// Engine sound family (GDD §16.2); plays the loop `assets/audio/sfx/<stem>.ogg`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineTimbre {
    /// Buggy whine.
    #[default]
    Buggy,
    /// Truck growl.
    Truck,
    /// Sports electric scream.
    Sports,
}

impl EngineTimbre {
    /// File stem under `audio/sfx/`, e.g. `"engine_buggy"`.
    pub fn stem(self) -> &'static str {
        match self {
            Self::Buggy => "engine_buggy",
            Self::Truck => "engine_truck",
            Self::Sports => "engine_sports",
        }
    }
}

/// Music cue (GDD §16.1); the stem is the file name under `audio/music/`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MusicCue {
    /// Front-end loop.
    #[default]
    MenuTheme,
    /// Race loop 1 (with a `_final` final-lap variant).
    Race1,
    /// Race loop 2.
    Race2,
    /// Race loop 3.
    Race3,
    /// Race loop 4.
    Race4,
    /// Results stinger.
    ResultsJingle,
    /// Podium stinger.
    PodiumFanfare,
    /// Cup win stinger.
    CupVictory,
}

impl MusicCue {
    /// File stem, e.g. `"race_1"` (final-lap variants append `"_final"`).
    pub fn stem(self) -> &'static str {
        match self {
            Self::MenuTheme => "menu_theme",
            Self::Race1 => "race_1",
            Self::Race2 => "race_2",
            Self::Race3 => "race_3",
            Self::Race4 => "race_4",
            Self::ResultsJingle => "results_jingle",
            Self::PodiumFanfare => "podium_fanfare",
            Self::CupVictory => "cup_victory",
        }
    }
}

/// A track layout: Forward/Reverse, optionally mirrored (24 variants of 6 tracks).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackVariant {
    /// The track.
    pub track: TrackId,
    /// Reverse layout.
    #[serde(default)]
    pub reverse: bool,
    /// Mirrored (X negated).
    #[serde(default)]
    pub mirror: bool,
}

impl TrackVariant {
    /// The forward, unmirrored layout of `track`.
    pub fn forward(track: impl Into<TrackId>) -> Self {
        Self {
            track: track.into(),
            reverse: false,
            mirror: false,
        }
    }
}

/// Condition that unlocks content (GDD §9).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnlockRule {
    /// Available from the start.
    #[default]
    Start,
    /// Finish the cup in the top 3.
    CupPodium(CupId),
    /// Win the cup.
    CupGold(CupId),
    /// Collect this many Time-Trial gold (or platinum) medals.
    GoldMedals(u32),
    /// Collect this many Golden Batteries.
    GoldenBatteries(u32),
}

/// Global unlockables that are not cars, tracks or cups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unlockable {
    /// Reverse layouts of all tracks.
    Reverse,
    /// Mirror mode.
    Mirror,
    /// An AI difficulty option in Quick Race / Versus.
    Difficulty(Difficulty),
    /// A paint set (one paint per car).
    PaintSet(PaintSet),
    /// The credits roll.
    Credits,
}

/// A car of the roster.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CarInfo {
    /// Id (file stem).
    pub id: CarId,
    /// Display name.
    pub name: String,
    /// Class.
    pub class: CarClass,
    /// Star ratings.
    pub stars: CarStars,
    /// Driven wheels.
    pub drive: Drivetrain,
    /// Model asset path, e.g. `"models/cars/sparky.glb"`.
    pub model: String,
    /// Paints: 4 base + one per unlockable set.
    pub paints: Vec<PaintInfo>,
    /// Unlock condition.
    pub unlock: UnlockRule,
    /// Engine sound family.
    pub engine: EngineTimbre,
}

/// A track (environment) with its allowed presets (GDD §7).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackInfo {
    /// Id (file stem).
    pub id: TrackId,
    /// Display name.
    pub name: String,
    /// Environment kind: barrier style, ambience, menu art (the lighting itself is the map's
    /// `maps/<id>/environment.ron`).
    pub environment: String,
    /// Indoor tracks get "outside weather" only (no grip effect).
    pub indoor: bool,
    /// Difficulty stars 1–3.
    pub difficulty: u8,
    /// Preview image asset path.
    pub preview: String,
    /// Default race music cue.
    pub music: MusicCue,
    /// Laps when not overridden (1–9).
    pub default_laps: u8,
    /// Allowed times of day.
    pub allowed_times: Vec<TimeOfDay>,
    /// Allowed weather presets.
    pub allowed_weather: Vec<WeatherKind>,
    /// Unlock condition of the forward layout (reverse/mirror add their own rules).
    pub unlock: UnlockRule,
    /// Dev tracks: never listed in menus or cups.
    #[serde(default)]
    pub hidden: bool,
}

/// One race of a cup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CupRace {
    /// Layout.
    pub variant: TrackVariant,
    /// Weather and time of day.
    pub weather: WeatherSetup,
    /// Lap override; `None` = the track's `default_laps`.
    #[serde(default)]
    pub laps: Option<u8>,
}

/// A championship cup (GDD §3.1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CupInfo {
    /// Id.
    pub id: CupId,
    /// Display name.
    pub name: String,
    /// AI difficulty of the cup.
    pub difficulty: Difficulty,
    /// Races in order.
    pub races: Vec<CupRace>,
    /// Unlock condition.
    pub unlock: UnlockRule,
}

/// A rival's car per class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassCars {
    /// Car in Rookie races.
    pub rookie: CarId,
    /// Car in Pro races.
    pub pro: CarId,
    /// Car in Elite (and Secret) races.
    pub elite: CarId,
}

impl ClassCars {
    /// The car for a race of `class` (Secret counts as Elite).
    pub fn for_class(&self, class: CarClass) -> &CarId {
        match class {
            CarClass::Rookie => &self.rookie,
            CarClass::Pro => &self.pro,
            CarClass::Elite | CarClass::Secret => &self.elite,
        }
    }
}

/// A named AI rival (GDD §8.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RivalInfo {
    /// Id.
    pub id: RivalId,
    /// Display name.
    pub name: String,
    /// Racer colour, sRGB 0–1.
    pub color: [f32; 3],
    /// Personality: aggression 0–1.
    pub aggression: f32,
    /// Personality: consistency 0–1.
    pub consistency: f32,
    /// Car per class.
    pub cars: ClassCars,
}

/// All content metadata. Written by rc_game in `Boot`; read-only afterwards.
#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// Car roster.
    pub cars: Vec<CarInfo>,
    /// Tracks, including hidden dev tracks.
    pub tracks: Vec<TrackInfo>,
    /// Championship cups in menu order.
    pub cups: Vec<CupInfo>,
    /// AI rivals.
    pub rivals: Vec<RivalInfo>,
    /// Global unlockables and their rules.
    pub unlocks: Vec<(Unlockable, UnlockRule)>,
}
