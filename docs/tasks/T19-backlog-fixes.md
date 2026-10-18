# T19 — Backlog fixes (two agents, disjoint ownership)

Fix the known follow-ups in `docs/tasks/BACKLOG.md`. Each item: smallest correct fix, a test where
the logic is non-trivial, the design note of the touched crate updated, and the backlog row marked
`~~DONE~~` in your report (the orchestrator edits BACKLOG.md). A QA agent is testing the game in
parallel and only writes `docs/qa/`; expect its report later.

**Read first:** PRACTICES; ARCHITECTURE §1–4 and the sections of the crates you touch; the design
notes of those crates; `docs/tasks/BACKLOG.md`.

## T19a — gameplay
**Owned:** `crates/rc_race/**`, `crates/rc_items/**`, `assets/data/items.ron`, `crates/rc_ai/**`,
`assets/data/ai.ron`, `crates/rc_camera/**`, `crates/rc_track/**` (lint only),
`tools/blender/props/street/**` + `assets/data/{props/street.ron,tracks/maple_street.ron,decor/maple_street.ron}`
(B19 data fixes only), `assets/data/tracks/test_oval.ron` (B5: turn 1 loses its banking, which sank
under the grass) and `assets/data/medals.ron` (recalibrate once at the end — AI changes move lap times).
- **B10** a finished human's car is handed to the AI (insert `AiDriver`) so it keeps driving sensibly
  while spectating/waiting.
- **B15** emit `RaceEvent { kind: RaceEventKind::MissedCheckpoint }` (already in rc_core) when a racer
  passes a checkpoint's `s` outside its gate — once per missed pass; the UI agent (T19b) shows it.
- **B11/B17** items: rockets may re-acquire a lock after losing one (same cone/range rules);
  explosions/EMP/storm check line of sight to walls (no hits through solid walls); the Shock Orb
  bounces off the barrier colliders' edge (or the drivable edge incl. walls) rather than the racing
  width; tune impulses/launches so item races average ≤ 0.12 to-track respawns per racer-lap on
  toy_town and test_oval (measure with the AI item race; report before/after).
- **B16/B25** AI: take intentional shortcuts when they're faster for the car (`Track::shortcuts()`)
  at Pro+ with a personality-scaled chance; line-of-sight check before firing rockets/leech;
  investigate the ~10 s outlier in ~1/8 Elite autopilot runs and fix the cause.
- **B23** look-back camera: apply the obstruction pull-in to the look-back pose.
- **B5** rc_track lint: banked ribbons whose inner edge (lip/barrier) sinks below a ground polygon
  or the floor (rule in `rc_track.md` §A5). Make sure all six shipped tracks stay lint-free.
- **B19** Maple Street data: leaf-pile collider height matches the visual (or lower the visual);
  decor beyond the bounds at the edge-lawn height; widen the reverse curb-hop turn (`TURN_R` in
  `plan.py`) if it doesn't break forward; keep lint-free and AI-passing.
- After all changes: `cargo test -p rc_ai --test race -- --ignored every_layout` passes.

## T19b — presentation, UI & infrastructure
**Owned:** `crates/rc_vfx/**`, `crates/rc_render/**`, `ui/**`, `crates/rc_ui/**`, `crates/recharged/**`,
`crates/rc_vehicle/src/outputs.rs` (+ whatever minimal rc_vehicle change B12 needs), additive edits in
`crates/rc_core/src/vehicle.rs`, workspace manifests + `crates/rc_pins/**` (B4 only).
- **Intro hitch** (from T13a): pickups and effect visuals compile their pipelines during the Intro —
  make the Loading warm-up include one instance of each pickup/projectile/effect material (or spawn
  them during Loading behind the loading screen) so Loading→Intro has no hitch; measure.
- **B12** add a world-space wheel contact point + normal to `WheelState` (rc_vehicle fills it from its
  ray hits); rc_vfx uses it for skid marks, debris and splashes so they sit exactly on ramps/curbs.
- **B13** (optional, if cheap) soft-depth fade for large particle puffs.
- **B2** the binary defers a human-initiated `AppExit` (window close, Quit) until the pipeline queue is
  idle (cap ~3 s), so quitting mid-compile can't hit the NVIDIA teardown crash.
- **B8** Credits screen content: team (orchestrated AI agents), tools (Bevy, bevy-react, rapier,
  kira, Blender, numpy), generators, the Saira OFL font notice; data from a small strings file.
- **B15 (UI side)** show a short "Missed checkpoint — go back" notice for
  `RaceEventKind::MissedCheckpoint` (replace the placeholder arm in `rc_ui/src/hud/view.rs`; T19a
  emits it from rc_race; the HUD demo fixture can show it before that lands).
- **B24** debounce `race.start` (one Confirm must send it once).
- **B27** remove `RECHARGED_UI_SCRIPT`'s `nav`/`pause` verbs (superseded by `--input-script`); keep
  the fixture verbs.
- **B4** remove unused deps (`serde`/`ron` in rc_weather and rc_audio; rc_ai keeps `bevy_rapier3d` — B16 uses it),
  then regenerate `rc_pins` (`python3 crates/rc_pins/update.py`) — do it last, in one go.
- After all changes: `./tools/ci.sh` passes.
