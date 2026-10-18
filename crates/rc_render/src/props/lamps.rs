//! Lamp props (art bible §3.1 "every visible lamp has a matching real light"): rc_render's
//! `lights` field of a prop kind in `props/*.ron` (rc_track ignores it) becomes real lights at
//! Night, and the prop glb's `lamp*` bulb materials barely glow by day.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::gltf::GltfMaterialName;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use rc_core::prelude::*;
use rc_track::PROPS_DIR;
use serde::Deserialize;

use super::PropModel;
use crate::content::is_night;
use crate::lighting::{ActiveEnvironment, lamp_lumens};

/// Bulb materials (`lamp*`) by day, relative to their authored "on" emission.
const LAMP_DAY: f32 = 0.05;

/// A real light of a lamp prop, switched on at Night (`lights: [...]` of a prop kind).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PropLight {
    /// Position in prop space (u; origin at the ground-contact centre, front −Z).
    pub offset: Vec3,
    /// Colour, linear RGB 0–1.
    #[serde(default = "warm_white")]
    pub color: [f32; 3],
    /// Real-world lumens (scaled by `lamp_lumens`).
    pub lumens: f32,
    /// Reach (u).
    pub range: f32,
    /// Spot cone: full apex angle in degrees (the bright core is 0.7 × it); 0 = point light.
    #[serde(default)]
    pub spot_deg: f32,
    /// Spot direction in prop space.
    #[serde(default = "down")]
    pub dir: Vec3,
    /// Casts shadows (at most 1–2 hero lamps per track).
    #[serde(default)]
    pub shadows: bool,
}

fn warm_white() -> [f32; 3] {
    [1.0, 0.82, 0.6]
}

fn down() -> Vec3 {
    Vec3::NEG_Y
}

/// rc_render's fields of a prop kind (rc_track ignores them).
#[derive(Debug, Clone, Default, Deserialize)]
struct PropVisualDef {
    #[serde(default)]
    lights: Vec<PropLight>,
}

#[derive(Debug, Deserialize)]
struct PropFile {
    props: BTreeMap<String, PropVisualDef>,
}

/// Lamp lights by prop kind, read from every `props/*.ron` when a session loads.
#[derive(Resource, Debug, Default)]
pub(crate) struct PropLamps(HashMap<String, Vec<PropLight>>);

impl PropLamps {
    /// The lights of a prop kind, if it is a lamp.
    pub(crate) fn get(&self, kind: &str) -> Option<&Vec<PropLight>> {
        self.0.get(kind)
    }
}

/// Reads the `lights` of every prop kind; errors name the file and kind.
pub(crate) fn load_lamps(assets_dir: &Path) -> Result<HashMap<String, Vec<PropLight>>, String> {
    let dir = assets_dir.join(PROPS_DIR);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .collect();
    files.sort();
    let options =
        ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
    let mut out = HashMap::new();
    for path in files {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file: PropFile = options
            .from_str(&text)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for (kind, def) in file.props {
            for (i, l) in def.lights.iter().enumerate() {
                let spot_ok = l.spot_deg == 0.0
                    || (l.dir.length() > 0.0 && (0.0..180.0).contains(&l.spot_deg));
                if l.lumens < 0.0 || l.range <= 0.0 || !spot_ok {
                    return Err(format!(
                        "{}: props.{kind}.lights[{i}]: lumens ≥ 0, range > 0, spot_deg 0–180 with a non-zero dir",
                        path.display()
                    ));
                }
            }
            if !def.lights.is_empty() {
                out.insert(kind, def.lights);
            }
        }
    }
    Ok(out)
}

/// `OnEnter(Loading)`: the lamp lights of every prop kind.
pub(super) fn read_lamps(
    paths: Option<Res<UserPaths>>,
    mut lamps: ResMut<PropLamps>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let Some(paths) = paths else {
        return;
    };
    match load_lamps(&paths.assets_dir) {
        Ok(found) => lamps.0 = found,
        Err(error) => {
            error!("props: {error}");
            failed.write(LoadingFailed {
                key: crate::content::LOADING_KEY,
                error,
            });
        }
    }
}

/// A prop lamp's real light under `root`.
pub(super) fn spawn_lamp(commands: &mut Commands, root: Entity, light: &PropLight) {
    let [r, g, b] = light.color;
    let color = Color::linear_rgb(r, g, b);
    let intensity = lamp_lumens(light.lumens);
    let mut at = Transform::from_translation(light.offset);
    let mut e = commands.spawn((Name::new("prop lamp"), ChildOf(root)));
    if light.spot_deg <= 0.0 {
        e.insert((
            PointLight {
                color,
                intensity,
                range: light.range,
                shadow_maps_enabled: light.shadows,
                ..default()
            },
            at,
        ));
        return;
    }
    let up = if light.dir.normalize().y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    at.look_to(light.dir, up);
    let outer = (light.spot_deg * 0.5).to_radians();
    e.insert((
        SpotLight {
            color,
            intensity,
            range: light.range,
            outer_angle: outer,
            inner_angle: outer * 0.7,
            shadow_maps_enabled: light.shadows,
            ..default()
        },
        at,
    ));
}

/// A prop scene is instanced: by day its `lamp*` bulb materials barely glow.
pub(super) fn dim_bulbs(
    ready: On<WorldInstanceReady>,
    roots: Query<(), With<PropModel>>,
    env: Option<Res<ActiveEnvironment>>,
    children: Query<&Children>,
    bulbs: Query<(&GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if roots.get(ready.entity).is_err() || is_night(env.as_deref()) {
        return;
    }
    let mut dimmed: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>> = HashMap::new();
    for entity in children.iter_descendants(ready.entity) {
        let Ok((name, current)) = bulbs.get(entity) else {
            continue;
        };
        if !name.starts_with("lamp") {
            continue;
        }
        let handle = match dimmed.get(&current.id()) {
            Some(h) => h.clone(),
            None => {
                let Some(mut m) = materials.get(&current.0).cloned() else {
                    continue;
                };
                m.emissive *= LAMP_DAY;
                let h = materials.add(m);
                dimmed.insert(current.id(), h.clone());
                h
            }
        };
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    /// The rc_render fields of every shipped prop catalog file parse and validate.
    #[test]
    fn every_prop_file_has_valid_lamps() {
        load_lamps(&assets()).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn lamps_become_point_or_spot_lights_with_world_lumens() {
        let mut world = World::new();
        let root = world.spawn(Transform::default()).id();
        let spot = PropLight {
            offset: Vec3::new(0.0, 29.0, -6.0),
            color: [1.0, 0.7, 0.38],
            lumens: 60_000.0,
            range: 70.0,
            spot_deg: 75.0,
            dir: Vec3::new(0.0, -1.0, -0.15),
            shadows: false,
        };
        let point = PropLight {
            spot_deg: 0.0,
            ..spot.clone()
        };
        let mut commands = world.commands();
        spawn_lamp(&mut commands, root, &spot);
        spawn_lamp(&mut commands, root, &point);
        world.flush();
        let (s, t) = world
            .query::<(&SpotLight, &Transform)>()
            .single(&world)
            .expect("one spot");
        assert!(
            (s.outer_angle - 37.5f32.to_radians()).abs() < 1e-5,
            "full cone 75°"
        );
        assert!((s.intensity - lamp_lumens(60_000.0)).abs() < 1.0);
        assert!(t.forward().y < -0.95, "points down");
        assert_eq!(world.query::<&PointLight>().iter(&world).count(), 1);
    }

    #[test]
    fn lamp_fields_parse_with_defaults_and_bad_ones_fail() {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let text = r#"(props: {
            "street_lamp": (model: "x.glb", colliders: [], lights: [
                (offset: (0, 18, 1), lumens: 1500, range: 40, spot_deg: 55, dir: (0, -1, -0.1), shadows: true),
                (offset: (0, 1, 0), lumens: 200, range: 8),
            ]),
            "crate": (model: "y.glb", colliders: []),
        })"#;
        let file: PropFile = options.from_str(text).expect("parses");
        let lamp = &file.props["street_lamp"].lights;
        assert_eq!(lamp.len(), 2);
        assert_eq!(lamp[0].spot_deg, 55.0);
        assert_eq!(lamp[1].spot_deg, 0.0, "a point light by default");
        assert_eq!(lamp[1].color, warm_white());
        assert!(file.props["crate"].lights.is_empty());
        let typo = r#"(props: {"a": (lights: [(offset: (0, 1, 0), lumen: 5, range: 3)])})"#;
        assert!(
            options.from_str::<PropFile>(typo).is_err(),
            "unknown lamp fields are errors"
        );
    }
}
