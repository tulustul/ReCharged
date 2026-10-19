//! Derived placements on a built centreline (pure): shortcuts, checkpoints, respawn points, the
//! grid, item rows and props. Keep-out zones (gaps, shortcuts) are cyclic `(start, length)`
//! intervals of arc length.

use bevy::math::{Quat, Vec3};
use bevy::prelude::Transform;
use rc_core::MAX_RACERS;

use crate::def::RibbonStyle;
use crate::layout::Layout;
use crate::track::*;

/// Target spacing of checkpoints (u); never fewer than [`MIN_CHECKPOINTS`].
pub const CHECKPOINT_SPACING_U: f32 = 75.0;
/// Fewest checkpoints per lap (GDD §7.2).
pub const MIN_CHECKPOINTS: usize = 8;
/// Spacing of respawn points (u, GDD §5.4).
pub const RESPAWN_SPACING_U: f32 = 15.0;
/// No respawn point this close before a lip: a car must reach takeoff speed (u).
pub const JUMP_RUNUP_U: f32 = 40.0;
/// Every jump gets a respawn point this far past its landing (u): a car that fails after the
/// landing restarts there instead of before the ramp (where it may fail again — a loop seen on
/// Wrench Works).
pub const LANDING_RESPAWN_U: f32 = 6.0;
/// Respawned cars appear this far above the surface (u).
pub const RESPAWN_DROP_U: f32 = 0.5;
/// Pole position: distance of the first car behind the start line (u).
pub const GRID_FRONT_U: f32 = 3.0;
/// Distance between grid rows (u).
pub const GRID_ROW_U: f32 = 5.0;
/// The right column sits this much further back (u).
pub const GRID_STAGGER_U: f32 = 2.5;
/// Lateral offset of each column from the centreline (u).
pub const GRID_COLUMN_U: f32 = 2.2;
/// Grid cars appear this far above the surface (u).
pub const GRID_DROP_U: f32 = 0.15;
/// Outer cubes of a row stay this far inside the edges (u).
pub const ITEM_EDGE_MARGIN_U: f32 = 1.5;
/// Cube centres float this high above the surface (u).
pub const ITEM_HEIGHT_U: f32 = 0.8;
/// The racing line is centred from this far before a lip…
pub(crate) const LINE_LOCK_BEFORE_U: f32 = 20.0;
/// …to this far after the landing (u).
pub(crate) const LINE_LOCK_AFTER_U: f32 = 6.0;
/// Shortcut ends must lie within half width + this of the centreline (u).
const SHORTCUT_JOIN_U: f32 = 2.0;
/// Checkpoints keep this far from gaps and shortcut ends (u).
const ZONE_PAD_U: f32 = 2.0;
/// Closest two checkpoints may be (u).
const MIN_CHECKPOINT_GAP_U: f32 = 10.0;
/// Path points of a shortcut are checked every this many u.
const SHORTCUT_CHECK_STEP_U: f32 = 2.0;

impl Track {
    fn inside(&self, s: f32, (start, len): (f32, f32)) -> bool {
        (s - start).rem_euclid(self.length) < len
    }

    fn span(&self, from: f32, to: f32) -> f32 {
        (to - from).rem_euclid(self.length)
    }

    /// Fills every placement field from the layout; `gaps` are (lip, landing) arc lengths.
    pub(crate) fn place(&mut self, layout: &Layout, gaps: &[(f32, f32)]) -> Result<(), String> {
        self.shortcuts = self.place_shortcuts(layout)?;
        let mut keep_out: Vec<(f32, f32)> = gaps
            .iter()
            .map(|&(a, b)| (a - ZONE_PAD_U, self.span(a, b) + 2.0 * ZONE_PAD_U))
            .collect();
        keep_out.extend(self.shortcuts.iter().map(|s| {
            (
                s.entry_s - ZONE_PAD_U,
                self.span(s.entry_s, s.exit_s) + 2.0 * ZONE_PAD_U,
            )
        }));
        self.checkpoints = self.place_checkpoints(&keep_out)?;
        self.respawns = self.place_respawns(gaps)?;
        self.grid = self.place_grid()?;
        self.item_rows = self.place_item_rows(layout)?;
        self.props = layout
            .props
            .iter()
            .map(|p| PlacedProp {
                kind: p.kind.clone(),
                transform: Transform::from_translation(p.pos)
                    .with_rotation(Quat::from_rotation_y(p.yaw)),
            })
            .collect();
        Ok(())
    }

    fn place_shortcuts(&self, layout: &Layout) -> Result<Vec<Shortcut>, String> {
        let mut out = Vec::new();
        for (path, width) in &layout.shortcuts {
            let name = format!("shortcut from ({:.0}, {:.0})", path[0].x, path[0].z);
            let entry = self.project(path[0], None);
            let exit = self.project(path[path.len() - 1], None);
            for (p, tp) in [(path[0], entry), (path[path.len() - 1], exit)] {
                let f = self.frame_at(tp.s);
                if tp.lateral.abs() > f.half_width + SHORTCUT_JOIN_U || tp.height.abs() > 3.0 {
                    return Err(format!(
                        "{name}: end ({:.0}, {:.0}, {:.0}) must lie on the track (within half width + {SHORTCUT_JOIN_U} u)",
                        p.x, p.y, p.z
                    ));
                }
            }
            if self.span(entry.s, exit.s) > 0.5 * self.length {
                return Err(format!("{name}: runs against the travel direction"));
            }
            let mut hint = entry.s;
            for p in densify(path, SHORTCUT_CHECK_STEP_U) {
                let tp = self.project(p, Some(hint));
                hint = tp.s;
                let f = self.frame_at(tp.s);
                if tp.lateral.abs() > f.half_width + f.offroad_margin {
                    return Err(format!(
                        "{name}: ({:.0}, {:.0}) is {:.1} u off the centreline at s = {:.0} u; raise offroad_margin there to at least {:.1}",
                        p.x,
                        p.z,
                        tp.lateral.abs(),
                        tp.s,
                        tp.lateral.abs() - f.half_width + 0.5
                    ));
                }
            }
            out.push(Shortcut {
                path: path.clone(),
                width: *width,
                entry_s: entry.s,
                exit_s: exit.s,
            });
        }
        Ok(out)
    }

    fn place_checkpoints(&self, keep_out: &[(f32, f32)]) -> Result<Vec<TrackFrame>, String> {
        if keep_out.iter().any(|&z| self.inside(0.0, z)) {
            return Err("the start line is inside a gap or shortcut zone".into());
        }
        let count = MIN_CHECKPOINTS.max((self.length / CHECKPOINT_SPACING_U).ceil() as usize);
        let mut s_list: Vec<f32> = (0..count)
            .map(|i| {
                let mut s = i as f32 * self.length / count as f32;
                // Nudge out of keep-out zones (a few rounds in case zones touch).
                for _ in 0..4 {
                    let Some(&(start, len)) = keep_out.iter().find(|&&z| self.inside(s, z)) else {
                        break;
                    };
                    let early = self.span(start, s) < 0.5 * len;
                    s = self.wrap(if early {
                        start - 0.5
                    } else {
                        start + len + 0.5
                    });
                }
                s
            })
            .collect();
        s_list.sort_by(f32::total_cmp);
        let wrap_around = s_list[0] + self.length;
        let too_close = s_list
            .iter()
            .zip(s_list.iter().skip(1).chain(std::iter::once(&wrap_around)))
            .any(|(a, b)| b - a < MIN_CHECKPOINT_GAP_U);
        if too_close
            || keep_out
                .iter()
                .any(|&z| s_list.iter().any(|&s| self.inside(s, z)))
        {
            return Err(format!(
                "cannot place {count} checkpoints: gaps and shortcuts cover too much of the lap"
            ));
        }
        Ok(s_list.into_iter().map(|s| self.frame_at(s)).collect())
    }

    fn place_respawns(&self, gaps: &[(f32, f32)]) -> Result<Vec<RespawnPoint>, String> {
        let zones: Vec<(f32, f32)> = gaps
            .iter()
            .map(|&(a, b)| (a - JUMP_RUNUP_U, self.span(a, b) + JUMP_RUNUP_U + 1.0))
            .collect();
        let count = ((self.length / RESPAWN_SPACING_U).round() as usize).max(1);
        let mut at: Vec<f32> = (0..count)
            .map(|k| k as f32 * self.length / count as f32)
            .chain(gaps.iter().map(|&(_, b)| self.wrap(b + LANDING_RESPAWN_U)))
            .collect();
        at.sort_by(f32::total_cmp);
        let out: Vec<RespawnPoint> = at
            .into_iter()
            .filter(|&s| !zones.iter().any(|&z| self.inside(s, z)))
            .map(|s| self.frame_at(s))
            .filter(|f| {
                self.style != RibbonStyle::Invisible
                    || self.ground.iter().any(|g| g.contains(f.position))
            })
            .map(|f| RespawnPoint {
                s: f.s,
                transform: Transform::from_translation(f.point(0.0, RESPAWN_DROP_U))
                    .with_rotation(f.rotation()),
            })
            .collect();
        if out.is_empty() {
            return Err(
                "no valid respawn point (gaps everywhere, or no ground under an Invisible ribbon)"
                    .into(),
            );
        }
        Ok(out)
    }

    fn place_grid(&self) -> Result<Vec<Transform>, String> {
        (0..MAX_RACERS)
            .map(|k| {
                let (row, col) = (k / 2, k % 2);
                let back = GRID_FRONT_U + row as f32 * GRID_ROW_U + col as f32 * GRID_STAGGER_U;
                let f = self.frame_at(-back);
                if f.in_gap || f.half_width < GRID_COLUMN_U + 1.0 {
                    return Err(format!(
                        "grid slot {k} (s = {:.1} u): the grid needs a gap-free, ≥ {:.0} u wide section behind the start",
                        f.s,
                        2.0 * (GRID_COLUMN_U + 1.0)
                    ));
                }
                let side = if col == 0 { -1.0 } else { 1.0 };
                Ok(Transform::from_translation(f.point(side * GRID_COLUMN_U, GRID_DROP_U)).with_rotation(f.rotation()))
            })
            .collect()
    }

    fn place_item_rows(&self, layout: &Layout) -> Result<Vec<ItemRow>, String> {
        let mut rows = Vec::new();
        for &(pos, count) in &layout.item_rows {
            let tp = self.project(pos, None);
            let f = self.frame_at(tp.s);
            let name = format!("item row at ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z);
            if f.in_gap {
                return Err(format!("{name}: over a gap"));
            }
            if tp.lateral.abs() > f.half_width + 5.0 || tp.height.abs() > 5.0 {
                return Err(format!("{name}: not on the track"));
            }
            let w = (f.half_width - ITEM_EDGE_MARGIN_U).max(0.5);
            let slots = (0..count)
                .map(|j| f.point(-w + 2.0 * w * j as f32 / (count - 1) as f32, ITEM_HEIGHT_U))
                .collect();
            rows.push(ItemRow { s: tp.s, slots });
        }
        rows.sort_by(|a, b| a.s.total_cmp(&b.s));
        Ok(rows)
    }
}

/// Points along a polyline at most `step` apart (both ends included).
fn densify(path: &[Vec3], step: f32) -> Vec<Vec3> {
    let mut out = vec![path[0]];
    for w in path.windows(2) {
        let n = ((w[0].distance(w[1]) / step).ceil() as usize).max(1);
        out.extend((1..=n).map(|k| w[0].lerp(w[1], k as f32 / n as f32)));
    }
    out
}
