# bevy_rapier3d 0.37 digest (with Bevy 0.20)

Versions: **bevy_rapier3d 0.37.0** → rapier3d 0.36.1 / parry3d 0.31.1, on Bevy 0.20.0. Cargo: `bevy_rapier3d = "0.37"`.

Tags:
- **[C]** compiled in the scratch crate.
- **[T]** a headless test ran it and passed.
- **UNVERIFIED** is marked where it applies.

Snippets assume `use bevy::prelude::*; use bevy_rapier3d::prelude::*;`. The prelude also exports `Vect` (= `Vec3`), `Rot` (= `Quat`), `QueryFilter`, the events and every component.

## 0. What changed compared to older bevy_rapier

- **`Velocity { linvel, angvel }` → `Velocity { linear, angular }`.**
  - Constructors: `Velocity::linear(v)`, `Velocity::angular(w)`, `Velocity::zero()`.
  - Helper: `vel.linear_velocity_at_point(point, world_com)`. [C]
- **`RapierConfiguration` is a component on the context entity**, not a resource. That entity is spawned in `PreStartup` with the `DefaultRapierContext` marker. [C]
- **Scene queries need a system param.** `ReadRapierContext` / `WriteRapierContext`, then `.single()?`, give a `RapierContext` (no more `Res<RapierContext>`). Overlap methods are named `intersect_*` and their callback returns `bool` to continue. [C]
- **Collision events are Bevy Messages:** `MessageReader<CollisionEvent>` / `MessageReader<ContactForceEvent>`. Not observers. [C][T]
- `Collider::trimesh(..)` / `trimesh_with_flags(..)` return **`Result`**. `convex_hull` returns `Option`. [C]
- Physics can run in `FixedUpdate`: `.in_fixed_schedule()`. The default is still `PostUpdate` with a *variable* timestep. [C][T]
- **The vehicle controller is a first-class component:** `RayCastVehicleController` + `VehicleWheel`. [C][T]
- Velocity and world-space COM are only read back for bodies that ask, so add `Velocity` / `ReadWorldMassProperties` yourself. `ReadMassProperties` is auto-required by `RigidBody`.
- **Headless tests must call `app.finish()` before `update()`.** The plugin adds `AssetPlugin`, `MeshPlugin` and `WorldSerializationPlugin` in its `finish()` (feature `async-collider`). [T]

## 1. Cargo features

- Default features: `dim3`, `async-collider`, `debug-render-3d`, `picking-backend`, `to-bevy-mesh`. They enable `bevy/bevy_pbr`, `bevy_gizmos`, `bevy_render` and `bevy_picking` (all already in our `3d` feature set).
- Optional features:
  - `parallel`: rayon-parallel solver.
  - `simd8`: faster but not cross-platform deterministic.
  - `enhanced-determinism`: cross-platform bit-determinism. It excludes `simd8`; avoid `parallel` too.
  - `serde-serialize`.
- Optional, unlikely for us: `debug-render-2d`, `fem`, `profiler`, `block-solver`, `meshloader`, `urdf`, `mjcf`.
- Headless/server build without rendering: `bevy_rapier3d = { version = "0.37", default-features = false, features = ["dim3", "async-collider"] }`. UNVERIFIED as a separate build; the in-crate headless tests below use the default features.

## 2. Plugin setup and timestep

```rust
pub const PHYSICS_HZ: f64 = 60.0;

app.insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ))
    // Advance rapier by exactly one fixed tick per FixedUpdate run:
    .insert_resource(TimestepMode::Fixed { dt: (1.0 / PHYSICS_HZ) as f32, substeps: 1 })
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule())
    .add_plugins(RapierDebugRenderPlugin::default().disabled())        // gizmo lines, toggle at runtime
    .add_systems(FixedUpdate, drive_car.before(PhysicsSet::SyncBackend)); // write forces BEFORE sync
```
[C][T]

- `PhysicsSet::{SyncBackend, StepSimulation, Writeback}` run chained in the chosen schedule.
  - `SyncBackend`: ECS → rapier, plus character/vehicle/PID controllers.
  - `StepSimulation`: the step itself.
  - `Writeback`: `Transform`, `Velocity`, mass props, `CollidingEntities`.
- Order game systems `.before(PhysicsSet::SyncBackend)` (inputs, forces) or `.after(PhysicsSet::Writeback)` (read results).
- `TimestepMode` variants. The resource default is `Variable { max_dt: 1/60, time_scale: 1, substeps: 1 }`.
  - `Fixed { dt, substeps }`: steps exactly `dt` per run of the schedule. **Use this with `in_fixed_schedule()`** and keep `dt` equal to the `Time<Fixed>` timestep. The plugin warns if you combine FixedUpdate with a non-Fixed mode.
  - `Variable { max_dt, time_scale, substeps }`: `dt = min(Time::delta × time_scale, max_dt)`. Not deterministic; frame-rate dependent.
  - `Interpolated { dt, time_scale, substeps }`: for the default `PostUpdate` schedule. It steps fixed `dt` as often as real time requires and interpolates bodies that have the `TransformInterpolation` component.
- **Pause and slow-mo.**
  - In FixedUpdate mode, `Time<Virtual>::pause()` / `set_relative_speed()` already pause or slow physics, because FixedUpdate runs less.
  - Physics-only pause: `RapierConfiguration::physics_pipeline_active = false`.
- Other options:
  - `RapierPhysicsPlugin::with_length_unit(x)`: scale tolerances for non-metre worlds. Keep 1.0: model the RC cars at real scale (≈0.4 m), or scale the world up and set `length_unit` to match.
  - `.with_custom_initialization(RapierContextInitialization::…)`, `.with_default_system_setup(false)` + `RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::X)` for custom scheduling.

### Configuration at runtime [C]

```rust
fn tweak_physics(mut cfg: Single<&mut RapierConfiguration, With<DefaultRapierContext>>) {
    cfg.gravity = Vec3::new(0.0, -9.81 * 1.5, 0.0);   // toy cars feel better with "heavier" gravity
}
fn pause_physics(keys: Res<ButtonInput<KeyCode>>, mut cfgs: Query<&mut RapierConfiguration>) {
    if keys.just_pressed(KeyCode::KeyP) {
        for mut c in &mut cfgs { c.physics_pipeline_active = !c.physics_pipeline_active; }
    }
}
fn solver_tuning(mut sim: Single<&mut RapierContextSimulation, With<DefaultRapierContext>>) {
    sim.integration_parameters.num_solver_iterations = 8;   // default 4
}
fn toggle_debug_render(keys: Res<ButtonInput<KeyCode>>, mut ctx: ResMut<DebugRenderContext>) {
    if keys.just_pressed(KeyCode::F3) { ctx.enabled = !ctx.enabled; }   // per collider: ColliderDebug::NeverRender
}
```
- More `RapierConfiguration` fields: `simulation_mode: SimulationMode::{Full, CollisionOnly}`, `scaled_shape_subdivision`, `force_update_from_transform_changes`, `num_threads`.
- Run these from `Startup` or later: the context entity doesn't exist until `PreStartup` has run.
- The debug renderer draws gizmos on every camera (gizmo `RenderLayers`). Feature `debug-render-3d`, on by default.

## 3. Components

| Component | Notes |
|---|---|
| `RigidBody::{Dynamic, Fixed, KinematicPositionBased, KinematicVelocityBased}` | A collider with no `RigidBody` on itself or an ancestor is static. Child colliders attach to the nearest ancestor body |
| `Collider` | See constructors below. Half-extents everywhere |
| `Friction::coefficient(0.9)` / `{ coefficient, combine_rule }` | `CoefficientCombineRule::{Average, Min, Multiply, Max, ClampedSum}` |
| `Restitution::coefficient(0.1)` | |
| `Damping { linear_damping, angular_damping }` | |
| `Velocity { linear, angular }` | Read and write. **Must be added to read** |
| `ExternalForce { force, torque }`, `ExternalForce::at_point(f, world_point, world_com)` | Persistent: re-applied every step until changed. `Add`/`AddAssign` to sum. Writing it (even an equal value) triggers re-application |
| `ExternalImpulse { impulse, torque_impulse }`, `::at_point(..)` | One-shot: applied once, then reset to zero by the plugin |
| `GravityScale(f32)` | |
| `Ccd::enabled()` | Continuous collision for fast bodies (cars at speed vs thin walls). `SoftCcd { prediction }` is a cheaper alternative |
| `Sensor` | Trigger volume (needs `ActiveEvents` to report) |
| `ActiveEvents::COLLISION_EVENTS \| CONTACT_FORCE_EVENTS` | Needed on at least one collider of a pair |
| `ActiveCollisionTypes` | Default `DYNAMIC_DYNAMIC \| DYNAMIC_KINEMATIC \| DYNAMIC_STATIC`. Add `KINEMATIC_STATIC` etc. for kinematic sensors |
| `ContactForceEventThreshold(f32)` | Newtons. **Without the component the threshold is 0**, so a force event fires every step while touching. `::default()` is `f32::MAX` (never). Set it explicitly |
| `CollisionGroups::new(memberships, filters)` | `Group::GROUP_1..GROUP_32`, `ALL`, `NONE`, combined with `\|`. Affects contacts **and** queries (via the filter's `groups`). `SolverGroups` only affects the solver response |
| `LockedAxes::ROTATION_LOCKED_X \| TRANSLATION_LOCKED_Y …` | |
| `ColliderMassProperties::{Density(f32), Mass(f32), MassProperties(..)}` | Per collider; default density 1.0 |
| `AdditionalMassProperties::{Mass(f32), MassProperties(MassProperties { local_center_of_mass, mass, principal_inertia, .. })}` | On the body. Use it to lower the COM |
| `ReadMassProperties` (auto) | `.get()` / deref → `MassProperties { mass, local_center_of_mass, principal_inertia, .. }` |
| `ReadWorldMassProperties` (add yourself) | `center_of_mass` (world), `effective_inv_mass`, `effective_world_inv_inertia` |
| `Sleeping { sleeping, .. }`, `Sleeping::disabled()` | Disable sleeping on player cars |
| `Dominance { groups }`, `RigidBodyDisabled`, `ColliderDisabled` | |
| `CollidingEntities` | Set of colliders currently touching (needs `COLLISION_EVENTS`) |
| `TransformInterpolation` | Only with `TimestepMode::Interpolated` |
| `AsyncCollider(ComputedColliderShape)`, `AsyncSceneCollider { shape, named_shapes }` | Build colliders from a `Mesh3d` / a glTF scene once loaded |

[C] for the components used in the example and snippets.

### Collider constructors

- `Collider::cuboid(hx, hy, hz)`, `round_cuboid(hx, hy, hz, border)`, `ball(r)`, `capsule_y(half_h, r)` (and `capsule_x`/`capsule_z`/`capsule(a, b, r)`), `cylinder(half_h, r)`, `cone(half_h, r)`, `segment(a, b)`, `triangle(a, b, c)`, `halfspace(normal) -> Option`.
- `Collider::trimesh(Vec<Vec3>, Vec<[u32; 3]>) -> Result<Collider, TriMeshBuilderError>`.
- `Collider::trimesh_with_flags(v, i, TriMeshFlags::FIX_INTERNAL_EDGES)`. **Use `FIX_INTERNAL_EDGES`** (it includes `MERGE_DUPLICATE_VERTICES`) on track meshes: it stops wheels and bodies bumping on triangle seams. `FIX_INTERNAL_EDGES_TWO_SIDED`, `ORIENTED`, `DELETE_DEGENERATE_TRIANGLES` also exist.
- `Collider::convex_hull(&[Vec3]) -> Option<Collider>`, `convex_mesh(..)`, `convex_decomposition(&verts, &indices)` (V-HACD; `_with_params`), plus `round_*` variants.
- `Collider::compound(vec![(offset: Vec3, rot: Quat, Collider), ..])`.
- `Collider::heightfield(heights: Vec<f32>, num_rows, num_cols, scale: Vec3)`.
- `Collider::voxels(..)`, `voxelized_mesh(..)` (new).
- `Collider::from_bevy_mesh(&Mesh, &ComputedColliderShape) -> Option<Collider>`. The mesh must keep CPU data (`RenderAssetUsages::MAIN_WORLD`).
- `ComputedColliderShape::{TriMesh(TriMeshFlags), ConvexHull, ConvexDecomposition(VHACDParameters), Voxels { .. }, Converted(..)}`.

### Track collision from glTF [C]

```rust
fn spawn_track_scene(mut commands: Commands, server: Res<AssetServer>) {
    use bevy::platform::collections::HashMap;
    commands.spawn((
        WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset("tracks/toytown.glb"))),
        RigidBody::Fixed,                       // child colliders attach to the nearest ancestor RigidBody
        AsyncSceneCollider {
            shape: Some(ComputedColliderShape::TriMesh(TriMeshFlags::FIX_INTERNAL_EDGES)),
            // keys = primitive entity Name: "<glTF mesh name>.<material name>" (or just mesh name)
            named_shapes: HashMap::from([
                ("Flags.Cloth".to_string(), None),                                   // no collider
                ("Barrel.Wood".to_string(), Some(ComputedColliderShape::ConvexHull)),
            ]),
        },
    ));
}
fn collider_from_mesh(mesh: &Mesh) -> Option<Collider> {
    Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(TriMeshFlags::FIX_INTERNAL_EDGES))
}
```
- glTF meshes keep their CPU data by default (`GltfLoaderSettings::load_meshes` = `MAIN_WORLD | RENDER_WORLD`).
- Alternative: a separate low-poly collision mesh exported from Blender (for example node names with a `-col` suffix) and built with `collider_from_mesh`. This keeps physics independent of render detail.

## 4. Scene queries [C]

```rust
fn queries(rapier: ReadRapierContext,
           missiles: Query<(Entity, &GlobalTransform, &Velocity), With<Missile>>,
           mut gizmos: Gizmos) -> Result {
    let ctx = rapier.single()?;                                  // RapierContext
    for (e, t, v) in &missiles {
        let filter = QueryFilter::new().exclude_rigid_body(e).exclude_sensors();
        // Ray: Option<(collider Entity, RayIntersection { time_of_impact, point, normal, feature })>
        let _ray = ctx.cast_ray_and_get_normal(t.translation(), Vec3::NEG_Y, 2.0, true, filter);
        // Shape cast along a velocity:
        let opts = ShapeCastOptions { max_time_of_impact: 0.1, ..default() };   // seconds along v
        if let Some((hit_collider, hit)) =
            ctx.cast_shape(t.translation(), Quat::IDENTITY, v.linear, &Collider::ball(0.2), opts, filter)
        {
            // Queries and events report COLLIDER entities; map to the rigid body:
            let body = ctx.colliders.collider_parent(ctx.rigidbody_set, hit_collider);
            let _ = (body, hit.time_of_impact, hit.details.map(|d| d.normal1));
        }
        // Overlaps (explosion radius). Return true to keep iterating:
        ctx.intersect_shape(t.translation(), Quat::IDENTITY, &Collider::ball(3.0), filter, |other, _| {
            let _ = other; true
        });
    }
    Ok(())
}
```
- Also available:
  - `cast_ray(origin, dir, max_toi, solid, filter) -> Option<(Entity, f32)>`
  - `intersect_ray(.., |e, collider, hit| bool)`, `intersect_point(p, filter, |e, c| bool)`, `intersect_aabb_conservative(..)`
  - `project_point(p, max_dist, solid, filter)`, `distance_to_shape`, `closest_points_to_shape`, `contact_with_shape`, `cast_shape_nonlinear`
  - `contact_pair(c1, c2)`, `intersection_pair(c1, c2)`
  - `with_query_pipeline(filter, |qp| ..)` for many queries under one filter
- `QueryFilter` builders: `new()`, `exclude_fixed()`, `exclude_dynamic()`, `only_dynamic()`, `.exclude_sensors()`, `.exclude_solids()`, `.groups(CollisionGroups::new(G_CAR, G_TRACK))`, `.exclude_collider(e)`, `.exclude_rigid_body(e)`, `.predicate(&|e, collider| ..)`.
- `solid = true`: a ray starting inside a convex shape hits at toi 0. Trimeshes are hollow surfaces.
- Queries see the state after the **last completed step**. Colliders spawned this frame become queryable after the next `SyncBackend`.
- `WriteRapierContext` → `.single_mut()?` gives mutable access, e.g. `move_shape`.

## 5. Collision and contact-force events [C][T]

```rust
fn read_collisions(mut collisions: MessageReader<CollisionEvent>,
                   mut forces: MessageReader<ContactForceEvent>,
                   pickups: Query<(), With<Pickup>>, mut commands: Commands) {
    for ev in collisions.read() {
        match *ev {
            CollisionEvent::Started(a, b, _flags) => {            // flags: SENSOR, REMOVED
                for (pickup, other) in [(a, b), (b, a)] {         // order of a/b is arbitrary
                    if pickups.contains(pickup) { commands.entity(pickup).despawn(); }
                }
            }
            CollisionEvent::Stopped(..) => {}
        }
    }
    for ev in forces.read() {
        if ev.started {   // first step above ContactForceEventThreshold
            info!("impact {} <-> {}: {:.0} N", ev.collider1, ev.collider2, ev.total_force_magnitude);
        }   // also: total_force, max_force_direction, max_force_magnitude
    }
}
```
- Events carry **collider** entities. Map one to its body with `ctx.colliders.collider_parent(ctx.rigidbody_set, e)`, or put a marker component on the collider entity.
- Physics in FixedUpdate, reader in Update: this works, because messages are double-buffered [T: the pickup got collected].
- `MassModifiedEvent` and `PhysicsQuarantineEvent` (NaN state detected; the body gets `RigidBodyDisabled`) are messages too.
- **Contact details:** read the narrow phase with `ctx.contact_pair(c1, c2)` while the pair exists (manifolds, points, normals, impulses). For sensors use `ctx.intersection_pair`.
- Contact filtering and modification hooks (`ActiveHooks`, `BevyPhysicsHooks` as the plugin's generic parameter) exist; see the `contact_filter3`/`one_way_platforms3` examples. UNVERIFIED here.

## 6. Smooth rendering with a fixed physics step [C]

rapier writes `Transform` only on fixed ticks. At 144 Hz with 60 Hz physics that judders. Option A is rapier's `Interpolated` mode (PostUpdate). Option B, below, is what we compiled: keep physics in FixedUpdate and interpolate a **separate, non-child visual entity**.

```rust
#[derive(Component, Default)] pub struct PhysicsPoses { pub prev: Transform, pub curr: Transform }  // on the body
#[derive(Component)] pub struct InterpolateFrom(pub Entity);                                        // on the visual

app.add_systems(FixedPostUpdate, record_poses)
   .add_systems(RunFixedMainLoop, interpolate_visuals.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop));

fn record_poses(mut q: Query<(&Transform, &mut PhysicsPoses)>) {
    for (t, mut p) in &mut q { p.prev = p.curr; p.curr = *t; }
}
fn interpolate_visuals(fixed: Res<Time<Fixed>>, bodies: Query<&PhysicsPoses>,
                       mut visuals: Query<(&InterpolateFrom, &mut Transform), Without<PhysicsPoses>>) {
    let a = fixed.overstep_fraction();
    for (from, mut t) in &mut visuals {
        if let Ok(p) = bodies.get(from.0) {
            t.translation = p.prev.translation.lerp(p.curr.translation, a);
            t.rotation = p.prev.rotation.slerp(p.curr.rotation, a);
        }
    }
}
// visual: commands.spawn((InterpolateFrom(car), WorldAssetRoot(car_model), Transform::default()));
// Chase cameras follow the visual entity, not the body.
```

## 7. Headless and deterministic runs [T]

```rust
let mut app = App::new();
app.add_plugins((MinimalPlugins, TransformPlugin, bevy::state::app::StatesPlugin))
    .insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1))   // 1 fixed step per update()
    .add_plugins(CarPhysicsPlugin);                                          // the example below
app.finish();     // REQUIRED: rapier's finish() adds AssetPlugin + MeshPlugin (+ WorldSerializationPlugin)
app.cleanup();
app.update();     // Startup (and the first update has delta 0: no fixed step)
for _ in 0..180 { app.update(); }
```
- Without `finish()`, the first update panics: "Resource does not exist".
- Results from the example below:
  - After 3 s: rests on all 4 wheels at y ≈ 0.42.
  - After 2 s at full throttle: drives −Z at ≈18 m/s and collects the pickup through `CollisionEvent`.
- The rapier ball test settles at y = 0.600.

**Determinism**
- rapier is deterministic for the same binary, same inputs and the same order of insertion and removal. For bit-identical results across machines and OSes, enable `enhanced-determinism` (no `simd8`/`parallel`).
- On the Bevy side:
  - Use `TimestepMode::Fixed`, never `Variable`.
  - Write inputs into components inside FixedUpdate, before `SyncBackend`.
  - Give systems that touch physics an explicit order.
  - Spawn bodies in a stable order; don't iterate a `HashMap` to spawn.
  - Don't feed frame-time (`Time<Real>`) into physics.
- Replays: record per-tick inputs, re-simulate. Network lockstep: same rules. UNVERIFIED at scale.

## 8. Built-in raycast vehicle: `RayCastVehicleController` [C][T]

bevy_rapier wraps rapier's `DynamicRayCastVehicleController` (`controller.raw`) as a component on the chassis body.

```rust
fn spawn_vehicle(mut commands: Commands) {
    let tuning = WheelTuning { suspension_stiffness: 60.0, suspension_damping: 6.0, ..WheelTuning::default() };
    // defaults: stiffness 5.88, compression 0.83, damping 0.88, max_suspension_travel 5.0,
    //           side_friction_stiffness 1.0, friction_slip 10.5, max_suspension_force 6000
    let wheels = [(-0.45, -0.7), (0.45, -0.7), (-0.45, 0.7), (0.45, 0.7)]
        .map(|(x, z)| VehicleWheel::new(Vec3::new(x, -0.1, z), -Vec3::Y /*suspension dir*/,
                                        Vec3::X /*axle*/, 0.25 /*rest*/, 0.15 /*radius*/, tuning))
        .to_vec();
    let mut controller = RayCastVehicleController::new(wheels);
    controller.index_forward_axis = 2;      // default 0 (= +X forward!); up axis default 1 (Y)
    commands.spawn((RigidBody::Dynamic, Collider::cuboid(0.5, 0.15, 0.9),
                    ColliderMassProperties::Mass(12.0), Velocity::default(), controller,
                    Transform::from_xyz(0.0, 1.0, 0.0)));
}
fn drive_vehicle(mut q: Query<&mut RayCastVehicleController>) {   // in FixedUpdate, before SyncBackend
    for mut v in &mut q {
        for (i, w) in v.wheels.iter_mut().enumerate() {
            let front = i < 2;
            w.engine_force = if front { 0.0 } else { 40.0 };  // N, along contact_normal x axle
            w.steering = 0.0;                                 // rad around the suspension direction
            w.brake = 0.0;                                    // max braking impulse
        }
    }
}
```
- **Axis conventions (measured):**
  - Force direction is `contact_normal × axle`. With axle `+X` and up `+Y` that is `−Z`, so a positive `engine_force` drives along Bevy's forward. The test moved the car to z ≈ −23.6 in 3 s.
  - `current_vehicle_speed` is measured along `index_forward_axis` (here local +Z), so it is **negative when driving forward** (−17.6). Use `-v.current_vehicle_speed`.
  - The official example instead uses axle `+Z` with the car facing +X.
- After each update, read:
  - `wheel.state`: `suspension_length`, `is_in_contact`, `contact_point`, `contact_normal`, `ground_entity`, `rotation` (spin angle), `suspension_force`, `forward_impulse`, `side_impulse`, `center`, `axle_direction`.
  - `controller.current_vehicle_speed`, `controller.any_wheel_in_contact()`.
- Rendering: `wheel.local_transform()` gives the wheel's chassis-relative transform (suspension + steering + spin). Put it on child wheel entities `.after(PhysicsSet::Writeback)`.
- The ray filter excludes the chassis automatically. Also available: `filter_flags` (default `EXCLUDE_SENSORS`), `filter_groups`, `exclude_colliders`, `exclude_rigid_bodies`, `filter_predicate`, and the `ControllerIgnored` component.
- Impulses are computed **once per plugin update** with the last `dt`, not per substep. Use `TimestepMode::Fixed`.
- The simple friction model gives arcade drift, not a tyre model. For Re-Volt feel, our own raycast suspension (the example below) gives full control of grip, drift and air control. The built-in controller is a quick baseline for comparison.

## 9. The compile-verified example (box car, trimesh ground, per-wheel raycast suspension, collision events)

[C][T] This is the file from the scratch crate (`src/rapier_car.rs`), tested headless (§7). The only abridgement is the four interpolation items, which are printed in full in §6.

```rust
//! bevy_rapier3d 0.37 + Bevy 0.20: box car on a trimesh ground, per-wheel raycast
//! suspension applying forces at points, collision-event reader. Physics in FixedUpdate.
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub const PHYSICS_HZ: f64 = 60.0;

/// Collision layers.
pub const G_CAR: Group = Group::GROUP_1;
pub const G_TRACK: Group = Group::GROUP_2;
pub const G_PICKUP: Group = Group::GROUP_3;

#[derive(Component)]
pub struct RaycastCar {
    /// Wheel hard points in the chassis' local space (suspension top).
    pub wheels: [Vec3; 4],
    pub rest_length: f32,
    pub wheel_radius: f32,
    pub stiffness: f32, // N/m
    pub damping: f32,   // N·s/m
    pub throttle: f32,  // -1..1, written by input
    pub steer: f32,     // -1..1
    pub engine_force: f32,
    pub grip: f32, // lateral N per m/s of side slip, per wheel
}

#[derive(Component, Default)]
pub struct WheelContacts(pub [Option<Vec3>; 4]);

#[derive(Component)]
pub struct Pickup;

pub struct CarPhysicsPlugin;

impl Plugin for CarPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ))
            // Advance rapier by exactly one fixed tick per FixedUpdate run.
            .insert_resource(TimestepMode::Fixed { dt: (1.0 / PHYSICS_HZ) as f32, substeps: 1 })
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule())
            .add_systems(Startup, spawn_world)
            .add_systems(
                FixedUpdate,
                // Our forces must be written before rapier syncs ECS -> rapier.
                drive_car.before(PhysicsSet::SyncBackend),
            )
            .add_systems(FixedPostUpdate, record_poses)
            .add_systems(
                RunFixedMainLoop,
                interpolate_visuals.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .add_systems(Update, read_collisions);
    }
}

// PhysicsPoses, InterpolateFrom, record_poses, interpolate_visuals: exactly as in §6.

/// A gently bumpy 40x40 m trimesh ground (stand-in for a glTF track mesh).
pub fn ground_collider() -> Collider {
    let n = 20usize;
    let size = 40.0;
    let mut vertices = Vec::with_capacity((n + 1) * (n + 1));
    for z in 0..=n {
        for x in 0..=n {
            let fx = x as f32 / n as f32 * size - size / 2.0;
            let fz = z as f32 / n as f32 * size - size / 2.0;
            vertices.push(Vec3::new(fx, 0.15 * (fx * 0.5).sin() * (fz * 0.5).cos(), fz));
        }
    }
    let mut indices = Vec::with_capacity(n * n * 2);
    for z in 0..n as u32 {
        for x in 0..n as u32 {
            let i = z * (n as u32 + 1) + x;
            let row = n as u32 + 1;
            // counter-clockwise seen from above (+Y) => normals point up
            indices.push([i, i + row, i + 1]);
            indices.push([i + 1, i + row, i + row + 1]);
        }
    }
    Collider::trimesh_with_flags(vertices, indices, TriMeshFlags::FIX_INTERNAL_EDGES)
        .expect("valid trimesh")
}

pub fn spawn_world(mut commands: Commands) {
    commands.spawn((
        RigidBody::Fixed,
        ground_collider(),
        Friction::coefficient(0.9),
        CollisionGroups::new(G_TRACK, Group::ALL),
        Transform::default(),
    ));

    let half = Vec3::new(0.5, 0.15, 0.9);
    commands.spawn((
        Name::new("Car"),
        RigidBody::Dynamic,
        Collider::cuboid(half.x, half.y, half.z),
        ColliderMassProperties::Mass(12.0),
        // lower the centre of mass a bit for stability
        AdditionalMassProperties::MassProperties(MassProperties {
            local_center_of_mass: Vec3::new(0.0, -0.1, 0.0),
            mass: 0.0,
            ..default()
        }),
        // Bundles are tuples of at most 15 elements: nest them.
        (
            Velocity::default(),                // needed to READ velocities
            ReadWorldMassProperties::default(), // needed to READ world-space COM
            ExternalForce::default(),
            Damping { linear_damping: 0.05, angular_damping: 0.5 },
            Ccd::enabled(),
            Sleeping::disabled(),
        ),
        (
            Friction::coefficient(0.3), // body scraping walls
            Restitution::coefficient(0.1),
            CollisionGroups::new(G_CAR, G_TRACK | G_CAR | G_PICKUP),
            ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS,
            ContactForceEventThreshold(500.0),
        ),
        Transform::from_xyz(0.0, 1.5, 0.0),
        RaycastCar {
            wheels: [
                Vec3::new(-0.45, -0.1, -0.7),
                Vec3::new(0.45, -0.1, -0.7),
                Vec3::new(-0.45, -0.1, 0.7),
                Vec3::new(0.45, -0.1, 0.7),
            ],
            rest_length: 0.25,
            wheel_radius: 0.15,
            stiffness: 450.0,
            damping: 35.0,
            throttle: 0.0,
            steer: 0.0,
            engine_force: 60.0,
            grip: 25.0,
        },
        WheelContacts::default(),
        PhysicsPoses::default(),
    ));

    // A battery pickup: sensor collider, only reports collision events.
    commands.spawn((
        Pickup,
        Collider::ball(0.4),
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
        CollisionGroups::new(G_PICKUP, G_CAR),
        Transform::from_xyz(0.0, 0.5, -6.0),
    ));
}

/// Per-wheel raycast suspension + engine + lateral grip, as forces at points.
pub fn drive_car(
    rapier: ReadRapierContext,
    mut cars: Query<(
        Entity,
        &GlobalTransform,
        &Velocity,
        &ReadWorldMassProperties,
        &RaycastCar,
        &mut ExternalForce,
        &mut WheelContacts,
    )>,
) -> Result {
    let ctx = rapier.single()?;
    for (entity, gt, vel, mass, car, mut force, mut contacts) in &mut cars {
        let filter = QueryFilter::new()
            .exclude_rigid_body(entity) // never hit our own chassis
            .exclude_sensors()
            .groups(CollisionGroups::new(G_CAR, G_TRACK));
        let rot = gt.rotation();
        let up = rot * Vec3::Y;
        let fwd = rot * Vec3::NEG_Z;
        let com = mass.center_of_mass;
        let max_len = car.rest_length + car.wheel_radius;

        let mut total = ExternalForce::default();
        for (i, local) in car.wheels.iter().enumerate() {
            let origin = gt.transform_point(*local);
            contacts.0[i] = None;
            let Some((_ground, hit)) =
                ctx.cast_ray_and_get_normal(origin, -up, max_len, true, filter)
            else {
                continue;
            };
            contacts.0[i] = Some(hit.point);
            let compression = (max_len - hit.time_of_impact).max(0.0);
            let point_vel = vel.linear_velocity_at_point(origin, com);
            // spring-damper along the suspension axis, never pulling the car down
            let susp = (compression * car.stiffness - point_vel.dot(up) * car.damping).max(0.0);
            let mut f = up * susp;
            // steering: front wheels (0, 1) yaw their forward/side axes
            let steer = if i < 2 { car.steer * 0.5 } else { 0.0 };
            let wheel_fwd = Quat::from_axis_angle(up, -steer) * fwd;
            let wheel_side = wheel_fwd.cross(up);
            // drive on the rear axle, lateral grip on all four
            if i >= 2 {
                f += wheel_fwd * car.throttle * car.engine_force;
            }
            f -= wheel_side * point_vel.dot(wheel_side) * car.grip;
            total += ExternalForce::at_point(f, hit.point, com);
        }
        // Assigning (even an equal value) marks it changed; rapier re-applies it this step.
        *force = total;
    }
    Ok(())
}

pub fn read_collisions(
    mut collisions: MessageReader<CollisionEvent>,
    mut forces: MessageReader<ContactForceEvent>,
    pickups: Query<(), With<Pickup>>,
    mut commands: Commands,
) {
    for ev in collisions.read() {
        match *ev {
            CollisionEvent::Started(a, b, _flags) => {
                for (pickup, other) in [(a, b), (b, a)] {
                    if pickups.contains(pickup) {
                        info!("{other} collected pickup {pickup}");
                        commands.entity(pickup).despawn();
                    }
                }
            }
            CollisionEvent::Stopped(..) => {}
        }
    }
    for ev in forces.read() {
        if ev.started {
            info!(
                "impact {} <-> {}: {:.0} N",
                ev.collider1, ev.collider2, ev.total_force_magnitude
            );
        }
    }
}
```

Notes on the example:
- **The suspension.** Ray length is `rest_length + wheel_radius`, so `compression = max_len − toi`.
  - The spring-damper force is `k·x − c·v_up`, clamped ≥ 0 so the suspension never pulls the car down. It is applied at the contact point.
  - With 12 kg and k = 450 N/m per wheel, static compression ≈ 12·9.81/(4·450) ≈ 0.065 m.
  - Tune k/c per car; critical damping ≈ 2·√(k·m/4).
- **Steering, drive and grip.** Steering rotates the front wheels' forward/side axes. Drive is rear-wheel. Grip removes side-slip velocity (simple arcade model).
  - For drift: lower the rear grip under handbrake, cap the lateral force by `μ·suspension_force` (a friction circle), add downforce `∝ v²`.
  - Air control: apply `ExternalForce::torque` when no wheel touches.
- **Group filters.** The car collides with track, cars and pickups (`G_CAR` with filters `G_TRACK | G_CAR | G_PICKUP`). The wheel rays only hit `G_TRACK`, so they ignore other cars and pickups; widen that if wheels should ride on other cars.
- `Ccd::enabled()` prevents tunnelling through thin walls at high speed. `Sleeping::disabled()` stops a parked player car from falling asleep and ignoring forces.
- `ReadWorldMassProperties` gives the true world COM for `ExternalForce::at_point`, including the lowered COM from `AdditionalMassProperties`.
