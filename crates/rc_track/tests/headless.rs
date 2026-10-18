//! Headless ECS behaviour of `TrackPlugin` (rc_testkit): colliders for both tracks, surfaces
//! under wheel rays, prop resets on restart, load errors, and no leaks across `Unloading`.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_track::{LOADING_KEY, Track, TrackPlugin};

fn config(track: &str) -> RaceConfig {
    RaceConfig {
        variant: TrackVariant::forward(track),
        ..default()
    }
}

/// What a downward ray from `from` hits first: (track piece kind, surface).
fn ray_down(app: &mut App, from: Vec3) -> Option<(Option<TrackPieceKind>, Option<SurfaceKind>)> {
    app.world_mut()
        .run_system_once(
            move |rapier: ReadRapierContext,
                  pieces: Query<(Option<&TrackPiece>, Option<&Surface>)>| {
                let ctx = rapier.single().ok()?;
                let (hit, _) = ctx.cast_ray(from, Vec3::NEG_Y, 40.0, true, QueryFilter::new())?;
                let (piece, surface) = pieces.get(hit).ok()?;
                Some((piece.map(|p| p.kind), surface.map(|s| s.0)))
            },
        )
        .expect("system runs")
}

#[test]
fn colliders_spawn_for_both_tracks() {
    for id in ["test_oval", "toy_town"] {
        let mut app = headless_app(TrackPlugin);
        load_race(&mut app, config(id)).expect("loads");
        let track = app.world().resource::<Track>();
        let geo = track.geometry();
        let expected = geo.ribbons.len() + geo.walls.len() + geo.ground.len() + 5; // + shell
        let props = track.props().len();
        let dynamic = track
            .props()
            .iter()
            .filter(|p| track.prop_def(&p.kind).is_some_and(|d| d.dynamic))
            .count();
        let summary = app.world().resource::<TrackSummary>();
        assert_eq!(summary.geometry_hash, track.geometry_hash(), "{id}");
        assert_eq!(summary.checkpoints, track.checkpoints().len(), "{id}");
        assert_eq!(
            count_entities::<(With<TrackPiece>, With<Collider>)>(&mut app),
            expected,
            "{id}"
        );
        assert_eq!(
            count_entities::<(With<Prop>, With<Collider>)>(&mut app),
            props,
            "{id}"
        );
        assert_eq!(
            count_entities::<(With<Prop>, With<Interpolated>, With<RigidBody>)>(&mut app),
            dynamic,
            "{id}"
        );
        assert!(
            dynamic > 0 && dynamic <= rc_track::MAX_DYNAMIC_PROPS,
            "{id}"
        );
    }
}

#[test]
fn ray_cast_down_hits_the_right_surface() {
    let mut app = headless_app(TrackPlugin);
    start_race(&mut app, config("test_oval")).expect("loads");
    step(&mut app, 2);
    use SurfaceKind::*;
    use TrackPieceKind::*;
    let cases = [
        (
            Vec3::new(0.0, 3.0, -20.0),
            Some(Ribbon),
            Some(Asphalt),
            "main straight",
        ),
        (
            Vec3::new(44.0, 5.0, -20.0),
            Some(Ribbon),
            Some(Wood),
            "wooden ramp",
        ),
        (
            Vec3::new(22.0, 3.0, 20.0),
            Some(Ground),
            Some(Grass),
            "infield",
        ),
        (Vec3::new(-7.2, 5.0, 0.0), Some(Wall), None, "outside wall"),
        (
            Vec3::new(22.0, 5.0, 5.0),
            None,
            Some(Wood),
            "kicker ramp prop",
        ),
        (
            Vec3::new(80.0, 5.0, 0.0),
            Some(Shell),
            None,
            "beyond the field: kill floor",
        ),
    ];
    for (from, piece, surface, what) in cases {
        assert_eq!(ray_down(&mut app, from), Some((piece, surface)), "{what}");
    }
    // Over the jump's gap the ray falls through to the grass below.
    let track = app.world().resource::<Track>().clone();
    let jump = track.jumps()[0];
    let gap = track.frame_at((jump.lip_s + jump.landing_s) * 0.5);
    assert_eq!(
        ray_down(&mut app, gap.point(0.0, 3.0)),
        Some((Some(Ground), Some(Grass))),
        "gap"
    );
}

fn dynamic_props(app: &mut App) -> Vec<(Entity, Transform)> {
    let mut q = app.world_mut().query::<(Entity, &Prop, &Transform)>();
    q.iter(app.world())
        .filter(|(_, p, _)| p.dynamic)
        .map(|(e, _, t)| (e, *t))
        .collect()
}

#[test]
fn dynamic_props_reset_on_restart() {
    let mut app = headless_app(TrackPlugin);
    start_race(&mut app, config("test_oval")).expect("loads");
    step(&mut app, 30);
    let before = dynamic_props(&mut app);
    for (e, _) in &before {
        app.world_mut()
            .entity_mut(*e)
            .insert(Velocity::linear(Vec3::new(12.0, 6.0, -4.0)));
    }
    step(&mut app, 30);
    let moved = dynamic_props(&mut app);
    assert!(
        before
            .iter()
            .zip(&moved)
            .all(|((_, a), (_, b))| a.translation.distance(b.translation) > 2.0),
        "knocked"
    );
    restart(&mut app);
    step(&mut app, 5);
    for ((e, spawn), (_, now)) in before.iter().zip(dynamic_props(&mut app)) {
        assert!(
            spawn.translation.distance(now.translation) < 0.05,
            "{e} back at its spawn pose"
        );
        let history = app.world().get::<PoseHistory>(*e).expect("interpolated");
        assert!(
            history.prev.translation.distance(history.curr.translation) < 0.05,
            "no streak"
        );
    }
}

#[test]
fn bad_track_fails_loading_without_leaks() {
    let mut app = headless_app(TrackPlugin);
    let baseline = count_entities::<()>(&mut app);
    let failure = load_race(&mut app, config("no_such_track")).expect_err("must fail");
    assert_eq!(failure.key, LOADING_KEY);
    assert!(failure.error.contains("no_such_track"), "{}", failure.error);
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

#[test]
fn nothing_leaks_across_unloading() {
    let mut app = headless_app(TrackPlugin);
    let baseline = count_entities::<()>(&mut app);
    start_race(&mut app, config("test_oval")).expect("loads");
    let oval = count_entities::<With<TrackPiece>>(&mut app);
    restart(&mut app);
    assert_eq!(
        count_entities::<With<TrackPiece>>(&mut app),
        oval,
        "restart keeps the session"
    );
    next_race(&mut app, Some(config("toy_town"))).expect("loads");
    let track = app.world().resource::<Track>();
    assert_eq!(track.variant().track.as_str(), "toy_town");
    let geo = track.geometry();
    let toy = geo.ribbons.len() + geo.walls.len() + geo.ground.len() + 5;
    let props = track.props().len();
    assert_eq!(
        count_entities::<With<TrackPiece>>(&mut app),
        toy,
        "only the new track's pieces"
    );
    assert_eq!(count_entities::<With<Prop>>(&mut app), props);
    quit_to_menu(&mut app);
    assert!(app.world().get_resource::<Track>().is_none());
    assert!(app.world().get_resource::<TrackSummary>().is_none());
    assert_eq!(count_entities::<()>(&mut app), baseline);
}
