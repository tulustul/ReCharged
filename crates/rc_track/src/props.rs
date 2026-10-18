//! Prop catalog: every `assets/data/props/*.ron` file (one per environment plus `common.ron`),
//! merged; a kind defined twice is a load error naming both files. Shared with rc_render, which
//! reads `model` and adds its own fields (unknown fields are ignored here): kind → model,
//! collider primitives, static/dynamic, mass, drivable surface. Origins are at the
//! ground-contact centre, front −Z.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::math::{EulerRot, Quat, Vec3};
use rc_core::SurfaceKind;
use serde::{Deserialize, Serialize};

use crate::def::{read, ron_options};

/// Prop catalog files relative to the assets root: every `data/props/*.ron`.
pub const PROPS_DIR: &str = "data/props";
/// Most knockable props one track may place (ARCHITECTURE §9).
pub const MAX_DYNAMIC_PROPS: usize = 40;

/// A collider primitive in prop space (u). `rot` is in degrees, applied yaw (Y), pitch (X),
/// roll (Z).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Primitive {
    /// Box with half extents.
    Box {
        /// Half extents.
        half: Vec3,
        /// Centre.
        #[serde(default)]
        offset: Vec3,
        /// Rotation (degrees).
        #[serde(default)]
        rot: Vec3,
    },
    /// Sphere.
    Ball {
        /// Radius.
        radius: f32,
        /// Centre.
        #[serde(default)]
        offset: Vec3,
    },
    /// Upright cylinder.
    Cylinder {
        /// Radius.
        radius: f32,
        /// Half height.
        half_height: f32,
        /// Centre.
        #[serde(default)]
        offset: Vec3,
        /// Rotation (degrees).
        #[serde(default)]
        rot: Vec3,
    },
    /// Ramp: the box `half` with its top face sloping from full height at −Z (front) down to
    /// zero at +Z, i.e. driven up while heading −Z.
    Wedge {
        /// Half extents of the bounding box.
        half: Vec3,
        /// Centre of the bounding box.
        #[serde(default)]
        offset: Vec3,
        /// Rotation (degrees).
        #[serde(default)]
        rot: Vec3,
    },
}

impl Primitive {
    /// Centre and rotation in prop space. `rot` is degrees about each axis (x = pitch, y = yaw,
    /// z = roll), applied yaw → pitch → roll.
    pub fn pose(&self) -> (Vec3, Quat) {
        let (offset, rot) = match *self {
            Self::Box { offset, rot, .. }
            | Self::Cylinder { offset, rot, .. }
            | Self::Wedge { offset, rot, .. } => (offset, rot),
            Self::Ball { offset, .. } => (offset, Vec3::ZERO),
        };
        let r = rot * (std::f32::consts::PI / 180.0);
        (offset, Quat::from_euler(EulerRot::YXZ, r.y, r.x, r.z))
    }

    /// Corners of a wedge around its own centre (convex hull input).
    pub fn wedge_points(half: Vec3) -> [Vec3; 6] {
        let (x, y, z) = (half.x, half.y, half.z);
        [
            Vec3::new(-x, -y, -z),
            Vec3::new(x, -y, -z),
            Vec3::new(-x, -y, z),
            Vec3::new(x, -y, z),
            Vec3::new(-x, y, -z),
            Vec3::new(x, y, -z),
        ]
    }
}

fn default_mass() -> f32 {
    1.0
}

/// One prop kind.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropDef {
    /// glb model path relative to the assets root (may not exist yet: rc_render then draws the
    /// collider primitives).
    pub model: String,
    /// Collision shape (one compound collider).
    pub colliders: Vec<Primitive>,
    /// Knockable rigid body (reset on restart) instead of a static obstacle.
    #[serde(default)]
    pub dynamic: bool,
    /// Mass of a dynamic prop (vehicle mass units; a car is ≈ 1.5–4).
    #[serde(default = "default_mass")]
    pub mass: f32,
    /// Drivable surface (ramps, box tops): wheel rays report it.
    #[serde(default)]
    pub surface: Option<SurfaceKind>,
}

/// One `props/*.ron` file, or all of them merged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropCatalog {
    /// Prop kinds by id.
    pub props: BTreeMap<String, PropDef>,
}

impl PropCatalog {
    /// Reads, validates and merges every `<assets_dir>/data/props/*.ron` (in file-name order).
    pub fn load(assets_dir: &Path) -> Result<Self, String> {
        let dir = assets_dir.join(PROPS_DIR);
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.extension().is_some_and(|x| x == "ron"))
            .collect();
        files.sort();
        let parts = files
            .iter()
            .map(|path| {
                let part =
                    Self::parse(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))?;
                Ok((path.display().to_string(), part))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Self::merge(parts)
    }

    /// Merges per-file catalogs `(file name, catalog)`; a kind in two files is an error.
    pub fn merge(parts: impl IntoIterator<Item = (String, PropCatalog)>) -> Result<Self, String> {
        let mut merged = Self::default();
        let mut origin: BTreeMap<String, String> = BTreeMap::new();
        for (file, part) in parts {
            for (kind, def) in part.props {
                if let Some(first) = origin.get(&kind) {
                    return Err(format!(
                        "prop \"{kind}\" is defined in both {first} and {file}"
                    ));
                }
                origin.insert(kind.clone(), file.clone());
                merged.props.insert(kind, def);
            }
        }
        Ok(merged)
    }

    /// Parses and validates the RON of one catalog file.
    pub fn parse(text: &str) -> Result<Self, String> {
        let catalog: Self = ron_options().from_str(text).map_err(|e| e.to_string())?;
        for (kind, def) in &catalog.props {
            if def.colliders.is_empty() {
                return Err(format!("props.{kind}.colliders: at least one primitive"));
            }
            if def.dynamic && (def.mass.is_nan() || def.mass <= 0.0) {
                return Err(format!(
                    "props.{kind}.mass: a dynamic prop needs a mass > 0"
                ));
            }
            let bad = def.colliders.iter().any(|c| match *c {
                Primitive::Box { half, .. } | Primitive::Wedge { half, .. } => {
                    half.min_element() <= 0.0
                }
                Primitive::Ball { radius, .. } => radius <= 0.0,
                Primitive::Cylinder {
                    radius,
                    half_height,
                    ..
                } => radius <= 0.0 || half_height <= 0.0,
            });
            if bad {
                return Err(format!("props.{kind}.colliders: sizes must be > 0"));
            }
        }
        Ok(catalog)
    }

    /// The prop kind `kind`.
    pub fn get(&self, kind: &str) -> Option<&PropDef> {
        self.props.get(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_ignores_render_fields() {
        let cat = PropCatalog::parse(
            r#"(props: {
                "block": (model: "models/props/toy_store/block.glb", dynamic: true, mass: 0.5,
                    colliders: [Box(half: (0.7, 0.7, 0.7), offset: (0, 0.7, 0))], lights: []),
                "ramp": (model: "x.glb", surface: Wood,
                    colliders: [Wedge(half: (2, 1.5, 6), offset: (0, 1.5, 0))]),
            })"#,
        )
        .expect("parses");
        assert!(cat.get("block").is_some_and(|b| b.dynamic && b.mass == 0.5));
        assert_eq!(
            cat.get("ramp").and_then(|r| r.surface),
            Some(SurfaceKind::Wood)
        );
        assert!(PropCatalog::parse(r#"(props: {"x": (model: "", colliders: [])})"#).is_err());
        assert!(
            PropCatalog::parse(r#"(props: {"x": (model: "", dynamic: true, mass: 0, colliders: [Ball(radius: 1)])})"#)
                .is_err()
        );
    }

    #[test]
    fn files_merge_and_duplicates_name_both_files() {
        let one = |kind: &str| PropCatalog {
            props: BTreeMap::from([(kind.to_owned(), PropDef::default())]),
        };
        let merged = PropCatalog::merge([("a.ron".into(), one("x")), ("b.ron".into(), one("y"))])
            .expect("distinct kinds merge");
        assert!(merged.get("x").is_some() && merged.get("y").is_some());
        let err = PropCatalog::merge([("a.ron".into(), one("x")), ("b.ron".into(), one("x"))])
            .expect_err("duplicate kind");
        assert!(
            err.contains("a.ron") && err.contains("b.ron") && err.contains("\"x\""),
            "{err}"
        );
    }

    #[test]
    fn wedge_rises_toward_the_front() {
        let pts = Primitive::wedge_points(Vec3::new(1.0, 1.0, 2.0));
        let top: Vec<_> = pts.iter().filter(|p| p.y > 0.0).collect();
        assert!(top.iter().all(|p| p.z < 0.0), "high edge at −Z");
        let (_, rot) = Primitive::Box {
            half: Vec3::ONE,
            offset: Vec3::ZERO,
            rot: Vec3::new(0.0, 90.0, 0.0),
        }
        .pose();
        assert!((rot * Vec3::NEG_Z).abs_diff_eq(Vec3::NEG_X, 1e-5));
    }
}
