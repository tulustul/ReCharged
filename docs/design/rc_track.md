# rc_track — track system & track authoring guide

Part A is for **track designers** (how to build a layout file and get it QA-ready). Part B is for
**programmers** using tracks (race rules, AI, items, cameras, graphics). Part C records decisions,
tests and limitations.

```
tracks/<id>.ron ─┐                                     ┌─ Track (resource: queries + data)
props/*.ron ─────┴─ TrackDef ─ Layout(variant) ─ centreline ─ geometry ─ placements ─ racing line ─┤
                    validate   dir filter,        spline,      meshes    checkpoints,  speed       ├─ TrackSummary
                               reverse, mirror    frames       per run   grid, rows…   profile     └─ colliders + props (ECS)
```

---

# Part A — Authoring guide

## A1. Units and scale (GDD §4.2)
- **1 u = 22.5 cm.** Props and furniture are real size × 4.44 (1 m = 4.44 u). Rooms and gardens
  may be up to 1.5× their real floor area.
- The car is ≈ 2.0 u long and 1.1 u wide; its body is 0.4–1.1 u tall. Gravity is 25 u/s².
- Top speeds are 24–36 u/s (the reference car of the speed profile: 30 u/s). One lap takes
  roughly `length / 25` seconds.
- Axes: **+Y up, −Z forward ("north" in plots), +X right ("east")**. Plots draw X to the right
  and Z downwards, so −Z is up on the page.

## A2. Files
| File | Content |
|---|---|
| `assets/data/tracks/<id>.ron` | one layout ([`TrackDef`](../../crates/rc_track/src/def.rs)); `<id>` is the catalog `TrackInfo.id` |
| `assets/data/props/<environment>.ron` | prop kinds of one environment, **prefixed** with it (`toy_block_cube`, `garden_flower_pot`) |
| `assets/data/props/common.ron` | generic props any track may use (unprefixed: `crate_small`, `tyre_stack`, `ramp_wedge`) |

All `props/*.ron` files are merged; a kind defined twice is a load error naming both files. Each
environment owns one props file, so parallel work never collides.

Data files use RON with `implicit_some`: write `width: 12`, not `width: Some(12)`. Unknown fields
in track files are errors (typos are caught); integers are fine for floats (`(0, 0.3, 10)`).

## A3. A minimal track, annotated
```ron
(
    style: Asphalt,          // ToyTrack | Asphalt | CarpetRunner | Invisible
    start: 1,                // control point on the start/finish line (same place in reverse)
    kill_height: -10,        // cars below this height respawn (≥ 2 u below every drivable point)
    wall_height: 1.2,        // barrier height above the surface (default 1.2)
    points: [                // closed loop, forward order; ≥ 4 points, ≥ 1 u apart
        (pos: (0, 0.3, 28), width: 12, surface: Asphalt, wall_left: true, wall_right: false, offroad_margin: 6),
        (pos: (0, 0.3, 10)),                  // inherits width/surface/walls/margin
        (pos: (0, 0.3, -30)),
        (pos: (2.9, 0.3, -51), bank_deg: 15), // turn: a point every 20–30°
        // …
    ],
    gaps: [(from: 12, to: 13, dir: Fwd)],     // jumps
    ground: [(surface: Grass, height: 0, points: [(-25, -85), (70, -85), (70, 85), (-25, 85)])],
    item_rows: [(pos: (0, 0.3, -20)), (pos: (22, 0.3, 62), count: 4)],
    props: [(kind: "crate_small", pos: (20, 0, -51), yaw: 0, dir: Both)],
    shortcuts: [(path: [(41, 0.3, 51), (22, 0, 51), (3, 0.3, 51)], width: 6, dir: Both)],
    golden_battery: (pos: (22, 1.0, 51)),     // forward layouts only
    bounds: (min: (-40, -100), max: (85, 100)), // optional: world/room box (x, z)
)
```
`test_oval.ron` (simple) and `toy_town.ron` (helix, both jump directions, shortcut, props) are
complete, commented examples.

## A4. Format reference

### Control points
| Field | Default | Kind | Meaning |
|---|---|---|---|
| `pos` | — | point | centreline point; **`y` is the driving-surface height** (the ribbon hangs below it) |
| `width` | 10 (point 0) | point, inherited | racing width; ≥ 5 hard minimum, < 8 counts as a chokepoint |
| `bank_deg` | 0 | point, **not** inherited | positive raises the **left** edge (right-hand turns); negative for left-hand turns |
| `surface` | style's (point 0) | segment, inherited | ribbon surface from this point to the next |
| `wall_left` / `wall_right` | false (point 0) | segment, inherited | barrier on that edge from this point to the next |
| `offroad_margin` | 6 (point 0) | segment, inherited | off-track tolerance beyond the edge (GDD §5.4) |

- *Inherited* = a point without the field keeps the previous point's value (file order).
  *Segment* fields apply from this point to the next; *point* fields are interpolated between
  points (smoothstep, so they never overshoot).
- **Left/right** are relative to the forward travel direction. Reverse and mirror swap them for
  you — always author the forward layout.
- The centreline is a closed **centripetal Catmull-Rom** spline through the points, resampled
  every 0.5 u. It passes through every point; between unevenly spaced points it can bulge
  slightly (see A5).

### Ribbon styles (cross-sections)
| Style | Thickness | Edges | Barriers stand at |
|---|---|---|---|
| `ToyTrack` | 0.25 | raised lips 0.25 high × 0.35 wide, 0.7 u sloped outer sides | half width + 0.35 |
| `Asphalt` | 0.3 | sloped curbs 0.9 wide rising 0.08, 1 u sloped sides | half width + 0.9 |
| `CarpetRunner` | 0.06 | thin rug, 0.15 u bevels | half width |
| `Invisible` | — | no visual, no collider: the floor is a ground polygon or props | half width (walls sink 0.2 into the floor) |

A ribbon lying on a floor at height `h` has `y = h + thickness` (toy track on a mat at 0.05:
`y = 0.3`). Sloped outer sides let cars that leave the ribbon drive back on (except where
barriers are). Barriers are 0.6 u thick boxes along the edge, `wall_height` above the surface.

### Gaps (jumps)
`(from: <lip point>, to: <landing point>, dir: Fwd | Rev | Both)` — no ribbon between the two
points (forward order; several segments allowed; may wrap past the last point). In a direction
where the gap is active the flight is a straight line and the lip/landing tangents are
one-sided: **the takeoff angle is the slope from the point before the lip to the lip**. Where the
gap is not active, the ribbon is continuous (a hump/climb) — make that drivable too (A7).
Gaps may not touch each other or the start point.

### Ground polygons
`(surface, height, points: [(x, z), …], dir)` — a flat drivable area: floors, lawns, mats, table
tops. Simple polygons (no self-intersection), any winding, no holes (split instead). Overlaps
are fine: the higher polygon is driven on (a mat at 0.05 over a floor at 0). For `Invisible`
ribbons, the surface under the centreline comes from the topmost ground polygon there.

### Item rows
`(pos, count: 4 | 5 = 5, dir)` — `pos` is any point on or near the centreline (it is projected);
the row spans the width (1.5 u margins) with cubes 0.8 u above the surface. Never over a gap.
GDD §6.1: 4–6 rows per lap.

### Props
`(kind, pos, yaw = 0, dir = Both)` — `pos` is the prop's ground-contact centre (put it exactly on
the floor height); `yaw` in degrees turns its front (−Z) towards −X for positive values. Mirror
layouts mirror the position and yaw but never the mesh (text stays readable).

### Shortcuts
`(path: [entry, …, exit], width = 6, dir)` — an alternative route (forward order). It has **no
geometry of its own**: build it from ground polygons, props and openings in barriers. Rules:
both ends within half width + 2 u of the centreline; every path point within
`half width + offroad_margin` of it (raise `offroad_margin` on those segments — the loader tells
you how much); no checkpoint is placed inside a shortcut span (+ 2 u); it must go forward.

### Golden Battery, bounds, kill height
- `golden_battery: (pos)` — must be `Fwd` (GDD §7.2), off the main line (a ledge or risky
  shortcut).
- `bounds: (min: (x, z), max: (x, z))` — the room (indoor) or play area; the shell walls stand
  there. Omitted = content box + 15 u.
- `kill_height` — a kill floor sits 3 u below it, so cars falling off come to rest under it and
  rc_race respawns them.

## A5. Rules of thumb
| Topic | Rule | Checked by |
|---|---|---|
| Width | ≥ 8 u racing width; chokepoints ≥ 5 u, ≤ 2 per lap | 5 u: load error; chokepoints: lint |
| Slopes | ≤ 25° | lint |
| Banking | ≤ 30° (more only on helices / half-pipes); hard limit 60° | lint / load error |
| Loops | none: frames tilting past ~84° are rejected | load error |
| Turn radius | > half width + barrier/edge reach + 0.25 u | load error naming `s` |
| Stacking | parts of the track that overlap in plan must be ≥ 3.5 u apart in height (car + barriers fit under the upper ribbon) | lint |
| Start | the line on a straight with ≥ 25 u behind it in both directions; the 8-car grid (2 columns, rows 5 u apart) is 20.5 u deep and needs ≥ 6.4 u width, no gap | grid on a gap or too narrow: load error |
| Points | one every 20–30° in turns; on a straight, a point one turn-step (≈ R·30° in radians) before and after each turn; change heights and widths on straights | plot (speed ripple, edge bulges) |
| Jumps | land at 0.8× and 1.0× the target speed; 0.6–1.2 s air (GDD §5.1) | lint + plot |
| Items | 4–6 rows; never over gaps | tests / load error |
| Dynamic props | ≤ 40 per layout | load error |

Catmull-Rom tips: a straight segment between collinear points is perfectly straight; a lone
point far from its neighbours makes the curve bulge. If the plot shows a wobbly speed profile in
a turn or a bulging edge where a straight meets a turn, add points (A3, `test_oval` turns).

## A6. Jump recipe
1. **Run-up**: ≥ 40 u of straight before the lip (respawn points are never placed closer, so a
   respawned car can still make it), no turn just before it.
2. **Ramp**: a point at the ramp foot and one at the lip; takeoff angle = their slope, 5–10°.
   Kickers on an elevated section can be shallow (5°).
3. **Gap**: 8–12 u. **Landing lower than the lip** (1–4 u), so slower cars still make it:
   flight at speed `v` and angle `θ` from the lip: `x(t) = v cos θ · t`,
   `y(t) = v sin θ · t − 12.5 t²` (gravity 25). At 30 u/s and 8° the car is back at lip height
   after ≈ 10 u and ≈ 2 u below it after 18 u; at 24 u/s, ≈ 2 u below after 13 u.
4. **Landing slope**: 5–10° down over ≥ 12 u, then ≥ 20 u straight before the next corner (no
   braking in the air; landings at an angle bounce).
5. Plot it: the side view draws the arcs at 1.0× and 0.8× target speed; both must touch down past
   the landing point (the loader lints "lands short" otherwise).

| Example | Lip → landing | Gap | Air at 30 u/s |
|---|---|---|---|
| test_oval wooden ramp | 8° ramp, landing 1.0 u lower | 8 u | 0.61 s |
| toy_town elevated drop | 5° kicker at 6.8 u, landing 3.5 u lower | 12 u | 0.76 s |

## A7. Both directions (`dir`)
Every layout is driven forward and in reverse (and both mirrored). The loop's shape is the same
both ways; anything that only works one way carries `dir`:
- **Jumps**: a `Fwd` gap becomes a continuous ramp/hump in reverse, so the "filled" profile must be
  drivable: landing → lip slope ≤ 25° (toy_town: the 12 u gap filled climbs 3.5 u = 16°). For a
  `Rev` jump, author the points in forward order as *landing (low) then lip (high)*; in forward
  it is a hump.
- **Ramps and props**: give one-way ramps `dir` (toy_town's plank ramp is `Fwd`).
- **Item rows, ground, shortcuts** can be `Fwd`/`Rev` too; checkpoints follow the active shortcuts.
- **Golden Battery**: forward only.
- Plot `--rev` and look for: walls on the correct sides, a grid on a straight behind the line,
  climbs ≤ 25°, the reverse jumps landing.

## A8. Open floors and lawns
For rooms and gardens use `style: Invisible` with ground polygons for every floor (wood floor,
rug, kitchen tile, lawn, patio): the centreline still drives checkpoints, AI, respawns and the
off-track rule (`offroad_margin` defines how far from it the open area is legal). Props carry the
rest (book-stack ramps, furniture with drivable tops via `surface`). Respawn points are kept only
where a ground polygon lies under the centreline. Barriers (`wall_*`) stand at the corridor edge
(skirting boards, book rows — rc_render themes them).

## A9. Props
```ron
// assets/data/props/toy_town.ron
(props: {
    "toy_block_cube": (
        model: "models/props/toy_store/block_cube.glb",  // may not exist yet: colliders are drawn
        dynamic: true, mass: 0.35,                       // knockable (≤ 40 per layout), reset on restart
        colliders: [Box(half: (0.7, 0.7, 0.7), offset: (0, 0.7, 0))],
    ),
    "toy_plank_ramp": (model: "…", surface: Wood,        // `surface` = drivable (wheel rays read it)
        colliders: [Wedge(half: (2, 1.25, 4.5), offset: (0, 1.25, 0))]),
})
```
- Primitives (prop space, u): `Box(half, offset, rot)`, `Ball(radius, offset)`,
  `Cylinder(radius, half_height, offset, rot)` (upright), `Wedge(half, offset, rot)` (a ramp rising
  towards −Z, the front). `rot` is degrees (yaw, pitch, roll). One compound collider per prop.
- Static props are obstacles or (with `surface`) drivable; dynamic props are rigid bodies that
  start asleep, settle with light damping and are restored on every restart.
- Mass is in vehicle units (cars weigh ≈ 1.5–4): blocks 0.35, crates 0.6.
- Stack dynamic props exactly (block height = level spacing) so towers start at rest.

## A10. The plot tool
```sh
cargo run -p rc_track --example plot -- <track> [--rev] [--mirror] [--out <dir>] [--assets <dir>]
magick target/track-plots/<track>_top.svg target/track-plots/<track>_top.png
```
- **Top view** (`<track>[_rev][_mirror]_top.svg`): coordinate grid with labels (read positions
  for your data), ground polygons, ribbon by surface (dashed red = gap), barriers (thick black),
  centreline, racing line coloured by target speed (blue slow → red fast), direction arrows,
  START + checkpoints, grid (yellow = pole), respawn points (grey dots), item rows (cyan), props
  (collider footprints: orange dynamic, brown static; white edge = a wedge's high end), shortcuts
  (dashed green), Golden Battery (star).
- **Side view** (`_side.svg`): height of the centreline and both edges (banking), jump arcs at 1.0×
  and 0.8× target speed, kill height; bank angle; target speed — all against arc length `s`, with
  checkpoints marked.
- The console prints length, counts, the reference lap time, each jump (takeoff speed, touchdown,
  air time) and every **lint**.

## A11. Checklist before handing a layout to QA
1. `cargo run -p rc_track --example plot -- <id>` with no flags, `--rev`, `--mirror`,
   `--rev --mirror`: all load and print **no lints**.
2. Look at all eight images: barriers where cars must stay in, openings only where intended,
   grid behind the line on a straight, checkpoints spread out, rows on the line, props on the floor,
   no edge bulges, smooth speed profile, every jump arc landing in both directions.
3. Lap length within ~10 % of GDD §7.1; 4–6 item rows; ≥ 8 checkpoints (automatic); one Golden
   Battery (forward) off the main line.
4. Props: every kind exists in `props/<environment>.ron` with the environment prefix; ≤ 40
   dynamic; stacked props rest on each other.
5. `cargo test -p rc_track` passes (every track file is loaded in all four variants and checked).
6. Drive it: `cargo run -p recharged -- --user-data-dir target/agent-data/<you> --quick-race <id>
   [--reverse] [--mirror] --exit-after 60`.
7. The AI 3-lap test (rc_ai, run by QA) completes on every variant.

---

# Part B — Track API (for programmers)

`Track` is a session resource inserted in `OnEnter(AppState::Loading)` (before `LoadingDone("track")`)
and removed in `OnEnter(AppState::Unloading)`; `TrackSummary` alongside it. Everything is pure and
usable without an `App`: `Track::load(assets_dir, &variant)` (e.g. from tools and tests).

**Arc length `s`** starts at the start line, runs in the variant's travel direction and wraps at
`length()`; every query accepts any `s` (wrapped). **Track coordinates** of a point: `s`,
`lateral` (+ = right, along the banked surface) and `height` (along the surface normal).

| Query | Returns |
|---|---|
| `length() -> f32` | lap length (u) |
| `wrap(s) -> f32`, `delta_s(from, to) -> f32` | `s` in `[0, length)`; shortest signed distance across the start line |
| `frame_at(s) -> TrackFrame` | `{ s, position, forward, right, up, half_width, bank, surface, offroad_margin, in_gap }` + `rotation()`, `transform()`, `point(lateral, height)` |
| `project(pos, hint: Option<f32>) -> TrackPos` | `{ s, lateral, height }`. Pass the previous `s` as `hint` every tick (searches ±30 u: continuous on helices, crossings and across the line); `None` after teleports (global, nearest in 3D) |
| `racing_line_at(s) -> Vec3`, `racing_line_offset_at(s) -> f32` | racing line point / lateral offset at centreline `s` |
| `target_speed_at(s) -> f32` | target speed of `SpeedModel::REFERENCE` (30 u/s top, 30 u/s² lateral, 12 accel, 25 brake; surface grip/top speed and banking included) |
| `walls_at(s) -> (bool, bool)` | barriers (left, right) |
| `checkpoint_of(s) -> usize` | last checkpoint at or before `s` (checkpoint 0 = start line at `s = 0`) |
| `respawn_before(s, last_valid_checkpoint) -> Transform` | last respawn point at or before `s`, never past the next checkpoint still to validate |
| `is_on_drivable(pos) -> bool` | over the ribbon (not Invisible, not a gap) or a ground polygon, from 1 u below to 3 u above it; props not counted |
| `outline_2d() -> Vec<Vec2>`, `summary() -> TrackSummary` | minimap outline (x, z, every ≈ 2 u); the rc_core summary |
| `checkpoints() -> &[TrackFrame]` | ≥ 8 gates (position ± half width), sorted by `s` |
| `respawn_points() -> &[RespawnPoint]` | `{ s, transform }` every ≈ 15 u, 0.5 u above the surface |
| `grid() -> &[Transform]` | 8 poses, pole first, 2 staggered columns, 0.15 u above the surface |
| `item_rows() -> &[ItemRow]` | `{ s, slots: Vec<Vec3> }` |
| `props() -> &[PlacedProp]`, `prop_def(kind)` | `{ kind, transform }`; the `PropDef` (model, colliders, dynamic, mass, surface) |
| `ground() -> &[GroundArea]`, `shortcuts() -> &[Shortcut]`, `jumps() -> &[Jump]` | polygons; `{ path, width, entry_s, exit_s }`; `{ lip_s, landing_s, takeoff_position, takeoff_direction, takeoff_speed, touchdown_s, air_time }` |
| `golden_battery() -> Option<Vec3>`, `kill_height()`, `wall_height()`, `bounds() -> (Vec3, Vec3)` | layout facts (bounds = shell box) |
| `geometry() -> &TrackGeometry` | `ribbons: [RibbonMesh { surface, top, trim, body }]`, `walls: [MeshData]`, `ground: [GroundMesh { surface, mesh }]`; `MeshData { positions, normals, uvs, indices }` — world space, CCW front faces, UVs in world units (`v` = s, `u` = lateral on driving surfaces) |
| `style()`, `variant()`, `geometry_hash() -> u64`, `lints()`, `sample_spacing()` | |

**Entities** (all `DespawnOnExit(RaceSession)`):
- `TrackPiece { kind: Ribbon | Ground }` + `Surface(kind)` + trimesh collider (FIX_INTERNAL_EDGES);
  `TrackPiece { kind: Wall }` (friction 0.1 Min, restitution 0.3 Max); `TrackPiece { kind: Shell }`
  (kill floor 3 u below `kill_height`, four walls at `bounds()`). Layer `TRACK`, filter all.
- `Prop { kind, dynamic }` + `Transform` + compound collider (+ `Surface` if drivable), layer
  `PROP`; dynamic ones add `RigidBody::Dynamic`, `Velocity`, `Interpolated`, mass, damping and
  start asleep. They are reset in `OnEnter(RacePhase::Setup)` with `Teleported`.
- Wheel rays should hit `TRACK | PROP` and read `Surface` from the hit entity.

---

# Part C — Decisions, verification, limitations

## Decisions
- **Inherited, terse control points** keep 40–60-point files readable; variants operate on the
  fully resolved layout, so reverse/mirror never see inheritance.
- **Reverse = same physical loop**: per-segment fields move to the segment's other end, sides swap,
  banks flip. Mirror negates X and swaps sides again. Geometry is built from the transformed layout,
  so winding is always right; triangulation accepts both windings.
- **Gaps cut the spline**: flights are straight and lips/landings use one-sided tangents —
  otherwise Catmull-Rom flattens every takeoff. The exact lip frame (not an interpolated one) seeds
  the jump simulation.
- **Shortcuts have no geometry**: environments build them from ground, props and barrier openings;
  the data only feeds AI, checkpoint placement and the off-track margin check.
- **Racing line** = minimum squared curvature inside `half width − 1.6 u`, projected SOR coarse to
  fine (8 → 4 → 2 → 0.5 u), centred around jumps (20 u before the lip to 6 u after the landing).
  Tested by lap time: the reference car laps a test stadium ≥ 5 % faster on it than on the
  centreline. **Speed profile** uses horizontal curvature (crests and lips are not corners).
- **Respawn rule**: "never ahead of the last validated checkpoint" is read as "never past the next
  checkpoint still to validate" — keeps the 15 u granularity and still denies skipped checkpoints.
- **geometry_hash** is FNV-1a over quantised samples, ground, props and heights — stable across
  Rust versions (records and ghosts persist it).
- **Props in per-environment files** (orchestrator request): no write conflicts between
  environment agents; duplicates are load errors.
- Build cost: toy_town builds in ≈ 40 ms (debug), well inside the 3 s loading budget.

## Tests (`cargo test -p rc_track`)
- Unit (per module): RON round-trip and terse syntax, field validation messages, `dir` filtering,
  inheritance, reverse twice = identity, mirror, spline length/frames/closure, bank sign, gap
  tangents, helix frames, loop rejection, triangulation (concave, both windings, degenerate,
  self-intersection), outward-facing swept profiles, UVs, racing line margins/locking/lap time,
  speed limits, curvature, prop catalog parsing/merging/duplicates, wedge orientation.
- Pipeline (`src/tests.rs`): every track file in all four variants without lints; loops close;
  mirror/reverse invariants (length, mirrored outline, same curve backwards, hashes differ);
  feature `dir` filtering; projection continuity on a helix and across the start line; checkpoint
  and respawn wrap-around and clamping; racing line, speed, respawn, grid rules on real tracks;
  drivable area (ribbon, ground, outside, over a gap, Invisible); precise load errors; overlap lint;
  determinism.
- Headless (`tests/headless.rs`, rc_testkit): colliders for both tracks; downward rays hit
  Asphalt/Wood ribbons, Grass ground, a wall, a prop ramp, the kill floor, and fall through gaps;
  dynamic props reset on restart without interpolation streaks; a missing track fails loading
  without leaks; nothing leaks across `Unloading` (restart, next race, quit).

## Known limitations
- `is_on_drivable` and the overlap lint are linear/quadratic scans over samples (fine at 1–2 k
  samples per lap); add a grid if profiles show them.
- Catmull-Rom bulges slightly between unevenly spaced points (authoring rule in A5).
- Ground polygons are flat and have no holes; slopes come from the ribbon or props.
- Ribbons have one style per track; a shortcut needing its own visible ribbon must be built from
  props (e.g. a plank).
- The racing line ignores shortcuts and props; rc_ai handles obstacles and may use `shortcuts()`.
- Checkpoint validation by projected `s` cannot tell a legal line from an interior cut; rc_race
  may also require passing through the gate (`position ± half_width`).
