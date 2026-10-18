//! Track visuals (ARCHITECTURE §5.9): the session's `TrackGeometry` as meshes — ribbon runs
//! (driving surface, lips or curbs, sides), barriers in the environment's style ([`barriers`])
//! and ground polygons — plus the room shell or the outdoor ground beyond the play area
//! ([`shell`]), all in the environment's
//! `MaterialLibrary` materials (surfaces by `SurfaceKind`, everything else by role name, so each
//! environment file picks its look). Built once per session while `Loading`, session-scoped.

mod barriers;
pub(crate) mod shell;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{MeshData, RibbonStyle, Track};

use crate::content::{ContentLoad, mesh_from_data};
use crate::lighting::ActiveEnvironment;
use crate::materials::{MaterialLibrary, SurfaceMaterial};

/// Ground polygons this far above the lowest one (table tops, shelves) cast shadows.
const RAISED_GROUND_U: f32 = 0.5;

pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        build_track
            .in_set(VisualSet)
            .run_if(in_state(AppState::Loading)),
    );
}

/// Library materials of a ribbon run's `[top, trim, body]`: a moulded toy-track piece is one
/// plastic; road slabs and rug runners take the `trim` (curbs) and `ribbon_side` roles.
fn ribbon_materials(
    style: RibbonStyle,
    surface: SurfaceKind,
    library: &MaterialLibrary,
) -> [Handle<SurfaceMaterial>; 3] {
    let top = library.surface(surface);
    match style {
        RibbonStyle::ToyTrack => {
            let plastic = library.surface(SurfaceKind::ToyTrack);
            [top, plastic.clone(), plastic]
        }
        _ => {
            let role = |name: &str| library.named(name).unwrap_or_else(|| top.clone());
            [top.clone(), role("trim"), role("ribbon_side")]
        }
    }
}

/// `Loading`: once the track, the environment and its material library are ready, spawn the
/// track's meshes under one session-scoped root.
fn build_track(
    mut commands: Commands,
    mut load: ResMut<ContentLoad>,
    track: Option<Res<Track>>,
    env: Option<Res<ActiveEnvironment>>,
    library: Res<MaterialLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(track), Some(env)) = (track, env) else {
        return;
    };
    if load.track || library.environment() != Some(env.id.as_str()) {
        return;
    }
    let root = commands
        .spawn((
            Name::new("track visuals"),
            Transform::IDENTITY,
            Visibility::default(),
            DespawnOnExit(RaceSession),
        ))
        .id();
    let mut piece =
        |data: &MeshData, material: Handle<SurfaceMaterial>, casts: bool, name: &'static str| {
            let Some(mesh) = mesh_from_data(data) else {
                return;
            };
            let mut e = commands.spawn((
                Name::new(name),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                ChildOf(root),
            ));
            if !casts {
                e.insert(NotShadowCaster);
            }
        };
    let geo = track.geometry();
    for run in &geo.ribbons {
        let [top, trim, body] = ribbon_materials(track.style(), run.surface, &library);
        piece(&run.top, top, true, "ribbon top");
        piece(&run.trim, trim, true, "ribbon trim");
        piece(&run.body, body, true, "ribbon body");
    }
    let barrier = library.named("barrier").unwrap_or_default();
    match barriers::block_style(&env.id) {
        Some(style) => {
            for (mesh, name) in barriers::block_meshes(&track, &style)
                .iter()
                .zip(style.materials)
            {
                let material = library.named(name).unwrap_or_else(|| barrier.clone());
                piece(mesh, material, true, "barrier blocks");
            }
        }
        None => {
            for wall in &geo.walls {
                piece(wall, barrier.clone(), true, "barrier");
            }
        }
    }
    let floor = track
        .ground()
        .iter()
        .map(|g| g.height)
        .fold(f32::INFINITY, f32::min);
    for (g, area) in geo.ground.iter().zip(track.ground()) {
        let raised = area.height > floor + RAISED_GROUND_U;
        piece(&g.mesh, library.surface(g.surface), raised, "ground");
    }
    shell::spawn(&mut commands, root, &track, &env, &library, &mut meshes);
    let tris = geo
        .ribbons
        .iter()
        .map(|r| r.top.triangle_count() + r.trim.triangle_count() + r.body.triangle_count())
        .sum::<usize>()
        + geo
            .walls
            .iter()
            .map(MeshData::triangle_count)
            .sum::<usize>();
    info!(
        "track visuals: {} ribbon runs, {} barriers, {} ground areas ({} k track triangles), {} environment",
        geo.ribbons.len(),
        geo.walls.len(),
        geo.ground.len(),
        tris / 1000,
        env.id
    );
    load.track = true;
}
