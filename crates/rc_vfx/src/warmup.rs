//! Pipeline warm-up (ARCHITECTURE §9: no hitch from Loading into the Intro). Bevy compiles a
//! render pipeline the first time a material/mesh combination is drawn; rc_render's hidden
//! warm-up cameras draw a race's scene in the menu (every view variant, once per run) and under
//! the loading screen until no pipeline is left to compile. These entities add one instance of
//! every effect material to what they draw (particles and sprites, Volt Cube glass, oil and name
//! tags, skid marks, energy shells, item parts, lightning, precipitation), so none of them
//! compiles in the Intro or mid-race. They are never culled, so every 3D camera of the menu and of
//! Loading draws them; they exist in every Loading and, in human runs, in every menu once the
//! showroom is ready (its camera's view is final then: earlier its first frames would compile
//! variants without the environment map; far below the showroom, where rc_render's menu warm-up
//! scene is) and shader assets have stopped arriving. Automated runs skip the menu warm-up
//! (synchronous compilation), so these too.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::shader::Shader;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::fx::sprite;
use crate::particles::Look;
use crate::weather::{PrecipMaterial, WeatherFx};
use crate::{lightning, skids};

/// Scale of the warm-up instances (they only need to be drawn, not seen).
const SCALE: f32 = 0.01;
/// Where the menu's instances sit: rc_render's menu warm-up scene, far below the showroom.
const MENU_AT: Vec3 = Vec3::new(0.0, -600.0, 0.0);
/// The menu's instances wait until no shader asset has arrived for this long (s), as rc_render's
/// menu warm-up does: a shader arriving while compile tasks hold Bevy's shader cache stalls the
/// frame until they are done.
const SHADERS_QUIET_S: f32 = 1.0;

/// Marks the warm-up instances.
#[derive(Component, Debug, Clone, Copy)]
struct WarmupInstance;

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), |c: Commands, a: Params| {
        spawn(c, a, AppState::Loading, Vec3::ZERO)
    })
    .add_systems(
        Update,
        spawn_in_menu.run_if(in_state(AppState::Menu).and_then(resource_exists::<ShowroomReady>)),
    );
}

/// The menu's instances, once per visit, in human runs.
fn spawn_in_menu(
    commands: Commands,
    params: Params,
    automation: Option<Res<AutomationConfig>>,
    existing: Query<(), With<WarmupInstance>>,
    (time, mut shaders, mut quiet): (
        Res<Time<Real>>,
        MessageReader<AssetEvent<Shader>>,
        Local<f32>,
    ),
) {
    *quiet = if shaders.read().count() > 0 {
        0.0
    } else {
        *quiet + time.delta_secs()
    };
    if *quiet >= SHADERS_QUIET_S
        && existing.is_empty()
        && !automation.is_some_and(|a| a.is_automated())
    {
        spawn(commands, params, AppState::Menu, MENU_AT);
    }
}

/// What the warm-up instances are made of.
#[derive(bevy::ecs::system::SystemParam)]
struct Params<'w> {
    assets: Res<'w, VfxAssets>,
    weather: Res<'w, WeatherFx>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    precips: ResMut<'w, Assets<PrecipMaterial>>,
}

/// One never-culled instance of every effect mesh/material pair at `origin`, scoped to `state`.
fn spawn(mut commands: Commands, params: Params, state: AppState, origin: Vec3) {
    let Params {
        assets,
        weather,
        mut meshes,
        mut materials,
        mut precips,
    } = params;
    let at = Transform::from_translation(origin).with_scale(Vec3::splat(SCALE));
    let scope = (
        Name::new("Effects warm-up"),
        WarmupInstance,
        DespawnOnExit(state),
    );
    let pbr = |mesh: &Handle<Mesh>, material: Handle<StandardMaterial>| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            at,
            NoFrustumCulling,
        )
    };
    // Particles, sprites, icons and markers (sprites are never culled anyway).
    commands.spawn((sprite(&assets, &Look::DEFAULT, at), scope.clone()));
    // Item parts (shadowless), Volt Cube glass, oil slicks (and the name tags: same pipeline).
    for (mesh, material) in [
        (&assets.cylinder, &assets.white),
        (&assets.cube, &assets.cube_glass),
        (&assets.decal, &assets.oil),
    ] {
        commands.spawn((pbr(mesh, material.clone()), NotShadowCaster, scope.clone()));
    }
    // The Golden Battery's parts also cast shadows.
    commands.spawn((pbr(&assets.cylinder, assets.gold.clone()), scope.clone()));
    let skid = (
        meshes.add(skids::empty_mesh()),
        materials.add(skids::material()),
    );
    commands.spawn((pbr(&skid.0, skid.1), NotShadowCaster, scope.clone()));
    let bolt = lightning::bolt_mesh(&[(vec![Vec3::ZERO, Vec3::Y], 1.0)], Vec3::Z);
    let bolt = (
        meshes.add(bolt),
        materials.add(lightning::bolt_material(LinearRgba::BLACK)),
    );
    commands.spawn((pbr(&bolt.0, bolt.1), NotShadowCaster, scope.clone()));
    // Shields, EMP shells and the orb's field.
    commands.spawn((
        Mesh3d(assets.sphere.clone()),
        MeshMaterial3d(assets.orb_field.clone()),
        at,
        NotShadowCaster,
        NoFrustumCulling,
        scope.clone(),
    ));
    // Rain and snow (no drops at amount 0).
    commands.spawn((
        Mesh3d(weather.mesh.clone()),
        MeshMaterial3d(precips.add(PrecipMaterial::default())),
        Transform::from_translation(origin),
        NotShadowCaster,
        NoFrustumCulling,
        scope,
    ));
}
