//! Baking in the editor: a queue of (layout, time of day) jobs. Each reloads the session in that
//! layout and time in clear weather, lets the look settle, gathers the scene ([`gather`]), traces
//! it on a background thread ([`crate::bake`]) and writes the volume rc_render loads. Editing waits
//! meanwhile; afterwards the editing session comes back (with its new volume).

use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy::asset::UntypedAssetId;
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_render::{
    ActiveEnvironment, IrradianceGrid, SurfaceMaterial, environment_radiance, volume_file,
};
use rc_track::Track;

use crate::bake::{GridSpec, Lamp, Scene, SceneBuilder, Sun, bake};
use crate::{Editor, Mode};

/// Rays per probe and pass.
const RAYS: usize = 512;
/// Frames the session runs before it is gathered (lights and look settle).
const SETTLE_FRAMES: u32 = 30;
/// About this many probe columns (x × z) at most: big outdoor tracks get a coarser grid.
const MAX_COLUMNS: f32 = 12_000.0;
/// Finest spacing (u): horizontal, and the vertical range.
const MIN_SPACING_U: f32 = 2.0;
const VERTICAL_U: (f32, f32) = (1.5, 4.0);
/// Layers of probes aimed at (vertical spacing = height / this, within [`VERTICAL_U`]).
const LAYERS: f32 = 8.0;
/// Outdoors the volume reaches this far above the highest drivable point (u).
const OUTDOOR_HEADROOM_U: f32 = 12.0;

/// One volume to bake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BakeJob {
    pub mirror: bool,
    pub time: TimeOfDay,
}

/// A running bake queue.
#[derive(Debug)]
pub(crate) struct BakeRun {
    jobs: VecDeque<BakeJob>,
    total: usize,
    settle: u32,
    tracing: Option<Tracing>,
    /// Quit when done (`--bake`).
    pub exit: bool,
    /// Progress line for the status.
    pub status: String,
}

#[derive(Debug)]
struct Tracing {
    thread: JoinHandle<IrradianceGrid>,
    progress: Arc<AtomicUsize>,
    probes: usize,
    started: Instant,
}

impl BakeRun {
    /// Every allowed time of day, in both layouts (`all`), or just `time` forward.
    pub(crate) fn new(times: &[TimeOfDay], time: TimeOfDay, all: bool, exit: bool) -> Self {
        let jobs: VecDeque<BakeJob> = if all {
            [false, true]
                .into_iter()
                .flat_map(|mirror| times.iter().map(move |&time| BakeJob { mirror, time }))
                .collect()
        } else {
            [BakeJob {
                mirror: false,
                time,
            }]
            .into()
        };
        Self {
            total: jobs.len(),
            jobs,
            settle: 0,
            tracing: None,
            exit,
            status: String::new(),
        }
    }

    /// The job whose session should be running.
    pub(crate) fn current(&self) -> Option<BakeJob> {
        self.jobs.front().copied()
    }
}

/// Where the probes of a session go: the track's bounds; indoors floor to ceiling (no seam on tall
/// walls), outdoors up to the highest drivable point plus headroom.
pub(crate) fn grid_for(track: &Track, ceiling: Option<f32>) -> GridSpec {
    let (lo, hi) = track.bounds();
    let mut heights: Vec<f32> = (0..(track.length() / 2.0) as usize)
        .map(|i| track.frame_at(i as f32 * 2.0).position.y)
        .collect();
    heights.extend(track.ground().iter().map(|g| g.height));
    let bottom = heights.iter().copied().fold(0.0, f32::min) - 1.0;
    let highest = heights.iter().copied().fold(0.0, f32::max);
    let top = ceiling.unwrap_or(highest + OUTDOOR_HEADROOM_U);
    let area = (hi.x - lo.x) * (hi.z - lo.z);
    let horizontal = (area / MAX_COLUMNS).sqrt().max(MIN_SPACING_U);
    let vertical = ((top - bottom) / LAYERS).clamp(VERTICAL_U.0, VERTICAL_U.1);
    GridSpec::new(
        Vec3::new(lo.x, bottom, lo.z),
        Vec3::new(hi.x, top, hi.z),
        horizontal,
        vertical,
    )
}

/// What [`gather`] reads.
#[derive(SystemParam)]
pub(crate) struct SceneParams<'w, 's> {
    meshes: Query<
        'w,
        's,
        (
            &'static Mesh3d,
            &'static MeshMaterial3d<StandardMaterial>,
            &'static GlobalTransform,
        ),
    >,
    /// Ground, ribbons, walls and the room shell (a `StandardMaterial` with weather on top).
    surfaces: Query<
        'w,
        's,
        (
            &'static Mesh3d,
            &'static MeshMaterial3d<SurfaceMaterial>,
            &'static GlobalTransform,
        ),
    >,
    mesh_assets: Res<'w, Assets<Mesh>>,
    materials: Res<'w, Assets<StandardMaterial>>,
    surface_materials: Res<'w, Assets<SurfaceMaterial>>,
    images: Res<'w, Assets<Image>>,
    suns: Query<'w, 's, (&'static DirectionalLight, &'static GlobalTransform)>,
    points: Query<'w, 's, (&'static PointLight, &'static GlobalTransform)>,
    spots: Query<'w, 's, (&'static SpotLight, &'static GlobalTransform)>,
}

fn linear(c: Color) -> Vec3 {
    let l = c.to_linear();
    Vec3::new(l.red, l.green, l.blue)
}

/// The mean linear colour of a texture (8 × 8 samples); `None` without CPU data.
fn texture_mean(image: &Image) -> Option<Vec3> {
    let size = image.size();
    if size.x == 0 || size.y == 0 {
        return None;
    }
    let mut sum = Vec3::ZERO;
    for j in 0..8 {
        for i in 0..8 {
            let x = (size.x * (2 * i + 1)) / 16;
            let y = (size.y * (2 * j + 1)) / 16;
            sum += linear(image.get_color_at(x, y).ok()?);
        }
    }
    Some(sum / 64.0)
}

/// The session's opaque meshes (standard and surface materials), materials and lights as a
/// [`Scene`] (environment map radiance of `env` for rays that leave it).
pub(crate) fn gather(p: &SceneParams, env: &ActiveEnvironment) -> Result<(Scene, String), String> {
    let mut builder = SceneBuilder::default();
    let mut materials: HashMap<UntypedAssetId, Option<u32>> = HashMap::new();
    let mut textures: HashMap<AssetId<Image>, Option<Vec3>> = HashMap::new();
    let (mut skipped, mut no_cpu, mut flat) = (0usize, 0usize, 0usize);
    // (mesh, material key, its StandardMaterial part, pose) of both material kinds.
    let standard = p.meshes.iter().map(|(mesh, material, transform)| {
        let id = material.id();
        (mesh, id.untyped(), p.materials.get(id), transform)
    });
    let surface = p.surfaces.iter().map(|(mesh, material, transform)| {
        let id = material.id();
        let base = p.surface_materials.get(id).map(|m| &m.base);
        (mesh, id.untyped(), base, transform)
    });
    for (mesh, id, material, transform) in standard.chain(surface) {
        let index = *materials.entry(id).or_insert_with(|| {
            let m = material?;
            if !matches!(m.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)) {
                return None; // glass, clouds, decals: light passes
            }
            let texture = m.base_color_texture.as_ref().map(|t| {
                *textures
                    .entry(t.id())
                    .or_insert_with(|| p.images.get(t.id()).and_then(texture_mean))
            });
            if matches!(texture, Some(None)) {
                flat += 1;
            }
            let tint = texture.flatten().unwrap_or(Vec3::splat(0.6));
            Some(builder.material(linear(m.base_color) * tint, m.double_sided))
        });
        let Some(index) = index else {
            skipped += 1;
            continue;
        };
        let Some(mesh) = p.mesh_assets.get(&mesh.0) else {
            no_cpu += 1;
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            no_cpu += 1;
            continue;
        };
        let affine = transform.affine();
        let world: Vec<Vec3> = positions
            .iter()
            .map(|&q| affine.transform_point3(Vec3::from(q)))
            .collect();
        let flipped = affine.matrix3.determinant() < 0.0;
        let mut emit = |[a, b, c]: [usize; 3]| {
            let (a, b, c) = (world[a], world[b], world[c]);
            builder.triangle(if flipped { [a, c, b] } else { [a, b, c] }, index);
        };
        match mesh.indices() {
            Some(Indices::U16(i)) => i
                .as_chunks::<3>()
                .0
                .iter()
                .for_each(|t| emit(t.map(usize::from))),
            Some(Indices::U32(i)) => i
                .as_chunks::<3>()
                .0
                .iter()
                .for_each(|t| emit(t.map(|v| v as usize))),
            None => (0..world.len() / 3).for_each(|t| emit([3 * t, 3 * t + 1, 3 * t + 2])),
        }
    }
    if builder.triangles() == 0 {
        return Err(format!(
            "no triangles: {} meshes, {skipped} not opaque, {no_cpu} without a CPU copy (rc_render::keep_cpu_copies)",
            p.meshes.iter().count() + p.surfaces.iter().count()
        ));
    }
    let suns = p
        .suns
        .iter()
        .filter(|(l, _)| l.illuminance > 0.0)
        .map(|(l, t)| Sun {
            toward: -t.forward().as_vec3(),
            irradiance: linear(l.color) * l.illuminance,
            shadows: l.shadow_maps_enabled,
        })
        .collect();
    let lamp = |color: Color, lumens: f32, range: f32, pos: Vec3, shadows: bool| Lamp {
        pos,
        candela: linear(color) * lumens / (4.0 * PI),
        inverse_range_squared: 1.0 / (range * range).max(1e-6),
        spot: None,
        shadows,
    };
    let mut lamps: Vec<Lamp> = p
        .points
        .iter()
        .filter(|(l, _)| l.intensity > 0.0)
        .map(|(l, t)| {
            lamp(
                l.color,
                l.intensity,
                l.range,
                t.translation(),
                l.shadow_maps_enabled,
            )
        })
        .collect();
    lamps.extend(
        p.spots
            .iter()
            .filter(|(l, _)| l.intensity > 0.0)
            .map(|(l, t)| {
                let cos_outer = l.outer_angle.cos();
                let scale = 1.0 / (l.inner_angle.cos() - cos_outer).max(1e-4);
                Lamp {
                    spot: Some((t.forward().as_vec3(), scale, -cos_outer * scale)),
                    ..lamp(
                        l.color,
                        l.intensity,
                        l.range,
                        t.translation(),
                        l.shadow_maps_enabled,
                    )
                }
            }),
    );
    let note = format!(
        "{} triangles, {} materials ({flat} textures without CPU data), {} suns, {} lamps; {skipped} transparent and {no_cpu} GPU-only meshes left out",
        builder.triangles(),
        materials.values().flatten().count(),
        p.suns.iter().filter(|(l, _)| l.illuminance > 0.0).count(),
        lamps.len(),
    );
    let def = env.def.clone();
    let time = env.time;
    let scene = builder.build(suns, lamps, |dir| environment_radiance(&def, time, dir))?;
    Ok((scene, note))
}

/// What [`run`] reads besides the editor.
#[derive(SystemParam)]
pub(crate) struct BakeSession<'w, 's> {
    state: Res<'w, State<AppState>>,
    config: Option<Res<'w, RaceConfig>>,
    track: Option<Res<'w, Track>>,
    env: Option<Res<'w, ActiveEnvironment>>,
    paths: Res<'w, UserPaths>,
    scene: SceneParams<'w, 's>,
}

/// Drives the bake queue: wait for each job's session, gather, trace, write; restore editing.
pub(crate) fn run(
    mut editor: ResMut<Editor>,
    session: BakeSession,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut run) = editor.bake.take() else {
        return;
    };
    let finished = step(&mut run, &mut editor, &session);
    if finished {
        info!("editor: baking done");
        editor.bake_done();
        if run.exit {
            exit.write(AppExit::Success);
        }
    } else {
        editor.bake = Some(run);
    }
}

/// One frame of the queue; true when every job is done.
fn step(run: &mut BakeRun, editor: &mut Editor, s: &BakeSession) -> bool {
    let Some(job) = run.current() else {
        return true;
    };
    let index = run.total - run.jobs.len() + 1;
    let label = format!(
        "{:?}{} ({index}/{})",
        job.time,
        if job.mirror { " · mirror" } else { "" },
        run.total
    );
    if let Some(tracing) = &run.tracing {
        let done = tracing.progress.load(Ordering::Relaxed);
        let percent = done * 100 / (2 * tracing.probes).max(1);
        let status = format!("Baking {label}: {percent} %");
        if run.status != status {
            run.status = status;
        }
        if !tracing.thread.is_finished() {
            return false;
        }
        let tracing = run.tracing.take().expect("checked");
        let seconds = tracing.started.elapsed().as_secs_f32();
        let grid = tracing.thread.join().expect("bake thread");
        let path = volume_file(
            &s.paths.assets_dir,
            editor.track.as_str(),
            job.mirror,
            job.time,
        );
        match write(&path, &grid.to_bytes()) {
            Ok(()) => info!("editor: baked {} in {seconds:.1} s", path.display()),
            Err(e) => {
                error!("editor: {e}");
                editor.set_error(e);
            }
        }
        run.jobs.pop_front();
        run.settle = 0;
        editor.bake_next();
        return run.jobs.is_empty();
    }
    run.status = format!("Baking {label}: loading");
    let ready = *s.state.get() == AppState::InRace
        && editor.session_current()
        && s.config
            .as_ref()
            .is_some_and(|c| c.variant.mirror == job.mirror)
        && s.env.as_ref().is_some_and(|e| e.time == job.time);
    let (Some(track), Some(env)) = (s.track.as_deref(), s.env.as_deref()) else {
        return false;
    };
    if !ready {
        return false;
    }
    run.settle += 1;
    if run.settle < SETTLE_FRAMES {
        return false;
    }
    let ceiling = env.def.room.as_ref().map(|r| r.height);
    let spec = grid_for(track, ceiling);
    let (scene, note) = match gather(&s.scene, env) {
        Ok(gathered) => gathered,
        Err(e) => {
            error!("editor: bake: {e}");
            editor.set_error(format!("bake: {e}"));
            return true;
        }
    };
    info!(
        "editor: baking {label}: {} probes ({}), {note}",
        spec.count(),
        spec.dims
    );
    let progress = Arc::new(AtomicUsize::new(0));
    let counter = progress.clone();
    run.tracing = Some(Tracing {
        thread: std::thread::spawn(move || bake(&scene, &spec, RAYS, &counter)),
        progress,
        probes: spec.count(),
        started: Instant::now(),
    });
    false
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(fail)?;
    }
    let tmp = path.with_extension("irv.tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(fail)
}

/// `--bake`: bake everything once the editing session is up, then quit.
#[derive(Resource, Debug)]
pub(crate) struct BakeOnStart;

pub(crate) fn bake_on_start(
    mut commands: Commands,
    state: Res<State<AppState>>,
    mut editor: ResMut<Editor>,
) {
    if *state.get() == AppState::InRace && editor.mode == Mode::Editing && editor.session_current()
    {
        commands.remove_resource::<BakeOnStart>();
        editor.start_bake(true, true);
    }
}
