//! The inspector: every field of the selected item as an [`EditorField`], and setting one back.
//! One list of slots (mutable references into the document) serves both directions, so a field
//! is declared once.

use bevy::math::{Vec2, Vec3};
use rc_core::prelude::*;
use rc_track::{BoundsDef, DEFAULT_OFFROAD_MARGIN_U, DEFAULT_WIDTH_U, Dir, RibbonStyle, Track};
use rc_ui::editor::{EditorField, EditorItem, FieldValue};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::doc::Doc;

const DIRS: [Dir; 3] = [Dir::Both, Dir::Fwd, Dir::Rev];
const STYLES: [RibbonStyle; 4] = [
    RibbonStyle::ToyTrack,
    RibbonStyle::Asphalt,
    RibbonStyle::CarpetRunner,
    RibbonStyle::Invisible,
];
// ponytail: listed by hand; a new SurfaceKind is missing from the dropdown until added here.
const SURFACES: [SurfaceKind; 12] = [
    SurfaceKind::Asphalt,
    SurfaceKind::Stone,
    SurfaceKind::ToyTrack,
    SurfaceKind::Wood,
    SurfaceKind::Tile,
    SurfaceKind::Carpet,
    SurfaceKind::Cardboard,
    SurfaceKind::Grass,
    SurfaceKind::Dirt,
    SurfaceKind::Ice,
    SurfaceKind::Oil,
    SurfaceKind::Water,
];
/// Bounds switched on without a built track: this square (u).
const DEFAULT_BOUNDS_U: f32 = 100.0;

/// The name of an enum variant as written in data files (`Asphalt`).
pub fn name<T: Serialize>(value: &T) -> String {
    ron::to_string(value).unwrap_or_default()
}

/// Parses a variant name.
pub fn parse<T: DeserializeOwned>(name: &str) -> Result<T, String> {
    ron::from_str(name).map_err(|_| format!("\"{name}\" is not an option"))
}

/// A field to pick from a list.
trait Pick {
    fn shown(&self) -> String;
    /// May be unset (inherited).
    fn optional(&self) -> bool {
        false
    }
    fn is_set(&self) -> bool;
    fn options(&self) -> Vec<String>;
    fn set(&mut self, name: Option<&str>) -> Result<(), String>;
}

struct EnumPick<'a, T: 'static> {
    value: &'a mut T,
    options: &'static [T],
}

impl<T: Serialize + DeserializeOwned> Pick for EnumPick<'_, T> {
    fn shown(&self) -> String {
        name(self.value)
    }
    fn is_set(&self) -> bool {
        true
    }
    fn options(&self) -> Vec<String> {
        self.options.iter().map(name).collect()
    }
    fn set(&mut self, name: Option<&str>) -> Result<(), String> {
        *self.value = parse(name.ok_or("this field cannot be unset")?)?;
        Ok(())
    }
}

/// An inherited enum (`None` = the previous point's value, `shown`).
struct OptPick<'a, T: 'static> {
    value: &'a mut Option<T>,
    shown: T,
    options: &'static [T],
}

impl<T: Serialize + DeserializeOwned> Pick for OptPick<'_, T> {
    fn shown(&self) -> String {
        name(self.value.as_ref().unwrap_or(&self.shown))
    }
    fn optional(&self) -> bool {
        true
    }
    fn is_set(&self) -> bool {
        self.value.is_some()
    }
    fn options(&self) -> Vec<String> {
        self.options.iter().map(name).collect()
    }
    fn set(&mut self, name: Option<&str>) -> Result<(), String> {
        *self.value = name.map(parse).transpose()?;
        Ok(())
    }
}

/// A string out of a list (prop kinds, environments).
struct StringPick<'a> {
    value: &'a mut String,
    options: Vec<String>,
}

impl Pick for StringPick<'_> {
    fn shown(&self) -> String {
        self.value.clone()
    }
    fn is_set(&self) -> bool {
        true
    }
    fn options(&self) -> Vec<String> {
        self.options.clone()
    }
    fn set(&mut self, name: Option<&str>) -> Result<(), String> {
        let name = name.ok_or("this field cannot be unset")?;
        if !self.options.iter().any(|o| o == name) {
            return Err(format!("\"{name}\" is not an option"));
        }
        name.clone_into(self.value);
        Ok(())
    }
}

/// A value edited as its RON text (lists, unlock rules).
trait RonText {
    fn text(&self) -> String;
    fn set(&mut self, text: &str) -> Result<(), String>;
}

impl<T: Serialize + DeserializeOwned> RonText for &mut T {
    fn text(&self) -> String {
        name(&**self)
    }
    fn set(&mut self, text: &str) -> Result<(), String> {
        **self = ron::from_str(text).map_err(|e| format!("\"{text}\": {e}"))?;
        Ok(())
    }
}

/// What fields choose from besides fixed enums.
#[derive(Debug, Clone, Default)]
pub struct Choices {
    /// Prop kinds.
    pub kinds: Vec<String>,
    /// Environment ids.
    pub environments: Vec<String>,
}

/// A mutable view of one field.
enum Slot<'a> {
    Num(&'a mut f32),
    Bool(&'a mut bool),
    Ron(Box<dyn RonText + 'a>),
    /// Inherited or absent; the second value is shown while unset.
    OptNum(&'a mut Option<f32>, f32),
    Index(&'a mut usize),
    Count(&'a mut u8),
    OptBool(&'a mut Option<bool>, bool),
    V3(&'a mut Vec3),
    V2(&'a mut Vec2),
    Text(&'a mut String),
    OptText(&'a mut Option<String>),
    Bounds(&'a mut Option<BoundsDef>, Option<(Vec3, Vec3)>),
    Pick(Box<dyn Pick + 'a>),
}

// ponytail: listed by hand like SURFACES.
const MUSIC: [MusicCue; 4] = [
    MusicCue::Race1,
    MusicCue::Race2,
    MusicCue::Race3,
    MusicCue::Race4,
];

fn pick<'a, T: Serialize + DeserializeOwned>(value: &'a mut T, options: &'static [T]) -> Slot<'a> {
    Slot::Pick(Box::new(EnumPick { value, options }))
}

/// What point `i` inherits for each optional field (the last value set before it).
struct Inherited {
    width: f32,
    surface: SurfaceKind,
    wall_left: bool,
    wall_right: bool,
    offroad_margin: f32,
}

fn inherited(doc: &Doc, i: usize) -> Inherited {
    let mut out = Inherited {
        width: DEFAULT_WIDTH_U,
        surface: doc.def.style.default_surface(),
        wall_left: false,
        wall_right: false,
        offroad_margin: DEFAULT_OFFROAD_MARGIN_U,
    };
    for p in &doc.def.points[..i] {
        out.width = p.width.unwrap_or(out.width);
        out.surface = p.surface.unwrap_or(out.surface);
        out.wall_left = p.wall_left.unwrap_or(out.wall_left);
        out.wall_right = p.wall_right.unwrap_or(out.wall_right);
        out.offroad_margin = p.offroad_margin.unwrap_or(out.offroad_margin);
    }
    out
}

/// Title and slots of `item`; `None` if it does not exist.
fn slots<'a>(
    doc: &'a mut Doc,
    item: EditorItem,
    choices: &Choices,
    track: Option<&Track>,
) -> Option<(String, Vec<(&'static str, Slot<'a>)>)> {
    if !doc.exists(item) {
        return None;
    }
    let i = |index: u32| index as usize;
    Some(match item {
        EditorItem::Track => {
            let bounds = track.map(Track::bounds);
            let (d, info) = (&mut doc.def, &mut doc.info);
            let environment = StringPick {
                value: &mut info.environment,
                options: choices.environments.clone(),
            };
            (
                "Track".into(),
                vec![
                    ("style", pick(&mut d.style, &STYLES)),
                    ("start", Slot::Index(&mut d.start)),
                    ("kill_height", Slot::Num(&mut d.kill_height)),
                    ("wall_height", Slot::Num(&mut d.wall_height)),
                    ("bounds", Slot::Bounds(&mut d.bounds, bounds)),
                    // The catalog entry.
                    ("name", Slot::Text(&mut info.name)),
                    ("environment", Slot::Pick(Box::new(environment))),
                    ("indoor", Slot::Bool(&mut info.indoor)),
                    ("difficulty", Slot::Count(&mut info.difficulty)),
                    ("default_laps", Slot::Count(&mut info.default_laps)),
                    ("music", pick(&mut info.music, &MUSIC)),
                    (
                        "allowed_times",
                        Slot::Ron(Box::new(&mut info.allowed_times)),
                    ),
                    (
                        "allowed_weather",
                        Slot::Ron(Box::new(&mut info.allowed_weather)),
                    ),
                    ("unlock", Slot::Ron(Box::new(&mut info.unlock))),
                    ("preview", Slot::Text(&mut info.preview)),
                    ("hidden", Slot::Bool(&mut info.hidden)),
                ],
            )
        }
        EditorItem::Point { index } => {
            let from = inherited(doc, i(index));
            let p = &mut doc.def.points[i(index)];
            let title = match &p.label {
                Some(label) => format!("Point {index} · {label}"),
                None => format!("Point {index}"),
            };
            let surface = OptPick {
                value: &mut p.surface,
                shown: from.surface,
                options: &SURFACES,
            };
            (
                title,
                vec![
                    ("pos", Slot::V3(&mut p.pos)),
                    ("width", Slot::OptNum(&mut p.width, from.width)),
                    ("bank_deg", Slot::Num(&mut p.bank_deg)),
                    ("surface", Slot::Pick(Box::new(surface))),
                    ("wall_left", Slot::OptBool(&mut p.wall_left, from.wall_left)),
                    (
                        "wall_right",
                        Slot::OptBool(&mut p.wall_right, from.wall_right),
                    ),
                    (
                        "offroad_margin",
                        Slot::OptNum(&mut p.offroad_margin, from.offroad_margin),
                    ),
                    ("label", Slot::OptText(&mut p.label)),
                ],
            )
        }
        EditorItem::Gap { index } => {
            let g = &mut doc.def.gaps[i(index)];
            (
                format!("Gap {index}"),
                vec![
                    ("from", Slot::Index(&mut g.from)),
                    ("to", Slot::Index(&mut g.to)),
                    ("dir", pick(&mut g.dir, &DIRS)),
                ],
            )
        }
        EditorItem::Ground { index } | EditorItem::Vertex { ground: index, .. } => {
            let vertex = match item {
                EditorItem::Vertex { index, .. } => Some(i(index)),
                _ => None,
            };
            let g = &mut doc.def.ground[i(index)];
            let mut fields = vec![
                ("surface", pick(&mut g.surface, &SURFACES)),
                ("height", Slot::Num(&mut g.height)),
                ("dir", pick(&mut g.dir, &DIRS)),
            ];
            let title = match vertex {
                Some(v) => {
                    fields.insert(0, ("vertex", Slot::V2(&mut g.points[v])));
                    format!("Ground {index} · vertex {v}")
                }
                None => format!("Ground {index}"),
            };
            (title, fields)
        }
        EditorItem::ItemRow { index } => {
            let r = &mut doc.def.item_rows[i(index)];
            (
                format!("Item row {index}"),
                vec![
                    ("pos", Slot::V3(&mut r.pos)),
                    ("count", Slot::Count(&mut r.count)),
                    ("dir", pick(&mut r.dir, &DIRS)),
                ],
            )
        }
        EditorItem::Prop { index } => {
            let p = &mut doc.def.props[i(index)];
            let kind = StringPick {
                value: &mut p.kind,
                options: choices.kinds.clone(),
            };
            (
                format!("Prop {index}"),
                vec![
                    ("kind", Slot::Pick(Box::new(kind))),
                    ("pos", Slot::V3(&mut p.pos)),
                    ("yaw", Slot::Num(&mut p.yaw)),
                    ("dir", pick(&mut p.dir, &DIRS)),
                ],
            )
        }
        EditorItem::Shortcut { index }
        | EditorItem::PathPoint {
            shortcut: index, ..
        } => {
            let point = match item {
                EditorItem::PathPoint { index, .. } => Some(i(index)),
                _ => None,
            };
            let s = &mut doc.def.shortcuts[i(index)];
            let mut fields = vec![
                ("width", Slot::Num(&mut s.width)),
                ("dir", pick(&mut s.dir, &DIRS)),
            ];
            let title = match point {
                Some(p) => {
                    fields.insert(0, ("point", Slot::V3(&mut s.path[p])));
                    format!("Shortcut {index} · point {p}")
                }
                None => format!("Shortcut {index}"),
            };
            (title, fields)
        }
        EditorItem::Battery => {
            let b = doc.def.golden_battery.as_mut()?;
            (
                "Golden Battery".into(),
                vec![
                    ("pos", Slot::V3(&mut b.pos)),
                    ("dir", pick(&mut b.dir, &DIRS)),
                ],
            )
        }
        EditorItem::Decor { index } => {
            let d = &mut doc.decor[i(index)];
            (
                format!("Decor {index}"),
                vec![
                    ("model", Slot::Text(&mut d.model)),
                    ("pos", Slot::V3(&mut d.pos)),
                    ("yaw", Slot::Num(&mut d.yaw)),
                    ("scale", Slot::Num(&mut d.scale)),
                    ("dir", pick(&mut d.dir, &DIRS)),
                ],
            )
        }
    })
}

fn field(key: &str, optional: bool, set: bool, value: FieldValue) -> EditorField {
    EditorField {
        key: key.into(),
        value,
        optional,
        set,
    }
}

fn read(key: &'static str, slot: &Slot) -> Vec<EditorField> {
    let plain = |value| vec![field(key, false, true, value)];
    let opt = |set: bool, value| vec![field(key, true, set, value)];
    match slot {
        Slot::Num(v) => plain(FieldValue::Number { value: **v }),
        Slot::Bool(v) => plain(FieldValue::Toggle { value: **v }),
        Slot::Ron(v) => plain(FieldValue::Text { value: v.text() }),
        Slot::OptNum(v, shown) => opt(
            v.is_some(),
            FieldValue::Number {
                value: v.unwrap_or(*shown),
            },
        ),
        Slot::Index(v) => plain(FieldValue::Integer { value: **v as i32 }),
        Slot::Count(v) => plain(FieldValue::Integer {
            value: i32::from(**v),
        }),
        Slot::OptBool(v, shown) => opt(
            v.is_some(),
            FieldValue::Toggle {
                value: v.unwrap_or(*shown),
            },
        ),
        Slot::V3(v) => plain(FieldValue::Vector {
            value: v.to_array().to_vec(),
        }),
        Slot::V2(v) => plain(FieldValue::Vector {
            value: v.to_array().to_vec(),
        }),
        Slot::Text(v) => plain(FieldValue::Text {
            value: (*v).clone(),
        }),
        Slot::OptText(v) => opt(
            v.is_some(),
            FieldValue::Text {
                value: (*v).clone().unwrap_or_default(),
            },
        ),
        Slot::Bounds(b, _) => {
            let mut out = vec![field(
                key,
                false,
                true,
                FieldValue::Toggle { value: b.is_some() },
            )];
            if let Some(b) = b {
                let v = |v: Vec2| FieldValue::Vector {
                    value: v.to_array().to_vec(),
                };
                out.push(field("bounds_min", false, true, v(b.min)));
                out.push(field("bounds_max", false, true, v(b.max)));
            }
            out
        }
        Slot::Pick(p) => vec![field(
            key,
            p.optional(),
            p.is_set(),
            FieldValue::Choice {
                value: p.shown(),
                options: p.options(),
            },
        )],
    }
}

/// Title and fields of `item` (`track`: the session's, for default bounds). `None` if the item
/// does not exist. The track item also lists its catalog entry's fields.
pub fn fields(
    doc: &Doc,
    item: EditorItem,
    choices: &Choices,
    track: Option<&Track>,
) -> Option<(String, Vec<EditorField>)> {
    let mut doc = doc.clone();
    let (title, slots) = slots(&mut doc, item, choices, track)?;
    Some((
        title,
        slots
            .iter()
            .flat_map(|(key, slot)| read(key, slot))
            .collect(),
    ))
}

fn number(value: Option<FieldValue>) -> Result<Option<f32>, String> {
    match value {
        None => Ok(None),
        Some(FieldValue::Number { value }) if value.is_finite() => Ok(Some(value)),
        Some(FieldValue::Integer { value }) => Ok(Some(value as f32)),
        Some(other) => Err(format!("expected a number, got {other:?}")),
    }
}

fn vector<const N: usize>(value: Option<FieldValue>) -> Result<[f32; N], String> {
    match value {
        Some(FieldValue::Vector { value })
            if value.len() == N && value.iter().all(|v| v.is_finite()) =>
        {
            let mut out = [0.0; N];
            out.copy_from_slice(&value);
            Ok(out)
        }
        other => Err(format!("expected {N} numbers, got {other:?}")),
    }
}

/// Sets field `key` of `item` (`None` unsets an optional field).
pub fn set_field(
    doc: &mut Doc,
    item: EditorItem,
    key: &str,
    value: Option<FieldValue>,
    choices: &Choices,
    track: Option<&Track>,
) -> Result<(), String> {
    let (_, slots) = slots(doc, item, choices, track).ok_or("the selection no longer exists")?;
    let target = match key {
        "bounds_min" | "bounds_max" => "bounds",
        key => key,
    };
    let (_, slot) = slots
        .into_iter()
        .find(|(k, _)| *k == target)
        .ok_or_else(|| format!("no field \"{key}\""))?;
    let required = |v: Option<f32>| v.ok_or_else(|| format!("{key} cannot be unset"));
    match slot {
        Slot::Num(v) => *v = required(number(value)?)?,
        Slot::OptNum(v, _) => *v = number(value)?,
        Slot::Index(v) => {
            let n = required(number(value)?)?;
            if n < 0.0 {
                return Err(format!("{key} must not be negative"));
            }
            *v = n as usize;
        }
        Slot::Count(v) => *v = required(number(value)?)?.clamp(0.0, 255.0) as u8,
        Slot::Bool(v) => match value {
            Some(FieldValue::Toggle { value }) => *v = value,
            other => return Err(format!("expected on/off, got {other:?}")),
        },
        Slot::Ron(mut v) => match value {
            Some(FieldValue::Text { value }) => v.set(&value)?,
            other => return Err(format!("{key} needs text, got {other:?}")),
        },
        Slot::OptBool(v, _) => match value {
            None => *v = None,
            Some(FieldValue::Toggle { value }) => *v = Some(value),
            other => return Err(format!("expected on/off, got {other:?}")),
        },
        Slot::V3(v) => *v = Vec3::from_array(vector(value)?),
        Slot::V2(v) => *v = Vec2::from_array(vector(value)?),
        Slot::Text(v) => match value {
            Some(FieldValue::Text { value }) if !value.trim().is_empty() => *v = value,
            other => return Err(format!("{key} needs text, got {other:?}")),
        },
        Slot::OptText(v) => match value {
            None => *v = None,
            Some(FieldValue::Text { value }) => {
                *v = (!value.trim().is_empty()).then(|| value.trim().to_owned());
            }
            other => return Err(format!("expected text, got {other:?}")),
        },
        Slot::Bounds(b, built) => match (key, value) {
            ("bounds", Some(FieldValue::Toggle { value })) => {
                *b = value.then(|| {
                    let (lo, hi) = built.unwrap_or((
                        Vec3::splat(-DEFAULT_BOUNDS_U / 2.0),
                        Vec3::splat(DEFAULT_BOUNDS_U / 2.0),
                    ));
                    BoundsDef {
                        min: Vec2::new(lo.x, lo.z),
                        max: Vec2::new(hi.x, hi.z),
                    }
                });
            }
            ("bounds_min" | "bounds_max", value) => {
                let corner = Vec2::from_array(vector(value)?);
                let b = b.as_mut().ok_or("switch bounds on first")?;
                if key == "bounds_min" {
                    b.min = corner;
                } else {
                    b.max = corner;
                }
            }
            (_, other) => return Err(format!("expected on/off, got {other:?}")),
        },
        Slot::Pick(mut p) => match value {
            None => p.set(None)?,
            Some(FieldValue::Choice { value, .. }) => p.set(Some(&value))?,
            Some(other) => return Err(format!("expected a choice, got {other:?}")),
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rc_track::TrackDef;

    use super::*;

    fn oval() -> Doc {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Doc {
            def: TrackDef::load(&assets, "test_oval").unwrap(),
            decor: Vec::new(),
            info: TrackInfo::default(),
        }
    }

    fn choices() -> Choices {
        Choices {
            kinds: vec!["crate_small".into()],
            environments: vec!["toy_store".into()],
        }
    }

    fn get(doc: &Doc, item: EditorItem, key: &str) -> EditorField {
        let (_, fields) = fields(doc, item, &choices(), None).unwrap();
        fields.into_iter().find(|f| f.key == key).unwrap()
    }

    #[test]
    fn inherited_fields_show_what_they_inherit_and_can_be_set_and_unset() {
        let mut doc = oval();
        let point = EditorItem::Point { index: 2 };
        let width = get(&doc, point, "width");
        assert!(width.optional && !width.set);
        assert_eq!(width.value, FieldValue::Number { value: 12.0 }, "point 0's");
        let surface = get(&doc, point, "surface");
        assert!(
            matches!(&surface.value, FieldValue::Choice { value, options } if value == "Asphalt" && options.len() == SURFACES.len())
        );
        let kinds = choices();
        let wood = Some(FieldValue::Choice {
            value: "Wood".into(),
            options: Vec::new(),
        });
        set_field(&mut doc, point, "surface", wood, &kinds, None).unwrap();
        assert_eq!(doc.def.points[2].surface, Some(SurfaceKind::Wood));
        set_field(&mut doc, point, "surface", None, &kinds, None).unwrap();
        assert_eq!(doc.def.points[2].surface, None);
        assert!(set_field(&mut doc, point, "bank_deg", None, &kinds, None).is_err());
    }

    #[test]
    fn choices_reject_unknown_names_and_bounds_switch_on_with_corners() {
        let mut doc = oval();
        let kinds = choices();
        let prop = EditorItem::Prop { index: 0 };
        let bogus = Some(FieldValue::Choice {
            value: "nope".into(),
            options: Vec::new(),
        });
        assert!(set_field(&mut doc, prop, "kind", bogus, &kinds, None).is_err());
        let on = Some(FieldValue::Toggle { value: true });
        set_field(&mut doc, EditorItem::Track, "bounds", on, &kinds, None).unwrap();
        assert!(doc.def.bounds.is_some());
        let corner = Some(FieldValue::Vector {
            value: vec![-40.0, -90.0],
        });
        set_field(
            &mut doc,
            EditorItem::Track,
            "bounds_min",
            corner,
            &kinds,
            None,
        )
        .unwrap();
        assert_eq!(doc.def.bounds.unwrap().min, Vec2::new(-40.0, -90.0));
        assert_eq!(get(&doc, EditorItem::Track, "bounds_max").key, "bounds_max");
    }
}
