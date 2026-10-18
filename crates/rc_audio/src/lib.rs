//! # rc_audio — sound (P)
//!
//! - **Responsibilities:** five kira buses (Music, Sfx, Engines, Ui, Ambience) whose volume
//!   effects follow `Settings.audio` (master × bus, mute when unfocused, music ducked while
//!   paused); one engine loop per racer and tyre/surface/drift/boost layers on local players'
//!   cars; positional one-shots from vehicle/item/race messages, loops on projectiles, beeps for
//!   lock-ons and goo fuses; UI cues; the music director (menu theme, race cue with a
//!   synchronized final-lap variant, results and cup stingers); ambience by environment, time of day and
//!   weather (thunder on lightning); pausing Engines/Sfx/Ambience while paused; the `"audio"`
//!   loading key (session music and ambience are preloaded, SFX stay resident).
//! - **Inputs:** `Settings`, `Catalog`, `RaceConfig`, `WeatherState`, `Racer`, `LocalPlayer`,
//!   `VehicleTelemetry`, `Charge`, `Projectile`, `Hazard`, `GameCamera` transforms (listeners),
//!   `CarImpact`, `CarLanded`, `CarRespawned`, `ItemEvent`, `HitEvent`, `Explosion`, `RaceEvent`,
//!   `WeatherEvent`, `UiSound`, `MusicStinger`, the lifecycle states, primary-window focus.
//! - **Outputs:** sound only, [`OutputState`], `LoadingDone("audio")`.
//! - **Must never:** panic or queue audio commands without an audio device (one probe at
//!   startup decides, then everything runs muted), or write gameplay state.
//!
//! Design note: `docs/design/rc_audio.md`; the sound files: `docs/design/sfx.md`.

mod ambience;
mod bank;
mod buses;
mod cars;
mod items;
mod music;
mod output;
mod shots;
mod spatial;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use output::OutputState;

/// Order of this crate's `Update` systems inside `AudioSet`: several share
/// `Assets<AudioInstance>`, the one-shot limiter, the bus controls or the output state.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Step {
    Probe,
    Load,
    Volumes,
    Listen,
    Music,
    Ambience,
    Cars,
    Shots,
    Items,
}

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "audio";

/// Audio buses, engines, effects, music and ambience.
pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_kira_audio::AudioPlugin>() {
            app.add_plugins(bevy_kira_audio::AudioPlugin);
        }
        LoadingTasks::register(app, LOADING_KEY);
        app.configure_sets(
            Update,
            (
                Step::Probe,
                Step::Load,
                Step::Volumes,
                Step::Listen,
                Step::Music,
                Step::Ambience,
                Step::Cars,
                Step::Shots,
                Step::Items,
            )
                .chain()
                .in_set(AudioSet),
        );
        buses::build(app);
        output::build(app);
        spatial::build(app);
        bank::build(app);
        music::build(app);
        ambience::build(app);
        cars::build(app);
        shots::build(app);
        items::build(app);
    }
}
