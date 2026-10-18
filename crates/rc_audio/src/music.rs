//! Music director: the menu theme in menus, silence while loading, the track's race cue from the
//! intro to the finish — switched to its phase-aligned `_final` variant at the same playback
//! position on the final lap — the results jingle when the race is over and cup stingers on
//! `MusicStinger`; every change is a crossfade. Pause ducking is a bus-volume change
//! (`buses.rs`). The director reconciles every frame (target vs. playing), so late device
//! probes and fast state changes need no special cases.

use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::bank::{Bank, Session, music_path};
use crate::buses::MusicBus;
use crate::output::audio_on;

/// Loops fade out over this when they stop, s.
const FADE_OUT_S: f32 = 1.0;
/// Menu theme fade-in, s.
const MENU_FADE_IN_S: f32 = 1.5;
/// Race cue fade-in, s.
const RACE_FADE_IN_S: f32 = 0.8;
/// Base → final-lap crossfade, s.
const FINAL_XFADE_S: f32 = 1.5;
/// A stinger cuts the current music with this fade, s.
const STINGER_CUT_S: f32 = 0.4;
/// Stinger length assumed while its file is not loaded yet, s.
const STINGER_FALLBACK_S: f64 = 6.0;

/// What should be playing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Target {
    /// Nothing (loading, setup, results, stinger playing).
    #[default]
    Silence,
    /// The menu theme.
    Menu,
    /// A race loop.
    Race {
        /// The track's cue.
        cue: MusicCue,
        /// The final-lap variant.
        final_lap: bool,
    },
}

/// The pure state machine: app state, race phase, the session's cue, the final-lap flag and
/// whether a stinger holds the floor → what should play.
pub(crate) fn target(
    app: AppState,
    phase: Option<RacePhase>,
    cue: Option<MusicCue>,
    final_lap: bool,
    stinger: bool,
) -> Target {
    match (app, phase, cue) {
        _ if stinger => Target::Silence,
        (AppState::Menu, _, _) => Target::Menu,
        (
            AppState::InRace,
            Some(RacePhase::Intro | RacePhase::Countdown | RacePhase::Racing),
            Some(cue),
        ) => Target::Race { cue, final_lap },
        _ => Target::Silence,
    }
}

#[derive(Resource, Default)]
struct Director {
    playing: Target,
    instance: Option<Handle<AudioInstance>>,
    final_lap: bool,
    menu_theme: Option<Handle<AudioSource>>,
    /// Real-time second until which a stinger holds the floor.
    stinger_until: f64,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Director>()
        .add_systems(
            Update,
            (
                watch_final_lap,
                play_stingers.run_if(audio_on),
                direct.run_if(audio_on),
            )
                .chain()
                .in_set(Step::Music),
        )
        .add_systems(OnEnter(RacePhase::Setup), |mut d: ResMut<Director>| {
            d.final_lap = false
        })
        .add_systems(
            OnEnter(RacePhase::Finished),
            results_jingle.run_if(audio_on),
        )
        // The menu theme (≈ 40 MB decoded) is not kept during races.
        .add_systems(OnEnter(AppState::Loading), |mut d: ResMut<Director>| {
            d.menu_theme = None
        });
}

fn secs(s: f32) -> AudioTween {
    AudioTween::linear(Duration::from_secs_f32(s))
}

/// The final lap of a local player (or a global one) switches to the final-lap variant.
fn watch_final_lap(
    mut events: MessageReader<RaceEvent>,
    locals: Query<(), With<LocalPlayer>>,
    mut director: ResMut<Director>,
) {
    for e in events.read() {
        if e.kind == RaceEventKind::FinalLap && e.racer.is_none_or(|r| locals.contains(r)) {
            director.final_lap = true;
        }
    }
}

fn results_jingle(
    bank: Res<Bank>,
    sources: Res<Assets<AudioSource>>,
    time: Res<Time<Real>>,
    music: Res<AudioChannel<MusicBus>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut director: ResMut<Director>,
) {
    stinger(
        MusicCue::ResultsJingle,
        &bank,
        &sources,
        &time,
        &music,
        &mut instances,
        &mut director,
    );
}

fn play_stingers(
    mut requests: MessageReader<MusicStinger>,
    bank: Res<Bank>,
    sources: Res<Assets<AudioSource>>,
    time: Res<Time<Real>>,
    music: Res<AudioChannel<MusicBus>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut director: ResMut<Director>,
) {
    if let Some(&MusicStinger(cue)) = requests.read().last() {
        stinger(
            cue,
            &bank,
            &sources,
            &time,
            &music,
            &mut instances,
            &mut director,
        );
    }
}

/// Cuts the current music, plays `cue` once and holds the floor for its length.
fn stinger(
    cue: MusicCue,
    bank: &Bank,
    sources: &Assets<AudioSource>,
    time: &Time<Real>,
    music: &AudioChannel<MusicBus>,
    instances: &mut Assets<AudioInstance>,
    director: &mut Director,
) {
    let Some(handle) = bank.stinger(cue) else {
        warn_once!("music stinger {cue:?} is not a resident stinger");
        return;
    };
    stop(director.instance.take(), STINGER_CUT_S, instances);
    director.playing = Target::Silence;
    let length = sources
        .get(&handle)
        .map_or(STINGER_FALLBACK_S, |s| s.sound.duration().as_secs_f64());
    director.stinger_until = time.elapsed_secs_f64() + length;
    music.play(handle);
}

fn stop(
    instance: Option<Handle<AudioInstance>>,
    fade_s: f32,
    instances: &mut Assets<AudioInstance>,
) {
    if let Some(mut i) = instance.and_then(|h| instances.get_mut(&h)) {
        i.stop(secs(fade_s));
    }
}

#[allow(clippy::too_many_arguments)] // one system reading the whole music context
fn direct(
    app: Res<State<AppState>>,
    phase: Option<Res<State<RacePhase>>>,
    session: Option<Res<Session>>,
    time: Res<Time<Real>>,
    server: Res<AssetServer>,
    music: Res<AudioChannel<MusicBus>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut director: ResMut<Director>,
) {
    let want = target(
        *app.get(),
        phase.map(|p| *p.get()),
        session.as_ref().map(|s| s.cue),
        director.final_lap,
        time.elapsed_secs_f64() < director.stinger_until,
    );
    if want == director.playing {
        return;
    }
    debug!("music: {:?} → {want:?}", director.playing);
    let old = director.instance.take();
    let pair = session.as_ref().and_then(|s| s.music.clone());
    director.instance = match (director.playing, want, pair) {
        (
            Target::Race {
                final_lap: false, ..
            },
            Target::Race {
                final_lap: true, ..
            },
            Some((_, finale)),
        ) => {
            // Same tempo and note timing: continue at the base loop's position (music.md).
            let at = old
                .as_ref()
                .and_then(|h| instances.get(h))
                .and_then(|i| i.state().position());
            stop(old, FINAL_XFADE_S, &mut instances);
            Some(
                music
                    .play(finale)
                    .looped()
                    .start_from(at.unwrap_or(0.0))
                    .fade_in(secs(FINAL_XFADE_S))
                    .handle(),
            )
        }
        (_, Target::Race { final_lap, .. }, Some((base, finale))) => {
            stop(old, FADE_OUT_S, &mut instances);
            let file = if final_lap { finale } else { base };
            Some(
                music
                    .play(file)
                    .looped()
                    .fade_in(secs(RACE_FADE_IN_S))
                    .handle(),
            )
        }
        (_, Target::Menu, _) => {
            stop(old, FADE_OUT_S, &mut instances);
            let theme = director
                .menu_theme
                .get_or_insert_with(|| server.load(music_path(MusicCue::MenuTheme.stem())))
                .clone();
            Some(
                music
                    .play(theme)
                    .looped()
                    .fade_in(secs(MENU_FADE_IN_S))
                    .handle(),
            )
        }
        _ => {
            stop(old, FADE_OUT_S, &mut instances);
            None
        }
    };
    director.playing = want;
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUE: Option<MusicCue> = Some(MusicCue::Race3);

    #[test]
    fn menus_play_the_theme_and_loading_is_silent() {
        assert_eq!(
            target(AppState::Menu, None, None, false, false),
            Target::Menu
        );
        assert_eq!(
            target(AppState::Boot, None, None, false, false),
            Target::Silence
        );
        assert_eq!(
            target(AppState::Loading, None, CUE, false, false),
            Target::Silence
        );
        assert_eq!(
            target(AppState::Unloading, None, CUE, false, false),
            Target::Silence
        );
    }

    #[test]
    fn race_cue_plays_from_intro_to_finish_with_the_final_lap_variant() {
        let race = |p, f| target(AppState::InRace, Some(p), CUE, f, false);
        assert_eq!(
            race(RacePhase::Setup, false),
            Target::Silence,
            "restart: fade, then from 0"
        );
        for phase in [RacePhase::Intro, RacePhase::Countdown, RacePhase::Racing] {
            assert_eq!(
                race(phase, false),
                Target::Race {
                    cue: MusicCue::Race3,
                    final_lap: false
                }
            );
        }
        assert_eq!(
            race(RacePhase::Racing, true),
            Target::Race {
                cue: MusicCue::Race3,
                final_lap: true
            }
        );
        assert_eq!(
            race(RacePhase::Finished, true),
            Target::Silence,
            "results jingle takes over"
        );
        assert_eq!(
            target(
                AppState::InRace,
                Some(RacePhase::Racing),
                None,
                false,
                false
            ),
            Target::Silence
        );
    }

    #[test]
    fn a_stinger_holds_the_floor_everywhere() {
        assert_eq!(
            target(AppState::Menu, None, None, false, true),
            Target::Silence
        );
        assert_eq!(
            target(AppState::InRace, Some(RacePhase::Racing), CUE, false, true),
            Target::Silence
        );
    }
}
