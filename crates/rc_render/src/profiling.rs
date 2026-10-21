//! Diagnostics for rc_ui's F12 profiler: Bevy's frame-time, entity-count and per-pass render
//! diagnostics, plus the `rc_core::perf` ones — the main world's CPU time, the entities the cameras draw,
//! the GPU time of the whole frame (a `frame` span around the render graph, so passes without
//! their own span count too) and the draw calls of camera and shadow views. GPU times need
//! timestamp queries (Vulkan, DX12).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::camera::visibility::{VisibilitySystems, VisibleEntities};
use bevy::core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy::core_pipeline::deferred::{AlphaMask3dDeferred, Opaque3dDeferred};
use bevy::core_pipeline::prepass::{AlphaMask3dPrepass, Opaque3dPrepass};
use bevy::diagnostic::{
    Diagnostic, Diagnostics, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    RegisterDiagnostic,
};
use bevy::pbr::{Shadow, Transmissive3d};
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::render::diagnostic::{
    DiagnosticsRecorder, RecordDiagnostics, RenderDiagnosticsPlugin, resolve_encoder,
};
use bevy::render::render_phase::{
    BinnedPhaseItem, SortedPhaseItem, ViewBinnedRenderPhases, ViewSortedRenderPhases,
};
use bevy::render::render_resource::{CommandEncoder, CommandEncoderDescriptor};
use bevy::render::renderer::{
    PendingCommandBuffers, RenderDevice, RenderGraph, RenderGraphSystems,
};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::ui_render::TransparentUi;
use rc_core::perf::{CPU_FRAME_MS, DRAW_CALLS, SHADOW_DRAW_CALLS, VISIBLE_ENTITIES};

/// Draw calls, indexed by [`VIEWS`] / [`SHADOWS`]: the render world counts the frame it renders
/// into the first pair and publishes it to the second, which the main world reads.
#[derive(Resource, Clone, Default)]
struct DrawCalls(Arc<[AtomicU32; 4]>);

const VIEWS: usize = 0;
const SHADOWS: usize = 1;

/// When the main world's update began.
#[derive(Resource, Default)]
struct FrameStart(Option<Instant>);

pub(crate) fn build(app: &mut App) {
    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        RenderDiagnosticsPlugin,
    ))
    .init_resource::<FrameStart>()
    .init_resource::<DrawCalls>()
    .register_diagnostic(Diagnostic::new(CPU_FRAME_MS).with_suffix("ms"))
    .register_diagnostic(Diagnostic::new(VISIBLE_ENTITIES))
    .register_diagnostic(Diagnostic::new(DRAW_CALLS))
    .register_diagnostic(Diagnostic::new(SHADOW_DRAW_CALLS))
    .add_systems(
        First,
        (|mut start: ResMut<FrameStart>| start.0 = Some(Instant::now()))
            .before(bevy::time::TimeSystems),
    )
    .add_systems(Last, (measure_cpu, measure_draws))
    .add_systems(
        PostUpdate,
        count_visible.after(VisibilitySystems::CheckVisibility),
    );
    let draws = app.world().resource::<DrawCalls>().clone();
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        // After rendering, before `Cleanup` empties the sorted phases.
        render_app.insert_resource(draws).add_systems(
            Render,
            (
                (
                    count_binned::<Opaque3d, VIEWS>,
                    count_binned::<AlphaMask3d, VIEWS>,
                    count_binned::<Opaque3dPrepass, VIEWS>,
                    count_binned::<AlphaMask3dPrepass, VIEWS>,
                    count_binned::<Opaque3dDeferred, VIEWS>,
                    count_binned::<AlphaMask3dDeferred, VIEWS>,
                    count_binned::<Opaque2d, VIEWS>,
                    count_binned::<AlphaMask2d, VIEWS>,
                    count_binned::<Shadow, SHADOWS>,
                    count_sorted::<Transmissive3d>,
                    count_sorted::<Transparent3d>,
                    count_sorted::<Transparent2d>,
                    count_sorted::<TransparentUi>,
                ),
                publish_draws,
            )
                .chain()
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
        render_app.add_systems(
            RenderGraph,
            (
                begin_gpu_frame.in_set(RenderGraphSystems::Begin),
                end_gpu_frame
                    .after(RenderGraphSystems::Render)
                    .before(resolve_encoder),
            ),
        );
    }
}

fn measure_cpu(start: Res<FrameStart>, mut diagnostics: Diagnostics) {
    if let Some(start) = start.0 {
        diagnostics.add_measurement(&CPU_FRAME_MS, || start.elapsed().as_secs_f64() * 1000.0);
    }
}

fn measure_draws(draws: Res<DrawCalls>, mut diagnostics: Diagnostics) {
    let published = |i: usize| f64::from(draws.0[2 + i].load(Ordering::Relaxed));
    diagnostics.add_measurement(&DRAW_CALLS, || published(VIEWS));
    diagnostics.add_measurement(&SHADOW_DRAW_CALLS, || published(SHADOWS));
}

/// One draw call per multidraw batch set (multi-draw-indirect: its bins are indirect commands the
/// GPU runs, which Bevy keeps private), batchable bin, unbatchable entity and custom item.
fn count_binned<P: BinnedPhaseItem, const SLOT: usize>(
    phases: Option<Res<ViewBinnedRenderPhases<P>>>,
    draws: Res<DrawCalls>,
) {
    let Some(phases) = phases else { return };
    let n: usize = phases
        .values()
        .map(|phase| {
            phase.multidrawable_meshes.len()
                + phase.batchable_meshes.len()
                + phase
                    .unbatchable_meshes
                    .values()
                    .map(|b| b.entities.len())
                    .sum::<usize>()
                + phase
                    .non_mesh_items
                    .values()
                    .map(|b| b.entities.len())
                    .sum::<usize>()
        })
        .sum();
    draws.0[SLOT].fetch_add(n as u32, Ordering::Relaxed);
}

/// One draw call per batch: batching leaves the merged items with an empty range.
fn count_sorted<P: SortedPhaseItem>(
    phases: Option<Res<ViewSortedRenderPhases<P>>>,
    draws: Res<DrawCalls>,
) {
    let Some(phases) = phases else { return };
    let n: usize = phases
        .values()
        .map(|phase| {
            phase
                .items
                .values()
                .filter(|item| !item.batch_range().is_empty())
                .count()
        })
        .sum();
    draws.0[VIEWS].fetch_add(n as u32, Ordering::Relaxed);
}

fn publish_draws(draws: Res<DrawCalls>) {
    for slot in [VIEWS, SHADOWS] {
        let n = draws.0[slot].swap(0, Ordering::Relaxed);
        draws.0[2 + slot].store(n, Ordering::Relaxed);
    }
}

// Cameras only: `ViewVisibility` also counts shadow views, and Bevy marks every entity without
// bounds (bare hierarchy nodes) visible.
fn count_visible(cameras: Query<(&Camera, &VisibleEntities)>, mut diagnostics: Diagnostics) {
    diagnostics.add_measurement(&VISIBLE_ENTITIES, || {
        cameras
            .iter()
            .filter(|(camera, _)| camera.is_active)
            .flat_map(|(_, visible)| visible.entities.values())
            .map(Vec::len)
            .sum::<usize>() as f64
    });
}

// The span stays open across systems only through the recorder's hidden begin/end methods (Bevy's
// camera driver opens `auxiliary_views` the same way). Spans are tracked per thread: both ends are
// exclusive systems, which run on the render thread.
fn begin_gpu_frame(world: &mut World) {
    gpu_frame_span(world, |recorder, encoder| {
        recorder.begin_time_span(encoder, "frame".into());
    });
}

fn end_gpu_frame(world: &mut World) {
    gpu_frame_span(world, |recorder, encoder| recorder.end_time_span(encoder));
}

fn gpu_frame_span(
    world: &mut World,
    write: impl FnOnce(&DiagnosticsRecorder, &mut CommandEncoder),
) {
    let Some(recorder) = world.get_resource::<DiagnosticsRecorder>() else {
        return;
    };
    let mut encoder =
        world
            .resource::<RenderDevice>()
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("gpu_frame_span"),
            });
    write(recorder, &mut encoder);
    world
        .resource_mut::<PendingCommandBuffers>()
        .push_encoder(encoder, "gpu_frame_span");
}
