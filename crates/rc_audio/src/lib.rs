//! # rc_audio — sound (P)
//!
//! - **Responsibilities:** kira buses (Music, Sfx, Engines, Ui, Ambience) driven by
//!   `Settings.audio`; engines per racer; one-shots from messages; projectile/hazard loops; UI
//!   cues; the music director; ambience; pausing Engines/Sfx/Ambience while paused.
//! - **Inputs:** `Settings`, `Catalog` (engine timbres, music cues), `VehicleTelemetry`,
//!   vehicle/item/race/weather messages, `UiSound`, `GameCamera` (listeners), lifecycle states.
//! - **Outputs:** sound only, `LoadingDone("audio")`.
//! - **Must never:** panic without an audio device, or write gameplay state.
//!
//! Status: skeleton (T01); implemented by T09.

use bevy::prelude::*;
use rc_core::prelude::*;

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "audio";

/// Audio buses, engines, effects, music and ambience.
pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_systems(OnEnter(AppState::Loading), report_loaded);
    }
}

// ponytail: stub — reports "loaded" at once so the gate opens; T09 preloads the banks first.
fn report_loaded(mut done: MessageWriter<LoadingDone>) {
    done.write(LoadingDone(LOADING_KEY));
}
