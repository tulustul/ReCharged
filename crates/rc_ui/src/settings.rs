//! `settings.*` channels: the Settings screen (GDD §15) reads and live-applies settings; rc_game
//! is their only writer (validates, applies, persists). Bindings travel separately
//! (`bindings.get`, `input.rebind`).

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::GameCommand;

use crate::content::UiDifficulty;

mirror_enum!(
    /// [`SpeedUnit`].
    UiSpeedUnit = SpeedUnit { Kmh, Mph }
);
mirror_enum!(
    /// [`CameraMode`].
    UiCameraMode = CameraMode { Chase, FarChase, Hood }
);
mirror_enum!(
    /// [`SplitOrientation`].
    UiSplit = SplitOrientation { Horizontal, Vertical }
);
mirror_enum!(
    /// [`QualityPreset`].
    UiQuality = QualityPreset { Low, Medium, High, Ultra, Custom }
);
mirror_enum!(
    /// [`DisplayMode`].
    UiDisplayMode = DisplayMode { Windowed, Borderless }
);
mirror_enum!(
    /// [`ShadowQuality`].
    UiShadows = ShadowQuality { Off, Low, High }
);
mirror_enum!(
    /// [`AntiAliasing`].
    UiAntiAliasing = AntiAliasing { Off, Fxaa, Smaa, Taa }
);
mirror_enum!(
    /// [`ParticleDensity`].
    UiParticles = ParticleDensity { Low, Medium, High }
);

/// [`GameplaySettings`].
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)] // fields mirror the documented rc_core struct
pub struct UiGameplay {
    pub speed_unit: UiSpeedUnit,
    pub camera_mode: UiCameraMode,
    pub camera_shake: bool,
    pub hud_scale: f32,
    pub minimap: bool,
    pub split: UiSplit,
    pub ai_catch_up: bool,
    pub ai_difficulty: UiDifficulty,
    pub hints: bool,
}

/// [`GraphicsSettings`].
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)] // fields mirror the documented rc_core struct
pub struct UiGraphics {
    pub preset: UiQuality,
    pub display_mode: UiDisplayMode,
    pub window_size: [u32; 2],
    pub vsync: bool,
    pub fov_deg: f32,
    pub shadows: UiShadows,
    pub anti_aliasing: UiAntiAliasing,
    pub ambient_occlusion: bool,
    pub bloom: bool,
    pub motion_blur: bool,
    pub particles: UiParticles,
    pub volumetric_fog: bool,
}

/// [`AudioSettings`] (volumes 0–1).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)] // fields mirror the documented rc_core struct
pub struct UiAudio {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    pub engines: f32,
    pub ui: f32,
    pub ambience: f32,
    pub mute_unfocused: bool,
}

/// [`ControlSettings`] without the bindings.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)] // fields mirror the documented rc_core struct
pub struct UiControls {
    pub steer_dead_zone: f32,
    pub steer_sensitivity: f32,
    pub vibration: bool,
    pub vibration_intensity: f32,
}

/// [`AccessibilitySettings`].
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)] // fields mirror the documented rc_core struct
pub struct UiAccessibility {
    pub reduce_flashing: bool,
    pub colorblind_markers: bool,
    pub steering_assist: bool,
    pub auto_accelerate: bool,
}

/// [`Settings`] as the Settings screen edits it (bindings excluded).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiSettings {
    /// Gameplay tab.
    pub gameplay: UiGameplay,
    /// Graphics tab.
    pub graphics: UiGraphics,
    /// Audio tab.
    pub audio: UiAudio,
    /// Controls tab (bindings: `bindings.get`).
    pub controls: UiControls,
    /// Accessibility tab.
    pub accessibility: UiAccessibility,
}

impl From<&Settings> for UiSettings {
    fn from(s: &Settings) -> Self {
        let (g, gr, a, c, x) = (
            &s.gameplay,
            &s.graphics,
            &s.audio,
            &s.controls,
            &s.accessibility,
        );
        Self {
            gameplay: UiGameplay {
                speed_unit: g.speed_unit.into(),
                camera_mode: g.camera_mode.into(),
                camera_shake: g.camera_shake,
                hud_scale: g.hud_scale,
                minimap: g.minimap,
                split: g.split.into(),
                ai_catch_up: g.ai_catch_up,
                ai_difficulty: g.ai_difficulty.into(),
                hints: g.hints,
            },
            graphics: UiGraphics {
                preset: gr.preset.into(),
                display_mode: gr.display_mode.into(),
                window_size: gr.window_size,
                vsync: gr.vsync,
                fov_deg: gr.fov_deg,
                shadows: gr.shadows.into(),
                anti_aliasing: gr.anti_aliasing.into(),
                ambient_occlusion: gr.ambient_occlusion,
                bloom: gr.bloom,
                motion_blur: gr.motion_blur,
                particles: gr.particles.into(),
                volumetric_fog: gr.volumetric_fog,
            },
            audio: UiAudio {
                master: a.master,
                music: a.music,
                sfx: a.sfx,
                engines: a.engines,
                ui: a.ui,
                ambience: a.ambience,
                mute_unfocused: a.mute_unfocused,
            },
            controls: UiControls {
                steer_dead_zone: c.steer_dead_zone,
                steer_sensitivity: c.steer_sensitivity,
                vibration: c.vibration,
                vibration_intensity: c.vibration_intensity,
            },
            accessibility: UiAccessibility {
                reduce_flashing: x.reduce_flashing,
                colorblind_markers: x.colorblind_markers,
                steering_assist: x.steering_assist,
                auto_accelerate: x.auto_accelerate,
            },
        }
    }
}

impl UiSettings {
    /// `base` with these edits applied; the bindings of `base` are kept (rc_game clamps values).
    pub fn applied_to(&self, base: &Settings) -> Settings {
        let (g, gr, a, c, x) = (
            &self.gameplay,
            &self.graphics,
            &self.audio,
            &self.controls,
            &self.accessibility,
        );
        let mut s = base.clone();
        s.gameplay = GameplaySettings {
            speed_unit: g.speed_unit.into(),
            camera_mode: g.camera_mode.into(),
            camera_shake: g.camera_shake,
            hud_scale: g.hud_scale,
            minimap: g.minimap,
            split: g.split.into(),
            ai_catch_up: g.ai_catch_up,
            ai_difficulty: g.ai_difficulty.into(),
            hints: g.hints,
        };
        s.graphics = GraphicsSettings {
            preset: gr.preset.into(),
            display_mode: gr.display_mode.into(),
            window_size: gr.window_size,
            vsync: gr.vsync,
            fov_deg: gr.fov_deg,
            shadows: gr.shadows.into(),
            anti_aliasing: gr.anti_aliasing.into(),
            ambient_occlusion: gr.ambient_occlusion,
            bloom: gr.bloom,
            motion_blur: gr.motion_blur,
            particles: gr.particles.into(),
            volumetric_fog: gr.volumetric_fog,
        };
        s.audio = AudioSettings {
            master: a.master,
            music: a.music,
            sfx: a.sfx,
            engines: a.engines,
            ui: a.ui,
            ambience: a.ambience,
            mute_unfocused: a.mute_unfocused,
        };
        s.controls.steer_dead_zone = c.steer_dead_zone;
        s.controls.steer_sensitivity = c.steer_sensitivity;
        s.controls.vibration = c.vibration;
        s.controls.vibration_intensity = c.vibration_intensity;
        s.accessibility = AccessibilitySettings {
            reduce_flashing: x.reduce_flashing,
            colorblind_markers: x.colorblind_markers,
            steering_assist: x.steering_assist,
            auto_accelerate: x.auto_accelerate,
        };
        s
    }
}

/// React → Bevy request `settings.get`: the current [`UiSettings`].
#[react_request(name = "settings.get", response = UiSettings)]
pub struct GetSettings;

/// React → Bevy message `settings.apply`: live-apply edited settings (every change).
#[react_message(name = "settings.apply")]
pub struct ApplySettings(pub UiSettings);

pub(crate) fn register(app: &mut App) {
    app.add_react_request_handler(
        |request: On<Request<GetSettings>>, settings: Option<Res<Settings>>| match settings {
            Some(settings) => request.respond(UiSettings::from(&*settings)),
            None => request.respond_err("settings not loaded"),
        },
    )
    .add_react_handler(
        |apply: On<ApplySettings>,
         settings: Option<Res<Settings>>,
         mut out: MessageWriter<GameCommand>| {
            let Some(settings) = settings else { return };
            let edited = apply.event().0.applied_to(&settings);
            if edited != *settings {
                out.write(GameCommand::ApplySettings(Box::new(edited)));
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_settings_and_bindings() {
        let mut base = Settings::default();
        base.controls
            .keyboard_a
            .get_mut(DriveAction::Boost)
            .push(Binding::Key(KeyCode::KeyB));
        let mut ui = UiSettings::from(&base);
        assert_eq!(ui.applied_to(&base), base);
        ui.audio.music = 0.25;
        ui.graphics.anti_aliasing = UiAntiAliasing::Smaa;
        let edited = ui.applied_to(&base);
        assert_eq!(edited.audio.music, 0.25);
        assert_eq!(edited.graphics.anti_aliasing, AntiAliasing::Smaa);
        assert_eq!(edited.controls.keyboard_a, base.controls.keyboard_a);
    }
}
