//! Weather & time-of-day contracts (GDD §10, ARCHITECTURE §4.2, §5.4).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Weather preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherKind {
    /// Sharp shadows.
    #[default]
    Clear,
    /// Overcast, soft shadows.
    Cloudy,
    /// Rain: wetness → grip (outdoors).
    Rain,
    /// Heavy rain, wind gusts, lightning.
    Storm,
    /// Dense distance fog.
    Fog,
    /// Falling snow, snow cover → grip (outdoors).
    Snow,
}

impl WeatherKind {
    /// Every preset.
    pub const ALL: [WeatherKind; 6] = [
        Self::Clear,
        Self::Cloudy,
        Self::Rain,
        Self::Storm,
        Self::Fog,
        Self::Snow,
    ];
}

/// Time of day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeOfDay {
    /// Low warm sun.
    Morning,
    /// High sun.
    #[default]
    Noon,
    /// Golden hour.
    Sunset,
    /// Lamps on, headlights on local players' cars.
    Night,
}

impl TimeOfDay {
    /// Every time of day.
    pub const ALL: [TimeOfDay; 4] = [Self::Morning, Self::Noon, Self::Sunset, Self::Night];
}

/// How the race weather is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherChoice {
    /// The same preset all race.
    Fixed(WeatherKind),
    /// One of the track's allowed presets, picked with the race seed.
    Random,
    /// Outdoor tracks: one seeded change at 30–70 % race distance to a neighbouring preset.
    Dynamic {
        /// Preset at the start.
        start: WeatherKind,
    },
}

impl Default for WeatherChoice {
    fn default() -> Self {
        Self::Fixed(WeatherKind::Clear)
    }
}

/// Weather + time of day of a race.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeatherSetup {
    /// Weather choice.
    pub weather: WeatherChoice,
    /// Time of day.
    pub time: TimeOfDay,
}

/// The current weather, blended. Written only by rc_weather; reset in `OnEnter(RacePhase::Setup)`.
///
/// Outside races it holds the default (Clear, Noon); it is absent only during the one
/// `AppState::Unloading` frame, so readers outside `FixedUpdate` take `Option<Res<_>>`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WeatherState {
    /// Current (or target, while blending) preset.
    pub kind: WeatherKind,
    /// Time of day.
    pub time: TimeOfDay,
    /// Precipitation intensity 0–1: snow when `snowing`, rain otherwise.
    pub precipitation: f32,
    /// Ground wetness 0–1 (always 0 on indoor tracks).
    pub wetness: f32,
    /// Snow cover 0–1 (always 0 on indoor tracks).
    pub snow_cover: f32,
    /// Wind velocity in u/s (gusts included; zero on indoor tracks).
    pub wind: Vec3,
    /// Fog density 0–1.
    pub fog: f32,
    /// Cloud cover 0–1.
    pub cloudiness: f32,
    /// Current lightning flash brightness 0–1.
    pub lightning: f32,
    /// Indoor track: weather is only visible/audible outside (no grip effect).
    pub outside_only: bool,
    /// Precipitation falls as snow: `Snow`, or a change to or from it (where `kind` alone cannot
    /// tell, e.g. snow still fading out after `kind` became `Cloudy`).
    pub snowing: bool,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            kind: WeatherKind::Clear,
            time: TimeOfDay::Noon,
            precipitation: 0.0,
            wetness: 0.0,
            snow_cover: 0.0,
            wind: Vec3::ZERO,
            fog: 0.0,
            cloudiness: 0.0,
            lightning: 0.0,
            outside_only: false,
            snowing: false,
        }
    }
}

/// Weather happenings (producer rc_weather; consumers render, vfx, audio).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum WeatherEvent {
    /// A lightning strike (flash + thunder).
    Lightning {
        /// Flash intensity 0–1.
        intensity: f32,
    },
    /// The dynamic weather started changing to this preset.
    Changed(WeatherKind),
}
