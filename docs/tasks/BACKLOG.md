# Backlog (small follow-ups, orchestrator-maintained)

Items found during integration that are not part of a running task's brief. Pick them up in the
QA/polish phase (T18) or when the owning area is touched next. T19a/T19b (`T19-backlog-fixes.md`)
take B2, B4, B5, B8, B10–B13, B15–B17, B19, B23–B25, B27.

| # | Item | Area | Source |
|---|---|---|---|
| B1 | ~~DONE~~ `--track-tour` camera flies through pickup volumes (translucent wash) — skip item rows in tour poses or hide pickups from capture cameras | rc_render capture / rc_vfx | T13a review |
| B2 | A human closing the window while pipelines compile can still hit the NVIDIA teardown crash — defer `AppExit` until the pipeline queue is idle | recharged | T13a diagnosis (`ponytail` in main.rs) |
| B3 | ~~DONE~~ Track previews for the remaining tracks once their environments are final (`--capture-previews`) | rc_render capture | T13b |
| B4 | Unused deps: `serde`/`ron` in `rc_weather` and `rc_audio`, `bevy_rapier3d` in `rc_ai` (remove + `rc_pins` refresh) | manifests | T15, T09, T11 |
| B5 | Lint for banked turns whose inner edge sinks below a floor (rule is documented in `rc_track.md` §A5) | rc_track | T16 toy |
| B6 | Human listening pass for SFX and music (only measured so far) | audio | T09, T10 |
| B7 | ~~DONE~~ (`docs/upstream-notes.md` U2) Upstream note: Bevy 0.20 atmosphere env-map probe panics every frame in our setup (worked around with a generated sky cubemap) | rc_render | T13a |
| B8 | Credits screen content: list tools, generators, the Saira font licence | ui | T05 |
| B9 | ~~DONE~~ Barrier styles for house, workshop and supermarket use the plain sweep — themed barrier blocks like toy/street | rc_render track_visuals | T13b |
| B10 | Finished human keeps car control after crossing the line — hand to the AI (one `AiDriver` insert) | rc_race | T06 |
| B11 | Rockets never re-acquire a lock; explosions/EMP ignore walls; orb uses racing-width edges not barriers | rc_items | T12 |
| B12 | World-space wheel contact point/normal on `WheelState` so skid marks, debris and splashes sit exactly on ramps/curbs | rc_core + rc_vehicle → rc_vfx | T14 |
| B13 | Soft-depth fade for big particle puffs (hard edge where they meet the ground) | rc_vfx | T14 |
| B14 | ~~DONE~~ Toy Town looked slow in agents' runs (≈¼ speed) — measure in isolation (no other game windows); if real: LODs/culling for the 70 shelves (~0.5 M tris) and instancing | rc_render / env toy | T05b |
| B15 | HUD: no message for a missed checkpoint (`RaceEventKind::MissedCheckpoint` added to rc_core; rc_race to emit, UI to show) | rc_race + ui | T05b |
| B16 | AI never takes shortcuts; rockets/leech fired without line-of-sight; one steering geometry for all cars | rc_ai | T11 |
| B17 | Item races are rough (0.14–0.33 respawns per racer-lap, mostly hits/Spring Pad launches over walls) — tune impulses, add wall checks | rc_items balance | T11 |
| B18 | Re-run `every_layout` AI test once all environments are final (incl. aisle_rush) | QA | T11 |
| B19 | Maple Street: leaf piles look 0.9 u but collide at 0.5 u; decor beyond bounds sits 0.6 u below edge lawns; reverse curb-hop turn very slow (widen TURN_R) | env street | T16 street |
| B20 | ~~DONE~~ Intro fly-by camera passes through walls (Aisle Rush lobby grid is 10 u from the glazing) — clamp the intro orbit inside `Track::bounds()` and under the ceiling | rc_camera | T16 supermarket |
| B21 | Prop glbs embed their own copies of texture sheets (~4 MB VRAM each in the supermarket) — share external textures | env art pipeline | T16 supermarket |
| B22 | ~~DONE~~ Decor has no time-of-day switch (supermarket ceiling light rows glow by day) — a "day-only"/"night-only" material prefix | rc_render | T16 supermarket |
| B23 | Look-back camera mostly black when stopped against a ramp (Backyard Bash deck) — obstruction pull-in for look-back | rc_camera | T17 |
| B24 | One Confirm on Car select once sent `race.start` twice (harmless: ignored in Loading) — debounce in the UI | ui | T17 |
| B25 | ~1 in 8 Elite autopilot TT runs loses ~10 s (outliers absorbed by median-of-3 calibration) | rc_ai | T17 |
| B26 | ~~DONE~~ `rc_render::quality::bench` overlaps the binary's `--bench` (logs from app start) — remove or keep log-only | rc_render | T17 |
| B27 | `RECHARGED_UI_SCRIPT` `nav`/`pause` verbs superseded by `--input-script` (keep go/demo/tab) | rc_ui | T17 |
