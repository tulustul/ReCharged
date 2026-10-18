//! Interpolation of fixed-step motion and visual proxies (ARCHITECTURE §3.3).
//!
//! Anything simulated at [`SIM_HZ`](crate::SIM_HZ) and shown on screen carries [`Interpolated`].
//! rc_core snapshots its `Transform` into [`PoseHistory`] in `FixedLast` ([`CoreSet::Snapshot`]);
//! rc_render poses a visual proxy (linked by [`VisualOf`] / [`Visual`]) between `prev` and `curr`
//! with `Time<Fixed>::overstep_fraction()`. Teleports send [`Teleported`] so the snapshot snaps
//! `prev = curr` and nothing streaks across the screen.
//!
//! [`CoreSet::Snapshot`]: crate::CoreSet::Snapshot

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

/// Marker: this simulation entity is shown through an interpolated visual proxy.
/// Added by the spawning sim crate (rc_race for racers, rc_items for projectiles, …).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[require(Transform, PoseHistory)]
pub struct Interpolated;

/// The last two fixed-tick poses of an [`Interpolated`] entity. Written only by rc_core.
///
/// Starts at the entity's spawn pose (`prev == curr`), so a fresh entity never interpolates from
/// the origin.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
#[component(on_add = init_pose_history)]
pub struct PoseHistory {
    /// Pose after the previous fixed tick.
    pub prev: Transform,
    /// Pose after the latest fixed tick.
    pub curr: Transform,
}

impl PoseHistory {
    /// A history resting at `pose` (`prev == curr`).
    pub fn at(pose: Transform) -> Self {
        Self {
            prev: pose,
            curr: pose,
        }
    }

    /// The pose `alpha` (0 = `prev`, 1 = `curr`) of the way through the latest tick.
    pub fn interpolate(&self, alpha: f32) -> Transform {
        let a = alpha.clamp(0.0, 1.0);
        Transform {
            translation: self.prev.translation.lerp(self.curr.translation, a),
            rotation: self.prev.rotation.slerp(self.curr.rotation, a),
            scale: self.prev.scale.lerp(self.curr.scale, a),
        }
    }
}

fn init_pose_history(mut world: DeferredWorld, ctx: HookContext) {
    let Some(pose) = world.get::<Transform>(ctx.entity).copied() else {
        return;
    };
    if let Some(mut history) = world.get_mut::<PoseHistory>(ctx.entity) {
        *history = PoseHistory::at(pose);
    }
}

/// "This entity jumped": sent by whoever teleports an [`Interpolated`] entity (vehicle on respawn,
/// track on prop reset, race/items on spawn). The next snapshot sets `prev = curr`.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Teleported {
    /// The teleported simulation entity.
    pub entity: Entity,
}

/// On a visual proxy: the simulation entity it shows. One-to-one; spawned by rc_render.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[relationship(relationship_target = Visual)]
pub struct VisualOf(pub Entity);

/// On a simulation entity: its visual proxy (relationship target of [`VisualOf`]).
/// `linked_spawn`: the proxy despawns with its simulation entity.
#[derive(Component, Debug, PartialEq, Eq)]
#[relationship_target(relationship = VisualOf, linked_spawn)]
pub struct Visual(Entity);

impl Visual {
    /// The visual proxy entity (follow this with cameras, VFX attachments and audio emitters).
    pub fn get(&self) -> Entity {
        self.0
    }
}

/// `FixedLast` / `CoreSet::Snapshot`: shifts `curr` into `prev` and records the new pose.
pub(crate) fn snapshot_poses(
    mut teleported: MessageReader<Teleported>,
    mut poses: Query<(Entity, &Transform, &mut PoseHistory), With<Interpolated>>,
    mut snap: Local<Vec<Entity>>,
) {
    snap.clear();
    snap.extend(teleported.read().map(|t| t.entity));
    for (entity, transform, mut history) in &mut poses {
        // A history inserted after its Transform was set elsewhere counts as a teleport too.
        let jumped = history.is_added() || snap.contains(&entity);
        history.prev = if jumped { *transform } else { history.curr };
        history.curr = *transform;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(x: f32, yaw: f32) -> Transform {
        Transform::from_xyz(x, 1.0, 0.0).with_rotation(Quat::from_rotation_y(yaw))
    }

    #[test]
    fn interpolation_hits_endpoints_and_midpoint() {
        let h = PoseHistory {
            prev: pose(0.0, 0.0),
            curr: pose(10.0, 1.0),
        };
        assert_eq!(h.interpolate(0.0), h.prev);
        assert!(h.interpolate(1.0).translation.distance(h.curr.translation) < 1e-6);
        let mid = h.interpolate(0.5);
        assert!((mid.translation.x - 5.0).abs() < 1e-5);
        let (yaw, _, _) = mid.rotation.to_euler(EulerRot::YXZ);
        assert!((yaw - 0.5).abs() < 1e-4, "slerp midpoint yaw {yaw}");
    }

    #[test]
    fn interpolation_clamps_alpha() {
        let h = PoseHistory {
            prev: pose(0.0, 0.0),
            curr: pose(2.0, 0.0),
        };
        assert_eq!(h.interpolate(-1.0).translation.x, 0.0);
        assert_eq!(h.interpolate(3.0).translation.x, 2.0);
    }

    fn snapshot_world() -> (World, bevy::ecs::system::SystemId) {
        let mut world = World::new();
        world.init_resource::<Messages<Teleported>>();
        let system = world.register_system(snapshot_poses);
        (world, system)
    }

    #[test]
    fn spawn_starts_at_rest_and_ticks_shift_poses() {
        let (mut world, snapshot) = snapshot_world();
        let e = world.spawn((Interpolated, pose(3.0, 0.0))).id();
        assert_eq!(
            world.get::<PoseHistory>(e),
            Some(&PoseHistory::at(pose(3.0, 0.0)))
        );

        world.run_system(snapshot).expect("snapshot runs");
        world
            .get_mut::<Transform>(e)
            .expect("transform")
            .translation
            .x = 4.0;
        world.run_system(snapshot).expect("snapshot runs");
        let h = world.get::<PoseHistory>(e).expect("history");
        assert_eq!((h.prev.translation.x, h.curr.translation.x), (3.0, 4.0));
    }

    #[test]
    fn teleport_snaps_prev_to_curr() {
        let (mut world, snapshot) = snapshot_world();
        let e = world.spawn((Interpolated, pose(0.0, 0.0))).id();
        world.run_system(snapshot).expect("snapshot runs");
        world
            .get_mut::<Transform>(e)
            .expect("transform")
            .translation
            .x = 50.0;
        world.write_message(Teleported { entity: e });
        world.run_system(snapshot).expect("snapshot runs");
        let h = world.get::<PoseHistory>(e).expect("history");
        assert_eq!((h.prev.translation.x, h.curr.translation.x), (50.0, 50.0));
    }

    #[test]
    fn proxy_despawns_with_its_sim_entity() {
        let mut world = World::new();
        let sim = world.spawn(Interpolated).id();
        let proxy = world.spawn(VisualOf(sim)).id();
        assert_eq!(world.get::<Visual>(sim).map(Visual::get), Some(proxy));
        world.despawn(sim);
        assert!(world.get_entity(proxy).is_err());
    }
}
