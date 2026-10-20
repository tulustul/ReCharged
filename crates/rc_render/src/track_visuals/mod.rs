//! Track visuals (ARCHITECTURE §5.9): the session's `TrackGeometry` as meshes — ribbon runs
//! (driving surface, lips or curbs, sides), barriers in the environment's style ([`barriers`]),
//! way-finding marks ([`guides`]: edge tape, corner arrows, chevron boards, crosses) and ground
//! polygons — plus the room shell or the outdoor ground beyond the play area
//! ([`shell`]), all in the environment's
//! `MaterialLibrary` materials (surfaces by `SurfaceKind`, everything else by role name, so each
//! environment file picks its look). Built while `Loading`, session-scoped; in a race they are
//! rebuilt with the same code when the [`MeshKey`] of the track they show changes: the track
//! editor's `TrackPreview` while a control point is dragged (else `Track`), or a replaced `Track`.

mod barriers;
mod guides;
pub(crate) mod shell;

use std::time::Instant;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{MeshData, RibbonStyle, Track, TrackKey, TrackPreview};

use crate::content::{ContentLoad, mesh_with_tangents};
use crate::lighting::ActiveEnvironment;
use crate::materials::{MaterialLibrary, SurfaceMaterial};

/// Ground polygons this far above the lowest one (table tops, shelves) cast shadows.
const RAISED_GROUND_U: f32 = 0.5;

pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (
            build_track.run_if(in_state(AppState::Loading)),
            follow_track.run_if(in_state(AppState::InRace)),
        )
            .in_set(VisualSet),
    )
    .add_systems(OnEnter(AppState::Unloading), |mut commands: Commands| {
        commands.remove_resource::<BuiltMeshes>();
    });
}

/// What the track meshes are built from: the layout's [`TrackKey`] and its way-finding marks
/// (boards and crosses step aside for props, so a prop edit can move them).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MeshKey {
    pub track: TrackKey,
    pub guides: guides::Guides,
}

impl MeshKey {
    pub(crate) fn of(track: &Track) -> Self {
        Self {
            track: track.key().clone(),
            guides: guides::guides(track),
        }
    }
}

/// The key the session's track meshes were built from; `preview`: from the editor's
/// `TrackPreview`, with quick tangents (rebuilt exactly once the session's `Track` shows again).
#[derive(Resource, Debug)]
struct BuiltMeshes {
    key: MeshKey,
    preview: bool,
}

/// The root of the session's track meshes.
#[derive(Component, Debug)]
struct TrackMeshes;

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
    let key = MeshKey::of(&track);
    let what = spawn_meshes(
        &mut commands,
        &track,
        &key,
        true,
        &env,
        &library,
        &mut meshes,
    );
    commands.insert_resource(BuiltMeshes {
        key,
        preview: false,
    });
    info!("track visuals: {what}");
    load.track = true;
}

/// `InRace`: rebuild the meshes when the track they should show (the editor's preview, else the
/// session's) has another [`MeshKey`]. The game never changes either in a race: one comparison
/// per change, none per frame.
#[allow(clippy::too_many_arguments)]
fn follow_track(
    mut commands: Commands,
    track: Option<Res<Track>>,
    preview: Option<Res<TrackPreview>>,
    built: Option<ResMut<BuiltMeshes>>,
    roots: Query<Entity, With<TrackMeshes>>,
    env: Option<Res<ActiveEnvironment>>,
    library: Res<MaterialLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut previewed: Local<bool>,
) {
    let (Some(track), Some(mut built), Some(env)) = (track, built, env) else {
        return;
    };
    let changed = track.is_changed()
        || preview.as_ref().is_some_and(|p| p.is_changed())
        || *previewed != preview.is_some();
    *previewed = preview.is_some();
    // `built.is_added()`: the session's first frame in the race, built from this very track.
    if !changed || built.is_added() {
        return;
    }
    let started = Instant::now();
    let shown = preview.as_deref().map_or(&*track, |p| &p.track);
    let key = MeshKey::of(shown);
    if built.key == key && built.preview == preview.is_some() {
        debug!(
            "track visuals: unchanged ({:.1} ms to check)",
            started.elapsed().as_secs_f32() * 1000.0
        );
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let exact = preview.is_none();
    let _ = spawn_meshes(
        &mut commands,
        shown,
        &key,
        exact,
        &env,
        &library,
        &mut meshes,
    );
    *built = BuiltMeshes {
        key,
        preview: !exact,
    };
    let ms = started.elapsed().as_secs_f32() * 1000.0;
    match exact {
        false => debug!("track visuals: preview rebuilt in {ms:.1} ms"),
        true => info!("track visuals: rebuilt in place in {ms:.1} ms"),
    }
}

/// The track's meshes (ribbons, barriers, guides, ground, shell) under one session-scoped root;
/// `exact`: mikktspace tangents (else quick ones, for previews). Returns what was built (for the
/// log).
fn spawn_meshes(
    commands: &mut Commands,
    track: &Track,
    key: &MeshKey,
    exact: bool,
    env: &ActiveEnvironment,
    library: &MaterialLibrary,
    meshes: &mut Assets<Mesh>,
) -> String {
    let root = commands
        .spawn((
            Name::new("track visuals"),
            TrackMeshes,
            Transform::IDENTITY,
            Visibility::default(),
            DespawnOnExit(RaceSession),
        ))
        .id();
    let mut piece =
        |data: &MeshData, material: Handle<SurfaceMaterial>, casts: bool, name: &'static str| {
            let Some(mesh) = mesh_with_tangents(data, exact) else {
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
        let [top, trim, body] = ribbon_materials(track.style(), run.surface, library);
        piece(&run.top, top, true, "ribbon top");
        piece(&run.trim, trim, true, "ribbon trim");
        piece(&run.body, body, true, "ribbon body");
    }
    let barrier = library.named("barrier").unwrap_or_default();
    match barriers::block_style(&env.id) {
        Some(style) => {
            for (mesh, name) in barriers::block_meshes(track, &style)
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
    let marks = &key.guides;
    for (mesh, role, casts, name) in [
        (&marks.tape, "guide_tape", false, "guide tape"),
        (&marks.boards, "guide_board", true, "guide boards"),
        (&marks.chevrons, "guide_chevron", false, "guide chevrons"),
    ] {
        let material = library.named(role).unwrap_or_else(|| barrier.clone());
        piece(mesh, material, casts, name);
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
    shell::spawn(commands, root, track, env, library, meshes);
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
    format!(
        "{} ribbon runs, {} barriers, {} ground areas ({} k track triangles), {} environment",
        geo.ribbons.len(),
        geo.walls.len(),
        geo.ground.len(),
        tris / 1000,
        env.id
    )
}
