//! Car paints (art bible §1.1, §2.3): `PaintInfo` → the clear-coated `paint` / `paint_accent`
//! materials of a car glb.

use bevy::prelude::*;
use rc_core::prelude::*;

/// Clear-coat roughness of solid paints (art bible: 0.03–0.06).
const COAT_ROUGHNESS_SOLID: f32 = 0.04;
/// Clear-coat roughness of metallic / pearl paints (0.04–0.08).
const COAT_ROUGHNESS_METALLIC: f32 = 0.06;
/// Paints at least this metallic count as metallic for the coat.
const METALLIC_PAINT: f32 = 0.3;
/// Mirror-like paints (chrome) get no coat: a second highlight would read as plastic.
const CHROME_PAINT: f32 = 0.95;

/// The two materials of one car paint (shared handles; see `MaterialLibrary::car_paint`).
#[derive(Debug, Clone)]
pub struct CarPaint {
    /// glTF material `paint`.
    pub base: Handle<StandardMaterial>,
    /// glTF material `paint_accent`.
    pub accent: Handle<StandardMaterial>,
}

impl CarPaint {
    pub(super) fn new(info: &PaintInfo, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            base: materials.add(paint_material(info.base, info.metallic, info.roughness)),
            accent: materials.add(paint_material(info.accent, info.metallic, info.roughness)),
        }
    }
}

/// The body material of a paint colour (sRGB 0–1): the "toy gloss" clear coat over the base.
pub fn paint_material(srgb: [f32; 3], metallic: f32, roughness: f32) -> StandardMaterial {
    let metallic = metallic.clamp(0.0, 1.0);
    let (clearcoat, coat_roughness) = if metallic >= CHROME_PAINT {
        (0.0, COAT_ROUGHNESS_METALLIC)
    } else if metallic >= METALLIC_PAINT {
        (1.0, COAT_ROUGHNESS_METALLIC)
    } else {
        (1.0, COAT_ROUGHNESS_SOLID)
    };
    StandardMaterial {
        base_color: Color::srgb(srgb[0], srgb[1], srgb[2]),
        metallic,
        perceptual_roughness: roughness.clamp(0.04, 1.0),
        clearcoat,
        clearcoat_perceptual_roughness: coat_roughness,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paints_follow_the_art_bible_coat_rules() {
        let solid = paint_material([0.8, 0.1, 0.1], 0.0, 0.35);
        assert_eq!(solid.clearcoat, 1.0);
        assert!((0.03..=0.06).contains(&solid.clearcoat_perceptual_roughness));
        let metallic = paint_material([0.3, 0.2, 0.6], 0.5, 0.38);
        assert!((0.04..=0.08).contains(&metallic.clearcoat_perceptual_roughness));
        let chrome = paint_material([0.9; 3], 1.0, 0.06);
        assert_eq!(chrome.clearcoat, 0.0, "no coat over chrome");
        assert_eq!(chrome.metallic, 1.0);
        let wild = paint_material([0.5; 3], 2.0, 0.0);
        assert!(wild.metallic <= 1.0 && wild.perceptual_roughness > 0.0);
    }

    #[test]
    fn paint_colours_are_srgb() {
        let m = paint_material([0.118, 0.384, 0.941], 0.0, 0.35);
        let srgba = m.base_color.to_srgba();
        assert!((srgba.red - 0.118).abs() < 1e-4 && (srgba.blue - 0.941).abs() < 1e-4);
    }
}
