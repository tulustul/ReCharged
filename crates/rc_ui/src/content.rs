//! The content vocabulary shared by the catalog, profile, race and settings channels: classes,
//! difficulties, paint sets, weather, times of day and unlock rules as React sees them.

use rc_core::prelude::*;

mirror_enum!(
    /// [`CarClass`].
    UiCarClass = CarClass { Rookie, Pro, Elite, Secret }
);
mirror_enum!(
    /// [`Difficulty`].
    UiDifficulty = Difficulty { Rookie, Pro, Elite, ElitePlus }
);
mirror_enum!(
    /// [`Drivetrain`].
    UiDrive = Drivetrain { Rwd, Awd }
);
mirror_enum!(
    /// [`PaintSet`].
    UiPaintSet = PaintSet { Base, Neon, Carbon, Gold, Chrome }
);
mirror_enum!(
    /// [`TimeOfDay`].
    UiTimeOfDay = TimeOfDay { Morning, Noon, Sunset, Night }
);
mirror_enum!(
    /// [`WeatherKind`].
    UiWeatherKind = WeatherKind { Clear, Cloudy, Rain, Storm, Fog, Snow }
);

/// [`WeatherChoice`].
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiWeatherChoice {
    /// The same preset all race.
    Fixed {
        /// The preset.
        weather: UiWeatherKind,
    },
    /// One of the track's presets, picked with the seed.
    Random,
    /// Outdoor tracks: one change during the race.
    Dynamic {
        /// Preset at the start.
        start: UiWeatherKind,
    },
}

impl From<WeatherChoice> for UiWeatherChoice {
    fn from(choice: WeatherChoice) -> Self {
        match choice {
            WeatherChoice::Fixed(weather) => Self::Fixed {
                weather: weather.into(),
            },
            WeatherChoice::Random => Self::Random,
            WeatherChoice::Dynamic { start } => Self::Dynamic {
                start: start.into(),
            },
        }
    }
}

impl From<UiWeatherChoice> for WeatherChoice {
    fn from(choice: UiWeatherChoice) -> Self {
        match choice {
            UiWeatherChoice::Fixed { weather } => Self::Fixed(weather.into()),
            UiWeatherChoice::Random => Self::Random,
            UiWeatherChoice::Dynamic { start } => Self::Dynamic {
                start: start.into(),
            },
        }
    }
}

/// [`UnlockRule`]: why something is locked (React words it).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiUnlockRule {
    /// Available from the start.
    Start,
    /// Podium in `cup`.
    CupPodium {
        /// Cup id.
        cup: String,
    },
    /// Win `cup`.
    CupGold {
        /// Cup id.
        cup: String,
    },
    /// Collect `count` Time-Trial gold medals.
    GoldMedals {
        /// Medals needed.
        count: u32,
    },
    /// Collect `count` Golden Batteries.
    GoldenBatteries {
        /// Batteries needed.
        count: u32,
    },
}

impl From<&UnlockRule> for UiUnlockRule {
    fn from(rule: &UnlockRule) -> Self {
        match rule {
            UnlockRule::Start => Self::Start,
            UnlockRule::CupPodium(cup) => Self::CupPodium { cup: cup.0.clone() },
            UnlockRule::CupGold(cup) => Self::CupGold { cup: cup.0.clone() },
            UnlockRule::GoldMedals(count) => Self::GoldMedals { count: *count },
            UnlockRule::GoldenBatteries(count) => Self::GoldenBatteries { count: *count },
        }
    }
}

/// [`Unlockable`]: global unlocks that are not cars, tracks or cups.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiUnlockable {
    /// Reverse layouts.
    Reverse,
    /// Mirror mode.
    Mirror,
    /// An AI difficulty option.
    Difficulty {
        /// The difficulty.
        difficulty: UiDifficulty,
    },
    /// A paint set.
    PaintSet {
        /// The set.
        set: UiPaintSet,
    },
    /// The credits roll.
    Credits,
}

impl From<Unlockable> for UiUnlockable {
    fn from(item: Unlockable) -> Self {
        match item {
            Unlockable::Reverse => Self::Reverse,
            Unlockable::Mirror => Self::Mirror,
            Unlockable::Difficulty(d) => Self::Difficulty {
                difficulty: d.into(),
            },
            Unlockable::PaintSet(set) => Self::PaintSet { set: set.into() },
            Unlockable::Credits => Self::Credits,
        }
    }
}

impl From<UiUnlockable> for Unlockable {
    fn from(item: UiUnlockable) -> Self {
        match item {
            UiUnlockable::Reverse => Self::Reverse,
            UiUnlockable::Mirror => Self::Mirror,
            UiUnlockable::Difficulty { difficulty } => Self::Difficulty(difficulty.into()),
            UiUnlockable::PaintSet { set } => Self::PaintSet(set.into()),
            UiUnlockable::Credits => Self::Credits,
        }
    }
}

/// A global unlockable and its rule.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiUnlock {
    /// What unlocks.
    pub item: UiUnlockable,
    /// When.
    pub rule: UiUnlockRule,
}
