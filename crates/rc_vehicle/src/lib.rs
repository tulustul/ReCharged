//! # rc_vehicle — RC car physics (L1)
//!
//! - **Responsibilities:** the rapier body of every `Racer` (built on `Added<Racer>` from
//!   `assets/data/vehicles/<car>.ron`), 4 ray-cast wheels (3 rays each approximate the round
//!   tyre) with spring-damper suspension, a slip-based tyre model (× surface × weather ×
//!   effects), speed-sensitive steering, drift, air control, downforce, auto-right, battery
//!   charge & boost rules, the 1.5× speed cap;
//!   executes `RespawnCar`, `ApplyImpulse`, `DrainCharge` / `GrantCharge`; owns rapier's
//!   configuration ([`PhysicsPlugin`]: gravity, fixed step, frozen while paused).
//! - **Inputs:** `CarControl`, `StatusEffects`, `CatchUp`, `WeatherState`, `Surface` of the
//!   ground colliders, `RacePhase` (cars are held in Intro/Countdown), `UserPaths`,
//!   `RaceConfig` (cars to check), the request messages above.
//! - **Outputs:** `VehicleTelemetry`, `Charge`, `CarImpact`, `CarLanded`, `CarRespawned`,
//!   `Teleported`, the bodies' `Transform` (only rc_vehicle and rapier write it),
//!   [`VehicleLibrary`], `LoadingDone("vehicles")` / `LoadingFailed`.
//! - **Must never:** decide respawns (rc_race does), read input devices, touch visuals, or treat
//!   AI cars differently from human ones.
//!
//! Design note: `docs/design/rc_vehicle.md`.

mod assists;
mod charge;
mod drive;
mod model;
mod outputs;
mod params;
mod requests;
mod spawn;
mod state;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

pub use model::{CarSpec, TOP_SPEED_REACHED, WheelSpec};
pub use params::{
    Axle, BodyDims, LOADING_KEY, Steering, Suspension, Tyres, VEHICLES_DIR, VehicleLibrary,
    VehicleParams,
};

/// The car physics model. Add after [`PhysicsPlugin`] and `CorePlugin`.
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_systems(OnEnter(AppState::Loading), params::load_library)
            .add_systems(OnEnter(AppState::Unloading), params::unload_library)
            .add_systems(
                FixedUpdate,
                (
                    spawn::spawn_bodies,
                    spawn::rebuild_bodies.run_if(resource_exists_and_changed::<VehicleLibrary>),
                    requests::execute_respawns,
                    requests::apply_impulses,
                    spawn::sync_ghosting,
                    drive::drive_cars,
                )
                    .chain()
                    .in_set(SimSet::Vehicle),
            )
            .add_systems(
                FixedUpdate,
                (
                    outputs::motion,
                    charge::update_charge,
                    outputs::write_telemetry,
                    outputs::report_impacts,
                )
                    .chain()
                    .in_set(SimSet::PostPhysics),
            );
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            params::hot_reload.run_if(resource_exists::<VehicleLibrary>),
        );
    }
}

/// Rapier in `FixedUpdate` at [`SIM_HZ`], exactly one physics step per tick, gravity
/// [`GRAVITY`], placed between `SimSet::Vehicle` and `SimSet::PostPhysics`, frozen while
/// `PauseState::Paused`. No debug render (the binary adds it behind its `debug-physics` feature).
/// Used by the game and `rc_testkit`.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let init = RapierContextInitialization::InitializeDefaultRapierContext {
            integration_parameters: IntegrationParameters::default(),
            rapier_configuration: RapierConfiguration {
                gravity: Vec3::NEG_Y * GRAVITY,
                ..RapierConfiguration::new(1.0)
            },
            broad_phase_optimization_strategy: default(),
        };
        app.insert_resource(TimestepMode::Fixed {
            dt: SIM_DT,
            substeps: 1,
        })
        .add_plugins(
            RapierPhysicsPlugin::<NoUserData>::default()
                .in_fixed_schedule()
                .with_custom_initialization(init),
        )
        .configure_sets(
            FixedUpdate,
            (
                SimSet::Vehicle.before(PhysicsSet::SyncBackend),
                SimSet::PostPhysics.after(PhysicsSet::Writeback),
            ),
        )
        .add_systems(
            OnEnter(PauseState::Paused),
            |q: Query<&mut RapierConfiguration>| set_physics_active(q, false),
        )
        .add_systems(
            OnExit(PauseState::Paused),
            |q: Query<&mut RapierConfiguration>| set_physics_active(q, true),
        );
    }
}

/// Pause gates the `SimSet`s; rapier's own sets need this switch (ARCHITECTURE §3.1).
fn set_physics_active(mut contexts: Query<&mut RapierConfiguration>, active: bool) {
    for mut config in &mut contexts {
        config.physics_pipeline_active = active;
    }
}
