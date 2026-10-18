//! What the drivers see this tick: the other racers, the items around them (rc_items' `Hazard` /
//! `Projectile` with `ItemRadius` and `ItemTarget`) and the static props, in track coordinates;
//! and the spatial helpers built on it — obstacles ahead, cars alongside, narrowing track,
//! barriers, jumps, the item view.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{Primitive, Track, TrackFrame};

use crate::brain::TrackView;
use crate::data::AiTuning;
use crate::items::ItemView;
use crate::steering::angle_to;
use crate::traffic::Obstacle;

/// Half the width of a car (u).
pub(crate) const CAR_HALF_WIDTH_U: f32 = 0.6;
/// Reach of a hazard without an `ItemRadius` (u, the oil slick's).
const FALLBACK_HAZARD_RADIUS_U: f32 = 2.5;
/// Cars within this distance along the track (u, either way) are alongside: they bound the
/// lateral range instead of being passed.
const ALONGSIDE_U: f32 = 3.0;
/// Step of the narrowing scan ahead (u).
const NARROWING_STEP_U: f32 = 2.0;
/// A car this far beyond the edge (u) on a side with a barrier is behind the barrier (ribbon edge
/// + barrier + half a car).
const BEHIND_WALL_U: f32 = 2.2;
/// Static props reaching into the legal corridor (ribbon + off-road margin) are obstacles…
/// …when their collider comes lower than this above the surface (u; a car drives under higher
/// parts: table tops, bench seats, the dollhouse's upper floor)…
const PROP_UNDER_U: f32 = 1.3;
/// …and higher than this (u; flat bits lying on the floor are driven over).
const PROP_OVER_U: f32 = 0.15;
/// A car this close behind (lateral, u) is on the line a dropped hazard lands on.
const DROP_LATERAL_U: f32 = 2.5;
/// Around a jump — from this far before the lip to this far after the landing (u) — the car
/// stays on its line: ramps and gaps have no barriers, so a lane change falls off the side.
const JUMP_LOCK_BEFORE_U: f32 = 25.0;
const JUMP_LOCK_AFTER_U: f32 = 10.0;

/// Another racer as the drivers see it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Other {
    pub entity: Entity,
    pub pos: Vec3,
    pub s: f32,
    pub lateral: f32,
    /// Speed along the track (u/s).
    pub speed: f32,
    pub ghosted: bool,
    pub shielded: bool,
}

/// A hazard to steer around, in track coordinates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HazardSpot {
    pub s: f32,
    pub lateral: f32,
    pub radius: f32,
}

// ponytail: shortcuts are never taken (the racing line ignores them; `Track::shortcuts` has the
// paths if a personality should gamble on one).

/// Footprint circles (centre, radius) and vertical extent (min, max y) of one collider primitive
/// placed at `at`. Boxes and wedges are covered by a row of circles along their long side, so a
/// long wall never turns into a disc that closes the track.
fn footprint(at: &Transform, primitive: &Primitive) -> (Vec<(Vec3, f32)>, (f32, f32)) {
    let (offset, rot) = primitive.pose();
    let centre = at.transform_point(offset);
    let q = at.rotation * rot;
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    match *primitive {
        Primitive::Ball { radius, .. } => (
            vec![(centre, radius)],
            (centre.y - radius, centre.y + radius),
        ),
        Primitive::Cylinder {
            radius,
            half_height,
            ..
        } => (
            vec![(centre, radius)],
            (centre.y - half_height, centre.y + half_height),
        ),
        Primitive::Box { half, .. } | Primitive::Wedge { half, .. } => {
            let (ax, az) = (flat(q * Vec3::X * half.x), flat(q * Vec3::Z * half.z));
            let (long, short) = if ax.length() >= az.length() {
                (ax, az)
            } else {
                (az, ax)
            };
            let b = short.length().max(0.1);
            let n = (long.length() / b).ceil().max(1.0) as i32;
            let circles = (0..n)
                .map(|i| {
                    let t = if n == 1 {
                        0.0
                    } else {
                        2.0 * i as f32 / (n - 1) as f32 - 1.0
                    };
                    let along = long * t * (1.0 - b / long.length().max(b));
                    (centre + along, b * std::f32::consts::SQRT_2)
                })
                .collect();
            // Half extent of the rotated box along world up.
            let reach = [Vec3::X * half.x, Vec3::Y * half.y, Vec3::Z * half.z]
                .iter()
                .map(|axis| (q * *axis).y.abs())
                .sum::<f32>();
            (circles, (centre.y - reach, centre.y + reach))
        }
    }
}

/// Static, non-drivable props near the ribbon as obstacles (track coordinates): chicanes of
/// flower pots, chair legs, tyre stacks. Dynamic props are ignored (the chassis knocks them
/// away), drivable ones (ramps, box tops) are driven.
pub(crate) fn static_obstacles(track: &Track) -> Vec<HazardSpot> {
    let mut out = Vec::new();
    for placed in track.props() {
        let Some(def) = track.prop_def(&placed.kind) else {
            continue;
        };
        if def.dynamic || def.surface.is_some() {
            continue;
        }
        for primitive in &def.colliders {
            let (circles, (low, high)) = footprint(&placed.transform, primitive);
            for (centre, radius) in circles {
                let tp = track.project(centre, None);
                let frame = track.frame_at(tp.s);
                let floor = frame.position.y;
                let near = tp.lateral.abs() - radius < frame.half_width + frame.offroad_margin;
                let in_band = low < floor + PROP_UNDER_U && high > floor + PROP_OVER_U;
                if near && in_band {
                    out.push(HazardSpot {
                        s: tp.s,
                        lateral: tp.lateral,
                        radius,
                    });
                }
            }
        }
    }
    out
}

/// Reused buffers of the scene (no allocation per tick).
#[derive(Debug, Default)]
pub(crate) struct SceneBuffers {
    pub others: Vec<Other>,
    pub hazards: Vec<HazardSpot>,
    /// Cars a Spark Rocket is locked onto.
    pub locked: Vec<Entity>,
    /// Cars carrying a Goo Bomb.
    pub goo: Vec<Entity>,
    pub obstacles: Vec<Obstacle>,
    pub alongside: Vec<f32>,
    /// Static prop obstacles of the current track and its geometry hash.
    pub props: Vec<HazardSpot>,
    pub props_of: Option<u64>,
}

impl SceneBuffers {
    /// Fills the per-tick lists from the racers and rc_items' entities (and, once per track, the
    /// static prop obstacles).
    pub fn fill<'a>(
        &mut self,
        track: &Track,
        racers: impl Iterator<Item = (Entity, &'a TrackView, &'a StatusEffects)>,
        hazards: impl Iterator<
            Item = (
                &'a Hazard,
                &'a Transform,
                Option<&'a ItemTarget>,
                Option<&'a ItemRadius>,
            ),
        >,
        projectiles: impl Iterator<
            Item = (
                &'a Projectile,
                &'a Transform,
                Option<&'a ItemTarget>,
                Option<&'a ItemRadius>,
            ),
        >,
    ) {
        if self.props_of != Some(track.geometry_hash()) {
            self.props = static_obstacles(track);
            self.props_of = Some(track.geometry_hash());
        }
        self.others.clear();
        self.others
            .extend(racers.map(|(entity, view, effects)| Other {
                entity,
                pos: view.pos,
                s: view.s,
                lateral: view.lateral,
                speed: view.vel.dot(track.frame_at(view.s).forward),
                ghosted: effects.ghosted(),
                shielded: effects.shielded(),
            }));
        self.hazards.clear();
        self.hazards.extend_from_slice(&self.props);
        self.locked.clear();
        self.goo.clear();
        let spot = |pos: Vec3, radius: Option<&ItemRadius>| {
            let tp = track.project(pos, None);
            HazardSpot {
                s: tp.s,
                lateral: tp.lateral,
                radius: radius.map_or(FALLBACK_HAZARD_RADIUS_U, |r| r.0),
            }
        };
        for (hazard, tf, target, radius) in hazards {
            match hazard.kind {
                HazardKind::GooBomb => self.goo.extend(target.and_then(|t| t.0)),
                _ => self.hazards.push(spot(tf.translation, radius)),
            }
        }
        for (projectile, tf, target, radius) in projectiles {
            match projectile.kind {
                ItemKind::SparkRocket => self.locked.extend(target.and_then(|t| t.0)),
                ItemKind::ShockOrb => self.hazards.push(spot(tf.translation, radius)),
                _ => {}
            }
        }
    }
}

/// Everything shared by the drivers this tick.
pub(crate) struct Scene<'a> {
    pub track: &'a Track,
    pub tuning: &'a AiTuning,
    pub tick: u64,
    pub seed: u64,
    pub wet: (f32, f32),
    pub others: &'a [Other],
    pub hazards: &'a [HazardSpot],
    pub locked: &'a [Entity],
    pub goo: &'a [Entity],
    /// Racers by rank.
    pub standings: &'a [Entity],
}

impl Scene<'_> {
    fn rivals(&self, me: Entity) -> impl Iterator<Item = &Other> {
        self.others
            .iter()
            .filter(move |o| o.entity != me && !o.ghosted)
    }

    /// Cars and hazards ahead of `me` into `out` (`seek` = a car to bump, not avoid), the laterals
    /// of cars alongside into `alongside`.
    pub fn obstacles(
        &self,
        me: Entity,
        view: &TrackView,
        seek: Option<Entity>,
        out: &mut Vec<Obstacle>,
        alongside: &mut Vec<f32>,
    ) {
        let t = &self.tuning.traffic;
        let ahead = |s: f32| self.track.delta_s(view.s, s);
        let cars = self
            .rivals(me)
            .filter(|o| Some(o.entity) != seek)
            .map(|o| Obstacle {
                ahead: ahead(o.s),
                lateral: o.lateral,
                half_width: CAR_HALF_WIDTH_U
                    + t.side_gap_u
                    + if self.goo.contains(&o.entity) {
                        t.goo_clearance_u
                    } else {
                        0.0
                    },
                speed: o.speed,
                car: true,
            });
        let hazards = self.hazards.iter().map(|h| Obstacle {
            ahead: ahead(h.s),
            lateral: h.lateral,
            half_width: h.radius + t.hazard_clearance_u,
            speed: 0.0,
            car: false,
        });
        out.clear();
        out.extend(
            cars.chain(hazards)
                .filter(|o| o.ahead > 0.0 && o.ahead < t.look_ahead_u),
        );
        alongside.clear();
        alongside.extend(
            self.rivals(me)
                .filter(|o| Some(o.entity) != seek && ahead(o.s).abs() < ALONGSIDE_U)
                .map(|o| o.lateral),
        );
    }

    /// The car a Goo Bomb carrier should bump: the nearest unshielded car ahead within reach.
    pub fn goo_target(&self, me: Entity, view: &TrackView) -> Option<&Other> {
        if !self.goo.contains(&me) {
            return None;
        }
        let reach = self.tuning.items.goo_seek_u;
        self.rivals(me)
            .filter(|o| !o.shielded)
            .map(|o| (o, self.track.delta_s(view.s, o.s)))
            .filter(|(_, d)| *d > 0.0 && *d < reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(o, _)| o)
    }

    /// What the item decisions see from `me` (heading, up; rank; own state).
    pub fn item_view(
        &self,
        me: Entity,
        view: &TrackView,
        heading: Vec3,
        up: Vec3,
        own: ItemView,
    ) -> ItemView {
        let items = &self.tuning.items;
        let cone = items.rocket_cone_deg.to_radians();
        let mut v = ItemView {
            locked_on: self.locked.contains(&me),
            ..own
        };
        let closer = |slot: &mut Option<f32>, d: f32| *slot = Some(slot.map_or(d, |x| x.min(d)));
        for o in self.rivals(me) {
            let d = o.pos - view.pos;
            let dist = d.length();
            let angle = angle_to(heading, d - up * d.dot(up), up).abs();
            if angle <= cone {
                closer(&mut v.cone_ahead, dist);
            } else if angle >= std::f32::consts::PI - cone {
                closer(&mut v.cone_behind, dist);
            }
            let along = self.track.delta_s(view.s, o.s);
            if along > 0.0 {
                closer(&mut v.ahead_on_track, along);
            } else if (o.lateral - view.lateral).abs() < DROP_LATERAL_U {
                closer(&mut v.behind_on_line, -along);
            }
            closer(&mut v.nearest, dist);
            if dist <= items.emp_radius_u {
                v.in_emp_radius += 1;
            }
        }
        let rank = self.standings.iter().position(|&e| e == me);
        v.next_in_race = rank
            .and_then(|r| r.checked_sub(1))
            .and_then(|r| self.others.iter().find(|o| o.entity == self.standings[r]))
            .map(|o| o.pos.distance(view.pos));
        v
    }
}

/// `s` (with `ahead` u of lookahead) is on a jump's run-up, flight or landing.
pub(crate) fn near_jump(track: &Track, s: f32, ahead: f32) -> bool {
    track.jumps().iter().any(|j| {
        let span = track.delta_s(j.lip_s, j.landing_s).max(0.0);
        let from = j.lip_s - JUMP_LOCK_BEFORE_U - ahead;
        let d = (s - from).rem_euclid(track.length());
        d <= JUMP_LOCK_BEFORE_U + ahead + span + JUMP_LOCK_AFTER_U
    })
}

/// Reference takeoff speed (u/s) of a jump whose lip is within `within` u ahead of `s`.
pub(crate) fn jump_takeoff_ahead(track: &Track, s: f32, within: f32) -> Option<f32> {
    track
        .jumps()
        .iter()
        .filter(|j| (0.0..within).contains(&track.delta_s(s, j.lip_s)))
        .map(|j| j.takeoff_speed)
        .reduce(f32::max)
}

/// The car is beyond the barrier line on a side that has a barrier (it went through an opening
/// or over a wall).
pub(crate) fn behind_wall(track: &Track, view: &TrackView, frame: &TrackFrame) -> bool {
    let (left, right) = track.walls_at(view.s);
    let beyond = frame.half_width + BEHIND_WALL_U;
    (left && view.lateral < -beyond) || (right && view.lateral > beyond)
}

/// Half the legal width (u) of the narrowest track within `distance` ahead, `edge` inside the
/// edges.
pub(crate) fn narrowest_ahead(track: &Track, s: f32, distance: f32, edge: f32) -> f32 {
    let mut x = 0.0;
    let mut half = f32::MAX;
    while x <= distance {
        half = half.min(track.frame_at(s + x).half_width);
        x += NARROWING_STEP_U;
    }
    (half - edge).max(0.0)
}

/// Distance to the nearest car among `obstacles` (u).
pub(crate) fn nearest_car_ahead(obstacles: &[Obstacle]) -> Option<f32> {
    obstacles
        .iter()
        .filter(|o| o.car)
        .map(|o| o.ahead)
        .min_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_core::TrackVariant;

    #[test]
    fn a_long_wall_is_a_row_of_small_circles() {
        let wall = Primitive::Box {
            half: Vec3::new(1.0, 3.4, 5.0),
            offset: Vec3::new(5.0, 3.4, 0.0),
            rot: Vec3::ZERO,
        };
        let (circles, (low, high)) = footprint(&Transform::IDENTITY, &wall);
        assert_eq!(circles.len(), 5);
        assert!(
            circles
                .iter()
                .all(|&(c, r)| (c.x - 5.0).abs() < 1e-5 && r < 1.5)
        );
        let span = circles.iter().map(|(c, _)| c.z).fold(f32::MIN, f32::max);
        assert!((span - 4.0).abs() < 1e-4, "ends 1 u inside the box: {span}");
        assert!((low - 0.0).abs() < 1e-5 && (high - 6.8).abs() < 1e-5);
        let post = Primitive::Cylinder {
            radius: 0.5,
            half_height: 2.0,
            offset: Vec3::Y * 2.0,
            rot: Vec3::ZERO,
        };
        let at = Transform::from_xyz(3.0, 0.0, -2.0);
        let (circles, _) = footprint(&at, &post);
        assert_eq!(circles, vec![(Vec3::new(3.0, 2.0, -2.0), 0.5)]);
    }

    #[test]
    fn no_prop_obstacle_sits_on_toy_towns_racing_line() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let track = Track::load(&root, &TrackVariant::forward("toy_town")).expect("loads");
        let props = static_obstacles(&track);
        assert!(!props.is_empty(), "the dollhouse walls flank the tunnel");
        for p in &props {
            let line = track.racing_line_offset_at(p.s);
            assert!(
                (line - p.lateral).abs() > p.radius,
                "a prop at s {:.1} lateral {:.2} (r {:.2}) blocks the line at {line:.2}",
                p.s,
                p.lateral,
                p.radius
            );
        }
    }
}
