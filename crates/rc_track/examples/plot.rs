//! Track plot tool: top-down and side-elevation SVGs of a layout.
//!
//! ```text
//! cargo run -p rc_track --example plot -- <track> [--rev] [--mirror] [--out <dir>] [--assets <dir>]
//! magick target/track-plots/<track>_top.svg target/track-plots/<track>_top.png
//! ```
//! Top: ground polygons and ribbon by surface, walls, gaps (dashed), centreline, racing line
//! coloured by target speed, checkpoints, start line, grid, respawn points, item rows, props
//! (collider footprints), shortcuts, Golden Battery. Side: height (+ banked edges) with jump arcs
//! at 1.0× / 0.8× target speed, bank, target speed, all against arc length. Lints are printed.

use std::fmt::Write as _;
use std::path::PathBuf;

use bevy::math::{Vec2, Vec3};
use rc_core::{GRAVITY, SurfaceKind, TrackVariant};
use rc_track::{Primitive, SpeedModel, Track, variant_name};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut track_id = None;
    let mut variant = TrackVariant::default();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = manifest.join("../../target/track-plots");
    let mut assets = manifest.join("../../assets");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--rev" => variant.reverse = true,
            "--mirror" => variant.mirror = true,
            "--out" => out = it.next().map(PathBuf::from).unwrap_or(out),
            "--assets" => assets = it.next().map(PathBuf::from).unwrap_or(assets),
            id if !id.starts_with('-') => track_id = Some(id.to_owned()),
            other => return eprintln!("unknown flag {other}"),
        }
    }
    let Some(id) = track_id else {
        return eprintln!("usage: plot <track> [--rev] [--mirror] [--out <dir>] [--assets <dir>]");
    };
    variant.track = id.as_str().into();
    let track = match Track::load(&assets, &variant) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let stem = format!(
        "{id}{}{}",
        if variant.reverse { "_rev" } else { "" },
        if variant.mirror { "_mirror" } else { "" }
    );
    if let Err(e) = std::fs::create_dir_all(&out) {
        return eprintln!("cannot create {}: {e}", out.display());
    }
    for (suffix, svg) in [("top", top_view(&track)), ("side", side_view(&track))] {
        let path = out.join(format!("{stem}_{suffix}.svg"));
        match std::fs::write(&path, svg) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(e) => eprintln!("cannot write {}: {e}", path.display()),
        }
    }
    report(&track);
}

fn report(track: &Track) {
    println!(
        "{} ({}): {:.1} u, {} checkpoints, {} respawn points, {} item rows, {} props, hash {:016x}",
        track.variant().track,
        variant_name(track.variant()),
        track.length(),
        track.checkpoints().len(),
        track.respawn_points().len(),
        track.item_rows().len(),
        track.props().len(),
        track.geometry_hash()
    );
    let lap: f32 = (0..(track.length() / 0.5) as usize)
        .map(|k| 0.5 / track.target_speed_at(k as f32 * 0.5).max(1.0))
        .sum();
    println!("reference lap (racing line at target speed, no grid start): {lap:.1} s");
    for j in track.jumps() {
        println!(
            "jump s = {:.0}→{:.0}: takeoff {:.1} u/s, touchdown {}, air {:.2} s",
            j.lip_s,
            j.landing_s,
            j.takeoff_speed,
            j.touchdown_s
                .map_or("SHORT".into(), |s| format!("s = {s:.0}")),
            j.air_time
        );
    }
    if track.lints().is_empty() {
        println!("no lints");
    }
    for l in track.lints() {
        println!("lint: {l}");
    }
}

fn surface_color(s: SurfaceKind) -> &'static str {
    match s {
        SurfaceKind::Asphalt => "#5a5f66",
        SurfaceKind::Stone => "#9a948a",
        SurfaceKind::ToyTrack => "#f07a1f",
        SurfaceKind::Wood => "#c99a63",
        SurfaceKind::Tile => "#d9d4c7",
        SurfaceKind::Carpet => "#2e9c9a",
        SurfaceKind::Cardboard => "#b88a52",
        SurfaceKind::Grass => "#6aa84f",
        SurfaceKind::Dirt => "#8b6a43",
        SurfaceKind::Ice => "#bfe6f5",
        SurfaceKind::Oil => "#2b2b2b",
        SurfaceKind::Water => "#4a90d9",
    }
}

/// Blue (slow) → green → red (fast).
fn speed_color(v: f32, lo: f32, hi: f32) -> String {
    let t = ((v - lo) / (hi - lo).max(1e-3)).clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.5 {
        let u = t * 2.0;
        (0.0, u, 1.0 - u)
    } else {
        let u = (t - 0.5) * 2.0;
        (u, 1.0 - u, 0.0)
    };
    format!(
        "rgb({:.0},{:.0},{:.0})",
        r * 230.0 + 20.0,
        g * 200.0 + 30.0,
        b * 230.0 + 20.0
    )
}

struct View {
    min: Vec2,
    scale: f32,
    pad: f32,
}

impl View {
    fn p(&self, v: Vec3) -> (f32, f32) {
        (
            self.pad + (v.x - self.min.x) * self.scale,
            self.pad + (v.z - self.min.y) * self.scale,
        )
    }
    fn pts(&self, vs: impl IntoIterator<Item = Vec3>) -> String {
        vs.into_iter()
            .map(|v| self.p(v))
            .map(|(x, y)| format!("{x:.1},{y:.1} "))
            .collect()
    }
}

fn top_view(track: &Track) -> String {
    let (lo, hi) = track.bounds();
    let size = Vec2::new(hi.x - lo.x, hi.z - lo.z);
    let scale = 1600.0 / size.max_element();
    let view = View {
        min: Vec2::new(lo.x, lo.z),
        scale,
        pad: 40.0,
    };
    let (w, h) = (size.x * scale + 80.0, size.y * scale + 140.0);
    let mut s = String::new();
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" font-family="sans-serif" font-size="12"><rect width="100%" height="100%" fill="#f4f4f0"/>"##
    );
    // Coordinate grid: 10 u minor, 50 u major with labels.
    let mut x = (lo.x / 10.0).ceil() * 10.0;
    while x <= hi.x {
        let (px, _) = view.p(Vec3::new(x, 0.0, lo.z));
        let major = (x / 50.0).fract().abs() < 1e-3;
        let _ = writeln!(
            s,
            r##"<line x1="{px:.1}" y1="40" x2="{px:.1}" y2="{:.1}" stroke="{}" stroke-width="{}"/>"##,
            40.0 + size.y * scale,
            if major { "#c8c8c0" } else { "#e4e4dc" },
            if major { 1.5 } else { 0.7 }
        );
        if major {
            let _ = writeln!(
                s,
                r##"<text x="{px:.1}" y="34" text-anchor="middle" fill="#888">x={x:.0}</text>"##
            );
        }
        x += 10.0;
    }
    let mut z = (lo.z / 10.0).ceil() * 10.0;
    while z <= hi.z {
        let (_, py) = view.p(Vec3::new(lo.x, 0.0, z));
        let major = (z / 50.0).fract().abs() < 1e-3;
        let _ = writeln!(
            s,
            r##"<line x1="40" y1="{py:.1}" x2="{:.1}" y2="{py:.1}" stroke="{}" stroke-width="{}"/>"##,
            40.0 + size.x * scale,
            if major { "#c8c8c0" } else { "#e4e4dc" },
            if major { 1.5 } else { 0.7 }
        );
        if major {
            let _ = writeln!(
                s,
                r##"<text x="4" y="{:.1}" fill="#888">z={z:.0}</text>"##,
                py + 4.0
            );
        }
        z += 10.0;
    }
    let mut ground: Vec<_> = track.ground().iter().collect();
    ground.sort_by(|a, b| a.height.total_cmp(&b.height));
    for g in ground {
        let pts = view.pts(g.polygon.iter().map(|p| Vec3::new(p.x, g.height, p.y)));
        let _ = writeln!(
            s,
            r##"<polygon points="{pts}" fill="{}" fill-opacity="0.45" stroke="#444" stroke-width="1"/>"##,
            surface_color(g.surface)
        );
        let c = g.polygon.iter().fold(Vec2::ZERO, |a, p| a + *p) / g.polygon.len() as f32;
        let (cx, cy) = view.p(Vec3::new(c.x, 0.0, c.y));
        let _ = writeln!(
            s,
            r##"<text x="{cx:.0}" y="{cy:.0}" fill="#333" fill-opacity="0.6" text-anchor="middle">{:?} y={:.2}</text>"##,
            g.surface, g.height
        );
    }
    // Ribbon quads by surface; gaps as dashed outlines.
    let ds = track.sample_spacing();
    let n = (track.length() / ds).round() as usize;
    for k in 0..n {
        let (a, b) = (
            track.frame_at(k as f32 * ds),
            track.frame_at((k + 1) as f32 * ds),
        );
        let quad = view.pts([
            a.point(-a.half_width, 0.0),
            b.point(-b.half_width, 0.0),
            b.point(b.half_width, 0.0),
            a.point(a.half_width, 0.0),
        ]);
        if a.in_gap {
            let _ = writeln!(
                s,
                r##"<polygon points="{quad}" fill="none" stroke="#c00" stroke-width="0.6" stroke-dasharray="3,3"/>"##
            );
        } else if track.style() != rc_track::RibbonStyle::Invisible {
            let col = surface_color(a.surface);
            let _ = writeln!(
                s,
                r##"<polygon points="{quad}" fill="{col}" stroke="{col}" stroke-width="0.4"/>"##
            );
        } else {
            let _ = writeln!(
                s,
                r##"<polygon points="{quad}" fill="#ffffff" fill-opacity="0.25"/>"##
            );
        }
        let (wl, wr) = track.walls_at(k as f32 * ds);
        for (on, side) in [(wl, -1.0), (wr, 1.0)] {
            if on {
                let (p, q) = (
                    view.p(a.point(side * (a.half_width + 0.6), 0.0)),
                    view.p(b.point(side * (b.half_width + 0.6), 0.0)),
                );
                let _ = writeln!(
                    s,
                    r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#222" stroke-width="3"/>"##,
                    p.0, p.1, q.0, q.1
                );
            }
        }
    }
    let centre = view.pts(
        (0..=n)
            .step_by(2)
            .map(|k| track.frame_at(k as f32 * ds).position),
    );
    let _ = writeln!(
        s,
        r##"<polyline points="{centre}" fill="none" stroke="#fff" stroke-width="0.8" stroke-dasharray="4,4"/>"##
    );
    // Racing line coloured by target speed.
    let (vmin, vmax) = (0..n)
        .map(|k| track.target_speed_at(k as f32 * ds))
        .fold((f32::MAX, 0.0f32), |(a, b), v| (a.min(v), b.max(v)));
    for k in (0..n).step_by(2) {
        let (p, q) = (
            view.p(track.racing_line_at(k as f32 * ds)),
            view.p(track.racing_line_at((k + 2) as f32 * ds)),
        );
        let col = speed_color(track.target_speed_at(k as f32 * ds), vmin, vmax);
        let _ = writeln!(
            s,
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{col}" stroke-width="2.5"/>"##,
            p.0, p.1, q.0, q.1
        );
    }
    // Direction arrows every 40 u.
    let mut a = 20.0;
    while a < track.length() {
        let f = track.frame_at(a);
        let tri = view.pts([
            f.point(0.0, 0.0) + f.forward * 2.5,
            f.point(-1.2, 0.0),
            f.point(1.2, 0.0),
        ]);
        let _ = writeln!(
            s,
            r##"<polygon points="{tri}" fill="#fff" stroke="#333" stroke-width="0.6"/>"##
        );
        a += 40.0;
    }
    for r in track.respawn_points() {
        let (x, y) = view.p(r.transform.translation);
        let _ = writeln!(
            s,
            r##"<circle cx="{x:.1}" cy="{y:.1}" r="2" fill="#777"/>"##
        );
    }
    for (i, c) in track.checkpoints().iter().enumerate() {
        let (p, q) = (
            view.p(c.point(-c.half_width - 1.0, 0.0)),
            view.p(c.point(c.half_width + 1.0, 0.0)),
        );
        let (col, wid) = if i == 0 { ("#000", 4.0) } else { ("#c0c", 2.0) };
        let _ = writeln!(
            s,
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{col}" stroke-width="{wid}"/>"##,
            p.0, p.1, q.0, q.1
        );
        let label = if i == 0 {
            "START".to_owned()
        } else {
            format!("CP{i}")
        };
        let _ = writeln!(
            s,
            r##"<text x="{:.1}" y="{:.1}" fill="{col}" font-weight="bold">{label}</text>"##,
            q.0 + 3.0,
            q.1
        );
    }
    for (k, g) in track.grid().iter().enumerate() {
        let corners = [(-0.55, -1.0), (0.55, -1.0), (0.55, 1.0), (-0.55, 1.0)]
            .map(|(x, z)| g.transform_point(Vec3::new(x, 0.0, z)));
        let _ = writeln!(
            s,
            r##"<polygon points="{}" fill="{}" stroke="#000" stroke-width="0.5"/>"##,
            view.pts(corners),
            if k == 0 { "#ffd400" } else { "#ffffff" }
        );
    }
    for row in track.item_rows() {
        for p in &row.slots {
            let (x, y) = view.p(*p);
            let _ = writeln!(
                s,
                r##"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="#00d0ff" stroke="#036" stroke-width="0.8"/>"##,
                x - 3.0,
                y - 3.0
            );
        }
    }
    for sc in track.shortcuts() {
        let _ = writeln!(
            s,
            r##"<polyline points="{}" fill="none" stroke="#090" stroke-width="2.5" stroke-dasharray="8,4"/>"##,
            view.pts(sc.path.iter().copied())
        );
    }
    props(&mut s, &view, track);
    if let Some(b) = track.golden_battery() {
        let (x, y) = view.p(b);
        let star: String = (0..10)
            .map(|i| {
                let r = if i % 2 == 0 { 11.0 } else { 4.5 };
                let a = i as f32 * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
                format!("{:.1},{:.1} ", x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        let _ = writeln!(
            s,
            r##"<polygon points="{star}" fill="#ffc800" stroke="#7a5a00" stroke-width="1"/>"##
        );
    }
    let legend = format!(
        "{} ({}) — {:.0} u, {} checkpoints, {} item rows, {} jumps, lints: {} — racing line {:.0}–{:.0} u/s (blue→red); START black; grid yellow = pole; cyan = Volt Cubes; dashed green = shortcut; dashed red = gap; star = Golden Battery",
        track.variant().track,
        variant_name(track.variant()),
        track.length(),
        track.checkpoints().len(),
        track.item_rows().len(),
        track.jumps().len(),
        track.lints().len(),
        vmin,
        vmax
    );
    let _ = writeln!(
        s,
        r##"<text x="40" y="{:.0}" font-size="15">{legend}</text></svg>"##,
        h - 30.0
    );
    s
}

fn props(s: &mut String, view: &View, track: &Track) {
    for p in track.props() {
        let Some(def) = track.prop_def(&p.kind) else {
            continue;
        };
        let col = if def.dynamic { "#ff8c1a" } else { "#7a4a2a" };
        for prim in &def.colliders {
            let (offset, rot) = prim.pose();
            let place = |v: Vec3| p.transform.transform_point(offset + rot * v);
            match *prim {
                Primitive::Box { half, .. } | Primitive::Wedge { half, .. } => {
                    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                        .map(|(x, z)| place(Vec3::new(x * half.x, 0.0, z * half.z)));
                    let _ = writeln!(
                        s,
                        r##"<polygon points="{}" fill="{col}" fill-opacity="0.75" stroke="#000" stroke-width="0.6"/>"##,
                        view.pts(corners)
                    );
                    if matches!(prim, Primitive::Wedge { .. }) {
                        let edge = view.pts([
                            place(Vec3::new(-half.x, 0.0, -half.z)),
                            place(Vec3::new(half.x, 0.0, -half.z)),
                        ]);
                        let _ = writeln!(
                            s,
                            r##"<polyline points="{edge}" stroke="#fff" stroke-width="2.5"/>"##
                        );
                    }
                }
                Primitive::Ball { radius, .. } | Primitive::Cylinder { radius, .. } => {
                    let (x, y) = view.p(place(Vec3::ZERO));
                    let _ = writeln!(
                        s,
                        r##"<circle cx="{x:.1}" cy="{y:.1}" r="{:.1}" fill="{col}" fill-opacity="0.75" stroke="#000" stroke-width="0.6"/>"##,
                        radius * view.scale
                    );
                }
            }
        }
        let (x, y) = view.p(p.transform.translation);
        let _ = writeln!(
            s,
            r##"<text x="{:.0}" y="{:.0}" font-size="10" fill="#000">{}</text>"##,
            x + 4.0,
            y - 4.0,
            p.kind
        );
    }
}

/// Ballistic flight from a lip: (s, height) points until touchdown.
fn arc(track: &Track, jump: &rc_track::Jump, speed: f32) -> Vec<(f32, f32)> {
    let lip = jump.lip_s;
    let (mut pos, mut vel, mut s) = (jump.takeoff_position, jump.takeoff_direction * speed, lip);
    let mut out = vec![(lip, pos.y)];
    for _ in 0..960 {
        vel.y -= GRAVITY / 240.0;
        pos += vel / 240.0;
        let tp = track.project(pos, Some(s));
        s = tp.s;
        out.push((lip + track.delta_s(lip, s).max(0.0), pos.y));
        if out.len() > 12 && tp.height <= 0.0 {
            break;
        }
    }
    out
}

fn side_view(track: &Track) -> String {
    let len = track.length();
    let ds = track.sample_spacing();
    let n = (len / ds).round() as usize;
    let frames: Vec<_> = (0..=n).map(|k| track.frame_at(k as f32 * ds)).collect();
    let (ymin, ymax) = frames
        .iter()
        .fold((track.kill_height(), f32::MIN), |(a, b), f| {
            (a.min(f.position.y - 2.0), b.max(f.position.y + 4.0))
        });
    let (w, x0) = (1800.0, 60.0);
    let px = |s: f32| x0 + s / len * (w - 2.0 * x0);
    let mut out = String::new();
    let _ = writeln!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="900" font-family="sans-serif" font-size="12"><rect width="100%" height="100%" fill="#f4f4f0"/>"##
    );
    // Panel frames: (top, height, title).
    let panels = [
        (
            40.0,
            420.0,
            "height (u): centreline (black), left/right edges (blue/red), jump arcs at 1.0× / 0.8× target speed (orange), kill height (red dashed)",
        ),
        (500.0, 160.0, "bank (deg), ±30° guides"),
        (700.0, 160.0, "target speed (u/s) of the reference car"),
    ];
    for (top, hgt, title) in panels {
        let _ = writeln!(
            out,
            r##"<rect x="{x0}" y="{top}" width="{:.0}" height="{hgt}" fill="#fff" stroke="#999"/><text x="{x0}" y="{:.0}">{title}</text>"##,
            w - 2.0 * x0,
            top - 6.0
        );
        for c in track.checkpoints() {
            let _ = writeln!(
                out,
                r##"<line x1="{0:.1}" y1="{top}" x2="{0:.1}" y2="{1:.0}" stroke="#c0c" stroke-opacity="0.4"/>"##,
                px(c.s),
                top + hgt
            );
        }
        for f in frames.windows(2).filter(|f| f[0].in_gap) {
            let _ = writeln!(
                out,
                r##"<rect x="{:.1}" y="{top}" width="{:.2}" height="{hgt}" fill="#f00" fill-opacity="0.08"/>"##,
                px(f[0].s),
                px(f[0].s + ds) - px(f[0].s)
            );
        }
    }
    let hy = |y: f32| 40.0 + 420.0 * (1.0 - (y - ymin) / (ymax - ymin));
    let poly = |pts: Vec<(f32, f32)>| {
        pts.iter()
            .map(|(x, y)| format!("{x:.1},{y:.1} "))
            .collect::<String>()
    };
    for (lat, col) in [(-1.0, "#36c"), (1.0, "#c33"), (0.0, "#000")] {
        let pts = frames
            .iter()
            .map(|f| (px(f.s.min(len)), hy(f.point(lat * f.half_width, 0.0).y)))
            .collect();
        let _ = writeln!(
            out,
            r##"<polyline points="{}" fill="none" stroke="{col}" stroke-width="{}"/>"##,
            poly(pts),
            if lat == 0.0 { 2.0 } else { 1.0 }
        );
    }
    let kill = hy(track.kill_height());
    let _ = writeln!(
        out,
        r##"<line x1="{x0}" y1="{kill:.1}" x2="{:.0}" y2="{kill:.1}" stroke="#d00" stroke-dasharray="6,4"/>"##,
        w - x0
    );
    for j in track.jumps() {
        for scale in [1.0, 0.8] {
            let pts = arc(track, j, j.takeoff_speed * scale)
                .into_iter()
                .map(|(s, y)| (px(s), hy(y)))
                .collect();
            let _ = writeln!(
                out,
                r##"<polyline points="{}" fill="none" stroke="#f80" stroke-width="2" stroke-dasharray="{}"/>"##,
                poly(pts),
                if scale == 1.0 { "0" } else { "5,3" }
            );
        }
        let _ = writeln!(
            out,
            r##"<text x="{:.0}" y="{:.0}" fill="#a50">jump {:.0} u/s, air {:.2} s</text>"##,
            px(j.lip_s),
            hy(ymax) + 14.0,
            j.takeoff_speed,
            j.air_time
        );
    }
    let by = |b: f32| 500.0 + 80.0 - b / 45.0 * 80.0;
    for g in [-30.0, 30.0, 0.0] {
        let _ = writeln!(
            out,
            r##"<line x1="{x0}" y1="{0:.1}" x2="{1:.0}" y2="{0:.1}" stroke="#bbb" stroke-dasharray="{2}"/>"##,
            by(g),
            w - x0,
            if g == 0.0 { "0" } else { "4,4" }
        );
    }
    let pts = frames
        .iter()
        .map(|f| (px(f.s.min(len)), by(f.bank.to_degrees())))
        .collect();
    let _ = writeln!(
        out,
        r##"<polyline points="{}" fill="none" stroke="#063" stroke-width="1.5"/>"##,
        poly(pts)
    );
    let top = SpeedModel::REFERENCE.top_speed * 1.05;
    let vy = |v: f32| 700.0 + 160.0 * (1.0 - v / top);
    let pts = frames
        .iter()
        .map(|f| (px(f.s.min(len)), vy(track.target_speed_at(f.s))))
        .collect();
    let _ = writeln!(
        out,
        r##"<polyline points="{}" fill="none" stroke="#a0a" stroke-width="1.5"/>"##,
        poly(pts)
    );
    for v in [10.0, 20.0, 30.0] {
        let _ = writeln!(
            out,
            r##"<text x="{:.0}" y="{:.0}" fill="#888">{v:.0}</text>"##,
            x0 - 24.0,
            vy(v) + 4.0
        );
    }
    for y in [ymin.ceil(), ((ymin + ymax) * 0.5).round(), ymax.floor()] {
        let _ = writeln!(
            out,
            r##"<text x="{:.0}" y="{:.0}" fill="#888">{y:.0}</text>"##,
            x0 - 30.0,
            hy(y) + 4.0
        );
    }
    let mut s = 0.0;
    while s <= len {
        let _ = writeln!(
            out,
            r##"<text x="{:.0}" y="885" text-anchor="middle" fill="#666">s={s:.0}</text>"##,
            px(s)
        );
        s += 50.0;
    }
    out.push_str("</svg>\n");
    out
}
