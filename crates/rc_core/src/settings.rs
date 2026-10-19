//! Player settings (GDD §15) with their defaults. Written only by rc_game (applies UI edits and
//! persists `settings.ron`); every struct is `#[serde(default)]`, so files missing newer fields
//! still load.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog::Difficulty;
use crate::input::{ActionBindings, ControlScheme};

/// All settings.
#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Gameplay tab.
    pub gameplay: GameplaySettings,
    /// Graphics tab.
    pub graphics: GraphicsSettings,
    /// Audio tab.
    pub audio: AudioSettings,
    /// Controls tab.
    pub controls: ControlSettings,
    /// Accessibility tab.
    pub accessibility: AccessibilitySettings,
}

impl Settings {
    /// Pretty RON for `settings.ron`.
    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("settings contain only RON-serialisable data")
    }

    /// Parses `settings.ron` (missing fields take their defaults).
    pub fn from_ron(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }
}

/// Displayed speed unit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpeedUnit {
    /// km/h (u/s × 1.8).
    #[default]
    Kmh,
    /// mph.
    Mph,
}

/// Race camera mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraMode {
    /// Chase (default).
    #[default]
    Chase,
    /// Far chase.
    FarChase,
    /// Hood.
    Hood,
}

/// Two-player split orientation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplitOrientation {
    /// Top / bottom.
    #[default]
    Horizontal,
    /// Left / right.
    Vertical,
}

/// Gameplay settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    /// Speed unit.
    pub speed_unit: SpeedUnit,
    /// Camera mode at race start.
    pub camera_mode: CameraMode,
    /// Camera shake on impacts/landings.
    pub camera_shake: bool,
    /// HUD scale, 0.75–1.25.
    pub hud_scale: f32,
    /// Show the minimap.
    pub minimap: bool,
    /// Show the FPS meter (top-right corner, menus and races).
    pub fps_meter: bool,
    /// 2P split orientation.
    pub split: SplitOrientation,
    /// AI catch-up.
    pub ai_catch_up: bool,
    /// Default AI difficulty for Quick Race / Versus.
    pub ai_difficulty: Difficulty,
    /// Onboarding hints.
    pub hints: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            speed_unit: SpeedUnit::Kmh,
            camera_mode: CameraMode::Chase,
            camera_shake: true,
            hud_scale: 1.0,
            minimap: true,
            fps_meter: false,
            split: SplitOrientation::Horizontal,
            ai_catch_up: true,
            ai_difficulty: Difficulty::Pro,
            hints: true,
        }
    }
}

/// Graphics quality preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityPreset {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High (default).
    #[default]
    High,
    /// Ultra (volumetric fog in single view).
    Ultra,
    /// Individual options changed.
    Custom,
}

/// Window mode (no exclusive fullscreen, GDD §20).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisplayMode {
    /// Windowed.
    #[default]
    Windowed,
    /// Borderless fullscreen.
    Borderless,
}

/// Shadow quality.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShadowQuality {
    /// No shadow maps.
    Off,
    /// Low.
    Low,
    /// High.
    #[default]
    High,
}

/// Anti-aliasing (TAA only in single view; split-screen maps TAA → SMAA).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AntiAliasing {
    /// None.
    Off,
    /// FXAA.
    Fxaa,
    /// SMAA.
    Smaa,
    /// TAA.
    #[default]
    Taa,
}

/// Particle density.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticleDensity {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    #[default]
    High,
}

/// Graphics settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    /// Preset (`Custom` once an option differs from it).
    pub preset: QualityPreset,
    /// Window mode.
    pub display_mode: DisplayMode,
    /// Windowed size in logical pixels.
    pub window_size: [u32; 2],
    /// Vertical sync.
    pub vsync: bool,
    /// Horizontal field of view at 16:9 in degrees, 60–100 (Hor+ for other aspects).
    pub fov_deg: f32,
    /// Shadows.
    pub shadows: ShadowQuality,
    /// Anti-aliasing.
    pub anti_aliasing: AntiAliasing,
    /// Screen-space ambient occlusion.
    pub ambient_occlusion: bool,
    /// Bloom.
    pub bloom: bool,
    /// Motion blur.
    pub motion_blur: bool,
    /// Particle density.
    pub particles: ParticleDensity,
    /// Volumetric fog (Ultra only, never in split-screen).
    pub volumetric_fog: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            preset: QualityPreset::High,
            display_mode: DisplayMode::Windowed,
            window_size: [1600, 900],
            vsync: true,
            fov_deg: 90.0,
            shadows: ShadowQuality::High,
            anti_aliasing: AntiAliasing::Taa,
            ambient_occlusion: true,
            bloom: true,
            motion_blur: false,
            particles: ParticleDensity::High,
            volumetric_fog: false,
        }
    }
}

/// Audio settings; volumes are linear 0–1 (bus = master × bus volume).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Master volume.
    pub master: f32,
    /// Music bus.
    pub music: f32,
    /// Sound-effects bus.
    pub sfx: f32,
    /// Engines bus.
    pub engines: f32,
    /// UI bus.
    pub ui: f32,
    /// Ambience bus.
    pub ambience: f32,
    /// Mute while the window is unfocused.
    pub mute_unfocused: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master: 1.0,
            music: 0.7,
            sfx: 0.9,
            engines: 0.8,
            ui: 0.8,
            ambience: 0.7,
            mute_unfocused: true,
        }
    }
}

/// Controls settings: bindings per scheme (defaults GDD §12.1) and stick/rumble options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    /// Keyboard scheme A.
    pub keyboard_a: ActionBindings,
    /// Keyboard scheme B.
    pub keyboard_b: ActionBindings,
    /// Gamepads.
    pub gamepad: ActionBindings,
    /// Steering dead zone 0–0.5.
    pub steer_dead_zone: f32,
    /// Steering sensitivity multiplier (1 = linear).
    pub steer_sensitivity: f32,
    /// Rumble on/off.
    pub vibration: bool,
    /// Rumble intensity 0–1.
    pub vibration_intensity: f32,
}

impl ControlSettings {
    /// Bindings of one scheme.
    pub fn bindings(&self, scheme: ControlScheme) -> &ActionBindings {
        match scheme {
            ControlScheme::KeyboardA => &self.keyboard_a,
            ControlScheme::KeyboardB => &self.keyboard_b,
            ControlScheme::Gamepad => &self.gamepad,
        }
    }

    /// Mutable bindings of one scheme.
    pub fn bindings_mut(&mut self, scheme: ControlScheme) -> &mut ActionBindings {
        match scheme {
            ControlScheme::KeyboardA => &mut self.keyboard_a,
            ControlScheme::KeyboardB => &mut self.keyboard_b,
            ControlScheme::Gamepad => &mut self.gamepad,
        }
    }
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            keyboard_a: ActionBindings::keyboard_a(),
            keyboard_b: ActionBindings::keyboard_b(),
            gamepad: ActionBindings::gamepad(),
            steer_dead_zone: 0.12,
            steer_sensitivity: 1.0,
            vibration: true,
            vibration_intensity: 1.0,
        }
    }
}

/// Accessibility settings (all off by default).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilitySettings {
    /// Soft glows instead of flashes.
    pub reduce_flashing: bool,
    /// Colour-blind friendly racer markers.
    pub colorblind_markers: bool,
    /// Steering assist toward the racing line.
    pub steering_assist: bool,
    /// Auto-accelerate.
    pub auto_accelerate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Binding, DriveAction};

    #[test]
    fn settings_round_trip_through_ron() {
        let mut s = Settings::default();
        s.audio.music = 0.25;
        s.gameplay.speed_unit = SpeedUnit::Mph;
        s.controls
            .keyboard_a
            .get_mut(DriveAction::Boost)
            .push(Binding::Key(KeyCode::KeyB));
        let text = s.to_ron();
        assert_eq!(Settings::from_ron(&text).expect("parses"), s);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s = Settings::from_ron("(audio: (music: 0.1))").expect("partial file parses");
        assert_eq!(s.audio.music, 0.1);
        assert_eq!(s.audio.master, AudioSettings::default().master);
        assert_eq!(s.controls, ControlSettings::default());
    }

    #[test]
    fn defaults_follow_the_gdd() {
        let s = Settings::default();
        assert!((0.75..=1.25).contains(&s.gameplay.hud_scale));
        assert!((60.0..=100.0).contains(&s.graphics.fov_deg));
        assert_eq!(s.controls.steer_dead_zone, 0.12);
        assert!(!s.graphics.volumetric_fog, "Ultra only");
        assert_eq!(s.accessibility, AccessibilitySettings::default());
    }
}
