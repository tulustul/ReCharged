//! The colour-blind marker shapes of the name tags: their outlines (the HUD draws the same, see
//! `ui/src/hud/markers.ts`) and the strip image the tag rows' sprites slice, rasterized once per
//! session with supersampling (the game font has no such glyphs).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rc_core::prelude::*;

/// Marker strip: one cell (px) per shape, in [`MarkerShape::ALL`] order.
const MARKER_CELL_PX: u32 = 64;
/// Supersampling per axis when rasterizing the strip (antialiasing).
const MARKER_SS: u32 = 4;

/// The outline of a marker shape in −1…1 (y up): ● ▲ ■ ◆ ★ ✚ ⬟ ⬢ (the HUD draws the same).
pub fn marker_polygon(shape: MarkerShape) -> Vec<Vec2> {
    let ring = |n: usize, r: f32| -> Vec<Vec2> {
        (0..n)
            .map(|i| Vec2::from_angle(FRAC_PI_2 + TAU * i as f32 / n as f32) * r)
            .collect()
    };
    let pts = |p: &[(f32, f32)]| p.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
    let (a, b) = (0.3, 0.9);
    match shape {
        MarkerShape::Circle => ring(40, 0.9),
        MarkerShape::Triangle => pts(&[(0.0, 0.9), (-0.95, -0.75), (0.95, -0.75)]),
        MarkerShape::Square => pts(&[(-0.75, -0.75), (0.75, -0.75), (0.75, 0.75), (-0.75, 0.75)]),
        MarkerShape::Diamond => pts(&[(0.0, 0.95), (0.78, 0.0), (0.0, -0.95), (-0.78, 0.0)]),
        MarkerShape::Star => (0..10)
            .map(|i| {
                let r = if i % 2 == 0 { 0.98 } else { 0.42 };
                Vec2::from_angle(FRAC_PI_2 + PI * i as f32 / 5.0) * r
            })
            .collect(),
        MarkerShape::Cross => pts(&[
            (-a, b),
            (a, b),
            (a, a),
            (b, a),
            (b, -a),
            (a, -a),
            (a, -b),
            (-a, -b),
            (-a, -a),
            (-b, -a),
            (-b, a),
            (-a, a),
        ]),
        MarkerShape::Pentagon => ring(5, 0.95),
        MarkerShape::Hexagon => ring(6, 0.95),
    }
}

/// Even-odd point-in-polygon test (the star and the cross are concave).
fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[j];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Coverage (0–1) of pixel `(x, y)` of a [`MARKER_CELL_PX`] cell by `poly`.
fn coverage(poly: &[Vec2], x: u32, y: u32) -> f32 {
    let cell = MARKER_CELL_PX as f32;
    let sub = |v: u32, s: u32| (v as f32 + (s as f32 + 0.5) / MARKER_SS as f32) / cell * 2.0 - 1.0;
    let hits = (0..MARKER_SS * MARKER_SS)
        .filter(|k| {
            let p = Vec2::new(sub(x, k % MARKER_SS), -sub(y, k / MARKER_SS));
            inside(poly, p)
        })
        .count();
    hits as f32 / (MARKER_SS * MARKER_SS) as f32
}

/// The marker shapes side by side, white on transparent (sprites tint and slice it).
pub(super) fn marker_strip() -> Image {
    let (cells, size) = (MarkerShape::ALL.len() as u32, MARKER_CELL_PX);
    let width = cells * size;
    let mut data = vec![0u8; (width * size * 4) as usize];
    for (cell, shape) in MarkerShape::ALL.into_iter().enumerate() {
        let poly = marker_polygon(shape);
        for y in 0..size {
            for x in 0..size {
                let i = ((y * width + cell as u32 * size + x) * 4) as usize;
                let alpha = (coverage(&poly, x, y) * 255.0).round() as u8;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, alpha]);
            }
        }
    }
    Image::new(
        Extent3d {
            width,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// The cell of `shape` in the marker strip.
pub(super) fn marker_rect(shape: MarkerShape) -> Rect {
    let i = MarkerShape::ALL
        .iter()
        .position(|&s| s == shape)
        .unwrap_or(0) as f32;
    let cell = MARKER_CELL_PX as f32;
    Rect::new(i * cell, 0.0, (i + 1.0) * cell, cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_shapes_are_distinct_and_fill_their_cell() {
        let area = |shape| {
            let poly = marker_polygon(shape);
            assert!(
                poly.iter().all(|p| p.abs().max_element() <= 1.0),
                "{shape:?} fits"
            );
            assert!(inside(&poly, Vec2::ZERO), "{shape:?} covers its centre");
            (0..MARKER_CELL_PX * MARKER_CELL_PX)
                .map(|i| coverage(&poly, i % MARKER_CELL_PX, i / MARKER_CELL_PX))
                .sum::<f32>()
        };
        let areas: Vec<f32> = MarkerShape::ALL.into_iter().map(area).collect();
        for (i, a) in areas.iter().enumerate() {
            let cell = (MARKER_CELL_PX * MARKER_CELL_PX) as f32;
            assert!(*a > 0.2 * cell && *a < 0.8 * cell, "shape {i}: {a}");
        }
        // Concave shapes stay concave: between two star points, and in a cross's corner.
        let star = marker_polygon(MarkerShape::Star);
        assert!(!inside(&star, Vec2::from_angle(FRAC_PI_2 + PI / 5.0) * 0.7));
        assert!(!inside(
            &marker_polygon(MarkerShape::Cross),
            Vec2::splat(0.6)
        ));
        assert_eq!(
            marker_rect(MarkerShape::Diamond).min.x,
            3.0 * MARKER_CELL_PX as f32
        );
    }
}
