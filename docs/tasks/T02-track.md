# T02 — Track system (`rc_track`)

**Goal:** turn track data files into everything the game needs — spline, geometry buffers,
colliders, checkpoints, respawn points, grid, item rows, racing line & speed profile, drivable-area
queries — plus authoring tools and the first two layouts.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §5.1, §7; GDD §4.2 (scale), §5.2 (surfaces), §5.4
(off-track), §7 (tracks); digests: bevy §0, §3, §6, §17; rapier (colliders, trimesh, groups,
headless).

## Owned paths
`crates/rc_track/**`, `assets/data/tracks/test_oval.ron`, `assets/data/tracks/toy_town.ron`,
`assets/data/props.ron`, `docs/design/rc_track.md`. **rc_core grant:** additive edits in
`crates/rc_core/src/track.rs` only.

## Deliverables
1. **`TrackDef` RON format** (ARCHITECTURE §5.1): control points `{ pos, width, bank_deg, surface,
   wall_left, wall_right, offroad_margin }` (no `dir` on control points — the loop shape is the same
   both ways; one-way paths are shortcuts), ribbon `style`, `gap` spans with takeoff
   direction, `ground` polygons with surfaces (lawns, floors, furniture tops — at any height),
   item rows, props `{ kind, pos, yaw, dir }`, shortcuts, golden battery, kill height, start-line
   index. Every feature has `dir: Both | Fwd | Rev`.
2. **Variants** (pure, tested): reverse (traverse backwards, keep `Both|Rev`), mirror (negate X,
   mirror yaw, fix winding; meshes never mirrored).
3. **Spline & frames** (pure): closed centripetal Catmull-Rom, arc-length samples every 0.5 u,
   frames with banking; helices work (projection uses the hint); no loops (validate: reject frames
   whose up vector points below horizontal).
4. **Geometry** (pure `MeshData { positions, normals, uvs, indices }` per run): ribbon per surface
   run for styles ToyTrack (raised lips), Asphalt (curbs), CarpetRunner (thin), Invisible (no
   visual; drivable surface comes from ground polygons), walls ≥ 0.5 u thick, triangulated ground
   polygons. UVs in world units (v along length, u across) so materials tile consistently.
5. **Derived data**: ≥ 8 checkpoints, respawn points every 15 u, 8-slot grid (2 columns, staggered),
   item rows of 4–5 cubes across the width, racing line (curvature minimisation within width
   margins), speed profile for a reference car (lateral-accel limit; forward/backward passes),
   `geometry_hash`.
6. **Runtime**: `Track` resource + query API (`project`, `frame_at`, `racing_line_at`,
   `target_speed_at`, `respawn_before`, `checkpoint_of`, `length`, `is_on_drivable`, `outline_2d`);
   writes `TrackSummary`; spawns session-scoped colliders (ribbon runs and ground polygons with
   `Surface`, walls, room shell / kill floor) with rc_core collision layers; props from `props.ron`
   (primitive collider list, static or dynamic with mass, sleeping; dynamic → `Prop` +
   `Interpolated`; reset to spawn poses in `OnEnter(RacePhase::Setup)` + `Teleported`); registers the
   `"track"` `LoadingTasks` key and sends `LoadingDone("track")`; cleans up in
   `OnEnter(AppState::Unloading)`.
7. **`props.ron`** schema + placeholder entries for the props `test_oval` and `toy_town` use
   (`model` paths may not exist yet — rc_render falls back to the collider primitives).
8. **Tools**: `cargo run -p rc_track --example plot -- <track> [--rev] [--mirror] [--out <dir>]` →
   top-down SVG (centreline, widths, walls, ground polygons, checkpoints, racing line coloured by
   target speed, item rows, props, golden battery, grid) and side-elevation SVG (height & bank vs
   s, jump arcs at target speed). Convert with `magick` to PNG and inspect them.
9. **Layouts**:
   - `test_oval` (hidden dev track, ~300 u): one banked turn, one Fwd-only jump, a grass ground
     shortcut, two surfaces, 4 item rows. Simple and robust — everyone tests on it.
   - `toy_town` (GDD §7.1, ~650 u, 3 laps): ToyTrack ribbon over a Carpet play-mat floor (ground
     polygon) in a Wood-floored playroom; banked turns, one helix, a Fwd jump and a Rev jump,
     dollhouse drive-through and block-tower props (placeholders), ≥ 8 checkpoints, 5 item rows,
     golden battery on a risky Fwd shortcut. Both directions must make sense.
10. **`docs/design/rc_track.md`** — a **track authoring guide**: format reference with examples,
    units/scale (GDD §4.2), widths (≥ 8 u, chokepoints ≥ 5 u, ≤ 2 per lap), slopes (≤ 25°), banking
    (≤ 30° except helix/half-pipe), jump recipe (takeoff angle & speed → arc → lower landing),
    designing both directions with `dir`, using ground polygons for open floors/lawns, how to
    run the plot tool, checklist before handing a layout to QA.

## Tests
Unit: format round-trip; every track file loads & validates; loops close; projection continuity
(incl. helix and across the start line); reverse/mirror invariants (equal length, mirrored
outline, `dir` filtering); racing line inside margins; speed profile respects limits;
checkpoint wrap-around; ground triangulation; `is_on_drivable` on ribbon vs ground vs outside.
Headless (rc_testkit, in `tests/`): colliders spawn for both tracks; a ray cast down hits
the right `Surface`; dynamic props reset on restart; no leaks across Unloading.

## Acceptance
All tests pass; clippy/fmt clean; plots of both tracks in all variants inspected (attach paths in
the report); the authoring guide exists. Report the `Track` API exactly as implemented.
