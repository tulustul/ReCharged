//! Pipeline warm-up (ARCHITECTURE §9: no hitch from Loading into the Intro). Bevy compiles a
//! render pipeline the first time a material/mesh combination is drawn; rc_render's hidden
//! warm-up camera draws the session's scene under the loading screen until no pipeline is left to
//! compile. These entities add one instance of every effect material to it (particles and
//! sprites, Volt Cube glass, oil and name tags, skid marks, energy shells, item parts, lightning,
//! precipitation), so none of them compiles in the Intro or mid-race. They are never culled, so
//! whichever 3D camera renders during Loading draws them, and they go with the loading screen.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::assets::VfxAssets;
use crate::fx::sprite;
use crate::particles::Look;
use crate::weather::{PrecipMaterial, WeatherFx};
use crate::{lightning, skids};

/// Scale of the warm-up instances (they only need to be drawn, not seen).
const SCALE: f32 = 0.01;

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), spawn);
}

/// `OnEnter(Loading)`: one never-culled instance of every effect mesh/material pair.
fn spawn(
    mut commands: Commands,
    assets: Res<VfxAssets>,
    weather: Res<WeatherFx>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut precips: ResMut<Assets<PrecipMaterial>>,
) {
    let at = Transform::from_scale(Vec3::splat(SCALE));
    let scope = (
        Name::new("Effects warm-up"),
        DespawnOnExit(AppState::Loading),
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
        Transform::default(),
        NotShadowCaster,
        NoFrustumCulling,
        scope,
    ));
}
