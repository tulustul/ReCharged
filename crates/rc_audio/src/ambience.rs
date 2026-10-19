//! Ambience: a bed per environment (room tone / supermarket / workshop indoors, birds by day or
//! crickets at night outdoors) plus weather loops (wind; light/heavy rain, `_indoor` muffled
//! variants on indoor tracks), all looped from session start and mixed every frame from
//! `WeatherState` (so dynamic weather crossfades); fog low-passes the bus; lightning brings
//! thunder after a distance delay (low-passed indoors). GDD §10.2, `docs/design/sfx.md`.

use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::effect::Mix as WetDry;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::bank::Session;
use crate::buses::{AmbienceBus, BusControls, SILENCE_DB, gain_db, tween_ms};
use crate::output::audio_on;
use crate::shots::CueLog;

/// Thunder one-shots: close, medium, distant.
pub(crate) const THUNDER: [&str; 3] = ["thunder_1", "thunder_2", "thunder_3"];
/// Ambience loops start at silence and fade to their mix level over this, ms.
const MIX_TWEEN_MS: u64 = 1500;
/// Fog muffling: low-pass cutoff, Hz (the wet mix follows `WeatherState::fog`).
const FOG_CUTOFF_HZ: f64 = 1200.0;
/// Thunder heard through walls (indoor tracks): low-pass cutoff, Hz.
const INDOOR_THUNDER_CUTOFF_HZ: f64 = 700.0;
/// Thunder delay after a flash: distant (intensity 0) … close (intensity 1), s.
const THUNDER_DELAY_S: (f32, f32) = (2.6, 0.2);
/// Thunder level for the weakest flash, dB (a full flash is 0 dB).
const THUNDER_WEAK_DB: f32 = -6.0;
/// Rain precipitation where light rain is full / heavy rain has taken over (Rain 0.6, Storm 1.0).
const HEAVY_RAIN: (f32, f32) = (0.6, 0.9);
/// Wind speed where the wind loop starts / is full, u/s (Clear 2, Cloudy 5, Storm 14 + gusts).
const WIND_UPS: (f32, f32) = (1.5, 13.5);
/// Snow hush: wind-loop gain at full snowfall, reached at this precipitation (Snow 0.5).
const SNOW_HUSH: f32 = 0.35;
const SNOW_FULL: f32 = 0.5;
/// Birds/crickets: dimming at full cloud cover, and the precipitation that silences them.
const LIFE_CLOUD_DIM: f32 = 0.6;
const LIFE_GONE_AT: f32 = 0.3;

/// Where a race happens, for ambience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Env {
    /// Environment id from the catalog (`toy_store`, `house`, `garden`, …).
    pub(crate) id: String,
    /// Indoor track: weather is outside only.
    pub(crate) indoor: bool,
    /// Time of day.
    pub(crate) time: TimeOfDay,
}

pub(crate) fn path(stem: &str) -> String {
    format!("audio/ambience/{stem}.ogg")
}

fn bed(env: &Env) -> &'static [&'static str] {
    match (env.indoor, env.id.as_str()) {
        (true, "supermarket") => &["supermarket_hum"],
        (true, "workshop") => &["workshop_hum", "workshop_tools"],
        (true, _) => &["room_tone"],
        (false, _) if env.time == TimeOfDay::Night => &["crickets"],
        (false, _) => &["birds"],
    }
}

fn weather_loops(indoor: bool, kind: WeatherKind) -> &'static [&'static str] {
    match (indoor, kind) {
        (true, WeatherKind::Rain) => &["rain_light_indoor"],
        (true, WeatherKind::Storm) => &["rain_heavy_indoor"],
        (true, _) | (false, WeatherKind::Clear) => &[],
        (false, WeatherKind::Rain) => &["rain_light"],
        (false, WeatherKind::Storm) => &["rain_heavy", "wind"],
        (false, WeatherKind::Cloudy | WeatherKind::Fog | WeatherKind::Snow) => &["wind"],
    }
}

/// Loops to preload for a race that can show `weathers`, and whether it can storm (thunder).
pub(crate) fn preload(env: &Env, weathers: &[WeatherKind]) -> (Vec<&'static str>, bool) {
    let mut loops = bed(env).to_vec();
    for &kind in weathers {
        for stem in weather_loops(env.indoor, kind) {
            if !loops.contains(stem) {
                loops.push(stem);
            }
        }
    }
    (loops, weathers.contains(&WeatherKind::Storm))
}

/// Linear gain 0–1 of an ambience loop under the current weather. Only blended columns are used
/// (`kind` switches to the target at the start of a 20 s blend, rc_weather.md), so a dynamic change
/// crossfades with the visuals: rain from `precipitation` unless `snowing` (light → heavy between
/// [`HEAVY_RAIN`]), wind from the wind speed (gusts swell it; zero indoors) plus a hush while
/// snowing, birds/crickets thinning with cloud and gone in rain or snow.
pub(crate) fn gain(stem: &str, weather: &WeatherState) -> f32 {
    let falling = weather.precipitation.clamp(0.0, 1.0);
    let rain = if weather.snowing { 0.0 } else { falling };
    let heavy = ((rain - HEAVY_RAIN.0) / (HEAVY_RAIN.1 - HEAVY_RAIN.0)).clamp(0.0, 1.0);
    match stem {
        "birds" | "crickets" => {
            (1.0 - LIFE_CLOUD_DIM * weather.cloudiness.clamp(0.0, 1.0))
                * (1.0 - (falling / LIFE_GONE_AT).min(1.0))
        }
        "wind" => {
            let blow =
                ((weather.wind.length() - WIND_UPS.0) / (WIND_UPS.1 - WIND_UPS.0)).clamp(0.0, 1.0);
            let hush = if weather.snowing {
                SNOW_HUSH * (falling / SNOW_FULL).min(1.0)
            } else {
                0.0
            };
            blow.max(hush)
        }
        "rain_light" | "rain_light_indoor" => (rain / HEAVY_RAIN.0).min(1.0) * (1.0 - heavy),
        "rain_heavy" | "rain_heavy_indoor" => heavy,
        _ => 1.0, // room beds
    }
}

/// Thunder variant (index into the session's thunder), delay in s and level in dB for a flash.
pub(crate) fn thunder_for(intensity: f32) -> (usize, f32, f32) {
    let i = intensity.clamp(0.0, 1.0);
    let variant = if i >= 0.7 {
        0
    } else if i >= 0.35 {
        1
    } else {
        2
    };
    let delay = THUNDER_DELAY_S.0 + (THUNDER_DELAY_S.1 - THUNDER_DELAY_S.0) * i;
    (variant, delay, THUNDER_WEAK_DB * (1.0 - i))
}

struct Voice {
    stem: &'static str,
    instance: Handle<AudioInstance>,
    db: f32,
}

#[derive(Resource, Default)]
struct Ambience {
    voices: Vec<Voice>,
    fog: f32,
    /// Pending thunder: (real-time second, variant, dB).
    thunder: Vec<(f64, usize, f32)>,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Ambience>()
        .add_systems(
            Update,
            (start_ambience, mix_ambience, queue_thunder, play_thunder)
                .chain()
                .in_set(Step::Ambience)
                .run_if(in_state(AppState::InRace).and_then(audio_on)),
        )
        .add_systems(OnEnter(AppState::Unloading), stop_ambience.run_if(audio_on))
        .add_systems(OnEnter(RacePhase::Setup), |mut a: ResMut<Ambience>| {
            a.thunder.clear()
        });
}

fn start_ambience(
    session: Option<Res<Session>>,
    bus: Res<AudioChannel<AmbienceBus>>,
    mut amb: ResMut<Ambience>,
) {
    let Some(session) = session else { return };
    if !amb.voices.is_empty() || session.loops.is_empty() {
        return;
    }
    amb.voices = session
        .loops
        .iter()
        .map(|(stem, handle)| Voice {
            stem,
            instance: bus
                .play(handle.clone())
                .looped()
                .with_volume(Decibels::SILENCE)
                .handle(),
            db: SILENCE_DB,
        })
        .collect();
}

fn mix_ambience(
    weather: Option<Res<WeatherState>>,
    mut amb: ResMut<Ambience>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut controls: ResMut<BusControls>,
) {
    let weather = weather.map(|w| w.clone()).unwrap_or_default();
    for voice in &mut amb.voices {
        let db = gain_db(gain(voice.stem, &weather));
        if (db - voice.db).abs() > 0.1
            && let Some(mut i) = instances.get_mut(&voice.instance)
        {
            i.set_decibels(Decibels(db), tween_ms(MIX_TWEEN_MS));
            voice.db = db;
        }
    }
    let fog = (weather.fog.clamp(0.0, 1.0) * 20.0).round() / 20.0;
    if fog != amb.fog {
        let filter = &mut controls.ambience_filter;
        filter.set_cutoff(FOG_CUTOFF_HZ, tween_ms(10));
        filter.set_mix(WetDry(fog), tween_ms(MIX_TWEEN_MS));
        amb.fog = fog;
    }
}

fn queue_thunder(
    mut events: MessageReader<WeatherEvent>,
    time: Res<Time<Real>>,
    mut amb: ResMut<Ambience>,
) {
    for e in events.read() {
        if let WeatherEvent::Lightning { intensity } = *e {
            let (variant, delay, db) = thunder_for(intensity);
            amb.thunder
                .push((time.elapsed_secs_f64() + f64::from(delay), variant, db));
        }
    }
}

fn play_thunder(
    session: Option<Res<Session>>,
    time: Res<Time<Real>>,
    bus: Res<AudioChannel<AmbienceBus>>,
    mut amb: ResMut<Ambience>,
    mut log: ResMut<CueLog>,
) {
    let now = time.elapsed_secs_f64();
    let Some(session) = session else { return };
    amb.thunder.retain(|&(at, variant, db)| {
        if at > now {
            return true;
        }
        if let Some(handle) = session.thunder.get(variant).cloned() {
            log.started(THUNDER[variant.min(THUNDER.len() - 1)], "Ambience", db, now);
            let mut play = bus.play(handle);
            play.with_volume(Decibels(db));
            if session.env.indoor {
                play.with_effect(
                    FilterBuilder::new()
                        .mode(FilterMode::LowPass)
                        .cutoff(INDOOR_THUNDER_CUTOFF_HZ),
                );
            }
        }
        false
    });
}

fn stop_ambience(
    mut amb: ResMut<Ambience>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut controls: ResMut<BusControls>,
) {
    for voice in amb.voices.drain(..) {
        if let Some(mut i) = instances.get_mut(&voice.instance) {
            i.stop(AudioTween::linear(Duration::from_millis(600)));
        }
    }
    amb.thunder.clear();
    amb.fog = 0.0;
    controls.ambience_filter.set_mix(WetDry::DRY, tween_ms(10));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(id: &str, indoor: bool, time: TimeOfDay) -> Env {
        Env {
            id: id.into(),
            indoor,
            time,
        }
    }

    /// The steady state of a preset (rc_weather.md parameter table).
    fn preset(kind: WeatherKind) -> WeatherState {
        let (precipitation, cloudiness, wind, snowing) = match kind {
            WeatherKind::Clear => (0.0, 0.1, 2.0, false),
            WeatherKind::Cloudy => (0.0, 0.8, 5.0, false),
            WeatherKind::Rain => (0.6, 0.9, 6.0, false),
            WeatherKind::Storm => (1.0, 1.0, 14.0, false),
            WeatherKind::Fog => (0.0, 0.5, 1.0, false),
            WeatherKind::Snow => (0.5, 0.85, 3.0, true),
        };
        WeatherState {
            kind,
            precipitation,
            cloudiness,
            wind: Vec3::X * wind,
            snowing,
            ..default()
        }
    }

    #[test]
    fn beds_follow_environment_and_time_of_day() {
        let (day, _) = preload(
            &env("garden", false, TimeOfDay::Noon),
            &[WeatherKind::Clear],
        );
        let (night, _) = preload(
            &env("street", false, TimeOfDay::Night),
            &[WeatherKind::Clear],
        );
        assert_eq!((day, night), (vec!["birds"], vec!["crickets"]));
        assert_eq!(
            preload(&env("workshop", true, TimeOfDay::Night), &[]).0,
            ["workshop_hum", "workshop_tools"]
        );
        assert_eq!(
            preload(&env("toy_store", true, TimeOfDay::Noon), &[]).0,
            ["room_tone"]
        );
        assert_eq!(
            preload(&env("supermarket", true, TimeOfDay::Sunset), &[]).0,
            ["supermarket_hum"]
        );
    }

    #[test]
    fn indoor_tracks_get_muffled_outside_rain_and_thunder_only_when_it_can_storm() {
        let all = [
            WeatherKind::Clear,
            WeatherKind::Cloudy,
            WeatherKind::Rain,
            WeatherKind::Storm,
        ];
        let (loops, storm) = preload(&env("house", true, TimeOfDay::Noon), &all);
        assert_eq!(
            loops,
            ["room_tone", "rain_light_indoor", "rain_heavy_indoor"]
        );
        assert!(storm);
        let (loops, storm) = preload(&env("garden", false, TimeOfDay::Morning), &WeatherKind::ALL);
        assert_eq!(loops, ["birds", "wind", "rain_light", "rain_heavy"]);
        assert!(storm);
        assert!(!preload(&env("garden", false, TimeOfDay::Noon), &[WeatherKind::Rain]).1);
    }

    #[test]
    fn mix_follows_the_presets() {
        use WeatherKind::*;
        let g = |stem, kind| gain(stem, &preset(kind));
        assert!(
            g("birds", Clear) > 0.9 && g("wind", Clear) < 0.05 && g("rain_light", Clear) == 0.0
        );
        assert!(g("birds", Cloudy) > 0.4 && g("birds", Cloudy) < 0.6 && g("wind", Cloudy) > 0.2);
        assert_eq!(
            (
                g("rain_light", Rain),
                g("rain_heavy", Rain),
                g("birds", Rain)
            ),
            (1.0, 0.0, 0.0)
        );
        assert_eq!(
            (g("rain_heavy_indoor", Storm), g("rain_light_indoor", Storm)),
            (1.0, 0.0)
        );
        assert!(g("wind", Storm) > 0.9);
        assert_eq!(
            (
                g("rain_light", Snow),
                g("rain_heavy", Snow),
                g("birds", Snow)
            ),
            (0.0, 0.0, 0.0)
        );
        assert!((g("wind", Snow) - SNOW_HUSH).abs() < 1e-6, "snow hush");
        assert_eq!(g("room_tone", Storm), 1.0);
    }

    #[test]
    fn dynamic_changes_crossfade_through_the_blend_not_at_the_kind_switch() {
        // Rain → Storm: `kind` is Storm from the first blend tick; precipitation ramps 0.6 → 1.0.
        let blend = |p| WeatherState {
            kind: WeatherKind::Storm,
            precipitation: p,
            ..preset(WeatherKind::Rain)
        };
        assert_eq!(
            gain("rain_light", &blend(0.6)),
            1.0,
            "unchanged at the switch"
        );
        let mid = blend(0.75);
        assert!(gain("rain_light", &mid) > 0.0 && gain("rain_heavy", &mid) > 0.0);
        // Snow → Cloudy: `kind` is Cloudy while snow still falls.
        let fading = WeatherState {
            kind: WeatherKind::Cloudy,
            ..preset(WeatherKind::Snow)
        };
        assert_eq!(gain("rain_light", &fading), 0.0, "still snow, not rain");
    }

    #[test]
    fn close_flashes_bring_loud_early_thunder() {
        let (variant, delay, db) = thunder_for(1.0);
        assert_eq!(variant, 0);
        assert!((delay - 0.2).abs() < 1e-5 && db.abs() < 1e-6);
        let (variant, delay, db) = thunder_for(0.5);
        assert_eq!(variant, 1);
        assert!(delay > 0.2 && delay < 2.6 && db < 0.0);
        let (variant, delay, db) = thunder_for(0.0);
        assert_eq!((variant, delay, db), (2, 2.6, THUNDER_WEAK_DB));
    }
}
