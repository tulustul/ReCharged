//! The track's entry in `assets/data/catalog.ron`. The editor rewrites only that entry's text (or
//! appends a new one to `tracks: [...]`), so the rest of the hand-written file — comments
//! included — stays as it is (comments inside the entry itself are not kept); the result must
//! parse and validate as a whole.

use std::path::Path;

use rc_core::prelude::*;

/// Indent of an entry inside `tracks: [...]`.
const ENTRY_INDENT: &str = "        ";

/// One track entry as the file writes it: multi-line, lists on one line, indented for the list.
fn entry_ron(info: &TrackInfo) -> Result<String, String> {
    let pretty = ron::ser::PrettyConfig::new()
        .indentor("    ")
        .compact_arrays(true);
    let text = ron::ser::to_string_pretty(info, pretty).map_err(|e| e.to_string())?;
    Ok(text
        .lines()
        .map(|line| format!("{ENTRY_INDENT}{line}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Byte spans of the entries of the top-level `tracks: [...]` list (each from its `(` to its
/// `)`), and where the list's closing `]` is. Strings and comments are skipped.
fn track_entries(text: &str) -> Result<(Vec<(usize, usize)>, usize), String> {
    let bytes = text.as_bytes();
    let (mut i, mut depth) = (0, 0usize);
    let mut list: Option<usize> = None; // depth inside the tracks list
    let mut entries = Vec::new();
    let mut start = None;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = text[i + 2..].find("*/").ok_or("unterminated comment")?;
                i += end + 4;
                continue;
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                if list == Some(depth - 1) && bytes[i] == b'(' {
                    start = Some(i);
                }
                if list.is_none() && depth == 2 && bytes[i] == b'[' {
                    let key = text[..i].trim_end().strip_suffix(':').map(str::trim_end);
                    if key.is_some_and(|k| k.ends_with("tracks")) {
                        list = Some(depth);
                    }
                }
            }
            b')' | b']' | b'}' => {
                if list == Some(depth) && bytes[i] == b']' {
                    return Ok((entries, i));
                }
                if list == Some(depth - 1)
                    && let Some(s) = start.take()
                {
                    entries.push((s, i + 1));
                }
                depth = depth.checked_sub(1).ok_or("unbalanced brackets")?;
            }
            _ => {}
        }
        i += 1;
    }
    Err("no top-level `tracks: [...]` list".into())
}

/// `text` with the entry of `info.id` replaced by `info` (appended if there is none). The result
/// must parse and validate, and contain `info`.
pub fn with_entry(text: &str, info: &TrackInfo) -> Result<String, String> {
    let (entries, end) = track_entries(text)?;
    let found = entries
        .into_iter()
        .find(|&(s, e)| ron::from_str::<TrackInfo>(&text[s..e]).is_ok_and(|t| t.id == info.id));
    let entry = entry_ron(info)?;
    let out = match found {
        Some((s, e)) => {
            let indent = text[..s].rsplit('\n').next().unwrap_or("");
            format!("{}{}{}", &text[..s - indent.len()], entry, &text[e..])
        }
        None => {
            let before = text[..end].trim_end();
            let comma = if before.ends_with(['[', ',']) {
                ""
            } else {
                ","
            };
            let indent = text[..end].rsplit('\n').next().unwrap_or("");
            format!("{before}{comma}\n{entry},\n{indent}{}", &text[end..])
        }
    };
    let parsed = Catalog::parse(&out, Path::new(CATALOG_FILE)).map_err(|e| e.to_string())?;
    if parsed.track(&info.id) != Some(info) {
        return Err(format!(
            "catalog.ron: the entry of {} did not survive",
            info.id
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> String {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        std::fs::read_to_string(assets.join(CATALOG_FILE)).unwrap()
    }

    #[test]
    fn an_unchanged_entry_rewrites_to_an_equal_catalog_and_keeps_the_comments() {
        let text = shipped();
        let catalog = Catalog::parse(&text, Path::new(CATALOG_FILE)).unwrap();
        let (entries, _) = track_entries(&text).unwrap();
        assert_eq!(entries.len(), catalog.tracks.len());
        for (info, (s, e)) in catalog.tracks.iter().zip(entries) {
            let out = with_entry(&text, info).unwrap();
            assert_eq!(
                Catalog::parse(&out, Path::new("x")).unwrap(),
                catalog,
                "{}",
                info.id
            );
            let inside = text[s..e].matches("//").count();
            assert_eq!(
                out.matches("//").count(),
                text.matches("//").count() - inside,
                "{}: comments outside the entry are kept",
                info.id
            );
        }
    }

    #[test]
    fn a_changed_entry_is_replaced_and_a_new_one_appended() {
        let text = shipped();
        let catalog = Catalog::parse(&text, Path::new(CATALOG_FILE)).unwrap();
        let mut info = catalog.tracks[1].clone();
        info.name = "Renamed".into();
        info.default_laps = 7;
        let out = with_entry(&text, &info).unwrap();
        let parsed = Catalog::parse(&out, Path::new("x")).unwrap();
        assert_eq!(parsed.tracks[1], info);
        assert_eq!(parsed.tracks.len(), catalog.tracks.len());

        info.id = "brand_new".into();
        info.hidden = true;
        let out = with_entry(&text, &info).unwrap();
        let parsed = Catalog::parse(&out, Path::new("x")).unwrap();
        assert_eq!(parsed.tracks.len(), catalog.tracks.len() + 1);
        assert_eq!(parsed.tracks.last(), Some(&info));

        let mut bad = info.clone();
        bad.default_laps = 0;
        assert!(
            with_entry(&text, &bad)
                .unwrap_err()
                .contains("default_laps")
        );
    }
}
