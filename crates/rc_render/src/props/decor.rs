//! Decorative-only props (ARCHITECTURE §5.9): models physics never sees — backdrops, posters,
//! furniture beyond the barriers — from rc_track's decor format (`assets/maps/<track>/decor.ron`,
//! or the track editor's `TrackDraft`), placed by `DecorItem::pose` (mirror negates X, yaw and
//! roll), `dir` filters by direction. Each carries its [`DecorSource`]; the editor's
//! `DragPreview` moves one live.

use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{DecorItem, DragPreview, DragSource, Track, TrackDraft};

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

/// `Loading`: spawn the track's decorative models when the session's track appears.
pub(super) fn spawn_decor(
    mut commands: Commands,
    mut load: ResMut<ContentLoad>,
    track: Option<Res<Track>>,
    paths: Option<Res<UserPaths>>,
    draft: Option<Res<TrackDraft>>,
    server: Res<AssetServer>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let (Some(track), Some(paths)) = (track, paths) else {
        return;
    };
    if !track.is_added() {
        return;
    }
    let v = track.variant();
    let draft = draft.as_deref();
    let items = match load_decor(
        &paths.assets_dir,
        draft,
        v.track.as_str(),
        v.reverse,
        v.mirror,
    ) {
        Ok(items) => items,
        Err(error) => {
            error!("decor: {error}");
            failed.write(LoadingFailed {
                key: crate::content::LOADING_KEY,
                error,
            });
            return;
        }
    };
    for (source, model, pose) in items {
        if !paths.assets_dir.join(&model).is_file() {
            warn!("decor: {model} does not exist; skipped");
            continue;
        }
        let scene = crate::content::load_scene(&server, &model);
        load.scenes.push((scene.clone(), model, false));
        commands.spawn((
            Name::new("decor"),
            PropModel,
            DecorSource(source),
            WorldAssetRoot(scene),
            pose,
            DespawnOnExit(RaceSession),
        ));
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
