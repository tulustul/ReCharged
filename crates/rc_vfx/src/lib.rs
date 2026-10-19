//! # rc_vfx — visual effects (P)
//!
//! - **Responsibilities:** pooled CPU particles (drift sparks, wheel debris, boost flames and
//!   arcs, dust, splashes, explosions, EMP rings, confetti) drawn as instanced quads of one
//!   material; skid marks; visuals of `Pickup` / `Projectile` / `Hazard` / `GoldenBattery`
//!   entities, shields and the leech tether; per-`GameCamera` precipitation and sky lightning on
//!   the camera's own render layer ([`player_layer`]); 3D name tags and lock-on markers per
//!   camera; *Reduce flashing* variants of every flash.
//! - **Inputs:** `VehicleTelemetry`, `StatusEffects`, `Racer`, `LocalPlayer`, `Visual` proxies,
//!   `CarImpact` / `CarLanded` / `CarRespawned`, `ItemTarget` / `ItemTimer` / `ItemRadius`,
//!   `Explosion` / `HitEvent`, `RaceEvent` (finish, Golden Battery), `WeatherState` /
//!   `WeatherEvent`, `GameCamera`, `Settings` (particle density, reduce flashing), `RaceConfig` +
//!   `UserPaths` (car wheel layouts, `models/cars/<car>.wheels.ron`).
//! - **Outputs:** visual entities only, plus `RenderLayers` on each `GameCamera` (layer 0 + its
//!   player layer).
//! - **Must never:** write simulation state or rapier bodies, read input devices, or run gameplay
//!   visuals while paused (every system sits in [`VfxSystems`], gated on `PauseState::Running`).

mod assets;
mod blasts;
mod car_events;
mod cars;
mod driving;
mod fx;
mod hazards;
mod items;
mod lightning;
mod particles;
mod pool;
mod projectiles;
mod skids;
mod splashes;
mod tags;
mod warmup;
mod weather;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use cars::{CarShape, CarShapes};
pub use fx::Fx;
pub use particles::{Facing, Look, Particle, Shape};
pub use weather::player_layer;

/// Order of the effects inside `VisualSet`: emitters first, then the particle step.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VfxSystems {
    /// Effects react to telemetry, messages and item entities (emit particles, pose visuals).
    Emit,
    /// The particle pool steps and its slot entities are updated.
    Simulate,
}

/// Particles, skid marks and effect visuals.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        configure(app);
        assets::build(app);
        fx::build(app);
        cars::build(app);
        driving::build(app);
        car_events::build(app);
        skids::build(app);
        items::build(app);
        projectiles::build(app);
        hazards::build(app);
        blasts::build(app);
        weather::build(app);
        splashes::build(app);
        lightning::build(app);
        tags::build(app);
        warmup::build(app);
    }
}

/// Effects run in `VisualSet` (after the proxies and cameras moved) and only while the race runs.
fn configure(app: &mut App) {
    app.configure_sets(
        Update,
        (VfxSystems::Emit, VfxSystems::Simulate)
            .chain()
            .in_set(VisualSet)
            .run_if(in_state(PauseState::Running)),
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::state::app::StatesPlugin;
    use bevy::state::state::FreelyMutableState;
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// The simulation half of the plugin (no render, no materials): sets, pool, driving effects.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, CorePlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / 60.0,
            )))
            .insert_resource(assets::VfxAssets::default());
        configure(&mut app);
        fx::build(&mut app);
        cars::build(&mut app);
        driving::build(&mut app);
        app.finish();
        app.cleanup();
        app
    }

    fn set<S: FreelyMutableState>(app: &mut App, state: S) {
        app.world_mut().resource_mut::<NextState<S>>().set(state);
        app.update();
    }

    fn drifting() -> VehicleTelemetry {
        VehicleTelemetry {
            speed: 20.0,
            forward_speed: 18.0,
            slip_angle: 0.4,
            drifting: true,
            charge_rate: 0.2,
            grounded_wheels: 4,
            wheels: [WheelState {
                grounded: true,
                surface: Some(SurfaceKind::Asphalt),
                slip: 0.8,
                ..default()
            }; 4],
            ..default()
        }
    }

    fn live(app: &App) -> usize {
        app.world().resource::<Fx>().pool.count()
    }

    #[test]
    fn particles_run_only_while_racing_and_clear_on_restart() {
        let mut app = app();
        set(&mut app, AppState::InRace);
        set(&mut app, RacePhase::Racing);
        // Racers are attempt entities (rc_race): a restart despawns them.
        let racer = app
            .world_mut()
            .spawn((
                Racer::default(),
                Transform::default(),
                DespawnOnExit(RaceAttempt),
            ))
            .id();
        app.world_mut().entity_mut(racer).insert(drifting());
        for _ in 0..20 {
            app.update();
        }
        let n = live(&app);
        assert!(n > 20, "a charging drift throws sparks and smoke: {n}");
        let slots = app.world().resource::<fx::Slots>().len();
        assert_eq!(
            slots,
            particles::MAX_PARTICLES,
            "one slot entity per pooled particle"
        );

        set(&mut app, PauseState::Paused);
        let frozen = live(&app);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(live(&app), frozen, "paused: nothing emits, nothing ages");

        set(&mut app, PauseState::Running);
        set(&mut app, RacePhase::Setup);
        assert_eq!(live(&app), 0, "a restart clears every particle");
    }

    #[test]
    fn the_budget_holds_under_any_load() {
        let mut app = app();
        set(&mut app, AppState::InRace);
        set(&mut app, RacePhase::Racing);
        let mut s = Settings::default();
        s.graphics.particles = ParticleDensity::Low;
        app.insert_resource(s);
        for i in 0..8 {
            app.world_mut()
                .spawn((
                    Racer {
                        index: i,
                        ..default()
                    },
                    Transform::from_xyz(i as f32 * 3.0, 0.0, 0.0),
                ))
                .insert(drifting());
        }
        for _ in 0..120 {
            app.update();
            let (_, budget) = fx::density(ParticleDensity::Low, 1);
            assert!(live(&app) <= budget, "{} > {budget}", live(&app));
        }
    }
}
