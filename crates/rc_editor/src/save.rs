//! Ctrl+S: the document's files, each through a temporary file + rename so a failed write never
//! truncates one, and each checked to parse back before it is written.

use std::path::{Path, PathBuf};

use rc_core::prelude::*;
use rc_render::{ENVIRONMENT_FILE, EnvironmentDef};
use rc_track::{DECOR_FILE, TRACK_FILE, TrackDef, decor_to_ron, map_dir};

use crate::catalog;
use crate::doc::Doc;

/// Writes `<assets>/maps/<id>/track.ron`, `decor.ron` (none is created for no decor),
/// `environment.ron` and `catalog.ron`, the last two only when they differ from `saved` (an
/// untouched environment keeps its comments). Returns the track file and the catalog text now on
/// disk.
pub(crate) fn save(
    assets_dir: &Path,
    id: &TrackId,
    doc: &Doc,
    saved: &Doc,
    catalog_text: &str,
) -> Result<(PathBuf, String), String> {
    let ron = doc.def.to_ron()?;
    TrackDef::parse(&ron).map_err(|e| format!("the saved layout would not load: {e}"))?;
    let header = format!("// Written by the track editor: cargo run -p rc_editor -- {id}\n");
    let dir = map_dir(assets_dir, id.as_str());
    let (path, decor_path) = (dir.join(TRACK_FILE), dir.join(DECOR_FILE));
    write(&path, &format!("{header}{ron}"))?;
    if !doc.decor.is_empty() || decor_path.exists() {
        let decor = decor_to_ron(&doc.decor)?;
        write(&decor_path, &format!("{header}{decor}"))?;
    }
    if doc.env != saved.env {
        let env = doc.env.to_ron()?;
        EnvironmentDef::parse(&env)
            .map_err(|e| format!("the saved environment would not load: {e}"))?;
        write(&dir.join(ENVIRONMENT_FILE), &format!("{header}{env}"))?;
    }
    let mut catalog_text = catalog_text.to_owned();
    if doc.info != saved.info {
        catalog_text = catalog::with_entry(&catalog_text, &doc.info)?;
        write(&assets_dir.join(CATALOG_FILE), &catalog_text)?;
    }
    Ok((path, catalog_text))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorPlugin;

    #[test]
    fn save_writes_files_that_load_back_and_replace_the_old_ones() {
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let assets = std::env::temp_dir().join(format!("rc_editor-save-{}", std::process::id()));
        let dir = map_dir(&assets, "test_oval");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(assets.join("data")).unwrap();
        std::fs::write(dir.join(TRACK_FILE), "old").unwrap();
        let plugin = EditorPlugin::load(&real, "test_oval").unwrap();
        let mut doc = plugin.doc.clone();
        doc.def.points[1].pos.x += 0.5;
        doc.def.points[1].label = Some("start".into());
        doc.decor = rc_track::load_decor(&real, "toy_town").unwrap();
        doc.info.name = "Renamed Oval".into();
        doc.env = EnvironmentDef::load(&real, "toy_town").unwrap();
        doc.env.noon.exposure += 0.5;
        let (path, catalog_text) = save(
            &assets,
            &plugin.track,
            &doc,
            &plugin.doc,
            &plugin.catalog_text,
        )
        .unwrap();
        assert_eq!(TrackDef::load(&assets, "test_oval").unwrap(), doc.def);
        assert_eq!(
            rc_track::load_decor(&assets, "test_oval").unwrap(),
            doc.decor
        );
        assert_eq!(EnvironmentDef::load(&assets, "test_oval").unwrap(), doc.env);
        let catalog = Catalog::load(&assets).unwrap();
        assert_eq!(catalog.track(&plugin.track), Some(&doc.info));
        assert_eq!(
            std::fs::read_to_string(assets.join(CATALOG_FILE)).unwrap(),
            catalog_text
        );
        assert!(!path.with_extension("ron.tmp").exists());
        let _ = std::fs::remove_dir_all(&assets);
    }
}
