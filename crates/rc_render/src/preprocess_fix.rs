//! Workaround for a Bevy 0.20 leak (QA R3, `docs/upstream-notes.md` U5): bevy_pbr keeps the GPU
//! preprocessing bind groups of every view that ever rendered.
//!
//! `BinUnpackingBindGroups` and `UniformAllocationBindGroups` hold the bind groups of the
//! `unpack_bins` / `allocate_uniforms` compute passes per (phase, `RetainedViewEntity`), and
//! `prepare_preprocess_bind_groups` only ever inserts into them. So every camera and shadow view
//! that goes away (each race attempt and each menu return spawns new ones) keeps its last bind
//! groups, and the batch-set GPU buffers they reference, until the process exits. Bevy prunes its
//! other per-view maps against the live views (`SceneUnpackingBuffers::view_phase_buffers`, the
//! work-item buffers); [`prune_preprocess_bind_groups`] does the same for these two, just before
//! `prepare_preprocess_bind_groups` rebuilds the entries of the live views. Remove it once Bevy
//! prunes them itself.

use bevy::pbr::{
    BinUnpackingBindGroups, UniformAllocationBindGroups, prepare_preprocess_bind_groups,
};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::SceneUnpackingBuffersKey;
use bevy::render::view::{ExtractedView, RetainedViewEntity};
use bevy::render::{Render, RenderApp, RenderSystems};

pub(crate) fn build(app: &mut App) {
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(
            Render,
            prune_preprocess_bind_groups
                .in_set(RenderSystems::PrepareBindGroups)
                .before(prepare_preprocess_bind_groups),
        );
    }
}

/// Drops the preprocessing bind groups of views that were not extracted this frame.
fn prune_preprocess_bind_groups(
    views: Query<&ExtractedView>,
    bins: Option<ResMut<BinUnpackingBindGroups>>,
    uniforms: Option<ResMut<UniformAllocationBindGroups>>,
) {
    let live: HashSet<RetainedViewEntity> = views.iter().map(|v| v.retained_view_entity).collect();
    if let Some(mut bins) = bins {
        retain_live(&mut bins, &live);
    }
    if let Some(mut uniforms) = uniforms {
        retain_live(&mut uniforms, &live);
    }
}

fn retain_live<V>(
    map: &mut HashMap<SceneUnpackingBuffersKey, V>,
    live: &HashSet<RetainedViewEntity>,
) {
    map.retain(|key, _| live.contains(&key.view));
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;

    #[test]
    fn only_the_entries_of_live_views_stay() {
        let mut world = World::new();
        let mut view = || RetainedViewEntity::new(world.spawn_empty().id().into(), None, 0);
        let (live, dead) = (view(), view());
        let key = |view, phase| SceneUnpackingBuffersKey { phase, view };
        let mut map: HashMap<_, ()> = [
            (key(live, TypeId::of::<u8>()), ()),
            (key(live, TypeId::of::<u16>()), ()),
            (key(dead, TypeId::of::<u8>()), ()),
        ]
        .into_iter()
        .collect();
        retain_live(&mut map, &[live].into_iter().collect());
        assert_eq!(map.len(), 2, "both phases of the live view stay");
        assert!(map.keys().all(|key| key.view == live));
    }
}
