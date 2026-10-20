//! Decorative-only props (ARCHITECTURE §5.9): models physics never sees — backdrops, posters,
//! furniture beyond the barriers — from rc_track's decor format (`assets/maps/<track>/decor.ron`,
//! or the track editor's `TrackDraft`), placed by `DecorItem::pose` (mirror negates X, yaw and
//! roll), `dir` filters by direction. Each carries its [`DecorSource`]; the editor's
//! `DragPreview` moves one live. The load spawns the plan (source, model, pose) as changes from
//! nothing; in a race a changed draft is diffed against the built plan with the same code
//! (`rc_track::placement_changes`: a model change respawns, a pose change moves).

use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{
    DecorItem, DragPreview, DragSource, PlacementChanges, Track, TrackDraft, placement_changes,
};

use super::PropModel;
use crate::content::ContentLoad;

/// Which decor item (its index in the decor file or the editor's draft) a decor model shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecorSource(pub usize);

/// The decor of `track` for a variant as (index in the file, model, pose): the draft's when it
/// is of that track, else the file's; no file = no decor.
pub(crate) fn load_decor(
    assets_dir: &Path,
    draft: Option<&TrackDraft>,
    track: &str,
    reverse: bool,
    mirror: bool,
) -> Result<Vec<(usize, String, Transform)>, String> {
    let items = match draft.filter(|d| d.id.as_str() == track) {
        Some(draft) => draft.decor.clone(),
        None => rc_track::load_decor(assets_dir, track)?,
    };
    Ok(items
        .into_iter()
        .enumerate()
        .filter(|(_, d): &(usize, DecorItem)| d.dir.active(reverse))
        .map(|(i, d)| {
            let pose = d.pose(mirror);
            (i, d.model, pose)
        })
        .collect())
}

/// The decor models the session shows: (source, model, pose), sorted by source.
#[derive(Resource, Debug, Default)]
pub(super) struct BuiltDecor(Vec<(usize, String, Transform)>);

/// The decor plan of the session's variant (models that do not exist are left out, with a
/// warning).
fn plan(
    assets_dir: &Path,
    draft: Option<&TrackDraft>,
    track: &Track,
) -> Result<Vec<(usize, String, Transform)>, String> {
    let v = track.variant();
    let mut items = load_decor(assets_dir, draft, v.track.as_str(), v.reverse, v.mirror)?;
    items.retain(|(_, model, _)| {
        let exists = assets_dir.join(model).is_file();
        if !exists {
            warn!("decor: {model} does not exist; skipped");
        }
        exists
    });
    Ok(items)
}

/// What [`apply`] spawns with and moves.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct DecorSpawner<'w, 's> {
    commands: Commands<'w, 's>,
    load: ResMut<'w, ContentLoad>,
    server: Res<'w, AssetServer>,
    built: ResMut<'w, BuiltDecor>,
    decor: Query<'w, 's, (Entity, &'static DecorSource, &'static mut Transform)>,
}

/// Applies `changes` to the decor entities and records them as built: the load path (changes
/// from nothing) and the diff path.
fn apply(s: &mut DecorSpawner, changes: &PlacementChanges<String>) {
    for (entity, source, mut transform) in &mut s.decor {
        if changes.despawn.contains(&source.0) {
            s.commands.entity(entity).despawn();
        } else if let Some((_, pose)) = changes.moved.iter().find(|(i, _)| *i == source.0) {
            *transform = *pose;
        }
    }
    for (source, model, pose) in &changes.spawn {
        let scene = crate::content::load_scene(&s.server, model);
        s.load.scenes.push((scene.clone(), model.clone(), false));
        s.commands.spawn((
            Name::new("decor"),
            PropModel,
            DecorSource(*source),
            WorldAssetRoot(scene),
            *pose,
            DespawnOnExit(RaceSession),
        ));
    }
    changes.apply(&mut s.built.0);
}

/// `Loading`: spawn the track's decorative models when the session's track appears.
pub(super) fn spawn_decor(
    mut spawner: DecorSpawner,
    track: Option<Res<Track>>,
    paths: Option<Res<UserPaths>>,
    draft: Option<Res<TrackDraft>>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let (Some(track), Some(paths)) = (track, paths) else {
        return;
    };
    if !track.is_added() {
        return;
    }
    match plan(&paths.assets_dir, draft.as_deref(), &track) {
        Ok(items) => {
            spawner.built.0.clear();
            apply(&mut spawner, &placement_changes(&[], &items));
        }
        Err(error) => {
            error!("decor: {error}");
            failed.write(LoadingFailed {
                key: crate::content::LOADING_KEY,
                error,
            });
        }
    }
}

/// `InRace`, when the editor's draft changed: the decor follows it.
pub(super) fn follow_draft(
    mut spawner: DecorSpawner,
    track: Option<Res<Track>>,
    paths: Option<Res<UserPaths>>,
    draft: Res<TrackDraft>,
) {
    let (Some(track), Some(paths)) = (track, paths) else {
        return;
    };
    match plan(&paths.assets_dir, Some(&draft), &track) {
        Ok(items) => {
            let changes = placement_changes(&spawner.built.0, &items);
            if !changes.is_empty() {
                info!(
                    "decor: {} despawned, {} moved, {} spawned in place",
                    changes.despawn.len(),
                    changes.moved.len(),
                    changes.spawn.len()
                );
                apply(&mut spawner, &changes);
            }
        }
        Err(error) => error!("decor: {error}"),
    }
}

/// The editor's [`DragPreview`] changed: the previewed decor model goes to the preview pose, the
/// one previewed before back to where it was.
pub(super) fn preview_decor(
    preview: Res<DragPreview>,
    mut decor: Query<(Entity, &DecorSource, &mut Transform)>,
    mut moved: Local<Option<(Entity, Transform)>>,
) {
    let target = match preview.0 {
        Some((DragSource::Decor(i), pose)) => Some((i, pose)),
        _ => None,
    };
    if let Some((entity, original)) = *moved {
        match decor.get_mut(entity) {
            Ok((_, source, mut transform)) if target.is_none_or(|(i, _)| source.0 != i) => {
                *transform = original;
                *moved = None;
            }
            Ok(_) => {}
            Err(_) => *moved = None, // despawned by a session reload
        }
    }
    let Some((i, pose)) = target else {
        return;
    };
    for (entity, source, mut transform) in &mut decor {
        if source.0 == i {
            moved.get_or_insert((entity, *transform));
            *transform = pose;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track_visuals::MeshKey;
    use rc_track::{Dir, PropCatalog, TrackDef};

    /// The track-mesh key and the decor plan of the draft (forward layout), fresh.
    fn fresh(
        assets: &Path,
        props: &PropCatalog,
        draft: &TrackDraft,
    ) -> (MeshKey, Vec<(usize, String, Transform)>) {
        let track =
            Track::build(&draft.def, props, &TrackVariant::forward("toy_town")).expect("builds");
        let items = plan(assets, Some(draft), &track).expect("decor");
        (MeshKey::of(&track), items)
    }

    /// Edits applied incrementally (the key replaced when it differs, the decor diffed and
    /// applied) end where a fresh plan of the final draft does.
    #[test]
    fn incremental_plans_equal_fresh_ones() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let props = PropCatalog::load(&assets).expect("props");
        let mut draft = TrackDraft {
            id: "toy_town".into(),
            def: TrackDef::load(&assets, "toy_town").expect("layout"),
            decor: rc_track::load_decor(&assets, "toy_town").expect("decor"),
        };
        let (mut key, mut built) = fresh(&assets, &props, &draft);
        let edits: [fn(&mut TrackDraft); 8] = [
            |d| d.decor[0].pos.x += 2.0,
            |d| d.decor[1].rot.x = 20.0,
            |d| d.decor[2].model = "models/props/toy/sign.glb".into(),
            |d| {
                d.decor.remove(3); // later sources shift down
            },
            |d| {
                let mut rev = d.decor[0].clone();
                rev.dir = Dir::Rev; // not in the forward layout
                d.decor.push(rev);
            },
            |d| d.decor.last_mut().expect("added").dir = Dir::Both,
            |d| d.def.points[2].pos.x += 0.5,
            |d| d.def.props[0].pos.x += 1.0, // may move a guide board
        ];
        for edit in edits {
            edit(&mut draft);
            let (k, items) = fresh(&assets, &props, &draft);
            if k != key {
                key = k;
            }
            placement_changes(&built, &items).apply(&mut built);
        }
        let (fresh_key, fresh_items) = fresh(&assets, &props, &draft);
        assert!(key == fresh_key, "track-mesh key");
        assert_eq!(built, fresh_items);
        let mut loaded = Vec::new();
        placement_changes(&[], &fresh_items).apply(&mut loaded);
        assert_eq!(loaded, fresh_items, "a load is the changes from nothing");
    }

    /// Every shipped decor file parses in all four variants.
    #[test]
    fn every_decor_file_loads() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for track in rc_track::map_ids(&assets) {
            for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
                load_decor(&assets, None, &track, reverse, mirror)
                    .unwrap_or_else(|e| panic!("{e}"));
            }
        }
    }

    #[test]
    fn decor_follows_mirror_and_direction() {
        let dir = std::env::temp_dir().join(format!("rc_render_decor_{}", std::process::id()));
        let file = |map: &str| {
            let d = rc_track::map_dir(&dir, map);
            std::fs::create_dir_all(&d).expect("temp dir");
            d.join(rc_track::DECOR_FILE)
        };
        std::fs::write(
            file("t"),
            r#"(decor: [
                (model: "b.glb", pos: (1, 0, 1), scale: 2, dir: Fwd),
                (model: "a.glb", pos: (5, 0, 2), yaw: 30),
            ])"#,
        )
        .expect("write");
        let fwd = load_decor(&dir, None, "t", false, false).expect("loads");
        assert_eq!(fwd.len(), 2);
        assert_eq!(fwd[0].2.scale, Vec3::splat(2.0));
        let rev_mirror = load_decor(&dir, None, "t", true, true).expect("loads");
        assert_eq!(rev_mirror.len(), 1, "Fwd decor is gone in reverse");
        assert_eq!(rev_mirror[0].0, 1, "keeps its index in the file");
        assert_eq!(rev_mirror[0].2.translation.x, -5.0);
        let (yaw, ..) = rev_mirror[0].2.rotation.to_euler(EulerRot::YXZ);
        assert!(
            (yaw + 30f32.to_radians()).abs() < 1e-5,
            "yaw mirrored: {yaw}"
        );
        assert!(
            load_decor(&dir, None, "missing", false, false)
                .expect("no file")
                .is_empty()
        );
        std::fs::write(file("bad"), "(decor: [(model: 1)])").expect("write");
        let err = load_decor(&dir, None, "bad", false, false).expect_err("bad file");
        assert!(err.contains("bad"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
