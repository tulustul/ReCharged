//! In-place session updates (the track editor's commits, P7): when the session's [`Track`] is
//! replaced while `InRace`, the colliders follow it with the same spawn code as a load — all
//! track pieces when its [`TrackKey`] changed, and props per [`PropSource`] (a kind change, an
//! addition or a removal respawns, a pose change moves in place). The game never replaces
//! `Track` in a race, so this never fires there.

use std::collections::HashMap;
use std::time::Instant;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::PropSource;
use crate::spawn::{PropSpawn, spawn_colliders, spawn_prop};
use crate::track::{PlacedProp, Track, TrackKey};

/// What the session's colliders were built from (set by the load, updated by [`rebuild`]).
#[derive(Resource, Debug, Clone)]
pub(crate) struct Built {
    pub key: TrackKey,
    pub props: Vec<(usize, String, Transform)>,
}

impl Built {
    pub(crate) fn of(track: &Track) -> Self {
        Self {
            key: track.key().clone(),
            props: placements(track.props()),
        }
    }
}

fn placements(props: &[PlacedProp]) -> Vec<(usize, String, Transform)> {
    props
        .iter()
        .map(|p| (p.source, p.kind.clone(), p.transform))
        .collect()
}

/// How a list of placements keyed by document index changed (see [`placement_changes`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementChanges<K> {
    /// Sources whose entity goes: removed, or now of another kind (or model).
    pub despawn: Vec<usize>,
    /// Sources of the same kind whose pose changed: moved in place.
    pub moved: Vec<(usize, Transform)>,
    /// Placements to spawn: new sources, or replacements of despawned ones.
    pub spawn: Vec<(usize, K, Transform)>,
}

impl<K> PlacementChanges<K> {
    /// Nothing changed.
    pub fn is_empty(&self) -> bool {
        self.despawn.is_empty() && self.moved.is_empty() && self.spawn.is_empty()
    }
}

impl<K: Clone> PlacementChanges<K> {
    /// `built` (what exists) after these changes, sorted by source: what both the load path
    /// (changes from nothing) and the diff path record as built.
    pub fn apply(&self, built: &mut Vec<(usize, K, Transform)>) {
        built.retain(|(s, ..)| !self.despawn.contains(s));
        for (source, pose) in &self.moved {
            if let Some(entry) = built.iter_mut().find(|(s, ..)| s == source) {
                entry.2 = *pose;
            }
        }
        built.extend(self.spawn.iter().cloned());
        built.sort_by_key(|(s, ..)| *s);
    }
}

/// The changes from `old` to `new` (both keyed by unique document indices): a source whose kind
/// (`K`: a prop kind, a decor model) changed is despawned and spawned again, one whose pose
/// changed moves, one that is new spawns, one that is gone despawns.
pub fn placement_changes<K: PartialEq + Clone>(
    old: &[(usize, K, Transform)],
    new: &[(usize, K, Transform)],
) -> PlacementChanges<K> {
    let before: HashMap<usize, (&K, &Transform)> =
        old.iter().map(|(s, k, t)| (*s, (k, t))).collect();
    let after: HashMap<usize, &K> = new.iter().map(|(s, k, _)| (*s, k)).collect();
    let mut changes = PlacementChanges {
        despawn: old
            .iter()
            .filter(|(s, k, _)| after.get(s).is_none_or(|n| *n != k))
            .map(|(s, ..)| *s)
            .collect(),
        moved: Vec::new(),
        spawn: Vec::new(),
    };
    for (source, kind, pose) in new {
        match before.get(source) {
            Some((k, t)) if *k == kind => {
                if *t != pose {
                    changes.moved.push((*source, *pose));
                }
            }
            _ => changes.spawn.push((*source, kind.clone(), *pose)),
        }
    }
    changes
}

/// `InRace`, when `Track` was replaced: rebuild what differs from [`Built`].
pub(crate) fn rebuild(
    mut commands: Commands,
    track: Res<Track>,
    built: Option<ResMut<Built>>,
    pieces: Query<Entity, With<TrackPiece>>,
    mut props: Query<(Entity, &PropSource, &mut Transform, Option<&mut PropSpawn>), With<Prop>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let Some(mut built) = built else {
        return; // not loaded by this crate (a test's hand-made session)
    };
    let started = Instant::now();
    let rebuilt = built.key != *track.key();
    if rebuilt {
        for entity in &pieces {
            commands.entity(entity).despawn();
        }
        spawn_colliders(&mut commands, &track);
        built.key = track.key().clone();
    }
    let changes = placement_changes(&built.props, &placements(track.props()));
    if !rebuilt && changes.is_empty() {
        return;
    }
    for (entity, source, mut transform, spawn) in &mut props {
        if changes.despawn.contains(&source.0) {
            commands.entity(entity).despawn();
        } else if let Some((_, pose)) = changes.moved.iter().find(|(s, _)| *s == source.0) {
            *transform = *pose;
            if let Some(mut spawn) = spawn {
                spawn.0 = *pose;
                commands.entity(entity).insert(Velocity::zero());
                teleported.write(Teleported { entity });
            }
        }
    }
    for (source, ..) in &changes.spawn {
        if let Some(p) = track.props().iter().find(|p| p.source == *source) {
            spawn_prop(&mut commands, &track, p);
        }
    }
    changes.apply(&mut built.props);
    info!(
        "track: in-place update in {:.1} ms (track pieces {}; props: {} despawned, {} moved, {} spawned)",
        started.elapsed().as_secs_f32() * 1000.0,
        if rebuilt { "rebuilt" } else { "kept" },
        changes.despawn.len(),
        changes.moved.len(),
        changes.spawn.len(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> Transform {
        Transform::from_xyz(x, 0.0, 0.0)
    }

    #[test]
    fn placement_changes_move_same_kinds_and_respawn_the_rest() {
        let old = vec![(0, "a", at(0.0)), (1, "b", at(1.0)), (2, "c", at(2.0))];
        // 0 moved, 1 swapped to another kind, 2 deleted, 3 added.
        let new = vec![(0, "a", at(5.0)), (1, "x", at(1.0)), (3, "d", at(3.0))];
        let changes = placement_changes(&old, &new);
        assert_eq!(changes.moved, vec![(0, at(5.0))]);
        assert_eq!(changes.despawn, vec![1, 2]);
        assert_eq!(changes.spawn, vec![(1, "x", at(1.0)), (3, "d", at(3.0))]);
        let mut built = old.clone();
        changes.apply(&mut built);
        assert_eq!(built, new, "incremental = fresh");
        assert!(placement_changes(&new, &new).is_empty());
        let mut from_nothing = Vec::new();
        placement_changes(&[], &new).apply(&mut from_nothing);
        assert_eq!(from_nothing, new, "a load is the changes from nothing");
    }
}
