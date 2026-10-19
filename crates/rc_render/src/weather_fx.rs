//! Weather visuals (GDD §10.2): the frame's [`Look`] = the environment's time-of-day look blended
//! with `WeatherState` — cloud cover dims and softens the key light, greys the sky and the grading
//! and opens the exposure; fog shortens the visibility; lightning flashes (softened with *Reduce
//! flashing*); wetness, puddles and snow cover drive the surface materials. Indoors
//! (`outside_only`) the weather only reaches the window light and the window glass.

use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use rc_core::prelude::*;

use crate::lighting::{ActiveEnvironment, EnvironmentDef, SunDef, TimeLook};
use crate::materials::{MaterialLibrary, SurfaceMaterial};
use crate::sky::clouds::CloudLook;
use crate::sky::sun_direction;

/// Visibility (u) at fog density 1 (the Fog preset, 0.85, sees ≈ 40 u: the next corner, not the
/// far fence).
const FOG_MIN_VISIBILITY_U: f32 = 25.0;
/// Fog veils the sun and the sky like a low cloud deck: the overcast amount it brings, per unit
/// of fog density (outdoors).
const FOG_OVERCAST: f32 = 0.9;
/// Dense fog is a pale grey of the scene's brightness: share of that grey in the fog colour per
/// unit of fog density, and its brightness relative to the clear-weather fog colour.
const FOG_GREY_MIX: f32 = 0.85;
const FOG_GREY_GAIN: f32 = 1.15;
/// Peak illuminance of a lightning flash outdoors / through windows (lux), at most this many times
/// the illuminance the exposure renders at mid grey: a flash at noon, not a white frame at night.
const FLASH_LUX_OUTDOOR: f32 = 60_000.0;
const FLASH_LUX_INDOOR: f32 = 9_000.0;
const FLASH_MAX_EXPOSED: f32 = 2.5;
/// *Reduce flashing*: flash scale and smoothing time constant (s).
const SOFT_FLASH_SCALE: f32 = 0.25;
const SOFT_FLASH_TAU_S: f32 = 0.6;
/// Puddle amount per unit of wetness.
const PUDDLES_PER_WETNESS: f32 = 0.85;

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Flash>().add_systems(
        Update,
        (update_flash, compute_look, apply_surface_weather)
            .chain()
            .in_set(VisualSet)
            .in_set(LookSet),
    );
}

/// Systems that produce the frame's [`Look`] (others read it after this set).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LookSet;

/// One directional light of the look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LightLook {
    /// Unit vector toward the light.
    pub toward: Vec3,
    pub color: Color,
    pub lux: f32,
    pub shadows: bool,
}

impl LightLook {
    fn from_def(def: &SunDef, scale: f32) -> Self {
        Self {
            toward: sun_direction(def.elevation, def.azimuth),
            color: srgb(def.color),
            lux: def.lux * scale,
            shadows: def.shadows,
        }
    }
}

/// What the scene looks like this frame. Absent outside environments (Boot, Loading screens).
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct Look {
    pub key: LightLook,
    pub fill: Option<LightLook>,
    pub flash: LightLook,
    pub exposure: f32,
    pub env_light: f32,
    pub ambient_color: Color,
    pub ambient: f32,
    pub fog_color: Color,
    pub fog_visibility: f32,
    pub fog_glow: Color,
    pub grading: ColorGrading,
    pub bloom: f32,
    pub lamps: f32,
    pub clouds: CloudLook,
    pub window: [f32; 3],
    pub rain_on_glass: f32,
    pub wet: f32,
    pub puddles: f32,
    pub snow: f32,
}

/// Lightning brightness after the accessibility filter.
#[derive(Resource, Debug, Default)]
pub(crate) struct Flash(pub f32);

fn srgb(c: [f32; 3]) -> Color {
    Color::srgb(c[0], c[1], c[2])
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Overcast amount 0–1 from cloud cover (Clear 0.1 → 0, Cloudy 0.8 → ≈ 0.86, Storm → 1).
pub fn overcast(cloudiness: f32) -> f32 {
    smoothstep(0.15, 1.0, cloudiness)
}

/// Fraction of direct sun that gets through the clouds.
pub fn sun_through_clouds(cloudiness: f32) -> f32 {
    1.0 - 0.9 * overcast(cloudiness)
}

/// Visibility (u) with fog density `fog` 0–1, from the clear-weather `base`.
pub fn fog_visibility(base: f32, fog: f32) -> f32 {
    let t = fog.clamp(0.0, 1.0).powf(0.75);
    let (a, b) = (
        base.max(FOG_MIN_VISIBILITY_U).ln(),
        FOG_MIN_VISIBILITY_U.ln(),
    );
    (a + (b - a) * t).exp()
}

/// The fog colour at density `fog`: the clear-weather colour (sky haze, often blue) turns into a
/// pale, slightly cool grey of its own brightness, so dense fog reads as fog rather than haze.
pub fn fog_tone(clear: Color, fog: f32) -> Color {
    let lin = clear.to_linear();
    let luma = (0.2126 * lin.red + 0.7152 * lin.green + 0.0722 * lin.blue) * FOG_GREY_GAIN;
    let grey = Color::linear_rgb(luma * 0.97, luma, luma * 1.04);
    clear.mix(&grey, fog.clamp(0.0, 1.0) * FOG_GREY_MIX)
}

/// The illuminance (lux) that exposure `ev100` renders as mid grey on a mid-grey surface (Bevy:
/// pixel = luminance × 2^-EV / 1.2; a Lambertian albedo ρ under E lux has luminance ρE/π).
pub fn exposed_lux(ev100: f32) -> f32 {
    1.2 * std::f32::consts::PI * ev100.exp2()
}

/// Lightning as shown: as is, or with *Reduce flashing* a weak, smoothed glow (`prev` = last
/// frame's value).
pub fn filtered_flash(raw: f32, prev: f32, reduce: bool, dt: f32) -> f32 {
    if !reduce {
        return raw.clamp(0.0, 1.0);
    }
    let target = raw.clamp(0.0, 1.0) * SOFT_FLASH_SCALE;
    let k = 1.0 - (-dt / SOFT_FLASH_TAU_S).exp();
    prev + (target - prev) * k
}

/// The look of `env` at `time` under `w` with lightning brightness `flash`.
pub(crate) fn look(env: &EnvironmentDef, time: TimeOfDay, w: &WeatherState, flash: f32) -> Look {
    let base: &TimeLook = env.look(time);
    let indoor = env.indoor;
    let fog = if indoor { 0.0 } else { w.fog.clamp(0.0, 1.0) };
    // Fog is a cloud on the ground: it dims and softens the sun and greys the sky too.
    let cloud = w.cloudiness.clamp(0.0, 1.0).max(fog * FOG_OVERCAST);
    let s = overcast(cloud);
    let sun = sun_through_clouds(cloud);
    let snowy = if w.snowing {
        w.precipitation.max(w.snow_cover)
    } else {
        w.snow_cover
    };

    let key = LightLook::from_def(&base.sun, sun);
    let fill = base.fill.as_ref().map(|f| LightLook::from_def(f, 1.0));
    // Clouds: brighter exposure for the darker scene, less sky IBL, grey flat ambient instead.
    let exposure = base.exposure - 1.6 * s;
    let flash_lux = if indoor {
        FLASH_LUX_INDOOR
    } else {
        FLASH_LUX_OUTDOOR
    }
    .min(FLASH_MAX_EXPOSED * exposed_lux(exposure));
    let flash_light = LightLook {
        toward: if indoor {
            sun_direction(base.sun.elevation.max(20.0), base.sun.azimuth)
        } else {
            sun_direction(70.0, base.sun.azimuth + 40.0)
        },
        color: Color::srgb(0.82, 0.87, 1.0),
        lux: flash * flash_lux,
        shadows: false,
    };

    let env_light = base.env_light * (1.0 - 0.35 * s);
    let cloud_ambient = if indoor { 0.0 } else { s * base.sun.lux * 0.02 };
    let fog_base = srgb(base.fog_color);
    let fog_grey = fog_tone(
        fog_base.mix(&Color::srgb(0.62, 0.64, 0.67).mix(&fog_base, 0.3), s * 0.8),
        fog,
    );
    let fog_visibility = if indoor {
        base.fog_visibility
    } else {
        fog_visibility(base.fog_visibility, fog)
    };

    let g = &base.grading;
    let grading = ColorGrading {
        global: ColorGradingGlobal {
            exposure: 0.0,
            temperature: g.temperature - 0.05 * s - 0.04 * snowy,
            tint: g.tint,
            hue: 0.0,
            post_saturation: g.post_saturation * (1.0 - 0.1 * s),
            midtones_range: 0.2..0.7,
        },
        shadows: ColorGradingSection {
            lift: g.lift,
            ..section(g, s)
        },
        midtones: section(g, s),
        highlights: ColorGradingSection {
            gain: g.gain,
            ..section(g, s)
        },
    };

    let window_grey = [0.55, 0.58, 0.62].map(|v| v * base.window.iter().sum::<f32>() / 3.0);
    let window = std::array::from_fn(|i| {
        let c = base.window[i] + (window_grey[i] * 0.55 - base.window[i]) * s;
        c + flash * 2.5
    });
    let rain = if w.snowing {
        0.0
    } else {
        w.precipitation.clamp(0.0, 1.0)
    };

    Look {
        key,
        fill,
        flash: flash_light,
        exposure,
        env_light,
        ambient_color: srgb(base.ambient_color).mix(&Color::srgb(0.8, 0.82, 0.86), s),
        ambient: base.ambient + cloud_ambient,
        fog_color: fog_grey,
        fog_visibility,
        fog_glow: key
            .color
            .with_alpha(1.0)
            .mix(&Color::BLACK, 1.0 - 0.35 * sun),
        grading,
        bloom: base.bloom,
        lamps: base.lamps,
        clouds: CloudLook {
            lit: base.clouds.lit,
            shade: base.clouds.shade,
            coverage: cloud,
            rain: rain.max(s * 0.5),
            toward_sun: key.toward,
            flash,
            wind: w.wind,
        },
        window,
        rain_on_glass: rain,
        wet: w.wetness,
        puddles: w.wetness * PUDDLES_PER_WETNESS,
        snow: w.snow_cover,
    }
}

fn section(g: &crate::lighting::GradingDef, overcast: f32) -> ColorGradingSection {
    ColorGradingSection {
        saturation: g.saturation * (1.0 - 0.22 * overcast),
        contrast: g.contrast * (1.0 - 0.08 * overcast),
        gamma: 1.0,
        gain: 1.0,
        lift: 0.0,
    }
}

fn update_flash(
    time: Res<Time>,
    weather: Option<Res<WeatherState>>,
    settings: Option<Res<Settings>>,
    mut flash: ResMut<Flash>,
) {
    let raw = weather.map_or(0.0, |w| w.lightning);
    let reduce = settings.is_some_and(|s| s.accessibility.reduce_flashing);
    flash.0 = filtered_flash(raw, flash.0, reduce, time.delta_secs());
}

fn compute_look(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    weather: Option<Res<WeatherState>>,
    flash: Res<Flash>,
    current: Option<ResMut<Look>>,
) {
    let Some(env) = env else {
        if current.is_some() {
            commands.remove_resource::<Look>();
        }
        return;
    };
    let w = weather.map(|w| w.clone()).unwrap_or_default();
    let next = look(&env.def, env.time, &w, flash.0);
    match current {
        Some(mut look) => {
            look.set_if_neq(next);
        }
        None => commands.insert_resource(next),
    }
}

/// Wetness, puddles and snow → every environment material.
fn apply_surface_weather(
    look: Option<Res<Look>>,
    library: Option<Res<MaterialLibrary>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
) {
    let (Some(look), Some(library)) = (look, library) else {
        return;
    };
    for handle in library.environment_materials() {
        let changed = materials.get(handle).is_some_and(|m| {
            m.extension.weather.xyz() != Vec3::new(look.wet, look.puddles, look.snow)
        });
        if changed && let Some(mut m) = materials.get_mut(handle) {
            m.extension.set_weather(look.wet, look.puddles, look.snow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clouds_dim_the_sun_monotonically() {
        assert!(
            (sun_through_clouds(0.1) - 1.0).abs() < 1e-6,
            "Clear keeps the sun"
        );
        let cloudy = sun_through_clouds(0.8);
        assert!((0.15..0.35).contains(&cloudy), "{cloudy}");
        assert!((sun_through_clouds(1.0) - 0.1).abs() < 1e-6);
        let mut last = 1.0;
        for i in 0..=20 {
            let v = sun_through_clouds(i as f32 / 20.0);
            assert!(v <= last + 1e-6);
            last = v;
        }
    }

    #[test]
    fn fog_shortens_visibility_from_clear_to_the_fog_preset() {
        assert!((fog_visibility(2000.0, 0.0) - 2000.0).abs() < 1e-2);
        let fog = fog_visibility(2400.0, 0.85);
        assert!((32.0..50.0).contains(&fog), "dense but drivable: {fog}");
        let rain = fog_visibility(2000.0, 0.15);
        assert!(rain > 500.0 && rain < 2000.0, "{rain}");
        assert!((fog_visibility(2000.0, 1.0) - FOG_MIN_VISIBILITY_U).abs() < 1e-2);
    }

    /// The Fog preset outdoors: short visibility, a dimmed and softened sun, a greyer, paler fog
    /// colour than the clear sky haze; indoors only the window light would change.
    #[test]
    fn fog_reads_as_fog() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let env = EnvironmentDef::load(&assets, "garden").expect("garden loads");
        let clear = WeatherState {
            cloudiness: 0.1,
            ..default()
        };
        let foggy = WeatherState {
            cloudiness: 0.5,
            fog: 0.85,
            ..default()
        };
        let a = look(&env, TimeOfDay::Noon, &clear, 0.0);
        let b = look(&env, TimeOfDay::Noon, &foggy, 0.0);
        assert!(b.fog_visibility < 50.0, "{}", b.fog_visibility);
        assert!(b.key.lux < 0.35 * a.key.lux, "the sun is veiled");
        let saturation = |c: Color| {
            let l = c.to_linear();
            let (hi, lo) = (
                l.red.max(l.green).max(l.blue),
                l.red.min(l.green).min(l.blue),
            );
            (hi - lo) / hi.max(1e-6)
        };
        assert!(saturation(b.fog_color) < 0.5 * saturation(a.fog_color));
        assert!(b.clouds.coverage > 0.7, "a grey sky: {}", b.clouds.coverage);
    }

    /// A strike lights a noon storm as before (60 klx); at sunset and at night it lights the scene
    /// a few times as brightly as the exposure's mid grey instead of whiting the frame out (QA).
    #[test]
    fn lightning_flashes_without_whiting_out_the_night() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let storm = WeatherState {
            cloudiness: 1.0,
            ..default()
        };
        let street = EnvironmentDef::load(&assets, "street").expect("street loads");
        assert_eq!(
            look(&street, TimeOfDay::Noon, &storm, 1.0).flash.lux,
            FLASH_LUX_OUTDOOR
        );
        let toys = EnvironmentDef::load(&assets, "toy_store").expect("toy store loads");
        for (env, time) in [
            (&street, TimeOfDay::Sunset),
            (&street, TimeOfDay::Night),
            (&toys, TimeOfDay::Night),
        ] {
            let l = look(env, time, &storm, 1.0);
            let ratio = l.flash.lux / exposed_lux(l.exposure);
            assert!(
                (1.0..=FLASH_MAX_EXPOSED + 1e-3).contains(&ratio),
                "{time:?}: {ratio}"
            );
        }
        assert_eq!(look(&street, TimeOfDay::Night, &storm, 0.0).flash.lux, 0.0);
    }

    #[test]
    fn reduce_flashing_turns_strikes_into_soft_glows() {
        assert_eq!(filtered_flash(1.0, 0.0, false, 0.016), 1.0);
        let mut v = 0.0;
        for _ in 0..3 {
            v = filtered_flash(1.0, v, true, 0.016);
        }
        assert!(v < 0.05, "no sudden jump: {v}");
        for _ in 0..600 {
            v = filtered_flash(1.0, v, true, 0.016);
        }
        assert!(
            (v - SOFT_FLASH_SCALE).abs() < 0.01,
            "settles at a weak glow: {v}"
        );
    }
}
