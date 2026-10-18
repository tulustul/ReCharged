//! Split-screen layout and lens maths (GDD §11 CAM-1/CAM-2), as pure functions.

use std::f32::consts::FRAC_PI_2;

use bevy::math::{Rect, URect, UVec2};
use rc_core::prelude::*;

/// Aspect ratio the FOV setting refers to (GDD §11: "horizontal FOV at 16:9").
pub const REFERENCE_ASPECT: f32 = 16.0 / 9.0;

/// Widest horizontal FOV of a 2P top/bottom viewport, in degrees (GDD §11).
pub const MAX_2P_HFOV_DEG: f32 = 100.0;

/// Viewport cells (top-left origin, y down) of `players` local players in a target of `size`
/// physical pixels: 1 = full, 2 = halves (`Horizontal` = top/bottom), 3 = three quadrants (the
/// bottom-right one is left to the UI), 4 = quadrants. Cells tile the target without gaps.
pub fn split_rects(players: usize, split: SplitOrientation, size: UVec2) -> Vec<URect> {
    let (w, h) = (size.x, size.y);
    let (cx, cy) = (w / 2, h / 2);
    match (players, split) {
        (0 | 1, _) => vec![URect::new(0, 0, w, h)],
        (2, SplitOrientation::Horizontal) => vec![URect::new(0, 0, w, cy), URect::new(0, cy, w, h)],
        (2, SplitOrientation::Vertical) => vec![URect::new(0, 0, cx, h), URect::new(cx, 0, w, h)],
        (n, _) => [
            URect::new(0, 0, cx, cy),
            URect::new(cx, 0, w, cy),
            URect::new(0, cy, cx, h),
            URect::new(cx, cy, w, h),
        ]
        .into_iter()
        .take(n.min(MAX_LOCAL_PLAYERS))
        .collect(),
    }
}

/// A physical cell in logical pixels (for `PlayerViewports` and the React HUD).
pub fn to_logical(cell: URect, scale_factor: f32) -> Rect {
    let s = scale_factor.max(f32::EPSILON);
    Rect::new(
        cell.min.x as f32 / s,
        cell.min.y as f32 / s,
        cell.max.x as f32 / s,
        cell.max.y as f32 / s,
    )
}

/// Vertical FOV in radians of a horizontal FOV `hfov` (radians) at `aspect` (width / height).
pub fn vertical_fov(hfov: f32, aspect: f32) -> f32 {
    2.0 * ((hfov * 0.5).tan() / aspect).atan()
}

/// Horizontal FOV in radians of a vertical FOV `vfov` (radians) at `aspect`.
pub fn horizontal_fov(vfov: f32, aspect: f32) -> f32 {
    2.0 * ((vfov * 0.5).tan() * aspect).atan()
}

/// Hor+ lens: the vertical FOV (radians) of a viewport with `aspect`, for a horizontal FOV setting
/// of `hfov_16_9_deg` at 16:9, so wider viewports see more at the sides. `max_hfov_deg` caps the
/// resulting horizontal FOV (2P top/bottom viewports are 32:9).
pub fn hor_plus_vfov(hfov_16_9_deg: f32, aspect: f32, max_hfov_deg: Option<f32>) -> f32 {
    // Keep tan() finite: the setting is 60–100° plus at most 12° of widening.
    let hfov = hfov_16_9_deg.to_radians().clamp(0.1, FRAC_PI_2 * 1.9);
    let aspect = aspect.max(0.05);
    let vfov = vertical_fov(hfov, REFERENCE_ASPECT);
    match max_hfov_deg {
        Some(max) if horizontal_fov(vfov, aspect) > max.to_radians() => {
            vertical_fov(max.to_radians(), aspect)
        }
        _ => vfov,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [SplitOrientation; 2] = [SplitOrientation::Horizontal, SplitOrientation::Vertical];

    fn area(r: URect) -> u64 {
        u64::from(r.width()) * u64::from(r.height())
    }

    #[test]
    fn cells_tile_the_target_for_every_player_count_and_aspect() {
        for size in [
            UVec2::new(1920, 1080),
            UVec2::new(1281, 721), // odd sizes: no gaps, no overlaps
            UVec2::new(3440, 1440),
            UVec2::new(1080, 1920),
            UVec2::new(1, 1),
        ] {
            for split in ALL {
                for players in 1..=4 {
                    let cells = split_rects(players, split, size);
                    assert_eq!(cells.len(), players, "{players} players");
                    for (i, a) in cells.iter().enumerate() {
                        assert!(
                            a.max.x <= size.x && a.max.y <= size.y,
                            "{a:?} outside {size}"
                        );
                        for b in &cells[i + 1..] {
                            assert!(a.intersect(*b).is_empty(), "{a:?} overlaps {b:?}");
                        }
                    }
                    let covered: u64 = cells.iter().copied().map(area).sum();
                    let full = u64::from(size.x) * u64::from(size.y);
                    if players == 3 {
                        let quadrant = area(URect::new(size.x / 2, size.y / 2, size.x, size.y));
                        assert_eq!(covered + quadrant, full, "3P leaves exactly one quadrant");
                    } else {
                        assert_eq!(covered, full, "{players}P {split:?} {size}");
                    }
                }
            }
        }
    }

    #[test]
    fn layouts_follow_the_gdd() {
        let size = UVec2::new(1600, 900);
        assert_eq!(
            split_rects(1, SplitOrientation::Vertical, size),
            [URect::new(0, 0, 1600, 900)]
        );
        assert_eq!(
            split_rects(2, SplitOrientation::Horizontal, size),
            [URect::new(0, 0, 1600, 450), URect::new(0, 450, 1600, 900)],
            "2P horizontal = top/bottom"
        );
        assert_eq!(
            split_rects(2, SplitOrientation::Vertical, size),
            [URect::new(0, 0, 800, 900), URect::new(800, 0, 1600, 900)]
        );
        let three = split_rects(3, SplitOrientation::Horizontal, size);
        assert_eq!(three[2], URect::new(0, 450, 800, 900), "P3 bottom-left");
        assert_eq!(
            split_rects(4, SplitOrientation::Vertical, size)[3],
            URect::new(800, 450, 1600, 900)
        );
        assert_eq!(
            split_rects(0, SplitOrientation::Horizontal, size).len(),
            1,
            "spectator view"
        );
        assert_eq!(split_rects(9, SplitOrientation::Horizontal, size).len(), 4);
    }

    #[test]
    fn logical_rects_divide_by_the_scale_factor() {
        let r = to_logical(URect::new(0, 450, 1600, 900), 2.0);
        assert_eq!(r, Rect::new(0.0, 225.0, 800.0, 450.0));
    }

    #[test]
    fn hor_plus_keeps_the_setting_at_16_9_and_widens_wider_views() {
        let at_16_9 = hor_plus_vfov(90.0, REFERENCE_ASPECT, None);
        let h = horizontal_fov(at_16_9, REFERENCE_ASPECT).to_degrees();
        assert!((h - 90.0).abs() < 1e-3, "{h}");
        let wide = hor_plus_vfov(90.0, 21.0 / 9.0, None);
        assert!((wide - at_16_9).abs() < 1e-6, "Hor+ keeps the vertical FOV");
        assert!(horizontal_fov(wide, 21.0 / 9.0).to_degrees() > 100.0);
        // 4:3 shows less at the sides but the same height.
        assert!(
            horizontal_fov(hor_plus_vfov(90.0, 4.0 / 3.0, None), 4.0 / 3.0).to_degrees() < 80.0
        );
    }

    #[test]
    fn two_player_top_bottom_viewports_clamp_to_100_degrees() {
        let aspect = 32.0 / 9.0;
        let free = horizontal_fov(hor_plus_vfov(90.0, aspect, None), aspect).to_degrees();
        assert!(free > 120.0, "unclamped 32:9 would be {free}");
        let clamped = hor_plus_vfov(90.0, aspect, Some(MAX_2P_HFOV_DEG));
        let h = horizontal_fov(clamped, aspect).to_degrees();
        assert!((h - MAX_2P_HFOV_DEG).abs() < 1e-3, "{h}");
        // A narrow setting below the cap is untouched.
        let narrow = hor_plus_vfov(40.0, aspect, Some(MAX_2P_HFOV_DEG));
        assert_eq!(narrow, hor_plus_vfov(40.0, aspect, None));
    }

    #[test]
    fn degenerate_inputs_stay_finite() {
        for (fov, aspect) in [(0.0, 1.0), (500.0, 1.0), (90.0, 0.0), (90.0, f32::MAX)] {
            let v = hor_plus_vfov(fov, aspect, Some(MAX_2P_HFOV_DEG));
            assert!(v.is_finite() && v > 0.0, "{fov} {aspect} -> {v}");
        }
    }
}
