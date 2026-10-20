//! The lighting objects of the document (P9): the environment's point and spot lights
//! (`EditorItem::Light`, by index into `EnvironmentDef::lights`), a time of day's sun and fill
//! (`Sun` / `Fill`, of the time the item names: the previewed one) and its look (`Look`: the
//! Lighting tab). Where their markers are, how the gizmo poses them, placing, swapping and
//! deleting them, and their inspector slots ([`slots`]).
//!
//! A spot's pose aims at its `target` (moving it moves the target along; turning it swings the
//! target around it at the same distance). The sun and fill only turn: their markers float above
//! the map's middle along their direction, and a turn sets elevation and azimuth.

use bevy::math::{Quat, Vec3};
use rc_core::prelude::*;
use rc_render::{EnvLightDef, SunDef, TimeLook, sun_direction};
use rc_ui::content::UiTimeOfDay;
use rc_ui::editor::{EditorAsset, EditorItem};

use crate::doc::{Doc, Pose, Turn, tidy};
use crate::fields::Slot;

/// A placed point light hangs this far above where it was dropped (u) …
const POINT_LIFT_U: f32 = 3.0;
/// … a placed spot this far, aimed straight down at the drop point.
const SPOT_LIFT_U: f32 = 8.0;
/// A spot made from a point light aims this far straight down (u).
pub(crate) const SPOT_DROP_U: f32 = 5.0;
/// A new light (warm white; a point lights a pool, a spot a display).
const NEW_LIGHT_COLOR: [f32; 3] = [1.0, 0.9, 0.78];
const NEW_POINT_LUMENS: f32 = 2500.0;
const NEW_POINT_RANGE_U: f32 = 26.0;
const NEW_SPOT_LUMENS: f32 = 16000.0;
const NEW_SPOT_RANGE_U: f32 = 60.0;
const NEW_SPOT_ANGLE_DEG: f32 = 30.0;
/// A new fill: high, this share of the sun's lux, sky-blue, shadowless.
const NEW_FILL_ELEVATION_DEG: f32 = 70.0;
const NEW_FILL_SHARE: f32 = 0.15;
const NEW_FILL_COLOR: [f32; 3] = [0.8, 0.86, 1.0];
/// Sun and fill markers float this far from the map's middle: this share of its half size, at
/// least the minimum (u).
const SKY_REACH: f32 = 0.8;
const MIN_SKY_REACH_U: f32 = 20.0;
/// Duplicated lights land this far to the side (u).
const DUPLICATE_OFFSET_U: f32 = 2.0;

/// (elevation, azimuth) in degrees of the direction `toward` (azimuth clockwise from −Z, 0–360).
pub fn sky_angles(toward: Vec3) -> (f32, f32) {
    let t = toward.normalize_or(Vec3::Y);
    let elevation = t.y.clamp(-1.0, 1.0).asin().to_degrees();
    let azimuth = t.x.atan2(-t.z).to_degrees().rem_euclid(360.0);
    (elevation, azimuth)
}

/// The rotation that turns −Z (a light's forward) to `dir`.
pub(crate) fn aim(dir: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, dir.normalize_or(Vec3::NEG_Y))
}

/// Angles as files keep them (hundredths of a degree).
fn deg(v: f32) -> f32 {
    (v * 100.0).round() / 100.0 + 0.0
}

fn time(t: UiTimeOfDay) -> TimeOfDay {
    t.into()
}

impl Doc {
    fn look_mut(&mut self, t: TimeOfDay) -> &mut TimeLook {
        let e = &mut self.env;
        match t {
            TimeOfDay::Morning => &mut e.morning,
            TimeOfDay::Noon => &mut e.noon,
            TimeOfDay::Sunset => &mut e.sunset,
            TimeOfDay::Night => &mut e.night,
        }
    }

    /// The sun or fill `item` stands for (`None`: another item, or a time without a fill).
    pub(crate) fn sky(&self, item: EditorItem) -> Option<&SunDef> {
        match item {
            EditorItem::Sun { time: t } => Some(&self.env.look(time(t)).sun),
            EditorItem::Fill { time: t } => self.env.look(time(t)).fill.as_ref(),
            _ => None,
        }
    }

    fn sky_mut(&mut self, item: EditorItem) -> Option<&mut SunDef> {
        match item {
            EditorItem::Sun { time: t } => Some(&mut self.look_mut(time(t)).sun),
            EditorItem::Fill { time: t } => self.look_mut(time(t)).fill.as_mut(),
            _ => None,
        }
    }

    /// The map's middle (the control points' box, at its top) and how far from it sun and fill
    /// markers float.
    pub(crate) fn sky_anchor(&self) -> (Vec3, f32) {
        let points = self.def.points.iter().map(|p| p.pos);
        let lo = points.clone().fold(Vec3::splat(f32::MAX), Vec3::min);
        let hi = points.fold(Vec3::splat(f32::MIN), Vec3::max);
        if lo.x > hi.x {
            return (Vec3::ZERO, MIN_SKY_REACH_U);
        }
        let middle = ((lo + hi) / 2.0).with_y(hi.y);
        let half = ((hi - lo) / 2.0).with_y(0.0).length();
        (middle, (half * SKY_REACH).max(MIN_SKY_REACH_U))
    }

    /// Where a light's, the sun's or the fill's marker is.
    pub(crate) fn light_handle(&self, item: EditorItem) -> Option<Vec3> {
        match item {
            EditorItem::Light { index } => {
                (self.env.lights.get(index as usize)).map(|l| Vec3::from_array(l.pos))
            }
            _ => {
                let s = self.sky(item)?;
                let (middle, reach) = self.sky_anchor();
                Some(middle + sun_direction(s.elevation, s.azimuth) * reach)
            }
        }
    }

    /// A light's pose: points only move, spots move and aim, sun and fill only aim.
    pub(crate) fn light_pose(&self, item: EditorItem) -> Option<(Pose, Turn, bool)> {
        let pos = self.light_handle(item)?;
        let (rot, turn) = match item {
            EditorItem::Light { index } => match self.env.lights[index as usize].target {
                Some(t) => (aim(Vec3::from_array(t) - pos), Turn::Aim),
                None => (Quat::IDENTITY, Turn::No),
            },
            _ => {
                let s = self.sky(item)?;
                (aim(sun_direction(s.elevation, s.azimuth)), Turn::Sky)
            }
        };
        let pose = Pose {
            pos,
            rot,
            scale: Vec3::ONE,
        };
        Some((pose, turn, false))
    }

    /// Moves light `index` to `pos`; a spot's target moves along.
    pub(crate) fn move_light(&mut self, index: u32, pos: Vec3) {
        if let Some(l) = self.env.lights.get_mut(index as usize) {
            let by = pos - Vec3::from_array(l.pos);
            l.pos = pos.to_array();
            if let Some(t) = &mut l.target {
                *t = (Vec3::from_array(*t) + by).map(tidy).to_array();
            }
        }
    }

    /// Sets a light's pose (`turned`: its rotation changed): a spot keeps its target's distance;
    /// the sun and fill take elevation and azimuth from the direction, never a position.
    pub(crate) fn set_light_pose(&mut self, item: EditorItem, pose: Pose, turned: bool) {
        let dir = pose.rot * Vec3::NEG_Z;
        match item {
            EditorItem::Light { index } => {
                self.move_light(index, pose.pos.map(tidy));
                let l = &mut self.env.lights[index as usize];
                if let (true, Some(t)) = (turned, &mut l.target) {
                    let from = Vec3::from_array(l.pos);
                    let reach = from.distance(Vec3::from_array(*t)).max(1e-3);
                    *t = (from + dir * reach).map(tidy).to_array();
                }
            }
            _ if turned => {
                if let Some(s) = self.sky_mut(item) {
                    let (elevation, azimuth) = sky_angles(dir);
                    (s.elevation, s.azimuth) = (deg(elevation), deg(azimuth));
                }
            }
            _ => {}
        }
    }

    /// Light, sun and fill markers at `t` (lights off at `t` dimmed: no entity shows them).
    pub(crate) fn light_markers(&self, t: TimeOfDay) -> Vec<(EditorItem, Vec3, bool)> {
        let ui = UiTimeOfDay::from(t);
        let lights = (self.env.lights.iter().enumerate())
            .map(|(i, l)| (EditorItem::Light { index: i as u32 }, !l.on_at(t)));
        let skies =
            [EditorItem::Sun { time: ui }, EditorItem::Fill { time: ui }].map(|item| (item, false));
        lights
            .chain(skies)
            .filter_map(|(item, dim)| Some((item, self.light_handle(item)?, dim)))
            .collect()
    }

    /// Places a light asset at `at` (`t`: the previewed time, for the sun and fill).
    pub(crate) fn place_light(
        &mut self,
        asset: &EditorAsset,
        at: Vec3,
        t: TimeOfDay,
    ) -> Result<EditorItem, String> {
        let ui = UiTimeOfDay::from(t);
        match asset {
            EditorAsset::Light { spot } => {
                let lift = if *spot { SPOT_LIFT_U } else { POINT_LIFT_U };
                self.env.lights.push(EnvLightDef {
                    pos: (at + Vec3::Y * lift).map(tidy).to_array(),
                    target: spot.then(|| at.map(tidy).to_array()),
                    angle: NEW_SPOT_ANGLE_DEG,
                    color: NEW_LIGHT_COLOR,
                    lumens: if *spot {
                        NEW_SPOT_LUMENS
                    } else {
                        NEW_POINT_LUMENS
                    },
                    range: if *spot {
                        NEW_SPOT_RANGE_U
                    } else {
                        NEW_POINT_RANGE_U
                    },
                    times: Vec::new(),
                });
                Ok(EditorItem::Light {
                    index: self.env.lights.len() as u32 - 1,
                })
            }
            EditorAsset::Sun => {
                // From the drop point's side of the map, as high as it was.
                let (middle, _) = self.sky_anchor();
                let side = (at - middle).with_y(0.0);
                let sun = &mut self.look_mut(t).sun;
                if side.length() > 1e-3 {
                    sun.azimuth = deg(sky_angles(side).1);
                }
                Ok(EditorItem::Sun { time: ui })
            }
            EditorAsset::Fill => {
                let look = self.look_mut(t);
                if look.fill.is_none() {
                    look.fill = Some(SunDef {
                        elevation: NEW_FILL_ELEVATION_DEG,
                        azimuth: look.sun.azimuth,
                        lux: deg(look.sun.lux * NEW_FILL_SHARE),
                        color: NEW_FILL_COLOR,
                        shadows: false,
                    });
                }
                Ok(EditorItem::Fill { time: ui })
            }
            _ => Err("not a light".into()),
        }
    }

    /// Light `index` becomes a spot (aimed [`SPOT_DROP_U`] straight down) or a point light.
    pub(crate) fn swap_light(&mut self, index: u32, spot: bool) -> Result<(), String> {
        let l = (self.env.lights.get_mut(index as usize)).ok_or("no such light")?;
        l.target = match (spot, l.target) {
            (false, _) => None,
            (true, Some(t)) => Some(t),
            (true, None) => Some([l.pos[0], tidy(l.pos[1] - SPOT_DROP_U), l.pos[2]]),
        };
        Ok(())
    }

    /// Deletes a light or a fill; the sun and the look stay.
    pub(crate) fn delete_light(&mut self, item: EditorItem) -> Result<(), String> {
        match item {
            EditorItem::Light { index } => {
                self.env.lights.remove(index as usize);
            }
            EditorItem::Fill { time: t } => self.look_mut(time(t)).fill = None,
            EditorItem::Sun { .. } => {
                return Err("every time of day has a sun (turn its lux down instead)".into());
            }
            _ => return Err("the time look cannot be deleted".into()),
        }
        Ok(())
    }

    /// Copies a light a little to the side.
    pub(crate) fn duplicate_light(&mut self, index: u32) -> EditorItem {
        let mut copy = self.env.lights[index as usize].clone();
        copy.pos[0] = tidy(copy.pos[0] + DUPLICATE_OFFSET_U);
        if let Some(t) = &mut copy.target {
            t[0] = tidy(t[0] + DUPLICATE_OFFSET_U);
        }
        self.env.lights.push(copy);
        EditorItem::Light {
            index: self.env.lights.len() as u32 - 1,
        }
    }
}

type Slots<'a> = (String, Vec<(&'static str, Slot<'a>)>);

/// Title and inspector slots of a light, the sun, the fill or a time look.
pub(crate) fn slots(doc: &mut Doc, item: EditorItem) -> Option<Slots<'_>> {
    Some(match item {
        EditorItem::Light { index } => {
            let l = doc.env.lights.get_mut(index as usize)?;
            let spot = l.target.is_some();
            let mut slots = vec![("pos", Slot::Arr3(&mut l.pos))];
            if let Some(t) = &mut l.target {
                slots.push(("target", Slot::Arr3(t)));
                slots.push(("angle", Slot::Num(&mut l.angle)));
            }
            slots.extend([
                ("color", Slot::Arr3(&mut l.color)),
                ("lumens", Slot::Num(&mut l.lumens)),
                ("range", Slot::Num(&mut l.range)),
                ("times", Slot::Ron(Box::new(&mut l.times))),
            ]);
            let kind = if spot { "Spot light" } else { "Point light" };
            (format!("{kind} {index}"), slots)
        }
        EditorItem::Sun { time: t } | EditorItem::Fill { time: t } => {
            let name = if matches!(item, EditorItem::Sun { .. }) {
                "Sun"
            } else {
                "Fill"
            };
            let s = doc.sky_mut(item)?;
            (
                format!("{name} · {:?}", time(t)),
                vec![
                    ("elevation", Slot::Num(&mut s.elevation)),
                    ("azimuth", Slot::Num(&mut s.azimuth)),
                    ("lux", Slot::Num(&mut s.lux)),
                    ("color", Slot::Arr3(&mut s.color)),
                    ("shadows", Slot::Bool(&mut s.shadows)),
                ],
            )
        }
        EditorItem::Look { time: t } => {
            let practicals = doc.env.practicals(time(t));
            let lamps = (doc.env.room.as_mut()).and_then(|r| r.lamps.as_mut());
            let look = match time(t) {
                TimeOfDay::Morning => &mut doc.env.morning,
                TimeOfDay::Noon => &mut doc.env.noon,
                TimeOfDay::Sunset => &mut doc.env.sunset,
                TimeOfDay::Night => &mut doc.env.night,
            };
            let g = &mut look.grading;
            let mut slots = vec![
                ("exposure", Slot::Num(&mut look.exposure)),
                ("ambient", Slot::Num(&mut look.ambient)),
                ("ambient_color", Slot::Arr3(&mut look.ambient_color)),
                ("env_light", Slot::Num(&mut look.env_light)),
                ("fog_color", Slot::Arr3(&mut look.fog_color)),
                ("fog_visibility", Slot::Num(&mut look.fog_visibility)),
                ("temperature", Slot::Num(&mut g.temperature)),
                ("tint", Slot::Num(&mut g.tint)),
                ("saturation", Slot::Num(&mut g.saturation)),
                ("contrast", Slot::Num(&mut g.contrast)),
                ("post_saturation", Slot::Num(&mut g.post_saturation)),
                ("lift", Slot::Num(&mut g.lift)),
                ("gain", Slot::Num(&mut g.gain)),
                ("bloom", Slot::Num(&mut look.bloom)),
                ("lamps", Slot::Num(&mut look.lamps)),
                ("practicals", Slot::OptNum(&mut look.practicals, practicals)),
                ("clouds_lit", Slot::Arr3(&mut look.clouds.lit)),
                ("clouds_shade", Slot::Arr3(&mut look.clouds.shade)),
                ("window", Slot::Arr3(&mut look.window)),
            ];
            if let Some(grid) = lamps {
                slots.extend([
                    ("lamp_spacing", Slot::Num(&mut grid.spacing)),
                    ("lamp_color", Slot::Arr3(&mut grid.color)),
                    ("lamp_lumens", Slot::Num(&mut grid.lumens)),
                    ("lamp_range", Slot::Num(&mut grid.range)),
                    ("lamp_spot_angle", Slot::OptNum(&mut grid.spot_angle, 0.0)),
                    ("lamp_fixture", Slot::Num(&mut grid.fixture)),
                ]);
            }
            (format!("{:?} look", time(t)), slots)
        }
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;

    use rc_render::EnvironmentDef;
    use rc_track::TrackDef;

    use super::*;

    pub(crate) fn toy_town() -> Doc {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Doc {
            def: TrackDef::load(&assets, "toy_town").unwrap(),
            decor: Vec::new(),
            env: EnvironmentDef::load(&assets, "toy_town").unwrap(),
            info: TrackInfo::default(),
        }
    }

    const NOON: UiTimeOfDay = UiTimeOfDay::Noon;

    #[test]
    fn sky_angles_invert_the_sun_direction() {
        for (el, az) in [(52.0, 120.0), (9.0, 262.0), (-20.0, 5.0), (80.0, 359.0)] {
            let (e, a) = sky_angles(sun_direction(el, az));
            assert!(
                (e - el).abs() < 1e-3 && (a - az).abs() < 1e-3,
                "{el} {az}: {e} {a}"
            );
        }
    }

    #[test]
    fn a_spot_moves_with_its_target_and_turns_it_around_itself() {
        let mut doc = toy_town();
        let spot = EditorItem::Light { index: 0 };
        let (pose, turn, _) = doc.pose(spot).unwrap();
        assert_eq!(turn, Turn::Aim);
        let before = doc.env.lights[0].clone();
        let moved = Pose {
            pos: pose.pos + Vec3::X,
            ..pose
        };
        doc.set_pose(spot, moved);
        let l = &doc.env.lights[0];
        assert_eq!(l.pos[0], before.pos[0] + 1.0);
        assert_eq!(l.target.unwrap()[0], before.target.unwrap()[0] + 1.0);
        let (pose, ..) = doc.pose(spot).unwrap();
        let down = Pose {
            rot: aim(Vec3::NEG_Y),
            ..pose
        };
        let reach = Vec3::from_array(l.pos).distance(Vec3::from_array(l.target.unwrap()));
        doc.set_pose(spot, down);
        let l = &doc.env.lights[0];
        let t = Vec3::from_array(l.target.unwrap());
        assert!((t - (Vec3::from_array(l.pos) - Vec3::Y * reach)).length() < 0.01);
    }

    /// `Light(i)` is the document index whatever the time: a light off at the previewed time
    /// keeps its index and gets a dimmed marker.
    #[test]
    fn light_items_index_the_document_and_lights_off_now_are_dimmed() {
        let doc = toy_town();
        let at = |t: TimeOfDay| -> Vec<(EditorItem, bool)> {
            (doc.light_markers(t).into_iter())
                .filter(|(i, ..)| matches!(i, EditorItem::Light { .. }))
                .map(|(i, _, dim)| (i, dim))
                .collect()
        };
        let light = |index| EditorItem::Light { index };
        assert_eq!(
            at(TimeOfDay::Sunset),
            [(light(0), false), (light(1), false), (light(2), true)],
            "the night-only point light is off at sunset"
        );
        assert!(at(TimeOfDay::Noon).iter().all(|(_, dim)| *dim));
        assert_eq!(
            doc.handle(light(2)),
            Some(Vec3::from_array(doc.env.lights[2].pos))
        );
    }

    #[test]
    fn the_fill_is_added_and_deleted_the_sun_never_deleted() {
        let mut doc = toy_town();
        let fill = EditorItem::Fill { time: NOON };
        doc.delete(fill).unwrap();
        assert!(doc.env.noon.fill.is_none() && !doc.exists(fill));
        assert!(doc.env.sunset.fill.is_some(), "only noon's");
        let item = doc.place_light(&EditorAsset::Fill, Vec3::ZERO, TimeOfDay::Noon);
        assert_eq!(item, Ok(fill));
        assert!(doc.exists(fill) && !doc.env.noon.fill.as_ref().unwrap().shadows);
        let sun = EditorItem::Sun { time: NOON };
        assert!(doc.delete(sun).is_err());
        assert!(doc.exists(sun));
    }

    #[test]
    fn placing_and_swapping_lights() {
        let mut doc = toy_town();
        let at = Vec3::new(4.0, 0.0, -2.0);
        let spot = doc.place_light(&EditorAsset::Light { spot: true }, at, TimeOfDay::Noon);
        let Ok(EditorItem::Light { index }) = spot else {
            panic!("{spot:?}");
        };
        let l = &doc.env.lights[index as usize];
        assert_eq!((l.target, l.times.len()), (Some(at.to_array()), 0));
        doc.swap_light(index, false).unwrap();
        assert_eq!(doc.env.lights[index as usize].target, None, "spot → point");
        doc.swap_light(index, true).unwrap();
        let l = &doc.env.lights[index as usize];
        assert_eq!(
            l.target,
            Some([l.pos[0], l.pos[1] - SPOT_DROP_U, l.pos[2]]),
            "point → spot aims straight down"
        );
        // The sun comes from the drop point's side, as high as before.
        let (middle, _) = doc.sky_anchor();
        let east = middle + Vec3::X * 30.0;
        doc.place_light(&EditorAsset::Sun, east, TimeOfDay::Noon)
            .unwrap();
        assert_eq!(
            (doc.env.noon.sun.elevation, doc.env.noon.sun.azimuth),
            (52.0, 90.0)
        );
    }
}
