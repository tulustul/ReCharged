//! Visual proxies (ARCHITECTURE §3.3): one proxy entity per `Interpolated` simulation entity,
//! linked one-to-one by `VisualOf` / `Visual` (`linked_spawn`: the proxy despawns with its sim
//! entity) and posed every frame in `InterpolateSet` between the last two fixed-tick poses.
//! Models, wheels, lights, VFX, cameras and audio attach to the proxy, never to the rapier body.

use bevy::prelude::*;
use rc_core::prelude::*;

pub(crate) fn build(app: &mut App) {
    app.add_observer(spawn_proxy)
        .add_systems(Update, pose_proxies.in_set(InterpolateSet));
}

/// Spawns the proxy at the sim entity's spawn pose (the `PoseHistory` hook already ran). Queued
/// on the sim entity, so nothing is spawned for an entity despawned before commands apply.
fn spawn_proxy(add: On<Add<Interpolated>>, poses: Query<&PoseHistory>, mut commands: Commands) {
    let pose = poses.get(add.entity).map(|h| h.curr).unwrap_or_default();
    commands
        .entity(add.entity)
        .queue_silenced(move |mut sim: EntityWorldMut| {
            sim.with_related::<VisualOf>((Name::new("Visual proxy"), pose, Visibility::default()));
        });
}

/// `InterpolateSet`: proxy pose = `PoseHistory` interpolated by the fixed-step overstep.
fn pose_proxies(
    fixed: Res<Time<Fixed>>,
    sims: Query<&PoseHistory>,
    mut proxies: Query<(&VisualOf, &mut Transform)>,
) {
    let alpha = fixed.overstep_fraction();
    for (of, mut transform) in &mut proxies {
        if let Ok(history) = sims.get(of.0) {
            transform.set_if_neq(history.interpolate(alpha));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        build(&mut app);
        app.finish();
        app.cleanup();
        app
    }

    fn proxy_of(app: &mut App, sim: Entity) -> Entity {
        app.world()
            .get::<Visual>(sim)
            .map(Visual::get)
            .expect("the sim entity has a proxy")
    }

    #[test]
    fn every_interpolated_entity_gets_one_proxy_at_its_spawn_pose() {
        let mut app = app();
        let pose = Transform::from_xyz(3.0, 1.0, -2.0);
        let sim = app.world_mut().spawn((Interpolated, pose)).id();
        app.update();
        let proxy = proxy_of(&mut app, sim);
        assert_eq!(app.world().get::<VisualOf>(proxy), Some(&VisualOf(sim)));
        let at = app.world().get::<Transform>(proxy).expect("posed");
        assert!(at.translation.distance(pose.translation) < 1e-5);
    }

    #[test]
    fn proxies_interpolate_between_ticks_and_despawn_with_their_sim() {
        let mut app = app();
        let sim = app.world_mut().spawn(Interpolated).id();
        app.update();
        app.world_mut()
            .get_mut::<PoseHistory>(sim)
            .expect("history")
            .curr = Transform::from_xyz(10.0, 0.0, 0.0);
        app.update();
        let proxy = proxy_of(&mut app, sim);
        let x = app
            .world()
            .get::<Transform>(proxy)
            .expect("posed")
            .translation
            .x;
        assert!((0.0..=10.0).contains(&x), "between prev and curr: {x}");

        app.world_mut().despawn(sim);
        assert!(app.world().get_entity(proxy).is_err(), "linked_spawn");
    }
}
