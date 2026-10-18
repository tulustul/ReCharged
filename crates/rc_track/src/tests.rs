//! Whole-pipeline tests on the real data files and on small synthetic layouts.

use std::path::PathBuf;

use bevy::math::Vec3;
use rc_core::TrackVariant;

use super::*;

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn variants(id: &str) -> [TrackVariant; 4] {
    [(false, false), (true, false), (false, true), (true, true)].map(|(reverse, mirror)| {
        TrackVariant {
            track: id.into(),
            reverse,
            mirror,
        }
    })
}

fn track_ids() -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(assets().join(TRACKS_DIR))
        .expect("tracks dir")
        .filter_map(|e| e.ok()?.path().file_stem()?.to_str().map(str::to_owned))
        .collect();
    ids.sort();
    ids
}

fn build(ron: &str, reverse: bool, mirror: bool) -> Track {
    let def = TrackDef::parse(ron).expect("parses");
    let props = PropCatalog::load(&assets()).expect("props");
    let variant = TrackVariant {
        track: "synthetic".into(),
        reverse,
        mirror,
    };
    Track::build(&def, &props, &variant).expect("builds")
}

/// A loop with a one-turn helix climbing 6 u, an elevated straight over the helix entry, and a
/// long descent back to the start (s = 0 on the entry straight).
const HELIX: &str = "(style: Asphalt, kill_height: -10, points: [
    (pos: (0, 0.3, 40), width: 10, wall_left: true, wall_right: true),
    (pos: (0, 0.3, 20)), (pos: (0, 0.3, 0)),
    (pos: (2.144, 0.8, -8)), (pos: (8, 1.3, -13.856)), (pos: (16, 1.8, -16)),
    (pos: (24, 2.3, -13.856)), (pos: (29.856, 2.8, -8)), (pos: (32, 3.3, 0)),
    (pos: (29.856, 3.8, 8)), (pos: (24, 4.3, 13.856)), (pos: (16, 4.8, 16)),
    (pos: (8, 5.3, 13.856)), (pos: (2.144, 5.8, 8)), (pos: (0, 6.3, 0)),
    (pos: (0, 6.3, -15)), (pos: (0, 6.3, -30)),
    (pos: (7.322, 6.3, -47.678)), (pos: (25, 6.3, -55)), (pos: (42.678, 6.3, -47.678)), (pos: (50, 6.3, -30)),
    (pos: (50, 4.3, 0)), (pos: (50, 2.3, 30)), (pos: (50, 0.3, 60)),
    (pos: (42.678, 0.3, 77.678)), (pos: (25, 0.3, 85)), (pos: (7.322, 0.3, 77.678)), (pos: (0, 0.3, 60)),
])";

#[test]
fn every_track_file_loads_in_every_variant_without_lints() {
    let ids = track_ids();
    assert!(
        ids.contains(&"test_oval".to_owned()) && ids.contains(&"toy_town".to_owned()),
        "{ids:?}"
    );
    for id in &ids {
        for v in variants(id) {
            let t = Track::load(&assets(), &v).unwrap_or_else(|e| panic!("{e}"));
            let name = format!("{id} {}", variant_name(&v));
            assert!(t.lints().is_empty(), "{name}: {:?}", t.lints());
            assert!(t.checkpoints().len() >= MIN_CHECKPOINTS, "{name}");
            assert_eq!(t.grid().len(), rc_core::MAX_RACERS, "{name}");
            assert!(
                (4..=6).contains(&t.item_rows().len()),
                "{name}: GDD §7.2 asks for 4–6 rows"
            );
            assert!(
                t.item_rows()
                    .iter()
                    .all(|r| (4..=5).contains(&r.slots.len())),
                "{name}"
            );
            assert!(
                !t.geometry().walls.is_empty() && !t.geometry().ground.is_empty(),
                "{name}"
            );
            assert_eq!(
                t.golden_battery().is_some(),
                !v.reverse,
                "{name}: battery in forward layouts only"
            );
            assert!(t.jumps().iter().all(|j| j.touchdown_s.is_some()), "{name}");
        }
    }
}

#[test]
fn loops_close_without_kinks() {
    for id in track_ids() {
        let t = Track::load(&assets(), &TrackVariant::forward(id.as_str())).expect("loads");
        let (last, first) = (t.frame_at(t.length() - t.sample_spacing()), t.frame_at(0.0));
        assert!(
            (last.position.distance(first.position) - t.sample_spacing()).abs() < 0.05,
            "{id}"
        );
        assert!(
            last.forward.dot(first.forward) > 0.99,
            "{id}: no kink at the start line"
        );
        assert_eq!(
            t.frame_at(t.length()).position,
            first.position,
            "{id}: s wraps"
        );
    }
}

#[test]
fn mirror_and_reverse_keep_the_shape() {
    for reverse in [false, true] {
        let base = build(HELIX, reverse, false);
        let mirror = build(HELIX, reverse, true);
        assert!((base.length() - mirror.length()).abs() < 1e-3);
        for (a, b) in base.outline_2d().iter().zip(mirror.outline_2d()) {
            assert!(
                (a.x + b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3,
                "mirrored outline"
            );
        }
    }
    // Without one-way gaps, reverse is the same curve traversed backwards.
    let (fwd, rev) = (build(HELIX, false, false), build(HELIX, true, false));
    assert!(
        (fwd.length() - rev.length()).abs() < 0.05,
        "{} vs {}",
        fwd.length(),
        rev.length()
    );
    for k in 1..50 {
        let s = k as f32 * fwd.length() / 50.0;
        let (a, b) = (fwd.frame_at(s), rev.frame_at(rev.length() - s));
        assert!(
            a.position.distance(b.position) < 0.05,
            "same point at s = {s}"
        );
        assert!(a.forward.dot(b.forward) < -0.99, "opposite directions");
        assert!(a.right.dot(b.right) < -0.99, "left and right swap");
    }
    // Real tracks: lengths agree up to the one-way gaps' straight flights.
    for id in track_ids() {
        let [f, r, m, rm] = variants(&id).map(|v| Track::load(&assets(), &v).expect("loads"));
        assert!(
            (f.length() - m.length()).abs() < 1e-2 && (r.length() - rm.length()).abs() < 1e-2,
            "{id}"
        );
        assert!((f.length() - r.length()).abs() < 0.01 * f.length(), "{id}");
        assert_ne!(f.geometry_hash(), r.geometry_hash(), "{id}");
        assert_ne!(f.geometry_hash(), m.geometry_hash(), "{id}");
    }
}

#[test]
fn features_follow_their_direction() {
    let def = TrackDef::parse(&HELIX.replace(
        "])",
        "], gaps: [(from: 21, to: 22, dir: Rev)],
        item_rows: [(pos: (0, 0.3, 30), dir: Fwd), (pos: (25, 6.3, -55), dir: Rev), (pos: (25, 0.3, 85))],
        props: [(kind: \"crate_small\", pos: (60, 0, 0), dir: Fwd), (kind: \"tyre_stack\", pos: (60, 0, 20), yaw: 30)],
        golden_battery: (pos: (25, 1, 70)),
        ground: [(surface: Grass, height: 0, points: [(-20, -70), (75, -70), (75, 100), (-20, 100)])])",
    ))
    .expect("parses");
    let props = PropCatalog::load(&assets()).expect("props");
    let fwd = Track::build(&def, &props, &TrackVariant::forward("x")).expect("forward builds");
    let rev = Track::build(
        &def,
        &props,
        &TrackVariant {
            reverse: true,
            ..TrackVariant::forward("x")
        },
    )
    .expect("reverse builds");
    assert_eq!(
        (fwd.jumps().len(), rev.jumps().len()),
        (0, 1),
        "Rev gap only in reverse"
    );
    assert_eq!((fwd.item_rows().len(), rev.item_rows().len()), (2, 2));
    assert_eq!((fwd.props().len(), rev.props().len()), (2, 1));
    assert!(fwd.golden_battery().is_some() && rev.golden_battery().is_none());
    let mirrored = Track::build(
        &def,
        &props,
        &TrackVariant {
            mirror: true,
            ..TrackVariant::forward("x")
        },
    )
    .expect("mirror builds");
    let (a, b) = (&fwd.props()[1].transform, &mirrored.props()[1].transform);
    assert_eq!(
        b.translation,
        Vec3::new(-a.translation.x, a.translation.y, a.translation.z)
    );
    let (fa, fb) = (a.rotation * Vec3::NEG_Z, b.rotation * Vec3::NEG_Z);
    assert!(
        (fa.x + fb.x).abs() < 1e-5 && (fa.z - fb.z).abs() < 1e-5,
        "mirrored yaw"
    );
}

/// Walks the centreline with a lateral/vertical offset and projects with the previous `s` as
/// the hint: progress must follow the walk (helix levels, across the start line).
fn walk_projection(t: &Track, lateral: f32) {
    let mut hint = None;
    let mut walked = -10.0f32;
    while walked < t.length() + 10.0 {
        let f = t.frame_at(walked);
        let p = t.project(f.point(lateral, 0.6), hint);
        assert!(
            t.delta_s(walked, p.s).abs() < 0.3,
            "walked {walked:.1}, projected {:.1}",
            p.s
        );
        assert!(
            (p.lateral - lateral).abs() < 0.3 && (p.height - 0.6).abs() < 0.3,
            "{p:?}"
        );
        hint = Some(p.s);
        walked += 0.37;
    }
}

#[test]
fn projection_is_continuous_on_helices_and_across_the_start_line() {
    let helix = build(HELIX, false, false);
    walk_projection(&helix, 3.0);
    walk_projection(&helix, -3.0);
    // Without a hint the nearest level in 3D wins: entry straight vs the straight 6 u above.
    let low = helix.project(Vec3::new(0.0, 0.8, -1.0), None);
    let high = helix.project(Vec3::new(0.0, 6.8, -1.0), None);
    assert!(
        low.s > 35.0 && low.s < 50.0 && high.s > 100.0,
        "low {low:?} high {high:?}"
    );
    for id in track_ids() {
        for v in variants(&id) {
            walk_projection(&Track::load(&assets(), &v).expect("loads"), 2.0);
        }
    }
}

#[test]
fn checkpoints_and_respawns_wrap_around_the_start_line() {
    let t = Track::load(&assets(), &TrackVariant::forward("test_oval")).expect("loads");
    let n = t.checkpoints().len();
    let l = t.length();
    assert_eq!(t.checkpoints()[0].s, 0.0);
    assert!(
        t.checkpoints().windows(2).all(|w| w[0].s < w[1].s),
        "sorted"
    );
    assert_eq!(t.checkpoint_of(0.0), 0);
    assert_eq!(t.checkpoint_of(l - 0.1), n - 1);
    assert_eq!(t.checkpoint_of(-0.1), n - 1);
    assert_eq!(t.checkpoint_of(l + 0.1), 0);
    assert!(
        (t.delta_s(l - 1.0, 1.0) - 2.0).abs() < 1e-4
            && (t.delta_s(1.0, l - 1.0) + 2.0).abs() < 1e-4
    );
    // Just after the line with checkpoint 0 validated: the last respawn point before the line.
    let r = t.respawn_before(0.5, 0);
    assert_eq!(r, t.respawn_points()[0].transform);
    let r = t.respawn_before(l - 0.5, n - 1);
    let s = t.project(r.translation, None).s;
    assert!(s > l - 20.0 && s < l, "behind the line, got s = {s}");
    // Progress beyond an unvalidated checkpoint is not kept.
    let cp2 = t.checkpoints()[2].s;
    let s = t
        .project(t.respawn_before(cp2 + 30.0, 1).translation, None)
        .s;
    assert!(s < cp2, "clamped before checkpoint 2 ({cp2}), got {s}");
    let s = t
        .project(t.respawn_before(cp2 + 30.0, 2).translation, None)
        .s;
    assert!(s > cp2, "validated: kept, got {s}");
}

#[test]
fn derived_data_respects_its_rules() {
    for id in track_ids() {
        for v in variants(&id) {
            let t = Track::load(&assets(), &v).expect("loads");
            let name = format!("{id} {}", variant_name(&v));
            let ds = t.sample_spacing();
            for k in 0..(t.length() / ds) as usize {
                let s = k as f32 * ds;
                let f = t.frame_at(s);
                let o = t.racing_line_offset_at(s).abs();
                assert!(
                    o <= (f.half_width - LINE_MARGIN_U).max(LOCKED_OFFSET_U) + 1e-3,
                    "{name}: line at {s}"
                );
                let v = t.target_speed_at(s);
                assert!(
                    v > 5.0 && v <= SpeedModel::REFERENCE.top_speed * 1.02 + 1e-3,
                    "{name}: speed {v} at {s}"
                );
            }
            for r in t.respawn_points() {
                let near_jump = t.jumps().iter().any(|j| {
                    let d = t.delta_s(j.lip_s, r.s);
                    d > -JUMP_RUNUP_U && d < t.delta_s(j.lip_s, j.landing_s) + 1.0
                });
                assert!(
                    !near_jump && !t.frame_at(r.s).in_gap,
                    "{name}: respawn at {}",
                    r.s
                );
            }
            for (k, g) in t.grid().iter().enumerate() {
                let p = t.project(g.translation, None);
                assert!(
                    t.delta_s(0.0, p.s) < -2.0 && t.delta_s(0.0, p.s) > -25.0,
                    "{name}: slot {k} behind the line"
                );
                assert_eq!(p.lateral < 0.0, k % 2 == 0, "{name}: slot {k} column");
                assert!(
                    g.rotation
                        .mul_vec3(Vec3::NEG_Z)
                        .dot(t.frame_at(p.s).forward)
                        > 0.99,
                    "{name}: faces forward"
                );
            }
        }
    }
}

#[test]
fn drivable_area_is_ribbon_or_ground() {
    let t = Track::load(&assets(), &TrackVariant::forward("test_oval")).expect("loads");
    let f = t.frame_at(20.0);
    assert!(t.is_on_drivable(f.point(0.0, 0.5)), "on the ribbon");
    assert!(!t.is_on_drivable(f.point(0.0, 8.0)), "far above it");
    assert!(
        t.is_on_drivable(Vec3::new(22.0, 0.4, 0.0)),
        "on the grass infield"
    );
    assert!(
        !t.is_on_drivable(Vec3::new(120.0, 0.4, 0.0)),
        "outside every area"
    );
    let jump = t.jumps()[0];
    let gap = t.frame_at((jump.lip_s + jump.landing_s) * 0.5);
    assert!(
        gap.in_gap && !t.is_on_drivable(gap.point(0.0, 4.0)),
        "in the air over the gap"
    );
    // An Invisible ribbon is not a surface: only ground counts.
    let floor = "], ground: [(surface: Wood, height: 0.3, points: [(-20, -70), (75, -70), (75, 100), (-20, 100)])])";
    let invisible = build(
        &HELIX
            .replace("style: Asphalt", "style: Invisible")
            .replace("])", floor),
        false,
        false,
    );
    let f = invisible.frame_at(150.0);
    assert!(
        f.position.y > 5.0 && !invisible.is_on_drivable(f.point(0.0, 0.5)),
        "elevated, no ground"
    );
    assert!(
        invisible.is_on_drivable(invisible.frame_at(10.0).point(0.0, 0.5)),
        "over the floor"
    );
    assert!(invisible.geometry().ribbons.is_empty());
    let area: f32 = t.geometry().ground[0]
        .mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| {
            let [a, b, c] = [0, 1, 2]
                .map(|k| Vec3::from_array(t.geometry().ground[0].mesh.positions[c[k] as usize]));
            0.5 * (b - a).cross(c - a).length()
        })
        .sum();
    assert!(
        (area - 95.0 * 170.0).abs() < 1.0,
        "grass field triangulated, area {area}"
    );
}

#[test]
fn bad_layouts_fail_with_precise_messages() {
    let props = PropCatalog::load(&assets()).expect("props");
    let fail = |ron: &str| {
        let def = TrackDef::parse(ron).expect("parses");
        Track::build(&def, &props, &TrackVariant::forward("x")).expect_err("must fail")
    };
    let tight = HELIX.replace("width: 10", "width: 30");
    assert!(
        fail(&tight).contains("turn radius"),
        "turn tighter than the width"
    );
    let unknown = HELIX.replace("])", "], props: [(kind: \"nope\", pos: (0, 0, 0))])");
    assert!(fail(&unknown).contains("props[0].kind"));
    let low_kill = HELIX.replace("kill_height: -10", "kill_height: 0");
    assert!(fail(&low_kill).contains("kill_height"));
    let off = HELIX.replace("])", "], item_rows: [(pos: (200, 0, 0))])");
    assert!(fail(&off).contains("item row at (200, 0, 0)"));
}

#[test]
fn geometry_is_deterministic() {
    let v = TrackVariant::forward("test_oval");
    let (a, b) = (
        Track::load(&assets(), &v).expect("loads"),
        Track::load(&assets(), &v).expect("loads"),
    );
    assert_eq!(a.geometry_hash(), b.geometry_hash());
    assert_eq!(a.geometry(), b.geometry());
    assert_eq!(a.summary().checkpoints, a.checkpoints().len());
}

#[test]
fn a_track_crossing_itself_at_one_height_is_linted() {
    // Lemniscate of Bernoulli: a figure eight crossing at the origin.
    let pts: String = (0..24)
        .map(|k| {
            let t = k as f32 * std::f32::consts::TAU / 24.0;
            let d = 1.0 + t.sin().powi(2);
            format!(
                "(pos: ({:.3}, 0.3, {:.3})),",
                70.0 * t.cos() / d,
                70.0 * t.sin() * t.cos() / d
            )
        })
        .collect();
    let t = build(
        &format!("(style: CarpetRunner, kill_height: -5, points: [{pts}])"),
        false,
        false,
    );
    assert!(
        t.lints().iter().any(|l| l.contains("passes within")),
        "{:?}",
        t.lints()
    );
}
