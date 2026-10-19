//! Sound handles. SFX and stingers stay resident (loaded once a device is confirmed); each race
//! session preloads its music pair and its environment's ambience during `Loading` and completes
//! the `"audio"` key (at once when there is no device); `Unloading` releases them.

use std::collections::HashMap;

use bevy::asset::LoadState;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::ambience::{self, Env};
use crate::output::{OutputState, audio_on};
use crate::{LOADING_KEY, Step};

/// Every file in `assets/audio/sfx/` (`docs/design/sfx.md`).
pub(crate) const SFX: [&str; 59] = [
    "engine_buggy",
    "engine_truck",
    "engine_sports",
    "skid_loop",
    "scrub_loop",
    "rumble_grass_loop",
    "rumble_dirt_loop",
    "rumble_water_loop",
    "landing_thump_1",
    "landing_thump_2",
    "impact_soft_1",
    "impact_soft_2",
    "impact_soft_3",
    "impact_hard_1",
    "impact_hard_2",
    "impact_hard_3",
    "boost_whoosh",
    "boost_crackle_loop",
    "drift_sizzle_loop",
    "charge_ready",
    "pickup_grab",
    "roulette_tick",
    "roulette_lock",
    "rocket_launch",
    "rocket_flight_loop",
    "rocket_explosion",
    "lockon_beep",
    "leech_hum_loop",
    "leech_zap",
    "orb_hum_loop",
    "orb_zap",
    "orb_launch",
    "emp_charge",
    "emp_blast",
    "goo_beep",
    "goo_explosion",
    "oil_splat",
    "oil_slip",
    "spring_boing",
    "item_drop",
    "shield_up",
    "shield_hit",
    "shield_pop",
    "storm_strike",
    "countdown_beep",
    "countdown_go",
    "lap_chime",
    "final_lap_fanfare",
    "finish_horn",
    "wrong_way_buzzer",
    "respawn_materialise",
    "golden_battery_jingle",
    "ui_hover",
    "ui_select",
    "ui_back",
    "ui_toggle",
    "ui_slider_tick",
    "ui_error",
    "ui_unlock",
];
/// Music cues kept resident (one-shots, small).
const STINGERS: [MusicCue; 3] = [
    MusicCue::ResultsJingle,
    MusicCue::PodiumFanfare,
    MusicCue::CupVictory,
];

/// Resident handles by file stem (SFX) and cue (stingers).
#[derive(Resource, Default)]
pub(crate) struct Bank {
    sfx: HashMap<&'static str, Handle<AudioSource>>,
    stingers: HashMap<MusicCue, Handle<AudioSource>>,
}

impl Bank {
    /// A resident SFX by stem (`None` until loaded at startup, or for unknown stems).
    pub(crate) fn sfx(&self, stem: &str) -> Option<Handle<AudioSource>> {
        self.sfx.get(stem).cloned()
    }

    /// A resident stinger.
    pub(crate) fn stinger(&self, cue: MusicCue) -> Option<Handle<AudioSource>> {
        self.stingers.get(&cue).cloned()
    }
}

/// Audio of the current race session (inserted in `OnEnter(Loading)`, removed in `Unloading`).
#[derive(Resource)]
pub(crate) struct Session {
    /// The track's race cue.
    pub(crate) cue: MusicCue,
    /// Environment, indoor flag and time of day (ambience).
    pub(crate) env: Env,
    /// Weathers this race can show (fixed, or the track's presets for random/dynamic).
    weathers: Vec<WeatherKind>,
    /// Race loop and its final-lap variant.
    pub(crate) music: Option<(Handle<AudioSource>, Handle<AudioSource>)>,
    /// Ambience loops by stem.
    pub(crate) loops: Vec<(&'static str, Handle<AudioSource>)>,
    /// Thunder one-shots (close, medium, distant) when this race can storm.
    pub(crate) thunder: Vec<Handle<AudioSource>>,
    reported: bool,
}

/// Number of loaded audio sources, a Bevy diagnostic: the soak census (rc_game counts every
/// `assets/*` diagnostic; it cannot name kira's `AudioSource`) checks it returns to baseline in
/// the menu.
const AUDIO_SOURCES: DiagnosticPath = DiagnosticPath::const_new("assets/audio_sources");

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Bank>()
        .register_diagnostic(Diagnostic::new(AUDIO_SOURCES).with_max_history_length(1))
        .add_systems(Update, count_sources.in_set(Step::Load))
        .add_systems(
            Update,
            load_resident
                .in_set(Step::Load)
                .run_if(audio_on.and_then(|b: Res<Bank>| b.sfx.is_empty())),
        )
        .add_systems(OnEnter(AppState::Loading), begin_session)
        .add_systems(
            Update,
            load_session
                .in_set(Step::Load)
                .after(load_resident)
                .run_if(in_state(AppState::Loading)),
        )
        .add_systems(OnEnter(AppState::Unloading), end_session);
}

fn count_sources(sources: Res<Assets<AudioSource>>, mut diagnostics: Diagnostics) {
    diagnostics.add_measurement(&AUDIO_SOURCES, || sources.len() as f64);
}

pub(crate) fn sfx_path(stem: &str) -> String {
    format!("audio/sfx/{stem}.ogg")
}

pub(crate) fn music_path(stem: &str) -> String {
    format!("audio/music/{stem}.ogg")
}

fn load_resident(server: Res<AssetServer>, mut bank: ResMut<Bank>) {
    bank.sfx = SFX
        .iter()
        .map(|&stem| (stem, server.load(sfx_path(stem))))
        .collect();
    bank.stingers = STINGERS
        .iter()
        .map(|&cue| (cue, server.load(music_path(cue.stem()))))
        .collect();
}

fn begin_session(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    catalog: Option<Res<Catalog>>,
) {
    let track = config
        .as_ref()
        .zip(catalog.as_ref())
        .and_then(|(c, cat)| cat.track(&c.variant.track));
    let weathers = match config.as_ref().map(|c| c.weather.weather) {
        None => vec![WeatherKind::Clear],
        Some(WeatherChoice::Fixed(kind)) => vec![kind],
        // Random / Dynamic: any preset the track allows.
        Some(_) => track
            .map(|t| t.allowed_weather.clone())
            .filter(|w| !w.is_empty())
            .unwrap_or_else(|| WeatherKind::ALL.to_vec()),
    };
    commands.insert_resource(Session {
        cue: track.map_or(MusicCue::Race1, |t| t.music),
        env: Env {
            id: track.map_or_else(|| "test".to_owned(), |t| t.environment.clone()),
            indoor: track.is_some_and(|t| t.indoor),
            time: config.as_ref().map_or(TimeOfDay::Noon, |c| c.weather.time),
        },
        weathers,
        music: None,
        loops: Vec::new(),
        thunder: Vec::new(),
        reported: false,
    });
}

/// Requests the session's files once a device is confirmed, then reports the key when the race
/// loop and the ambience loops are loaded (failed files only warn: audio never blocks a race).
/// The final-lap variant and thunder load in the background.
fn load_session(
    session: Option<ResMut<Session>>,
    state: Res<OutputState>,
    server: Res<AssetServer>,
    mut done: MessageWriter<LoadingDone>,
) {
    let Some(mut session) = session else { return };
    if session.reported || *state == OutputState::Probing {
        return;
    }
    if *state == OutputState::Unavailable {
        session.reported = true;
        done.write(LoadingDone(LOADING_KEY));
        return;
    }
    if session.music.is_none() {
        let stem = session.cue.stem();
        session.music = Some((
            server.load(music_path(stem)),
            server.load(music_path(&format!("{stem}_final"))),
        ));
        let (loops, storm) = ambience::preload(&session.env, &session.weathers);
        session.loops = loops
            .iter()
            .map(|&s| (s, server.load(ambience::path(s))))
            .collect();
        if storm {
            session.thunder = ambience::THUNDER
                .iter()
                .map(|&s| server.load(ambience::path(s)))
                .collect();
        }
    }
    let gating = session
        .music
        .iter()
        .map(|(base, _)| base.id().untyped())
        .chain(session.loops.iter().map(|(_, h)| h.id().untyped()));
    let mut ready = true;
    for id in gating {
        match server.load_state(id) {
            LoadState::Loaded => {}
            LoadState::Failed(err) => {
                warn_once!("audio file failed to load ({err}); continuing without it")
            }
            _ => ready = false,
        }
    }
    if ready {
        info!(
            "audio: session ready ({:?}, {} ambience loops, environment {})",
            session.cue,
            session.loops.len(),
            session.env.id
        );
        session.reported = true;
        done.write(LoadingDone(LOADING_KEY));
    }
}

fn end_session(mut commands: Commands) {
    commands.remove_resource::<Session>();
}
