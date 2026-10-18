//! The five buses: a kira sub-track each with a volume effect (a channel's own `set_volume` would
//! overwrite every instance's volume, see `docs/tech/kira-notes.md`), driven by
//! `Settings.audio` (master × bus), muted while the window is unfocused (setting), Music ducked
//! and Engines/Sfx/Ambience paused while the race is paused. Ambience also carries a low-pass
//! filter (fog muffling).

use std::time::Duration;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_kira_audio::effect::Mix as WetDry;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;
use rc_core::settings::AudioSettings as AudioPrefs;

use crate::Step;
use crate::output::audio_on;

/// Music bus marker (channel resource type).
#[derive(Resource)]
pub(crate) struct MusicBus;
/// Sound-effects bus marker.
#[derive(Resource)]
pub(crate) struct SfxBus;
/// Engines bus marker.
#[derive(Resource)]
pub(crate) struct EngineBus;
/// UI bus marker.
#[derive(Resource)]
pub(crate) struct UiBus;
/// Ambience bus marker.
#[derive(Resource)]
pub(crate) struct AmbienceBus;

/// Quietest level we ever ask for; kira treats −60 dB as silence.
pub(crate) const SILENCE_DB: f32 = -60.0;
/// Music level change while the pause menu is open, dB.
const PAUSE_DUCK_DB: f32 = -10.0;
/// Fade of bus volume changes (settings sliders, focus), ms.
const VOLUME_TWEEN_MS: u64 = 150;
/// Fade when pausing/resuming the Engines/Sfx/Ambience buses, ms.
const PAUSE_TWEEN_MS: u64 = 120;

/// Volume (and ambience filter) handles of the bus tracks.
#[derive(Resource)]
pub(crate) struct BusControls {
    music: VolumeControlHandle,
    sfx: VolumeControlHandle,
    engines: VolumeControlHandle,
    ui: VolumeControlHandle,
    ambience: VolumeControlHandle,
    /// Low-pass on the ambience bus; dry unless fog muffles it.
    pub(crate) ambience_filter: FilterHandle,
}

pub(crate) fn build(app: &mut App) {
    let volume =
        |track: &mut AudioTrack| track.add_effect(VolumeControlBuilder::new(Decibels::SILENCE));
    let (mut music, mut sfx, mut engines, mut ui, mut ambience) = (
        AudioTrack::new(),
        AudioTrack::new(),
        AudioTrack::new(),
        AudioTrack::new(),
        AudioTrack::new(),
    );
    let ambience_filter = ambience.add_effect(
        FilterBuilder::new()
            .mode(FilterMode::LowPass)
            .mix(WetDry::DRY),
    );
    let controls = BusControls {
        music: volume(&mut music),
        sfx: volume(&mut sfx),
        engines: volume(&mut engines),
        ui: volume(&mut ui),
        ambience: volume(&mut ambience),
        ambience_filter,
    };
    app.add_audio_channel_with_track::<MusicBus>(music)
        .add_audio_channel_with_track::<SfxBus>(sfx)
        .add_audio_channel_with_track::<EngineBus>(engines)
        .add_audio_channel_with_track::<UiBus>(ui)
        .add_audio_channel_with_track::<AmbienceBus>(ambience)
        .insert_resource(controls)
        .add_systems(
            Update,
            apply_bus_volumes.in_set(Step::Volumes).run_if(audio_on),
        )
        .add_systems(OnEnter(PauseState::Paused), pause_buses.run_if(audio_on))
        .add_systems(OnExit(PauseState::Paused), resume_buses.run_if(audio_on));
}

/// Linear gain → dB, floored at [`SILENCE_DB`].
pub(crate) fn gain_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        SILENCE_DB
    } else {
        (20.0 * gain.log10()).max(SILENCE_DB)
    }
}

/// A short linear tween.
pub(crate) fn tween_ms(ms: u64) -> AudioTween {
    AudioTween::linear(Duration::from_millis(ms))
}

/// Bus levels in dB, in the order music, sfx, engines, ui, ambience.
pub(crate) fn bus_levels(audio: &AudioPrefs, muted: bool, paused: bool) -> [f32; 5] {
    let master = if muted {
        0.0
    } else {
        audio.master.clamp(0.0, 1.0)
    };
    let level = |bus: f32| gain_db(master * bus.clamp(0.0, 1.0));
    let duck = if paused { PAUSE_DUCK_DB } else { 0.0 };
    [
        (level(audio.music) + duck).max(SILENCE_DB),
        level(audio.sfx),
        level(audio.engines),
        level(audio.ui),
        level(audio.ambience),
    ]
}

fn apply_bus_volumes(
    settings: Option<Res<Settings>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pause: Option<Res<State<PauseState>>>,
    mut controls: ResMut<BusControls>,
    mut last: Local<Option<[f32; 5]>>,
) {
    let audio = settings.map(|s| s.audio.clone()).unwrap_or_default();
    let focused = windows.iter().next().is_none_or(|w| w.focused);
    let paused = pause.is_some_and(|p| *p.get() == PauseState::Paused);
    let levels = bus_levels(&audio, audio.mute_unfocused && !focused, paused);
    if *last == Some(levels) {
        return;
    }
    let t = tween_ms(VOLUME_TWEEN_MS);
    let [music, sfx, engines, ui, ambience] = levels;
    controls.music.set_volume(Decibels(music), t);
    controls.sfx.set_volume(Decibels(sfx), t);
    controls.engines.set_volume(Decibels(engines), t);
    controls.ui.set_volume(Decibels(ui), t);
    controls.ambience.set_volume(Decibels(ambience), t);
    *last = Some(levels);
}

fn pause_buses(
    engines: Res<AudioChannel<EngineBus>>,
    sfx: Res<AudioChannel<SfxBus>>,
    ambience: Res<AudioChannel<AmbienceBus>>,
) {
    let t = tween_ms(PAUSE_TWEEN_MS);
    engines.pause().fade_out(t);
    sfx.pause().fade_out(t);
    ambience.pause().fade_out(t);
}

fn resume_buses(
    engines: Res<AudioChannel<EngineBus>>,
    sfx: Res<AudioChannel<SfxBus>>,
    ambience: Res<AudioChannel<AmbienceBus>>,
) {
    let t = tween_ms(PAUSE_TWEEN_MS);
    engines.resume().fade_in(t);
    sfx.resume().fade_in(t);
    ambience.resume().fade_in(t);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(master: f32, bus: f32) -> AudioPrefs {
        AudioPrefs {
            master,
            music: bus,
            sfx: bus,
            engines: bus,
            ui: bus,
            ambience: bus,
            mute_unfocused: true,
        }
    }

    #[test]
    fn bus_volume_is_master_times_bus_in_db() {
        let [music, sfx, ..] = bus_levels(&audio(1.0, 1.0), false, false);
        assert_eq!((music, sfx), (0.0, 0.0));
        let [_, sfx, ..] = bus_levels(&audio(0.5, 0.5), false, false);
        assert!(
            (sfx - -12.04).abs() < 0.01,
            "0.5 × 0.5 = 0.25 ≈ −12 dB, got {sfx}"
        );
        assert!(
            bus_levels(&audio(1.0, 0.0), false, false)
                .iter()
                .all(|&db| db == SILENCE_DB)
        );
        assert!(
            bus_levels(&audio(0.0, 1.0), false, false)
                .iter()
                .all(|&db| db == SILENCE_DB)
        );
    }

    #[test]
    fn mute_silences_everything_and_pause_ducks_only_music() {
        assert!(
            bus_levels(&audio(1.0, 1.0), true, false)
                .iter()
                .all(|&db| db == SILENCE_DB)
        );
        let [music, sfx, engines, ui, ambience] = bus_levels(&audio(1.0, 1.0), false, true);
        assert_eq!(music, PAUSE_DUCK_DB);
        assert_eq!([sfx, engines, ui, ambience], [0.0; 4]);
    }

    #[test]
    fn out_of_range_settings_are_clamped() {
        let [music, ..] = bus_levels(&audio(2.0, 1.5), false, false);
        assert_eq!(music, 0.0);
        assert_eq!(gain_db(-1.0), SILENCE_DB);
        assert_eq!(gain_db(1e-9), SILENCE_DB);
    }
}
