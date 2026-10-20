//! The asset library (built once at start, `editor.library`): prop kinds grouped by the
//! `assets/data/props/<env>.ron` file they come from, decor models from a scan of
//! `assets/models/props/<env>/`, and the gameplay items; the track's own environment first and
//! expanded. Also what placing and swapping a library asset does to the document.
//!
//! Lights (point, spot, sun, fill) join in P9, with lights in the document.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::math::Vec3;
use rc_track::{DecorItem, Dir, GoldenBatteryDef, ItemRowDef, PropCatalog, PropPlacement};
use rc_ui::editor::{EditorAsset, EditorItem, EditorLibrary, LibraryEntry, LibraryGroup};

use crate::doc::Doc;

/// Decor models live in `<this>/<env>/*.glb` under the assets root.
const DECOR_DIR: &str = "models/props";
/// Prop kinds every environment uses (`common.ron`).
const COMMON: &str = "common";
/// The Golden Battery floats this high above where it is placed (u).
pub(crate) const BATTERY_LIFT_U: f32 = 1.0;
/// A placed item row holds this many pickups.
const NEW_ROW_COUNT: u8 = 5;

/// "toy_block_cube" → "Toy block cube".
pub fn pretty(id: &str) -> String {
    let spaced = id.replace('_', " ");
    let mut chars = spaced.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Whether props or decor made for `group` (a props file or model directory: `toy`) belong to
/// the track's environment kind (`toy_store`).
fn own(group: &str, environment: &str) -> bool {
    environment == group || environment.starts_with(&format!("{group}_"))
}

/// Every `.glb` in `<assets>/models/props/<env>/`, by environment (paths as data files write
/// them: `models/props/toy/sign.glb`).
pub fn decor_models(assets_dir: &Path) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let Ok(envs) = std::fs::read_dir(assets_dir.join(DECOR_DIR)) else {
        return out;
    };
    for env in envs.filter_map(Result::ok) {
        let name = env.file_name().to_string_lossy().into_owned();
        let Ok(files) = std::fs::read_dir(env.path()) else {
            continue; // not a directory
        };
        let mut models: Vec<String> = (files.filter_map(Result::ok))
            .map(|f| f.file_name().to_string_lossy().into_owned())
            .filter(|f| f.ends_with(".glb"))
            .map(|f| format!("{DECOR_DIR}/{name}/{f}"))
            .collect();
        models.sort();
        if !models.is_empty() {
            out.insert(name, models);
        }
    }
    out
}

/// The library for a track of environment kind `environment`.
pub fn build(assets_dir: &Path, props: &PropCatalog, environment: &str) -> EditorLibrary {
    let mut kinds: BTreeMap<&str, Vec<LibraryEntry>> = BTreeMap::new();
    for (kind, file) in &props.files {
        kinds.entry(file).or_default().push(LibraryEntry {
            name: pretty(kind),
            asset: EditorAsset::Prop { id: kind.clone() },
        });
    }
    let decor = decor_models(assets_dir);
    // (own environment, common, group name, kind): the sort order.
    let mut groups: Vec<((bool, bool, String, u8), LibraryGroup)> = Vec::new();
    let mut push = |env: &str, kind: u8, title: &str, mut entries: Vec<LibraryEntry>| {
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let mine = own(env, environment);
        let group = LibraryGroup {
            title: format!("{title} · {env}"),
            open: mine,
            entries,
        };
        groups.push(((!mine, env != COMMON, env.to_owned(), kind), group));
    };
    for (env, entries) in kinds {
        push(env, 0, "Props", entries);
    }
    for (env, models) in decor {
        let entries = models
            .into_iter()
            .map(|model| LibraryEntry {
                name: pretty(
                    Path::new(&model)
                        .file_stem()
                        .map_or("", |s| s.to_str().unwrap_or("")),
                ),
                asset: EditorAsset::Decor { model },
            })
            .collect();
        push(&env, 1, "Decor", entries);
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    let mut groups: Vec<LibraryGroup> = groups.into_iter().map(|(_, g)| g).collect();
    // Gameplay after the track's own groups, open.
    let at = groups.iter().take_while(|g| g.open).count();
    groups.insert(
        at,
        LibraryGroup {
            title: "Gameplay".into(),
            open: true,
            entries: vec![
                LibraryEntry {
                    name: "Item row".into(),
                    asset: EditorAsset::ItemRow,
                },
                LibraryEntry {
                    name: "Golden Battery".into(),
                    asset: EditorAsset::Battery,
                },
            ],
        },
    );
    EditorLibrary { groups }
}

fn last(len: usize) -> u32 {
    len as u32 - 1
}

impl Doc {
    /// The library asset `item` shows (props and decor).
    pub fn asset(&self, item: EditorItem) -> Option<EditorAsset> {
        match item {
            EditorItem::Prop { index } => (self.def.props.get(index as usize))
                .map(|p| EditorAsset::Prop { id: p.kind.clone() }),
            EditorItem::Decor { index } => {
                (self.decor.get(index as usize)).map(|d| EditorAsset::Decor {
                    model: d.model.clone(),
                })
            }
            _ => None,
        }
    }

    /// Adds `asset` standing at `at` (upright, default size); returns the new item.
    pub fn place(
        &mut self,
        asset: &EditorAsset,
        at: Vec3,
        props: &PropCatalog,
    ) -> Result<EditorItem, String> {
        let d = &mut self.def;
        Ok(match asset {
            EditorAsset::Prop { id } => {
                if props.get(id).is_none() {
                    return Err(format!("no prop kind \"{id}\""));
                }
                d.props.push(PropPlacement {
                    kind: id.clone(),
                    pos: at,
                    yaw: 0.0,
                    dir: Dir::Both,
                    on_line: false,
                });
                EditorItem::Prop {
                    index: last(d.props.len()),
                }
            }
            EditorAsset::Decor { model } => {
                self.decor.push(DecorItem {
                    model: model.clone(),
                    pos: at,
                    rot: Vec3::ZERO,
                    scale: Vec3::ONE,
                    dir: Dir::Both,
                });
                EditorItem::Decor {
                    index: last(self.decor.len()),
                }
            }
            EditorAsset::ItemRow => {
                d.item_rows.push(ItemRowDef {
                    pos: at,
                    count: NEW_ROW_COUNT,
                    dir: Dir::Both,
                });
                EditorItem::ItemRow {
                    index: last(d.item_rows.len()),
                }
            }
            EditorAsset::Battery => {
                if d.golden_battery.is_some() {
                    return Err("the track already has its Golden Battery".into());
                }
                d.golden_battery = Some(GoldenBatteryDef {
                    pos: at + Vec3::Y * BATTERY_LIFT_U,
                    dir: Dir::Fwd,
                });
                EditorItem::Battery
            }
        })
    }

    /// `item` becomes `asset`, keeping everything else (pose, scale, direction): a prop another
    /// prop kind, a decor item another model.
    pub fn swap(
        &mut self,
        item: EditorItem,
        asset: &EditorAsset,
        props: &PropCatalog,
    ) -> Result<(), String> {
        match (item, asset) {
            (EditorItem::Prop { index }, EditorAsset::Prop { id }) => {
                if props.get(id).is_none() {
                    return Err(format!("no prop kind \"{id}\""));
                }
                let p = (self.def.props.get_mut(index as usize)).ok_or("no such prop")?;
                id.clone_into(&mut p.kind);
            }
            (EditorItem::Decor { index }, EditorAsset::Decor { model }) => {
                let d = (self.decor.get_mut(index as usize)).ok_or("no such decor item")?;
                model.clone_into(&mut d.model);
            }
            (EditorItem::Prop { .. }, _) => return Err("a prop swaps only with a prop".into()),
            (EditorItem::Decor { .. }, _) => {
                return Err("a decor item swaps only with decor".into());
            }
            _ => return Err("only props and decor can be swapped".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rc_core::prelude::*;
    use rc_track::TrackDef;

    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    #[test]
    fn the_library_groups_props_by_file_and_decor_by_directory_own_environment_first() {
        let props = PropCatalog::load(&assets()).unwrap();
        assert_eq!(props.files["toy_block_cube"], "toy");
        assert_eq!(props.files["crate_small"], "common");
        let decor = decor_models(&assets());
        assert!(decor["toy"].contains(&"models/props/toy/sign.glb".to_owned()));
        assert!(decor.values().flatten().all(|m| m.ends_with(".glb")));

        let library = build(&assets(), &props, "toy_store");
        let titles: Vec<&str> = library.groups.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(
            titles[..4],
            ["Props · toy", "Decor · toy", "Gameplay", "Props · common"]
        );
        let open: Vec<bool> = library.groups.iter().map(|g| g.open).collect();
        assert_eq!(
            open[..4],
            [true, true, true, false],
            "only the track's own and gameplay"
        );
        let entries = |kind: &str| -> usize {
            (library.groups.iter())
                .filter(|g| g.title.starts_with(kind))
                .map(|g| g.entries.len())
                .sum()
        };
        assert_eq!(entries("Props"), props.props.len(), "every prop kind once");
        assert_eq!(
            entries("Decor"),
            decor.values().map(Vec::len).sum::<usize>()
        );
        let cube = (library.groups[0].entries.iter()).find(|e| e.name == "Toy block cube");
        assert_eq!(
            cube.map(|e| &e.asset),
            Some(&EditorAsset::Prop {
                id: "toy_block_cube".into()
            })
        );
    }

    fn oval() -> Doc {
        Doc {
            def: TrackDef::load(&assets(), "test_oval").unwrap(),
            decor: vec![DecorItem {
                model: "models/props/toy/sign.glb".into(),
                pos: Vec3::new(1.0, 2.0, 3.0),
                rot: Vec3::new(10.0, 20.0, 30.0),
                scale: Vec3::new(1.0, 2.0, 0.5),
                dir: Dir::Rev,
            }],
            info: TrackInfo::default(),
        }
    }

    #[test]
    fn swaps_keep_the_transform_and_the_type() {
        let props = PropCatalog::load(&assets()).unwrap();
        let mut doc = oval();
        let prop = EditorItem::Prop { index: 0 };
        let before = doc.def.props[0].clone();
        let bear = EditorAsset::Prop {
            id: "toy_bear".into(),
        };
        doc.swap(prop, &bear, &props).unwrap();
        let after = &doc.def.props[0];
        assert_eq!(after.kind, "toy_bear");
        assert_eq!(
            (after.pos, after.yaw, after.dir),
            (before.pos, before.yaw, before.dir)
        );
        assert_eq!(doc.asset(prop), Some(bear.clone()));

        let decor = EditorItem::Decor { index: 0 };
        let duck = EditorAsset::Decor {
            model: "models/props/toy/duck.glb".into(),
        };
        let pose = doc.pose(decor).unwrap();
        doc.swap(decor, &duck, &props).unwrap();
        assert_eq!(doc.decor[0].model, "models/props/toy/duck.glb");
        assert_eq!(doc.pose(decor).unwrap(), pose);
        assert_eq!(doc.decor[0].dir, Dir::Rev);

        assert!(doc.swap(prop, &duck, &props).is_err(), "prop ↔ prop only");
        assert!(
            doc.swap(decor, &bear, &props).is_err(),
            "decor ↔ decor only"
        );
        let nope = EditorAsset::Prop { id: "nope".into() };
        assert!(doc.swap(prop, &nope, &props).is_err());
        assert!(doc.swap(EditorItem::Battery, &bear, &props).is_err());
    }

    #[test]
    fn placing_adds_the_asset_upright_where_it_was_dropped() {
        let props = PropCatalog::load(&assets()).unwrap();
        let mut doc = oval();
        doc.def.golden_battery = None;
        let at = Vec3::new(5.0, 0.5, -3.0);
        let bear = EditorAsset::Prop {
            id: "toy_bear".into(),
        };
        let item = doc.place(&bear, at, &props).unwrap();
        assert_eq!(doc.asset(item), Some(bear));
        assert_eq!(doc.handle(item), Some(at));
        let row = doc.place(&EditorAsset::ItemRow, at, &props).unwrap();
        assert_eq!(doc.handle(row), Some(at));
        let battery = doc.place(&EditorAsset::Battery, at, &props).unwrap();
        assert_eq!(doc.handle(battery), Some(at + Vec3::Y * BATTERY_LIFT_U));
        assert!(
            doc.place(&EditorAsset::Battery, at, &props).is_err(),
            "one only"
        );
        let decor = EditorAsset::Decor {
            model: "models/props/toy/duck.glb".into(),
        };
        let item = doc.place(&decor, at, &props).unwrap();
        assert_eq!(doc.pose(item).unwrap().0.scale, Vec3::ONE);
    }
}
