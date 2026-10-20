//! Decorative-only models of a track, `assets/data/decor/<track>.ron` (ARCHITECTURE §5.9):
//! backdrops, posters, furniture beyond the barriers. Physics never sees them; rc_render draws
//! them, the track editor edits them. Placed like props: mirror negates X and the yaw, `dir`
//! filters by direction.
//!
//! ```ron
//! (decor: [(model: "models/props/toy/shelf_a.glb", pos: (70, 0, -50), yaw: 90, scale: 1.0, dir: Both)])
//! ```

use std::path::Path;

use bevy::math::{Quat, Vec3};
use bevy::transform::components::Transform;
use serde::{Deserialize, Serialize};

use crate::def::{Dir, read, ron_list, ron_options};

/// Decor files below the assets root.
pub const DECOR_DIR: &str = "data/decor";

/// One decorative model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorItem {
    /// glb path below the assets root.
    pub model: String,
    /// Ground-contact centre (u).
    pub pos: Vec3,
    /// Degrees; positive turns the front (−Z) toward −X, as for track props.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub yaw: f32,
    /// Uniform scale.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
    /// Layouts it appears in.
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

fn one() -> f32 {
    1.0
}

fn is_one(v: &f32) -> bool {
    *v == 1.0
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

fn is_both(dir: &Dir) -> bool {
    *dir == Dir::Both
}

impl DecorItem {
    /// Its pose in a layout (`mirror` negates X and the yaw).
    pub fn pose(&self, mirror: bool) -> Transform {
        let (x, yaw) = if mirror {
            (-self.pos.x, -self.yaw)
        } else {
            (self.pos.x, self.yaw)
        };
        Transform::from_xyz(x, self.pos.y, self.pos.z)
            .with_rotation(Quat::from_rotation_y(yaw.to_radians()))
            .with_scale(Vec3::splat(self.scale))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorFile {
    decor: Vec<DecorItem>,
}

/// The decor of `track`; no file = no decor.
pub fn load_decor(assets_dir: &Path, track: &str) -> Result<Vec<DecorItem>, String> {
    let path = assets_dir.join(DECOR_DIR).join(format!("{track}.ron"));
    if !path.is_file() {
        return Ok(Vec::new());
    }
    parse_decor(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parses decor RON.
pub fn parse_decor(text: &str) -> Result<Vec<DecorItem>, String> {
    let file: DecorFile = ron_options().from_str(text).map_err(|e| e.to_string())?;
    Ok(file.decor)
}

/// Decor RON, one line per item (what the track editor saves; comments are not kept).
pub fn decor_to_ron(items: &[DecorItem]) -> Result<String, String> {
    let mut out = String::from("(\n");
    if items.is_empty() {
        out.push_str("    decor: [],\n");
    }
    ron_list(&mut out, "decor", items, false)?;
    out.push_str(")\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_decor_file_survives_a_save() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for entry in std::fs::read_dir(assets.join(DECOR_DIR))
            .expect("decor dir")
            .flatten()
        {
            let path = entry.path();
            let track = path.file_stem().and_then(|s| s.to_str()).expect("stem");
            let items = load_decor(&assets, track).unwrap_or_else(|e| panic!("{e}"));
            let saved = decor_to_ron(&items).expect("serializes");
            assert_eq!(parse_decor(&saved).expect("parses"), items, "{track}");
        }
        assert_eq!(
            parse_decor(&decor_to_ron(&[]).unwrap()).unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn mirror_negates_x_and_yaw() {
        let item = DecorItem {
            model: "a.glb".into(),
            pos: Vec3::new(5.0, 0.0, 2.0),
            yaw: 30.0,
            scale: 2.0,
            dir: Dir::Both,
        };
        let pose = item.pose(true);
        assert_eq!(pose.translation, Vec3::new(-5.0, 0.0, 2.0));
        assert_eq!(pose.scale, Vec3::splat(2.0));
        let (yaw, ..) = pose.rotation.to_euler(bevy::math::EulerRot::YXZ);
        assert!((yaw + 30f32.to_radians()).abs() < 1e-5, "{yaw}");
    }
}
