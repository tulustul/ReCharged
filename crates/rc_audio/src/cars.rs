//! Car sounds. Every racer gets an engine loop on the Engines bus: playback rate from rpm and
//! load (× a per-car pitch from its weight), level from load, attenuated and panned by the
//! nearest camera — local players hear their own engine centred (−6 dB each in split-screen).
//! Local players' cars also loop layers on the Sfx bus, silent until needed: skid (hard floors) /
//! scrub (soft), surface rumble (grass, dirt, water), drift sizzle (rate follows the charge gain
//! rate, GDD §5.3) and boost crackle. One-shots: boost start (any boost, from telemetry) and the
//! charge-ready chime (local players). Voices are reconciled with the live racers every frame.

use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::bank::Bank;
use crate::buses::{EngineBus, SILENCE_DB, SfxBus};
use crate::output::audio_on;
use crate::shots::Shots;
use crate::spatial::{Ears, REACH_CAR, Spot, spot};

/// Playback rate: idle + span·rpm + load term (+ boost), clamped (engine loops are clean to 2.4).
const RATE_IDLE: f32 = 0.55;
const RATE_SPAN: f32 = 1.3;
const RATE_LOAD: f32 = 0.08;
const RATE_BOOST: f32 = 0.1;
const RATE_RANGE: (f32, f32) = (0.5, 2.2);
/// Engine level when coasting (load 0), dB; full load is 0 dB.
const ENGINE_COAST_DB: f32 = -9.0;
/// Each local player's own engine in split-screen, dB (GDD §16.3).
const SPLIT_OWN_ENGINE_DB: f32 = -6.0;
/// Per-car pitch: lightest (1★ weight) … heaviest (5★) car.
const PITCH_LIGHT: f64 = 1.12;
const PITCH_PER_WEIGHT_STAR: f64 = 0.05;
/// Speed that maps to "full" layer intensity, u/s (3★ top speed).
const SPEED_REF_UPS: f32 = 30.0;
/// Wheel slip where tyre layers start / reach full level; level at the start, dB.
const SLIP_START: f32 = 0.15;
const SLIP_FULL: f32 = 0.75;
const SLIP_QUIET_DB: f32 = -20.0;
/// Rumble: minimum speed, u/s, and level at walking pace, dB.
const RUMBLE_MIN_UPS: f32 = 1.5;
const RUMBLE_QUIET_DB: f32 = -22.0;
/// Drift sizzle: charge gain rate at full pitch (22 %/s, GDD §5.3) and its quietest level, dB.
const MAX_CHARGE_RATE: f32 = 0.22;
const DRIFT_QUIET_DB: f32 = -12.0;
/// Smoothing of the measured charge gain rate (per-frame EMA weight).
const CHARGE_RATE_SMOOTHING: f32 = 0.15;
/// Charge level at which boost is ready (GDD §5.3: minimum 15 %).
const CHARGE_READY: f32 = 0.15;
/// Parameter changes are tweened over this, ms.
const VOICE_TWEEN_MS: u64 = 60;
/// Voices of a despawned racer fade out over this, ms.
const STOP_TWEEN_MS: u64 = 150;

/// Looped layers on local players' cars, in [`layer_mix`] order.
const LAYERS: [&str; 7] = [
    "skid_loop",
    "scrub_loop",
    "rumble_grass_loop",
    "rumble_dirt_loop",
    "rumble_water_loop",
    "drift_sizzle_loop",
    "boost_crackle_loop",
];

/// What the layer mix needs to know about a car this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CarFrame {
    pub(crate) speed: f32,
    pub(crate) grounded: bool,
    /// Largest wheel slip 0–1.
    pub(crate) slip: f32,
    pub(crate) surface: Option<SurfaceKind>,
    pub(crate) drifting: bool,
    pub(crate) boosting: bool,
    /// Charge gained per second (0–1 scale).
    pub(crate) charge_rate: f32,
}

/// Engine playback rate.
pub(crate) fn engine_rate(rpm: f32, load: f32, boosting: bool) -> f64 {
    let boost = if boosting { RATE_BOOST } else { 0.0 };
    let rate =
        RATE_IDLE + RATE_SPAN * rpm.clamp(0.0, 1.0) + RATE_LOAD * load.clamp(0.0, 1.0) + boost;
    f64::from(rate.clamp(RATE_RANGE.0, RATE_RANGE.1))
}

/// Engine level from load, dB.
pub(crate) fn engine_db(load: f32) -> f32 {
    ENGINE_COAST_DB * (1.0 - load.clamp(0.0, 1.0))
}

/// Per-car pitch multiplier from the weight stars (heavier = lower).
pub(crate) fn pitch_mul(weight: u8) -> f64 {
    PITCH_LIGHT - PITCH_PER_WEIGHT_STAR * f64::from(weight.clamp(1, 5) - 1)
}

/// Level (dB) and playback rate of every layer.
pub(crate) fn layer_mix(f: &CarFrame) -> [(f32, f64); 7] {
    let off = (SILENCE_DB, 1.0);
    let mut mix = [off; 7];
    let speed = (f.speed / SPEED_REF_UPS).clamp(0.0, 1.5);
    if f.grounded && f.slip > SLIP_START {
        let t = ((f.slip - SLIP_START) / (SLIP_FULL - SLIP_START)).min(1.0);
        let soft = matches!(
            f.surface,
            Some(
                SurfaceKind::Carpet
                    | SurfaceKind::Grass
                    | SurfaceKind::Dirt
                    | SurfaceKind::Water
                    | SurfaceKind::Ice
            )
        );
        mix[usize::from(soft)] = (SLIP_QUIET_DB * (1.0 - t), f64::from(0.9 + 0.25 * speed));
    }
    let rumble = match f.surface {
        Some(SurfaceKind::Grass) => Some(2),
        Some(SurfaceKind::Dirt) => Some(3),
        Some(SurfaceKind::Water) => Some(4),
        _ => None,
    };
    if let Some(i) = rumble.filter(|_| f.grounded && f.speed > RUMBLE_MIN_UPS) {
        mix[i] = (
            RUMBLE_QUIET_DB * (1.0 - speed.min(1.0)),
            f64::from((0.6 + speed).clamp(0.6, 1.6)),
        );
    }
    if f.drifting && f.grounded && f.charge_rate > 0.01 {
        let t = (f.charge_rate / MAX_CHARGE_RATE).min(1.0);
        mix[5] = (DRIFT_QUIET_DB * (1.0 - t), f64::from(0.8 + 0.8 * t));
    }
    if f.boosting {
        mix[6] = (0.0, 1.0);
    }
    mix
}

struct CarVoice {
    engine: Handle<AudioInstance>,
    pitch: f64,
    /// Local players only.
    layers: Option<[Handle<AudioInstance>; 7]>,
    charge: f32,
    charge_rate: f32,
    boosting: bool,
}

#[derive(Resource, Default)]
struct CarVoices(HashMap<Entity, CarVoice>);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<CarVoices>().add_systems(
        Update,
        (sync_voices, drive_voices)
            .chain()
            .in_set(Step::Cars)
            .run_if(audio_on),
    );
}

fn tween(ms: u64) -> AudioTween {
    AudioTween::linear(Duration::from_millis(ms))
}

fn silent_loop<T: Resource>(
    bus: &AudioChannel<T>,
    source: Handle<AudioSource>,
) -> Handle<AudioInstance> {
    bus.play(source)
        .looped()
        .with_volume(Decibels::SILENCE)
        .handle()
}

/// Starts voices for new racers and stops those of despawned ones.
fn sync_voices(
    racers: Query<(Entity, &Racer, Has<LocalPlayer>)>,
    catalog: Option<Res<Catalog>>,
    bank: Res<Bank>,
    engines: Res<AudioChannel<EngineBus>>,
    sfx: Res<AudioChannel<SfxBus>>,
    mut voices: ResMut<CarVoices>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    voices.0.retain(|&entity, voice| {
        let alive = racers.contains(entity);
        if !alive {
            for h in std::iter::once(&voice.engine).chain(voice.layers.iter().flatten()) {
                if let Some(mut i) = instances.get_mut(h) {
                    i.stop(tween(STOP_TWEEN_MS));
                }
            }
        }
        alive
    });
    for (entity, racer, local) in &racers {
        if voices.0.contains_key(&entity) {
            continue;
        }
        let car = catalog.as_ref().and_then(|c| c.car(&racer.car));
        let timbre = car.map(|c| c.engine).unwrap_or_default();
        let Some(source) = bank.sfx(timbre.stem()) else {
            continue;
        };
        // Every layer is a resident SFX; the guard only matters if a stem were missing.
        let layers = local
            .then(|| LAYERS.map(|stem| bank.sfx(stem)))
            .filter(|sources| sources.iter().all(Option::is_some))
            .map(|sources| sources.map(|s| silent_loop(&sfx, s.unwrap_or_default())));
        voices.0.insert(
            entity,
            CarVoice {
                engine: silent_loop(&engines, source),
                pitch: pitch_mul(car.map_or(3, |c| c.stars.weight)),
                layers,
                charge: 0.0,
                charge_rate: 0.0,
                boosting: false,
            },
        );
    }
}

#[allow(clippy::type_complexity)]
fn drive_voices(
    cars: Query<(&Transform, &VehicleTelemetry, &Charge, Has<LocalPlayer>), With<Racer>>,
    ears: Res<Ears>,
    time: Res<Time>,
    mut voices: ResMut<CarVoices>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut shots: Shots,
) {
    let locals = cars.iter().filter(|c| c.3).count();
    let dt = time.delta_secs();
    let t = tween(VOICE_TWEEN_MS);
    for (&entity, voice) in &mut voices.0 {
        let Ok((transform, tel, charge, local)) = cars.get(entity) else {
            continue;
        };
        let pos = transform.translation;
        let place = if local {
            let own = if locals > 1 { SPLIT_OWN_ENGINE_DB } else { 0.0 };
            Spot { db: own, pan: 0.0 }
        } else {
            spot(&ears.0, pos, REACH_CAR)
        };
        if let Some(mut i) = instances.get_mut(&voice.engine) {
            i.set_playback_rate(
                engine_rate(tel.rpm, tel.throttle_load, tel.boosting) * voice.pitch,
                t,
            );
            i.set_decibels(Decibels(engine_db(tel.throttle_load) + place.db), t);
            i.set_panning(place.pan, t);
        }
        // Charge gain rate (drift sizzle pitch) and the boost / charge-ready edges.
        if dt > 0.0 {
            let rate = ((charge.0 - voice.charge) / dt).max(0.0);
            voice.charge_rate += CHARGE_RATE_SMOOTHING * (rate - voice.charge_rate);
        }
        if local && voice.charge < CHARGE_READY && charge.0 >= CHARGE_READY {
            shots.play("charge_ready", 0.0, None);
        }
        if tel.boosting && !voice.boosting {
            shots.play("boost_whoosh", 0.0, (!local).then_some(pos));
        }
        voice.charge = charge.0;
        voice.boosting = tel.boosting;
        let Some(layers) = &voice.layers else {
            continue;
        };
        let frame = CarFrame {
            speed: tel.speed,
            grounded: tel.grounded_wheels > 0,
            slip: tel
                .wheels
                .iter()
                .map(|w| if w.grounded { w.slip } else { 0.0 })
                .fold(0.0, f32::max),
            surface: tel.surface,
            drifting: tel.drifting,
            boosting: tel.boosting,
            charge_rate: voice.charge_rate,
        };
        for (h, (db, rate)) in layers.iter().zip(layer_mix(&frame)) {
            if let Some(mut i) = instances.get_mut(h) {
                i.set_decibels(Decibels(db + place.db), t);
                i.set_playback_rate(rate, t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_rate_rises_with_rpm_and_load_within_the_clean_range() {
        assert!((engine_rate(0.0, 0.0, false) - 0.55).abs() < 1e-6);
        assert!(engine_rate(0.5, 0.0, false) < engine_rate(1.0, 0.0, false));
        assert!(engine_rate(0.5, 0.0, false) < engine_rate(0.5, 1.0, false));
        assert!(engine_rate(1.0, 1.0, false) < engine_rate(1.0, 1.0, true));
        let top = engine_rate(5.0, 5.0, true);
        assert!(
            (top - 2.03).abs() < 1e-6,
            "inputs are clamped: top speed + boost ≈ 2.03, got {top}"
        );
        assert!(top <= f64::from(RATE_RANGE.1));
        assert!(
            (engine_rate(-1.0, -1.0, false) - 0.55).abs() < 1e-6,
            "negative inputs = idle"
        );
    }

    #[test]
    fn engine_level_and_per_car_pitch() {
        assert_eq!(engine_db(1.0), 0.0);
        assert_eq!(engine_db(0.0), ENGINE_COAST_DB);
        assert!((pitch_mul(1) - 1.12).abs() < 1e-9, "Zip / Bolt");
        assert!((pitch_mul(5) - 0.92).abs() < 1e-9, "Dumpster");
        assert_eq!(pitch_mul(0), pitch_mul(1));
    }

    #[test]
    fn layers_follow_slip_surface_drift_and_boost() {
        let idle = layer_mix(&CarFrame {
            grounded: true,
            ..default()
        });
        assert!(idle.iter().all(|&(db, _)| db == SILENCE_DB));
        let skid = layer_mix(&CarFrame {
            grounded: true,
            slip: 0.9,
            speed: 20.0,
            surface: Some(SurfaceKind::Tile),
            ..default()
        });
        assert_eq!(skid[0].0, 0.0, "full skid on tiles");
        assert_eq!(skid[1].0, SILENCE_DB);
        let scrub = layer_mix(&CarFrame {
            grounded: true,
            slip: 0.45,
            surface: Some(SurfaceKind::Carpet),
            ..default()
        });
        assert!(scrub[1].0 > SLIP_QUIET_DB && scrub[1].0 < 0.0 && scrub[0].0 == SILENCE_DB);
        let airborne = layer_mix(&CarFrame {
            slip: 1.0,
            speed: 30.0,
            surface: Some(SurfaceKind::Grass),
            ..default()
        });
        assert!(
            airborne[..5].iter().all(|&(db, _)| db == SILENCE_DB),
            "no tyre/rumble in the air"
        );
        let lawn = layer_mix(&CarFrame {
            grounded: true,
            speed: 30.0,
            surface: Some(SurfaceKind::Grass),
            ..default()
        });
        assert!(
            lawn[2].0.abs() < 1e-6 && (lawn[2].1 - 1.6).abs() < 1e-6,
            "full rumble, fastest rate"
        );
        let drift = |rate| {
            layer_mix(&CarFrame {
                grounded: true,
                drifting: true,
                charge_rate: rate,
                ..default()
            })[5]
        };
        assert!(
            drift(0.22).1 > drift(0.05).1,
            "sizzle pitch rises with the charge rate"
        );
        assert_eq!(drift(0.0).0, SILENCE_DB);
        assert_eq!(
            layer_mix(&CarFrame {
                boosting: true,
                ..default()
            })[6],
            (0.0, 1.0)
        );
    }
}
