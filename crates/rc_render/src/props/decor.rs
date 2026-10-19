//! Decorative-only props (ARCHITECTURE §5.9): models physics never sees — backdrops, posters,
//! furniture beyond the barriers — from the optional `assets/data/decor/<track>.ron`, placed like
//! rc_track's props (mirror negates X and the yaw, `dir` filters by direction).
//!
//! ```ron
//! (decor: [(model: "models/props/toy/shelf_a.glb", pos: (70, 0, -50), yaw: 90, scale: 1.0, dir: Both)])
//! ```

use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{Dir, Track};
use serde::Deserialize;

use super::PropModel;
use crate::content::ContentLoad;

/// Decor files below the assets root.
pub(crate) const DECOR_DIR: &str = "data/decor";

/// One decorative model.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecorItem {
    /// glb path below the assets root.
    pub model: String,
    /// Ground-contact centre (u).
    pub pos: Vec3,
    /// Degrees; positive turns the front (−Z) toward −X, as for track props.
    #[serde(default)]
    pub yaw: f32,
    /// Uniform scale.
    #[serde(default = "one")]
    pub scale: f32,
    /// Layouts it appears in.
    #[serde(default)]
    pub dir: Dir,
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorFile {
    decor: Vec<DecorItem>,
}

/// The decor of `track` for a variant as (model, pose); no file = no decor.
pub(crate) fn load_decor(
    assets_dir: &Path,
    track: &str,
    reverse: bool,
    mirror: bool,
) -> Result<Vec<(String, Transform)>, String> {
    let path = assets_dir.join(DECOR_DIR).join(format!("{track}.ron"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let file: DecorFile = ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(&text)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file
        .decor
        .into_iter()
        .filter(|d| d.dir.active(reverse))
        .map(|d| {
            let (x, yaw) = if mirror {
                (-d.pos.x, -d.yaw)
            } else {
                (d.pos.x, d.yaw)
            };
            let pose = Transform::from_xyz(x, d.pos.y, d.pos.z)
                .with_rotation(Quat::from_rotation_y(yaw.to_radians()))
                .with_scale(Vec3::splat(d.scale));
            (d.model, pose)
        })
        .collect())
}

/// `Loading`: spawn the track's decorative models when the session's track appears.
pub(super) fn spawn_decor(
    mut commands: Commands,
    mut load: ResMut<ContentLoad>,
    track: Option<Res<Track>>,
    paths: Option<Res<UserPaths>>,
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
    let items = match load_decor(&paths.assets_dir, v.track.as_str(), v.reverse, v.mirror) {
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
    for (model, pose) in items {
        if !paths.assets_dir.join(&model).is_file() {
            warn!("decor: {model} does not exist; skipped");
            continue;
        }
        let scene = crate::content::load_scene(&server, &model);
        load.scenes.push((scene.clone(), model, false));
        commands.spawn((
            Name::new("decor"),
            PropModel,
            WorldAssetRoot(scene),
            pose,
            DespawnOnExit(RaceSession),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped decor file parses in all four variants.
    #[test]
    fn every_decor_file_loads() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let Ok(dir) = std::fs::read_dir(assets.join(DECOR_DIR)) else {
            return;
        };
        for entry in dir.flatten() {
            let path = entry.path();
            let Some(track) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
                load_decor(&assets, track, reverse, mirror).unwrap_or_else(|e| panic!("{e}"));
            }
        }
    }

    #[test]
    fn decor_follows_mirror_and_direction() {
        let dir = std::env::temp_dir().join(format!("rc_render_decor_{}", std::process::id()));
        std::fs::create_dir_all(dir.join(DECOR_DIR)).expect("temp dir");
        std::fs::write(
            dir.join(DECOR_DIR).join("t.ron"),
            r#"(decor: [
                (model: "a.glb", pos: (5, 0, 2), yaw: 30),
                (model: "b.glb", pos: (1, 0, 1), scale: 2, dir: Fwd),
            ])"#,
        )
        .expect("write");
        let fwd = load_decor(&dir, "t", false, false).expect("loads");
        assert_eq!(fwd.len(), 2);
        assert_eq!(fwd[1].1.scale, Vec3::splat(2.0));
        let rev_mirror = load_decor(&dir, "t", true, true).expect("loads");
        assert_eq!(rev_mirror.len(), 1, "Fwd decor is gone in reverse");
        assert_eq!(rev_mirror[0].1.translation.x, -5.0);
        let (yaw, ..) = rev_mirror[0].1.rotation.to_euler(EulerRot::YXZ);
        assert!(
            (yaw + 30f32.to_radians()).abs() < 1e-5,
            "yaw mirrored: {yaw}"
        );
        assert!(
            load_decor(&dir, "missing", false, false)
                .expect("no file")
                .is_empty()
        );
        std::fs::write(dir.join(DECOR_DIR).join("bad.ron"), "(decor: [(model: 1)])")
            .expect("write");
        let err = load_decor(&dir, "bad", false, false).expect_err("bad file");
        assert!(err.contains("bad.ron"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
