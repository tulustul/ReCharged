# Backlog (small follow-ups, orchestrator-maintained)

Items found during integration that are not part of a running task's brief. Pick them up in the
QA/polish phase (T18) or when the owning area is touched next. T19a/T19b (`T19-backlog-fixes.md`)
take B2, B4, B5, B8, B10–B13, B15–B17, B19, B23–B25, B27.

| # | Item | Area | Source |
|---|---|---|---|
| B1 | ~~DONE~~ `--track-tour` camera flies through pickup volumes (translucent wash) — skip item rows in tour poses or hide pickups from capture cameras | rc_render capture / rc_vfx | T13a review |
| B2 | ~~DONE~~ (T19b) A human closing the window while pipelines compile can still hit the NVIDIA teardown crash — defer `AppExit` until the pipeline queue is idle | recharged | T13a diagnosis (`ponytail` in main.rs) |
| B3 | ~~DONE~~ Track previews for the remaining tracks once their environments are final (`--capture-previews`) | rc_render capture | T13b |
| B4 | ~~DONE~~ (T19b) Unused deps: `serde`/`ron` in `rc_weather` and `rc_audio`, `bevy_rapier3d` in `rc_ai` (remove + `rc_pins` refresh) | manifests | T15, T09, T11 |
| B5 | ~~DONE~~ (T19a) Lint for banked turns whose inner edge sinks below a floor (rule is documented in `rc_track.md` §A5) | rc_track | T16 toy |
| B6 | Human listening pass for SFX and music (only measured so far) | audio | T09, T10 |
| B7 | ~~DONE~~ (`docs/upstream-notes.md` U2) Upstream note: Bevy 0.20 atmosphere env-map probe panics every frame in our setup (worked around with a generated sky cubemap) | rc_render | T13a |
| B8 | ~~DONE~~ (T19b) Credits screen content: list tools, generators, the Saira font licence | ui | T05 |
| B9 | ~~DONE~~ Barrier styles for house, workshop and supermarket use the plain sweep — themed barrier blocks like toy/street | rc_render track_visuals | T13b |
| B10 | ~~DONE~~ (T19a) Finished human keeps car control after crossing the line — hand to the AI (one `AiDriver` insert) | rc_race | T06 |
| B11 | ~~DONE~~ (T19a) Rockets never re-acquire a lock; explosions/EMP ignore walls; orb uses racing-width edges not barriers | rc_items | T12 |
| B12 | ~~DONE~~ (T19b) World-space wheel contact point/normal on `WheelState` so skid marks, debris and splashes sit exactly on ramps/curbs | rc_core + rc_vehicle → rc_vfx | T14 |
| B13 | ~~DONE~~ (T19b) Soft-depth fade for big particle puffs (hard edge where they meet the ground) | rc_vfx | T14 |
| B14 | ~~DONE~~ Toy Town looked slow in agents' runs (≈¼ speed) — measure in isolation (no other game windows); if real: LODs/culling for the 70 shelves (~0.5 M tris) and instancing | rc_render / env toy | T05b |
| B15 | ~~DONE~~ (T19a) HUD: no message for a missed checkpoint (`RaceEventKind::MissedCheckpoint` added to rc_core; rc_race to emit, UI to show) | rc_race + ui | T05b |
| B16 | ~~DONE~~ (T19a) AI never takes shortcuts; rockets/leech fired without line-of-sight; one steering geometry for all cars | rc_ai | T11 |
| B17 | ~~DONE~~ (T19a) Item races are rough (0.14–0.33 respawns per racer-lap, mostly hits/Spring Pad launches over walls) — tune impulses, add wall checks | rc_items balance | T11 |
| B18 | ~~DONE~~ (T18: 23/24 pass; wrench_works mirror → QA Q4, T19a) Re-run `every_layout` AI test once all environments are final (incl. aisle_rush) | QA | T11 |
| B19 | ~~DONE~~ (T19a) Maple Street: leaf piles look 0.9 u but collide at 0.5 u; decor beyond bounds sits 0.6 u below edge lawns; reverse curb-hop turn very slow (widen TURN_R) | env street | T16 street |
| B20 | ~~DONE~~ Intro fly-by camera passes through walls (Aisle Rush lobby grid is 10 u from the glazing) — clamp the intro orbit inside `Track::bounds()` and under the ceiling | rc_camera | T16 supermarket |
| B21 | Prop glbs embed their own copies of texture sheets (~4 MB VRAM each in the supermarket) — share external textures | env art pipeline | T16 supermarket |
| B22 | ~~DONE~~ Decor has no time-of-day switch (supermarket ceiling light rows glow by day) — a "day-only"/"night-only" material prefix | rc_render | T16 supermarket |
| B23 | ~~DONE~~ (T19a) Look-back camera mostly black when stopped against a ramp (Backyard Bash deck) — obstruction pull-in for look-back | rc_camera | T17 |
| B24 | ~~DONE~~ (T19b) One Confirm on Car select once sent `race.start` twice (harmless: ignored in Loading) — debounce in the UI | ui | T17 |
| B25 | ~~DONE~~ (T19a rounds 1–2: Toy Town TT 0 outliers in 32 runs) ~1 in 8 Elite autopilot TT runs loses ~10 s (outliers absorbed by median-of-3 calibration) | rc_ai | T17 |
| B26 | ~~DONE~~ `rc_render::quality::bench` overlaps the binary's `--bench` (logs from app start) — remove or keep log-only | rc_render | T17 |
| B27 | ~~DONE~~ (T19b) `RECHARGED_UI_SCRIPT` `nav`/`pause` verbs superseded by `--input-script` (keep go/demo/tab) | rc_ui | T17 |
| B28 | ~~DONE~~ (T20c: shadowed warm-up lamp; nothing compiles after GO) Night lamps: a car entering a shadow-casting lamp's range compiles 3 shadow pipelines mid-race (warm-up cars sit far from lamps) | rc_render warm-up | T19b |
| B29 | ~~DONE~~ (T20c: menu flow compiles 0 after the warm-up; a cold split-screen `--quick-race` still compiles 2 at the start of the Intro, not at GO) Split-screen start compiles the overlay's sprite/blit pipelines at GO (frames < 30 ms) | rc_render warm-up | T19b |
| B30 | ~~DONE~~ (T20c `rc_vfx/src/splashes.rs`) Rain splashes extrapolate the plane under the car: far ones float/sink a little where the slope changes | rc_vfx | T19b |
| B31 | ~~DONE~~ (T20c menu warm-up: menu-flow loads 0.2–0.8 s; a cold automated `--quick-race` 3.7–4.8 s release, ARCHITECTURE §9) Loading → InRace is 4.6–4.8 s in a dev build after the warm-up (budget ≤ 3 s, ARCHITECTURE §9) — measure the release build; trim if over | rc_render / rc_vfx warm-up | T19b |
| B32 | ~~DONE~~ (T19a r2: raised turns' ends half banked) toy_town reverse T1: AI spins on the climb + bank onset on lap 1 (3–4 s); same hop on kerbed corners causes the remaining TT outliers | toy_town data / rc_ai | T19a (round 2 in progress) |
| B33 | ~~DONE~~ (T19a r2: kicker lead-in board) Backyard sandbox kicker flips an assisted Phantom at ~30 u/s | garden data | T19a (round 2 in progress) |
| B34 | ~~DONE~~ (T19a r2: near-miss pass, carrier 1.1×; self 64 % → 22 %) Goo Bomb still explodes on its carrier in most AI uses (10 of 20 hits) | rc_ai / rc_items | T19a (round 2 in progress) |
| B35 | ~~DONE~~ (T19a r2: throttle-only 1.21–1.29×) Assist too generous: throttle-only runs at reference pace (Silver/Gold medal pace) — target 1.2–1.3× | rc_ai assist | T19a (round 2 in progress) |
| B36 | Track select: the showroom car shows through the translucent track cards | ui / rc_render showroom | T20b |
| B37 | Cup end: podium thumbnails use one fixed foot offset, some cars sit a few px into/above their block | ui | T20b |
