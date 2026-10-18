//! Graphics quality (GDD §15, ARCHITECTURE §5.8, §9): `Settings.graphics` + the split-screen tier →
//! what each 3D view renders ([`ViewQuality`]) and how the sun casts shadows ([`ShadowTier`]).
//!
//! The individual options of `GraphicsSettings` are authoritative (the UI fills them from the
//! preset); the preset only picks the detail of things without an option of their own (SSAO and
//! SMAA quality, shadow-map resolution). Rules:
//! - TAA only in single view (broken with `Camera::viewport` in 0.20.0); split-screen maps it to
//!   SMAA. MSAA is never used (SSAO needs `Msaa::Off`; post AA covers it).
//! - 2 views: SSAO at most Medium, 3 shadow cascades. 3–4 views: no SSAO, no motion blur, shadows
//!   at most Low with 2 cascades. Volumetric fog only at Ultra in single view.

mod camera;
mod window;

use bevy::anti_alias::smaa::SmaaPreset;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel as SsaoLevel;
use bevy::prelude::*;
use rc_core::prelude::*;

pub(crate) use camera::{QualityCamera, loading_players, views};

pub(crate) fn build(app: &mut App) {
    camera::build(app);
    window::build(app);
}

/// Detail level for options without their own setting (`Custom` counts as High).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Detail {
    Low,
    Medium,
    High,
    Ultra,
}

impl Detail {
    fn of(preset: QualityPreset) -> Self {
        match preset {
            QualityPreset::Low => Self::Low,
            QualityPreset::Medium => Self::Medium,
            QualityPreset::High | QualityPreset::Custom => Self::High,
            QualityPreset::Ultra => Self::Ultra,
        }
    }
}

/// Anti-aliasing a view actually uses.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AaMode {
    /// None.
    Off,
    /// FXAA.
    Fxaa,
    /// SMAA at a preset.
    Smaa(SmaaPreset),
    /// TAA (single view only).
    Taa,
}

impl std::fmt::Debug for AaMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => f.write_str("Off"),
            Self::Fxaa => f.write_str("Fxaa"),
            Self::Smaa(p) => write!(
                f,
                "Smaa({})",
                match p {
                    SmaaPreset::Low => "Low",
                    SmaaPreset::Medium => "Medium",
                    SmaaPreset::High => "High",
                    SmaaPreset::Ultra => "Ultra",
                }
            ),
            Self::Taa => f.write_str("Taa"),
        }
    }
}

/// What one 3D view renders, from the settings and the number of player views.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewQuality {
    /// Anti-aliasing.
    pub aa: AaMode,
    /// SSAO quality, `None` = off.
    pub ssao: Option<SsaoLevel>,
    /// Bloom.
    pub bloom: bool,
    /// Motion blur.
    pub motion_blur: bool,
    /// Screen-space contact shadows (tyres on the ground).
    pub contact_shadows: bool,
    /// Volumetric fog and god rays.
    pub volumetric_fog: bool,
}

/// How the shadow-casting sun renders its cascades.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowTier {
    /// Shadow maps on.
    pub enabled: bool,
    /// `DirectionalLightShadowMap::size` (per cascade, px).
    pub map_size: usize,
    /// Number of cascades.
    pub cascades: usize,
    /// Multiplier on the environment's shadow distance.
    pub distance_scale: f32,
}

/// Quality of one view; `views` = number of player views on screen (1 in menus).
pub fn view_quality(g: &GraphicsSettings, views: usize) -> ViewQuality {
    let detail = Detail::of(g.preset);
    let split = views > 1;
    let smaa = |d: Detail| match d {
        Detail::Low => SmaaPreset::Low,
        Detail::Medium => SmaaPreset::Medium,
        Detail::High => SmaaPreset::High,
        Detail::Ultra => SmaaPreset::Ultra,
    };
    let aa = match g.anti_aliasing {
        AntiAliasing::Off => AaMode::Off,
        AntiAliasing::Fxaa => AaMode::Fxaa,
        AntiAliasing::Smaa => AaMode::Smaa(smaa(detail)),
        AntiAliasing::Taa if split => AaMode::Smaa(smaa(detail.max(Detail::High))),
        AntiAliasing::Taa => AaMode::Taa,
    };
    let ssao_detail = if views == 2 {
        detail.min(Detail::Medium)
    } else {
        detail
    };
    let ssao = (g.ambient_occlusion && views <= 2).then_some(match ssao_detail {
        Detail::Low => SsaoLevel::Low,
        Detail::Medium => SsaoLevel::Medium,
        Detail::High => SsaoLevel::High,
        Detail::Ultra => SsaoLevel::Ultra,
    });
    ViewQuality {
        aa,
        ssao,
        bloom: g.bloom,
        motion_blur: g.motion_blur && views <= 2,
        contact_shadows: g.shadows != ShadowQuality::Off && detail >= Detail::High && !split,
        volumetric_fog: g.volumetric_fog
            && !split
            && matches!(g.preset, QualityPreset::Ultra | QualityPreset::Custom),
    }
}

/// Sun shadows for `views` player views (directional shadow maps render once per view).
pub fn shadow_tier(g: &GraphicsSettings, views: usize) -> ShadowTier {
    let detail = Detail::of(g.preset);
    let shadows = if views >= 3 {
        g.shadows.min_low()
    } else {
        g.shadows
    };
    match shadows {
        ShadowQuality::Off => ShadowTier {
            enabled: false,
            map_size: 1024,
            cascades: 1,
            distance_scale: 1.0,
        },
        ShadowQuality::Low => ShadowTier {
            enabled: true,
            map_size: 1024,
            cascades: 2,
            distance_scale: 0.6,
        },
        ShadowQuality::High => ShadowTier {
            enabled: true,
            map_size: if detail == Detail::Ultra && views == 1 {
                4096
            } else {
                2048
            },
            cascades: if views == 1 { 4 } else { 3 },
            distance_scale: 1.0,
        },
    }
}

trait ShadowQualityExt {
    fn min_low(self) -> ShadowQuality;
}

impl ShadowQualityExt for ShadowQuality {
    fn min_low(self) -> ShadowQuality {
        match self {
            ShadowQuality::Off => ShadowQuality::Off,
            ShadowQuality::Low | ShadowQuality::High => ShadowQuality::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(f: impl FnOnce(&mut GraphicsSettings)) -> GraphicsSettings {
        let mut g = GraphicsSettings::default();
        f(&mut g);
        g
    }

    #[test]
    fn taa_only_in_single_view_and_split_screen_falls_back_to_smaa() {
        let g = settings(|g| g.anti_aliasing = AntiAliasing::Taa);
        assert_eq!(view_quality(&g, 1).aa, AaMode::Taa);
        for views in 2..=4 {
            assert!(
                matches!(view_quality(&g, views).aa, AaMode::Smaa(_)),
                "{views} views"
            );
        }
        let low = settings(|g| {
            g.anti_aliasing = AntiAliasing::Taa;
            g.preset = QualityPreset::Low;
        });
        assert_eq!(view_quality(&low, 2).aa, AaMode::Smaa(SmaaPreset::High));
    }

    #[test]
    fn explicit_aa_choices_are_kept_in_every_layout() {
        for (setting, expected) in [
            (AntiAliasing::Off, AaMode::Off),
            (AntiAliasing::Fxaa, AaMode::Fxaa),
        ] {
            let g = settings(|g| g.anti_aliasing = setting);
            for views in 1..=4 {
                assert_eq!(view_quality(&g, views).aa, expected);
            }
        }
        let g = settings(|g| {
            g.anti_aliasing = AntiAliasing::Smaa;
            g.preset = QualityPreset::Ultra;
        });
        assert_eq!(view_quality(&g, 3).aa, AaMode::Smaa(SmaaPreset::Ultra));
    }

    #[test]
    fn ssao_follows_the_setting_and_drops_with_more_views() {
        let g = settings(|g| g.preset = QualityPreset::Ultra);
        assert_eq!(view_quality(&g, 1).ssao, Some(SsaoLevel::Ultra));
        assert_eq!(view_quality(&g, 2).ssao, Some(SsaoLevel::Medium));
        assert_eq!(view_quality(&g, 3).ssao, None);
        let off = settings(|g| g.ambient_occlusion = false);
        assert_eq!(view_quality(&off, 1).ssao, None);
    }

    #[test]
    fn volumetric_fog_needs_ultra_single_view_and_the_setting() {
        let on = settings(|g| {
            g.preset = QualityPreset::Ultra;
            g.volumetric_fog = true;
        });
        assert!(view_quality(&on, 1).volumetric_fog);
        assert!(!view_quality(&on, 2).volumetric_fog);
        let high = settings(|g| g.volumetric_fog = true);
        assert!(!view_quality(&high, 1).volumetric_fog, "High preset");
        let unset = settings(|g| g.preset = QualityPreset::Ultra);
        assert!(!view_quality(&unset, 1).volumetric_fog);
    }

    #[test]
    fn defaults_give_the_high_single_view_look() {
        let q = view_quality(&GraphicsSettings::default(), 1);
        assert_eq!(q.aa, AaMode::Taa);
        assert_eq!(q.ssao, Some(SsaoLevel::High));
        assert!(q.bloom && q.contact_shadows && !q.motion_blur && !q.volumetric_fog);
    }

    #[test]
    fn shadow_tiers_shrink_with_split_screen() {
        let g = GraphicsSettings::default();
        let one = shadow_tier(&g, 1);
        assert!(one.enabled);
        assert_eq!((one.map_size, one.cascades), (2048, 4));
        assert_eq!(shadow_tier(&g, 2).cascades, 3);
        let four = shadow_tier(&g, 4);
        assert_eq!((four.map_size, four.cascades), (1024, 2));
        let ultra = settings(|g| g.preset = QualityPreset::Ultra);
        assert_eq!(shadow_tier(&ultra, 1).map_size, 4096);
        assert_eq!(shadow_tier(&ultra, 2).map_size, 2048);
        let off = settings(|g| g.shadows = ShadowQuality::Off);
        assert!(!shadow_tier(&off, 1).enabled);
    }
}
