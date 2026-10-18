//! Environment definitions: `assets/data/environments/<env>.ron` (one per `TrackInfo.environment`,
//! plus `showroom` for the menu). Pure data + loading/validation; light units are physical
//! (lux, real-world lumens via [`lamp_lumens`], EV100) and colours sRGB 0–1.

use std::collections::HashMap;
use std::path::Path;

use rc_core::prelude::*;
use serde::Deserialize;

use crate::materials::MaterialDef;

/// Directory of the environment files below the assets root.
pub const ENVIRONMENTS_DIR: &str = "data/environments";

/// World units per real metre is 4.44, so a real lamp's lumens are scaled by 4.44² to light the
/// same area (falloff is inverse-square in u).
pub const LUMEN_SCALE: f32 = 19.7;

/// Lumens of a lamp in world units from its real-world rating.
pub fn lamp_lumens(real: f32) -> f32 {
    real * LUMEN_SCALE
}

/// One environment (art bible §3): materials, surfaces, room and sky, and a look per time of day.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentDef {
    /// Display name (logs, debugging).
    pub name: String,
    /// Indoor: window light, room lamps and a room environment map instead of sun and sky.
    pub indoor: bool,
    /// Named materials (override the built-ins of the same name).
    #[serde(default)]
    pub materials: HashMap<String, MaterialDef>,
    /// Surface → material name (else the surface's built-in default).
    #[serde(default)]
    pub surfaces: HashMap<SurfaceKind, String>,
    /// Indoor room: ceiling height, windows, ceiling lamps.
    #[serde(default)]
    pub room: Option<RoomDef>,
    /// Outdoor sky.
    #[serde(default)]
    pub sky: SkyDef,
    /// Distance (u) up to which the key light casts shadows (High quality, single view).
    #[serde(default = "default_shadow_distance")]
    pub shadow_distance: f32,
    /// Fixed lights of this environment's track (display spots, local fills), in world units.
    #[serde(default)]
    pub lights: Vec<EnvLightDef>,
    /// Look at Morning.
    pub morning: TimeLook,
    /// Look at Noon.
    pub noon: TimeLook,
    /// Look at Sunset.
    pub sunset: TimeLook,
    /// Look at Night.
    pub night: TimeLook,
}

impl EnvironmentDef {
    /// The look of a time of day.
    pub fn look(&self, time: TimeOfDay) -> &TimeLook {
        match time {
            TimeOfDay::Morning => &self.morning,
            TimeOfDay::Noon => &self.noon,
            TimeOfDay::Sunset => &self.sunset,
            TimeOfDay::Night => &self.night,
        }
    }

    /// Reads and validates `<assets>/data/environments/<id>.ron`.
    pub fn load(assets_dir: &Path, id: &str) -> Result<Self, String> {
        let path = assets_dir.join(ENVIRONMENTS_DIR).join(format!("{id}.ron"));
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let def: Self = ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        def.validate()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(def)
    }

    /// Checks references and ranges.
    pub fn validate(&self) -> Result<(), String> {
        for (kind, name) in &self.surfaces {
            let known = self.materials.contains_key(name)
                || crate::materials::builtin_materials()
                    .iter()
                    .any(|(n, _)| n == name);
            if !known {
                return Err(format!("surfaces.{kind:?}: unknown material '{name}'"));
            }
        }
        for (name, m) in &self.materials {
            if let Some(set) = m.set.as_deref()
                && !crate::materials::TEXTURE_SETS
                    .iter()
                    .any(|(s, _)| *s == set)
            {
                return Err(format!("materials.{name}: unknown texture set '{set}'"));
            }
        }
        for (i, light) in self.lights.iter().enumerate() {
            if light.lumens < 0.0 || light.range <= 0.0 || !(1.0..=89.0).contains(&light.angle) {
                return Err(format!("lights[{i}]: lumens, range or angle out of range"));
            }
        }
        if self.shadow_distance <= 1.0 {
            return Err("shadow_distance must be > 1 u".into());
        }
        if self.indoor != self.room.is_some() {
            return Err("indoor environments need `room`, outdoor ones none".into());
        }
        if let Some(room) = &self.room {
            if room.height <= 0.0 {
                return Err("room.height must be > 0".into());
            }
            for (i, w) in room.windows.iter().enumerate() {
                let fits = (0.0..=1.0).contains(&w.center)
                    && w.width > 0.0
                    && w.height > 0.0
                    && w.sill >= 0.0
                    && w.sill + w.height <= room.height;
                if !fits {
                    return Err(format!(
                        "room.windows[{i}]: outside the wall or room height"
                    ));
                }
            }
        }
        for time in TimeOfDay::ALL {
            let look = self.look(time);
            let t = format!("{time:?}").to_lowercase();
            if !(-6.0..=20.0).contains(&look.exposure) {
                return Err(format!(
                    "{t}.exposure {} outside -6..20 EV100",
                    look.exposure
                ));
            }
            for sun in std::iter::once(&look.sun).chain(look.fill.as_ref()) {
                if !(-90.0..=90.0).contains(&sun.elevation) || sun.lux < 0.0 {
                    return Err(format!("{t}: sun/fill elevation or lux out of range"));
                }
            }
            if look.fog_visibility <= 0.0 {
                return Err(format!("{t}.fog_visibility must be > 0"));
            }
        }
        Ok(())
    }
}

/// A fixed light of the environment (one environment per track, so world positions are fine).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvLightDef {
    /// Position (u).
    pub pos: [f32; 3],
    /// Spot target (u); `None` = point light.
    #[serde(default)]
    pub target: Option<[f32; 3]>,
    /// Spot outer cone angle (degrees).
    #[serde(default = "default_spot_angle")]
    pub angle: f32,
    /// Colour (sRGB).
    pub color: [f32; 3],
    /// Real-world lumens (scaled by [`lamp_lumens`]).
    pub lumens: f32,
    /// Range (u).
    pub range: f32,
    /// Times of day it is on (empty = always).
    #[serde(default)]
    pub times: Vec<TimeOfDay>,
}

fn default_spot_angle() -> f32 {
    35.0
}

impl EnvLightDef {
    /// The light is on at `time`.
    pub fn on_at(&self, time: TimeOfDay) -> bool {
        self.times.is_empty() || self.times.contains(&time)
    }
}

/// An indoor room (the shell itself is built by the track visuals from `Track::bounds()`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomDef {
    /// Ceiling height (u above the floor at y = 0).
    pub height: f32,
    /// Window openings in the shell walls.
    #[serde(default)]
    pub windows: Vec<WindowDef>,
    /// Ceiling lamps over the room.
    #[serde(default)]
    pub lamps: Option<LampGrid>,
}

/// A wall of the room shell (`Track::bounds()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WindowWall {
    /// The −Z wall.
    North,
    /// The +Z wall.
    South,
    /// The +X wall.
    East,
    /// The −X wall.
    West,
}

/// A window opening; the track visuals cut it, rc_render glazes and lights it.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowDef {
    /// Wall.
    pub wall: WindowWall,
    /// Centre along the wall, 0–1 from its min-x / min-z end.
    pub center: f32,
    /// Width (u).
    pub width: f32,
    /// Height (u).
    pub height: f32,
    /// Sill height above the floor (u).
    pub sill: f32,
    /// An open door (roller door, patio door): no mullions or transom, no rain on the "glass".
    #[serde(default)]
    pub door: bool,
}

/// A grid of ceiling lamps over the room (on when the time's `lamps` is set).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LampGrid {
    /// Distance between lamps (u).
    pub spacing: f32,
    /// Lamp colour (sRGB).
    pub color: [f32; 3],
    /// Real-world lumens per lamp (scaled by [`lamp_lumens`]).
    pub lumens: f32,
    /// Light range (u).
    pub range: f32,
    /// Spot cone (outer angle, degrees) pointing down; `None` = point light.
    #[serde(default)]
    pub spot_angle: Option<f32>,
    /// Emissive strength of the lamp fixtures (art bible §1.2: 8–20).
    #[serde(default = "default_fixture")]
    pub fixture: f32,
}

fn default_shadow_distance() -> f32 {
    120.0
}

fn default_fixture() -> f32 {
    12.0
}

/// Outdoor sky options.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SkyDef {
    /// Average ground albedo for the atmosphere's multiple scattering.
    pub ground_albedo: [f32; 3],
    /// Star field brightness at night (cd/m²).
    pub stars: f32,
}

impl Default for SkyDef {
    fn default() -> Self {
        Self {
            ground_albedo: [0.3, 0.3, 0.3],
            stars: 2500.0,
        }
    }
}

/// A directional light: the sun (outdoor), window light (indoor), the moon or a fill.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SunDef {
    /// Degrees above the horizon.
    pub elevation: f32,
    /// Degrees clockwise from north (−Z) toward east (+X).
    pub azimuth: f32,
    /// Illuminance (lux). Outdoors the atmosphere attenuates and tints it.
    pub lux: f32,
    /// Colour (sRGB).
    pub color: [f32; 3],
    /// Casts shadows.
    pub shadows: bool,
}

impl Default for SunDef {
    fn default() -> Self {
        Self {
            elevation: 45.0,
            azimuth: 135.0,
            lux: 100_000.0,
            color: [1.0, 1.0, 1.0],
            shadows: true,
        }
    }
}

/// The look of one time of day.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeLook {
    /// Key light (sun / window light / moon).
    pub sun: SunDef,
    /// Second directional light without shadows (moon, sky bounce).
    pub fill: Option<SunDef>,
    /// Camera exposure (EV100).
    pub exposure: f32,
    /// Image-based lighting intensity multiplier (atmosphere outdoors, room map indoors).
    pub env_light: f32,
    /// Extra flat ambient light (cd/m²).
    pub ambient: f32,
    /// Its colour (sRGB).
    pub ambient_color: [f32; 3],
    /// Indoor environment-map colours (linear radiance scale set by `env_light`).
    pub room: RoomColors,
    /// Distance-fog colour (sRGB) in clear weather.
    pub fog_color: [f32; 3],
    /// Distance at which clear-weather fog hides 95 % of the scene (u).
    pub fog_visibility: f32,
    /// Colour grading.
    pub grading: GradingDef,
    /// Bloom intensity (emissives bloom above ≈ 1.0 after exposure).
    pub bloom: f32,
    /// Room lamps (indoor) on.
    pub lamps: bool,
    /// Cloud colours (display-referred: 1 ≈ a bright sky).
    pub clouds: CloudColors,
    /// Window glass emissive colour (indoor; display-referred).
    pub window: [f32; 3],
}

impl Default for TimeLook {
    fn default() -> Self {
        Self {
            sun: SunDef::default(),
            fill: None,
            exposure: 15.0,
            env_light: 1.0,
            ambient: 0.0,
            ambient_color: [1.0, 1.0, 1.0],
            room: RoomColors::default(),
            fog_color: [0.72, 0.78, 0.86],
            fog_visibility: 2000.0,
            grading: GradingDef::default(),
            bloom: 0.12,
            lamps: false,
            clouds: CloudColors::default(),
            window: [1.4, 1.5, 1.7],
        }
    }
}

/// Indoor environment-map colours (linear, before `env_light`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoomColors {
    /// Floor (below the horizon).
    pub floor: [f32; 3],
    /// Walls.
    pub walls: [f32; 3],
    /// Ceiling.
    pub ceiling: [f32; 3],
    /// Window patches (toward the key light).
    pub window: [f32; 3],
    /// Lamp spots on the ceiling (when lamps are on).
    pub lamp: [f32; 3],
}

impl Default for RoomColors {
    fn default() -> Self {
        Self {
            floor: [0.35, 0.28, 0.22],
            walls: [0.55, 0.52, 0.48],
            ceiling: [0.7, 0.7, 0.68],
            window: [6.0, 6.3, 7.0],
            lamp: [8.0, 6.5, 4.5],
        }
    }
}

/// Colour grading (Bevy `ColorGrading`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GradingDef {
    /// White balance shift (+ = warmer).
    pub temperature: f32,
    /// Tint (+ = magenta).
    pub tint: f32,
    /// Saturation before tonemapping.
    pub saturation: f32,
    /// Contrast.
    pub contrast: f32,
    /// Saturation after tonemapping.
    pub post_saturation: f32,
    /// Shadow lift (adds to the darks).
    pub lift: f32,
    /// Highlight gain.
    pub gain: f32,
}

impl Default for GradingDef {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            saturation: 1.0,
            contrast: 1.0,
            post_saturation: 1.0,
            lift: 0.0,
            gain: 1.0,
        }
    }
}

/// Cloud layer colours (display-referred).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CloudColors {
    /// Sun-lit tops.
    pub lit: [f32; 3],
    /// Shaded undersides.
    pub shade: [f32; 3],
}

impl Default for CloudColors {
    fn default() -> Self {
        Self {
            lit: [1.0, 0.98, 0.95],
            shade: [0.55, 0.6, 0.68],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    /// Every environment the catalog uses (and the showroom) loads and validates.
    #[test]
    fn every_environment_file_loads() {
        let catalog = Catalog::load(&assets()).expect("catalog loads");
        let mut ids: Vec<String> = catalog
            .tracks
            .iter()
            .map(|t| t.environment.clone())
            .collect();
        ids.push("showroom".into());
        ids.sort();
        ids.dedup();
        for id in ids {
            let def = EnvironmentDef::load(&assets(), &id).unwrap_or_else(|e| panic!("{e}"));
            let indoor_tracks = catalog
                .tracks
                .iter()
                .filter(|t| t.environment == id)
                .all(|t| t.indoor == def.indoor);
            assert!(
                indoor_tracks,
                "{id}: indoor flag disagrees with the catalog"
            );
        }
    }

    #[test]
    fn validation_names_the_bad_field() {
        let text = std::fs::read_to_string(assets().join(ENVIRONMENTS_DIR).join("test.ron"))
            .expect("test env exists");
        let mut def: EnvironmentDef = ron::from_str(&text).expect("parses");
        def.surfaces
            .insert(SurfaceKind::Ice, "no_such_material".into());
        let err = def.validate().expect_err("unknown material");
        assert!(err.contains("surfaces.Ice"), "{err}");
    }

    #[test]
    fn env_lights_follow_their_times() {
        let light = EnvLightDef {
            pos: [0.0; 3],
            target: None,
            angle: 35.0,
            color: [1.0; 3],
            lumens: 100.0,
            range: 10.0,
            times: vec![TimeOfDay::Night],
        };
        assert!(light.on_at(TimeOfDay::Night) && !light.on_at(TimeOfDay::Noon));
        let always = EnvLightDef {
            times: vec![],
            ..light
        };
        assert!(TimeOfDay::ALL.iter().all(|t| always.on_at(*t)));
    }

    #[test]
    fn lamp_lumens_scale_with_the_world() {
        assert!((lamp_lumens(1000.0) - 19_700.0).abs() < 1e-3);
    }
}
