//! # rc_vehicle — RC car physics (L1)
//!
//! - **Responsibilities:** the rapier body of every `Racer` (spawned on `Added<Racer>` from
//!   `assets/data/vehicles/<car>.ron`), suspension, tyres, drift, air control, downforce,
//!   boost/charge rules, speed cap, auto-right; executes `RespawnCar`, `ApplyImpulse`,
//!   `DrainCharge` / `GrantCharge`; owns rapier's configuration ([`PhysicsPlugin`]).
//! - **Inputs:** `CarControl`, `StatusEffects`, `CatchUp`, `WeatherState`, `Surface` of the
//!   ground, the request messages above, `PauseState`.
//! - **Outputs:** `VehicleTelemetry`, `Charge`, `CarImpact`, `CarLanded`, `CarRespawned`,
//!   `Teleported`, the car bodies' `Transform` (only rc_vehicle and rapier write it).
//! - **Must never:** decide respawns (rc_race does), read input devices, or touch visuals.
//!
//! Status: skeleton (T01) + rapier setup; implemented by T03 (`docs/tasks/T03-vehicle.md`).

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

/// The car physics model.
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, _app: &mut App) {}
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
