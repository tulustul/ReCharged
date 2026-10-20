//! Equivalence of the track editor's in-place updates (P7) and a fresh load: a sequence of edits
//! applied the way the editor commits them (a replaced `Track` + `TrackSummary` while `InRace`)
//! must leave the same `Track` and the same colliders, by `TrackPiece` and `PropSource`, as
//! loading the final draft from scratch.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_track::{
    Dir, DragPreview, DragSource, PropCatalog, PropPlacement, PropSource, Track, TrackDef,
    TrackDraft, TrackPlugin,
};

const TRACK: &str = "test_oval";

fn config() -> RaceConfig {
    RaceConfig {
        variant: TrackVariant::forward(TRACK),
        ..default()
    }
}

/// The session's track and its colliders, order-independent.
fn snapshot(app: &mut App) -> (String, String, Vec<String>) {
    let world = app.world_mut();
    let track = format!("{:?}", world.resource::<Track>());
    let summary = format!("{:?}", world.resource::<TrackSummary>());
    let mut q = world.query::<(
        Option<&TrackPiece>,
        Option<&PropSource>,
        Option<&Prop>,
        Option<&Surface>,
        &Transform,
        &Collider,
    )>();
    let mut colliders: Vec<String> = q
        .iter(world)
        .filter(|(piece, source, ..)| piece.is_some() || source.is_some())
        .map(|(piece, source, prop, surface, t, c)| {
            format!(
                "{piece:?} {source:?} {prop:?} {surface:?} {:?} {:?} {:?}",
                grid(t.translation.to_array(), 100.0),
                grid(t.rotation.to_array(), 1000.0),
                c.raw.compute_local_aabb()
            )
        })
        .collect();
    colliders.sort();
    (track, summary, colliders)
}

/// Values on a grid of `1 / per` (a knockable settles by a few mm while a session loads; the
/// in-place path places it exactly).
fn grid<const N: usize>(v: [f32; N], per: f32) -> [i32; N] {
    v.map(|x| (x * per).round() as i32)
}

fn pieces(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<TrackPiece>>();
    let mut v: Vec<Entity> = q.iter(app.world()).collect();
    v.sort();
    v
}

/// Commits `edit` the way the editor does: the draft follows, the session's `Track` and
/// `TrackSummary` are replaced, nothing reloads.
fn commit(app: &mut App, props: &PropCatalog, edit: impl FnOnce(&mut TrackDef)) {
    let mut def = app.world().resource::<TrackDraft>().def.clone();
    edit(&mut def);
    let track = Track::build(&def, props, &config().variant).expect("the edit builds");
    app.world_mut().resource_mut::<TrackDraft>().def = def;
    app.insert_resource(track.summary());
    app.insert_resource(track);
    step(app, 2);
    assert_eq!(app_state(app), AppState::InRace, "no reload");
}

#[test]
fn in_place_updates_match_a_fresh_load() {
    let props = PropCatalog::load(&assets_dir()).expect("props");
    let mut app = headless_app(TrackPlugin);
    app.insert_resource(TrackDraft {
        id: TRACK.into(),
        def: TrackDef::load(&assets_dir(), TRACK).expect("layout"),
        decor: Vec::new(),
    });
    load_race(&mut app, config()).expect("loads");
    pause(&mut app); // knockables would settle on: compare poses, not physics
    let loaded = pieces(&mut app);

    // Props only: the track pieces stay.
    commit(&mut app, &props, |d| {
        d.props[5].pos += Vec3::new(1.0, 0.0, 0.5); // tyre stack moved
        d.props[6].yaw = 30.0; // ramp turned
    });
    assert_eq!(
        pieces(&mut app),
        loaded,
        "a prop edit keeps the track pieces"
    );
    commit(&mut app, &props, |d| {
        d.props.push(PropPlacement {
            kind: "tyre_stack".into(),
            pos: Vec3::new(10.0, 0.0, -10.0),
            ..default()
        });
        d.props.push(PropPlacement {
            kind: "ramp_wedge".into(),
            pos: Vec3::new(15.0, 0.0, 10.0),
            dir: Dir::Rev, // not in the forward layout
            ..default()
        });
    });
    commit(&mut app, &props, |d| {
        d.props.remove(1); // every later source shifts down by one
        d.props[4].kind = "ramp_wedge".into(); // swap a kind
        let rev = d.props.len() - 1;
        d.props[rev].pos.x += 2.0; // the reverse-only prop moves
        d.props[rev].dir = Dir::Both; // … and appears
    });
    // The layout: the track pieces are rebuilt.
    commit(&mut app, &props, |d| d.points[3].pos.x += 0.5);
    assert_ne!(
        pieces(&mut app),
        loaded,
        "a layout edit rebuilds the track pieces"
    );
    commit(&mut app, &props, |d| d.points[3].label = Some("T1".into()));
    let incremental = snapshot(&mut app);

    quit_to_menu(&mut app);
    load_race(&mut app, config()).expect("reloads");
    pause(&mut app);
    let fresh = snapshot(&mut app);
    assert_eq!(incremental.0, fresh.0, "Track");
    assert_eq!(incremental.1, fresh.1, "TrackSummary");
    assert_eq!(incremental.2.len(), fresh.2.len(), "collider count");
    for (a, b) in incremental.2.iter().zip(&fresh.2) {
        assert_eq!(a, b);
    }
}

/// The editor clears a drag's preview in the frame it replaces `Track`: the preview's restore
/// must run first, or the committed prop would end up back where it was.
#[test]
fn a_commit_in_the_frame_the_preview_clears_keeps_the_new_pose() {
    let props = PropCatalog::load(&assets_dir()).expect("props");
    let mut app = headless_app(TrackPlugin);
    app.init_resource::<DragPreview>();
    app.insert_resource(TrackDraft {
        id: TRACK.into(),
        def: TrackDef::load(&assets_dir(), TRACK).expect("layout"),
        decor: Vec::new(),
    });
    load_race(&mut app, config()).expect("loads");
    pause(&mut app);
    let ramp = 6; // static
    let mut def = app.world().resource::<TrackDraft>().def.clone();
    def.props[ramp].pos += Vec3::new(3.0, 0.0, 1.0);
    let track = Track::build(&def, &props, &config().variant).expect("builds");
    let pose = track
        .props()
        .iter()
        .find(|p| p.source == ramp)
        .expect("placed")
        .transform;
    app.insert_resource(DragPreview(Some((DragSource::Prop(ramp), pose))));
    step(&mut app, 2);
    app.insert_resource(DragPreview(None));
    app.insert_resource(track.summary());
    app.insert_resource(track);
    step(&mut app, 1);
    let mut q = app.world_mut().query::<(&PropSource, &Transform)>();
    let now = q
        .iter(app.world())
        .find(|(s, _)| s.0 == ramp)
        .map(|(_, t)| *t)
        .expect("the ramp");
    assert_eq!(now, pose);
}
