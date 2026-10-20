//! Indoor rooms and environment maps: the generated environment map for image-based lighting
//! (indoors the room — walls, floor, ceiling, bright windows toward the key light, lamp spots at
//! night; outdoors a clear sky, see `sky::sky_radiance`), the ceiling lamp grid over
//! `Track::bounds()` and the window glass in the shell's openings.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use super::glass::{WindowGlass, WindowMaterial};
use super::{ActiveEnvironment, LampGrid, WindowDef, WindowWall, lamp_lumens};
use crate::sky::{cube_image, cubemap_texels, sky_radiance, sun_direction};
use crate::weather_fx::{Look, LookSet};

/// Environment-map face size (px, power of two).
const ENV_MAP_FACE: u32 = 128;
/// Glass sits on the shell wall's centre plane, this far outside `Track::bounds()` (the track
/// visuals' walls are 1 u thick with their inner face on the bounds; frames and mullions occupy
/// the first 0.35 u).
const GLASS_INSET_U: f32 = 0.5;
/// Lamp fixture: disc radius and thickness (u), gap below the ceiling (u).
const FIXTURE_RADIUS_U: f32 = 0.9;
const FIXTURE_DEPTH_U: f32 = 0.25;
const FIXTURE_DROP_U: f32 = 0.3;
/// Inner cone of ceiling spots relative to the outer angle.
const SPOT_INNER: f32 = 0.7;

/// The current environment's map for image-based lighting (its radiance is scaled by the look's
/// `env_light`); outdoors it is regenerated when the cloud cover changes noticeably.
#[derive(Resource, Debug, Clone)]
pub(crate) struct EnvMap {
    pub env: String,
    pub image: Handle<Image>,
    overcast: f32,
}

/// Overcast change that regenerates the outdoor environment map.
const OVERCAST_STEP: f32 = 0.12;

/// The floor plan (min x, min z, max x, max z of the bounds) the session's room lamps and window
/// glass stand in (a track-editor commit can change it: the room is rebuilt then).
#[derive(Resource, Debug, PartialEq)]
struct RoomBuilt([f32; 4]);

/// A ceiling lamp or window pane of the room (despawned by tag when the room is rebuilt).
#[derive(Component, Debug, Clone, Copy)]
struct RoomPiece;

/// A ceiling lamp; `lumens` = its intensity when on.
#[derive(Component, Debug, Clone, Copy)]
struct Lamp {
    lumens: f32,
}

/// Material of the lamp fixtures (emissive toggled with the lamps).
#[derive(Resource, Debug, Clone)]
struct FixtureMaterial {
    handle: Handle<StandardMaterial>,
    strength: f32,
    color: [f32; 3],
}

pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (make_env_map, build_room, switch_lamps.after(LookSet)).in_set(VisualSet),
    )
    .add_systems(OnEnter(AppState::Unloading), |mut commands: Commands| {
        commands.remove_resource::<RoomBuilt>();
        commands.remove_resource::<FixtureMaterial>();
    });
}

/// Clear-weather radiance of the environment map toward `dir` (linear, cd/m²: already × the
/// look's `env_light`): the room or sky Bevy's image-based light shows. The track editor's light
/// baker uses it for rays that leave the scene.
pub fn environment_radiance(env: &super::EnvironmentDef, time: TimeOfDay, dir: Vec3) -> Vec3 {
    let look = env.look(time);
    let toward = sun_direction(look.sun.elevation, look.sun.azimuth);
    let [r, g, b] = if env.indoor {
        room_radiance(&look.room, toward, look.lamps, dir)
    } else {
        let daylight = time != TimeOfDay::Night;
        sky_radiance(dir, toward, env.sky.ground_albedo, daylight, 0.0)
    };
    Vec3::new(r, g, b) * look.env_light
}

/// Radiance of the room toward `dir` (linear; scaled by the look's `env_light`).
pub(crate) fn room_radiance(
    colors: &super::RoomColors,
    toward_key: Vec3,
    lamps: f32,
    dir: Vec3,
) -> [f32; 3] {
    let up = dir.y;
    let lerp3 =
        |a: [f32; 3], b: [f32; 3], t: f32| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
    let smooth = |e0: f32, e1: f32, x: f32| {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let mut c = if up >= 0.0 {
        lerp3(colors.walls, colors.ceiling, smooth(0.15, 0.7, up))
    } else {
        lerp3(colors.walls, colors.floor, smooth(0.0, 0.3, -up))
    };
    // Windows: a band of bright panes around the key light's azimuth, above the sill line.
    let az = |v: Vec3| v.x.atan2(-v.z);
    let mut d_az = az(dir) - az(toward_key);
    d_az = (d_az + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let el = up.clamp(-1.0, 1.0).asin();
    let band = smooth(0.75, 0.55, d_az.abs()) * smooth(0.02, 0.1, el) * smooth(0.62, 0.5, el);
    let mullions = 0.75 + 0.25 * (d_az * 9.0).cos().abs().powf(0.3);
    for (ch, w) in c.iter_mut().zip(colors.window) {
        *ch += w * band * mullions;
    }
    // Lamps: a grid of warm spots on the ceiling.
    if lamps > 0.0 && up > 0.35 {
        let p = Vec2::new(dir.x, dir.z) / up * 1.6;
        let f = Vec2::new(p.x - p.x.round(), p.y - p.y.round()).length();
        let spot = smooth(0.18, 0.05, f) * smooth(0.35, 0.6, up) * lamps;
        for (ch, l) in c.iter_mut().zip(colors.lamp) {
            *ch += l * spot;
        }
    }
    c
}

fn make_env_map(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    weather: Option<Res<WeatherState>>,
    current: Option<Res<EnvMap>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(env) = env else {
        return;
    };
    let overcast = if env.def.indoor {
        0.0
    } else {
        crate::weather_fx::overcast(weather.map_or(0.0, |w| w.cloudiness))
    };
    let fresh = current.as_ref().is_some_and(|m| {
        m.env == env.id && !env.is_changed() && (m.overcast - overcast).abs() < OVERCAST_STEP
    });
    if fresh {
        return;
    }
    let look = env.def.look(env.time);
    let toward = sun_direction(look.sun.elevation, look.sun.azimuth);
    let daylight = env.time != TimeOfDay::Night;
    let ground = env.def.sky.ground_albedo;
    let texels = cubemap_texels(ENV_MAP_FACE, |dir| {
        let [r, g, b] = if env.def.indoor {
            room_radiance(&look.room, toward, look.lamps, dir)
        } else {
            sky_radiance(dir, toward, ground, daylight, overcast)
        };
        [r, g, b, 1.0]
    });
    commands.insert_resource(EnvMap {
        env: env.id.clone(),
        image: images.add(cube_image(ENV_MAP_FACE, &texels)),
        overcast,
    });
}

/// Places lamps and window glass once the session's track (and so the shell) exists, and again
/// when its bounds change.
#[allow(clippy::too_many_arguments)]
fn build_room(
    mut commands: Commands,
    env: Option<Res<ActiveEnvironment>>,
    track: Option<Res<Track>>,
    session: Option<Res<State<RaceSession>>>,
    built: Option<Res<RoomBuilt>>,
    pieces: Query<Entity, With<RoomPiece>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glass: ResMut<Assets<WindowMaterial>>,
) {
    let (Some(env), Some(track), Some(_)) = (env, track, session) else {
        return;
    };
    let (min, max) = track.bounds();
    let plan = [min.x, min.z, max.x, max.z];
    if built.is_some_and(|b| b.0 == plan) {
        return;
    }
    commands.insert_resource(RoomBuilt(plan));
    for entity in &pieces {
        commands.entity(entity).despawn();
    }
    let Some(room) = env.def.room.as_ref() else {
        return;
    };
    if let Some(grid) = &room.lamps {
        spawn_lamps(
            &mut commands,
            grid,
            room.height,
            min,
            max,
            &mut meshes,
            &mut materials,
        );
    }
    let pane = glass.add(WindowMaterial::default());
    let door_pane = glass.add(WindowMaterial::default());
    for window in &room.windows {
        let (transform, size) = window_frame(window, min, max);
        let material = if window.door {
            door_pane.clone()
        } else {
            pane.clone()
        };
        commands.spawn((
            Name::new(if window.door {
                "Door opening"
            } else {
                "Window glass"
            }),
            Mesh3d(meshes.add(Rectangle::new(size.x, size.y))),
            MeshMaterial3d(material),
            transform,
            WindowGlass { door: window.door },
            RoomPiece,
            NotShadowCaster,
            NotShadowReceiver,
            DespawnOnExit(RaceSession),
        ));
    }
    info!(
        "visuals: room {}×{} u, {} windows",
        (max.x - min.x).round(),
        (max.z - min.z).round(),
        room.windows.len()
    );
}

/// Centre transform (facing into the room) and size of a window opening.
pub(crate) fn window_frame(w: &WindowDef, min: Vec3, max: Vec3) -> (Transform, Vec2) {
    let y = w.sill + w.height * 0.5;
    let along = |a: f32, b: f32| a + (b - a) * w.center;
    let (pos, inward) = match w.wall {
        WindowWall::North => (Vec3::new(along(min.x, max.x), y, min.z), Vec3::Z),
        WindowWall::South => (Vec3::new(along(min.x, max.x), y, max.z), Vec3::NEG_Z),
        WindowWall::East => (Vec3::new(max.x, y, along(min.z, max.z)), Vec3::NEG_X),
        WindowWall::West => (Vec3::new(min.x, y, along(min.z, max.z)), Vec3::X),
    };
    // A `Rectangle` faces +Z: turn it to face into the room.
    let transform =
        Transform::from_translation(pos - inward * GLASS_INSET_U).looking_to(-inward, Vec3::Y);
    (transform, Vec2::new(w.width, w.height))
}

/// Lamp positions of a grid over the room (x/z), inset half a spacing from the walls.
pub(crate) fn lamp_positions(spacing: f32, min: Vec3, max: Vec3) -> Vec<Vec2> {
    let axis = |lo: f32, hi: f32| {
        let n = (((hi - lo) / spacing).floor() as usize).max(1);
        let step = (hi - lo) / n as f32;
        (0..n).map(move |i| lo + step * (i as f32 + 0.5))
    };
    axis(min.x, max.x)
        .flat_map(|x| axis(min.z, max.z).map(move |z| Vec2::new(x, z)))
        .collect()
}

fn spawn_lamps(
    commands: &mut Commands,
    grid: &LampGrid,
    height: f32,
    min: Vec3,
    max: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let fixture = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.93, 0.9),
        emissive: LinearRgba::BLACK,
        ..default()
    });
    let mesh = meshes.add(Cylinder::new(FIXTURE_RADIUS_U, FIXTURE_DEPTH_U));
    let lumens = lamp_lumens(grid.lumens);
    let color = Color::srgb(grid.color[0], grid.color[1], grid.color[2]);
    let positions = lamp_positions(grid.spacing, min, max);
    let y = height - FIXTURE_DROP_U;
    for p in &positions {
        let at = Vec3::new(p.x, y, p.y);
        let mut lamp = commands.spawn((
            Name::new("Ceiling lamp"),
            Lamp { lumens },
            RoomPiece,
            Transform::from_translation(at),
            Visibility::default(),
            DespawnOnExit(RaceSession),
            children![(
                Mesh3d(mesh.clone()),
                MeshMaterial3d(fixture.clone()),
                Transform::from_xyz(0.0, FIXTURE_DEPTH_U * 0.5, 0.0),
                NotShadowCaster,
                crate::capture::Overhead,
            )],
        ));
        match grid.spot_angle {
            Some(angle) => {
                let outer = angle.to_radians();
                lamp.insert((
                    SpotLight {
                        color,
                        intensity: 0.0,
                        range: grid.range,
                        outer_angle: outer,
                        inner_angle: outer * SPOT_INNER,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(at).looking_to(Vec3::NEG_Y, Vec3::Z),
                ));
            }
            None => {
                lamp.insert(PointLight {
                    color,
                    intensity: 0.0,
                    range: grid.range,
                    radius: FIXTURE_RADIUS_U * 0.5,
                    shadow_maps_enabled: false,
                    ..default()
                });
            }
        }
    }
    commands.insert_resource(FixtureMaterial {
        handle: fixture,
        strength: grid.fixture,
        color: grid.color,
    });
}

fn switch_lamps(
    look: Option<Res<Look>>,
    fixture: Option<Res<FixtureMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut points: Query<(&Lamp, &mut PointLight)>,
    mut spots: Query<(&Lamp, &mut SpotLight)>,
    fresh: Query<(), Added<Lamp>>,
    mut last: Local<Option<f32>>,
) {
    let Some(look) = look else {
        return;
    };
    let scale = look.lamps;
    if *last == Some(scale) && fresh.is_empty() {
        return;
    }
    *last = Some(scale);
    for (lamp, mut light) in &mut points {
        light.intensity = lamp.lumens * scale;
    }
    for (lamp, mut light) in &mut spots {
        light.intensity = lamp.lumens * scale;
    }
    if let Some(f) = fixture
        && let Some(mut m) = materials.get_mut(&f.handle)
    {
        let s = f.strength * scale;
        m.emissive = LinearRgba::from(Color::srgb(f.color[0], f.color[1], f.color[2])) * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::RoomColors;

    #[test]
    fn windows_are_brightest_toward_the_key_light() {
        let colors = RoomColors::default();
        let key = sun_direction(25.0, 90.0);
        let toward_window = sun_direction(20.0, 90.0);
        let away = sun_direction(20.0, 270.0);
        let w = room_radiance(&colors, key, 0.0, toward_window);
        let a = room_radiance(&colors, key, 0.0, away);
        assert!(w[0] > a[0] * 3.0, "window {w:?} vs back wall {a:?}");
        let floor = room_radiance(&colors, key, 0.0, Vec3::NEG_Y);
        assert!((floor[1] - colors.floor[1]).abs() < 1e-4);
    }

    #[test]
    fn lamps_add_ceiling_spots_only_when_on() {
        let colors = RoomColors::default();
        let key = sun_direction(25.0, 90.0);
        let off = room_radiance(&colors, key, 0.0, Vec3::Y);
        let on = room_radiance(&colors, key, 1.0, Vec3::Y);
        assert!(on[0] > off[0] + 1.0);
    }

    #[test]
    fn lamp_grid_covers_the_room_inside_its_walls() {
        let (min, max) = (Vec3::new(-50.0, -10.0, -30.0), Vec3::new(50.0, 20.0, 30.0));
        let lamps = lamp_positions(25.0, min, max);
        assert_eq!(lamps.len(), 4 * 2);
        assert!(
            lamps
                .iter()
                .all(|p| p.x > min.x && p.x < max.x && p.y > min.z && p.y < max.z)
        );
        assert_eq!(
            lamp_positions(500.0, min, max).len(),
            1,
            "tiny rooms still get a lamp"
        );
    }

    #[test]
    fn window_glass_sits_in_its_wall_facing_inward() {
        let (min, max) = (Vec3::new(-50.0, 0.0, -30.0), Vec3::new(50.0, 20.0, 30.0));
        let w = WindowDef {
            wall: WindowWall::East,
            center: 0.25,
            width: 20.0,
            height: 10.0,
            sill: 4.0,
            door: false,
        };
        let (t, size) = window_frame(&w, min, max);
        assert_eq!(t.translation, Vec3::new(50.0 + GLASS_INSET_U, 9.0, -15.0));
        assert!(
            t.back().dot(Vec3::NEG_X) > 0.999,
            "the rectangle's +Z faces the room"
        );
        assert_eq!(size, Vec2::new(20.0, 10.0));
    }
}
