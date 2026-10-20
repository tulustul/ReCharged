//! Decorative-only models of a track, `assets/maps/<track>/decor.ron` (ARCHITECTURE §5.9):
//! backdrops, posters, furniture beyond the barriers. Physics never sees them; rc_render draws
//! them, the track editor edits them. Unlike props they turn about every axis and scale per axis
//! (no collider assumes them upright). Mirror negates X, the yaw and the roll (the mesh itself is
//! never mirrored); `dir` filters by direction.
//!
//! ```ron
//! (decor: [(model: "models/props/toy/shelf_a.glb", pos: (70, 0, -50), rot: (0, 90, 0), scale: (1, 2, 1), dir: Both)])
//! ```
//! The older form `yaw: 90, scale: 1.5` (uniform) still loads; saves write `rot` / `scale`.

use std::path::Path;

use bevy::math::{EulerRot, Quat, Vec3};
use bevy::transform::components::Transform;
use serde::{Deserialize, Serialize};

use crate::def::{Dir, read, ron_list, ron_options};

/// The decor file of a map directory.
pub const DECOR_FILE: &str = "decor.ron";

/// One decorative model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DecorItemFile")]
pub struct DecorItem {
    /// glb path below the assets root.
    pub model: String,
    /// Ground-contact centre (u).
    pub pos: Vec3,
    /// (pitch, yaw, roll) in degrees, applied Y, then X, then Z. Positive yaw turns the front
    /// (−Z) toward −X, as for track props.
    #[serde(skip_serializing_if = "is_zero")]
    pub rot: Vec3,
    /// Scale along the model's own axes.
    #[serde(skip_serializing_if = "is_one")]
    pub scale: Vec3,
    /// Layouts it appears in.
    #[serde(skip_serializing_if = "is_both")]
    pub dir: Dir,
}

/// A decor item as files write it: today's `rot` / per-axis `scale`, or the older `yaw` and a
/// uniform `scale`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorItemFile {
    model: String,
    pos: Vec3,
    #[serde(default)]
    rot: Option<Vec3>,
    #[serde(default)]
    yaw: Option<f32>,
    #[serde(default)]
    scale: Option<Scale>,
    #[serde(default)]
    dir: Dir,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Scale {
    Uniform(f32),
    Axes(Vec3),
}

impl TryFrom<DecorItemFile> for DecorItem {
    type Error = String;

    fn try_from(f: DecorItemFile) -> Result<Self, String> {
        let rot = match (f.rot, f.yaw) {
            (Some(_), Some(_)) => return Err(format!("{}: rot or yaw, not both", f.model)),
            (Some(rot), None) => rot,
            (None, yaw) => Vec3::new(0.0, yaw.unwrap_or(0.0), 0.0),
        };
        let scale = match f.scale {
            None => Vec3::ONE,
            Some(Scale::Uniform(s)) => Vec3::splat(s),
            Some(Scale::Axes(s)) => s,
        };
        Ok(Self {
            model: f.model,
            pos: f.pos,
            rot,
            scale,
            dir: f.dir,
        })
    }
}

fn is_one(v: &Vec3) -> bool {
    *v == Vec3::ONE
}

fn is_zero(v: &Vec3) -> bool {
    *v == Vec3::ZERO
}

fn is_both(dir: &Dir) -> bool {
    *dir == Dir::Both
}

impl DecorItem {
    /// Its pose in a layout (`mirror` negates X, the yaw and the roll; the pitch stays).
    pub fn pose(&self, mirror: bool) -> Transform {
        let (x, yaw, roll) = if mirror {
            (-self.pos.x, -self.rot.y, -self.rot.z)
        } else {
            (self.pos.x, self.rot.y, self.rot.z)
        };
        let r = |deg: f32| deg.to_radians();
        Transform::from_xyz(x, self.pos.y, self.pos.z)
            .with_rotation(Quat::from_euler(
                EulerRot::YXZ,
                r(yaw),
                r(self.rot.x),
                r(roll),
            ))
            .with_scale(self.scale)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorFile {
    decor: Vec<DecorItem>,
}

/// The decor of `track`; no file = no decor.
pub fn load_decor(assets_dir: &Path, track: &str) -> Result<Vec<DecorItem>, String> {
    let path = crate::map_dir(assets_dir, track).join(DECOR_FILE);
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
        for track in crate::map_ids(&assets) {
            let items = load_decor(&assets, &track).unwrap_or_else(|e| panic!("{e}"));
            let saved = decor_to_ron(&items).expect("serializes");
            assert_eq!(parse_decor(&saved).expect("parses"), items, "{track}");
        }
        assert_eq!(
            parse_decor(&decor_to_ron(&[]).unwrap()).unwrap(),
            Vec::new()
        );
        // Files written before rotation and per-axis scale.
        let old = r#"(decor: [
            (model: "a.glb", pos: (1, 0, 2), yaw: 30, scale: 2),
            (model: "b.glb", pos: (1, 0, 2), scale: 1.5, dir: Fwd),
            (model: "c.glb", pos: (1, 0, 2)),
        ])"#;
        let items = parse_decor(old).expect("old form");
        assert_eq!(items[0].rot, Vec3::new(0.0, 30.0, 0.0));
        assert_eq!(items[0].scale, Vec3::splat(2.0));
        assert_eq!(items[1].scale, Vec3::splat(1.5));
        assert_eq!((items[2].rot, items[2].scale), (Vec3::ZERO, Vec3::ONE));
        let saved = decor_to_ron(&items).expect("serializes");
        assert!(saved.contains("rot: (0.0, 30.0, 0.0)"), "{saved}");
        assert!(!saved.contains("yaw"), "{saved}");
        assert!(
            saved
                .lines()
                .any(|l| l.contains("c.glb") && !l.contains("rot") && !l.contains("scale")),
            "defaults are left out: {saved}"
        );
        assert_eq!(parse_decor(&saved).expect("parses"), items);
    }

    #[test]
    fn the_new_form_loads_and_bad_items_are_rejected() {
        let new =
            r#"(decor: [(model: "a.glb", pos: (1, 0, 2), rot: (10, 20, 30), scale: (1, 2, 3))])"#;
        let items = parse_decor(new).expect("new form");
        assert_eq!(items[0].rot, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(items[0].scale, Vec3::new(1.0, 2.0, 3.0));
        let both = r#"(decor: [(model: "a.glb", pos: (1, 0, 2), rot: (0, 1, 0), yaw: 1)])"#;
        assert!(parse_decor(both).is_err());
        let unknown = r#"(decor: [(model: "a.glb", pos: (1, 0, 2), spin: 1)])"#;
        assert!(parse_decor(unknown).is_err());
    }

    #[test]
    fn mirror_negates_x_yaw_and_roll_and_keeps_pitch() {
        let item = DecorItem {
            model: "a.glb".into(),
            pos: Vec3::new(5.0, 0.0, 2.0),
            rot: Vec3::new(10.0, 30.0, 20.0),
            scale: Vec3::new(1.0, 2.0, 3.0),
            dir: Dir::Both,
        };
        let pose = item.pose(true);
        assert_eq!(pose.translation, Vec3::new(-5.0, 0.0, 2.0));
        assert_eq!(pose.scale, Vec3::new(1.0, 2.0, 3.0));
        let (yaw, pitch, roll) = pose.rotation.to_euler(EulerRot::YXZ);
        let near = |a: f32, deg: f32| (a - deg.to_radians()).abs() < 1e-5;
        assert!(near(yaw, -30.0) && near(pitch, 10.0) && near(roll, -20.0));
        // A mirrored point of the model = the original's mirrored (meshes are not mirrored, so
        // this holds for its symmetry plane: a point on the model's own X = 0 plane).
        let p = Vec3::new(0.0, 1.0, -1.0);
        let m = item.pose(false).transform_point(p);
        assert!((pose.transform_point(p) - Vec3::new(-m.x, m.y, m.z)).length() < 1e-4);
    }
}
