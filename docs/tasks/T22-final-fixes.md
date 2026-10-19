# T22 — Final fixes after the regression QA (three agents, disjoint ownership)

Fix the new issues of `docs/qa/final-report.md` (T21, build 98fd0d4: DoD 1–8 pass, R1–R7) and the
open backlog polish. Each item: root-cause fix, a test where the logic is non-trivial, the design
note of the touched area updated, and a line in your final report (the orchestrator edits
BACKLOG.md, STATUS.md, the GDD and ARCHITECTURE). GDD v2.3 (status line) already describes the
design changes of T22a.

**Read first:** PRACTICES; ARCHITECTURE §1–5 and §8–9; `docs/qa/final-report.md` (your items in
full, with repro commands); the design notes of the areas you touch; `docs/tech/bevy-0.20-notes.md`
or `docs/tech/bevy-react-notes.md` as relevant.

**Shared rules for all three agents**
- Edit only your owned paths (below). Need a change elsewhere? SendMessage `main` with the exact
  change; do not make it yourself. No commits; the orchestrator commits.
- Every game run uses `--user-data-dir target/agent-data/<task>/data/<run>`; never the real profile.
- Before every windowed run: `pgrep -a recharged`. A `recharged` without `--user-data-dir target/...`
  is the user playing: do not open a window, wait.
- GPU sharing: wrap every windowed run in `flock -s target/agent-data/gpu.lock …`; a timing-sensitive
  run (frame times, load times) takes `flock -x target/agent-data/gpu.lock …` instead. At most two
  windows per agent at once.
- The QA scripts in `tools/scripts/qa/` reproduce the issues (`live.sh`/`run2.sh` take `QA_BIN`;
  build your own binary from the live tree with `cargo build -p recharged`). Other agents edit the
  tree at the same time: for long runs, copy your binary to `target/agent-data/<task>/bin/` first.
- A game started as a background job ignores SIGINT: stop it with `kill -TERM`.
- After all changes: `./tools/ci.sh` passes.

## T22a — race start, hazards, AI traffic
**Owned:** `crates/rc_ai/**`, `crates/rc_items/**`, `crates/rc_race/**`,
`crates/rc_game/src/{race_setup.rs,series.rs}` (and rc_game tests of the grid),
`assets/data/{items.ron,ai.ron}`, additive edits in `crates/rc_core/**`; docs `rc_ai.md`,
`rc_items.md`, `rc_race.md`, and the "Race setup per mode" section of `rc_game.md`.
- **R1a Grid (GDD §3.2):** AI rivals fill the front of the grid in their seeded order; the local
  players start behind them in slot order; from the second race of a Versus session on, the humans
  line up in reverse order of the session points (leader last). Check every consumer that assumed
  humans on the front rows (intro/countdown cameras, HUD, colour-blind markers use the slot for
  humans, tests). Update the grid tests and add one for the Versus order.
- **R1b Hazards (GDD §6.2):** an Oil Slick is used up once it has caught 3 cars, a Spring Pad after
  3 launches (data in `items.ron`; the hazard fades out quickly rather than popping). Test.
- **R2 AI traffic (GDD §8.1):** an AI never queues behind a stopped or crawling car (≈ < 2 u/s, or
  far slower than itself): it steers around it like a hazard, on the roomier side; other cars
  following the same blocker must not veto every free side (`traffic.rs::choose_lane`,
  `side_bounds`, `pace.rs`). Add an rc_ai race test: a parked car on the racing line ahead of the
  AI field (e.g. on pole, 7 AI behind it) — every AI passes it within about 3 s of reaching it.
- **Verify:** `cargo test -p rc_ai --test race -- --ignored every_layout` 28/28; both determinism
  tests; hazard victims with `tools/scripts/qa/{item_stats.py,hazard_start.py}` over headless
  8-car item races (report victims per drop, overall and in the first 10 s, vs T21: Oil 2.5–3.0,
  Spring 1.5–2.25); `tools/scripts/qa/spot_idle_start.script` adapted to the new grid (a stalled car
  mid-pack); one windowed race per mode to see the new grid (Quick Race, Cup, 2P Versus with AI).

## T22b — presentation polish
**Owned:** `crates/rc_audio/**`, `crates/rc_render/**`, `crates/rc_vfx/**`, `crates/rc_ui/**`,
`ui/**`; docs `rc_audio.md`, `rc_render.md`, `rc_render_content.md`, `rc_vfx.md`, `ui.md`, `hud.md`.
- **R4** `thunder_3` must be able to play in a Storm: map the thunder variants over the strike
  intensities that actually occur (Storm strikes are 0.5–1.0), or pick weighted by intensity. Test.
- **R5** No tonemapping pipeline compiles at Intro start in non-clear weather: the loading warm-up
  camera uses the session weather's grading/tonemapping key (or the menu warm-up covers the weather
  variants). Verify with `RUST_LOG=rc_render=debug` on a few `weather_time.sh` cells: race start
  0 pipelines.
- **R6** Title: the showroom car stays visible but clear of the logo, tagline and progress bar
  (camera framing / backdrop focus), or the Room focus if that reads better. Screenshot.
- **R7** Garage and Track select footers: the move hint gets its own label ("Browse"), not
  "Select" twice.
- **B36** The showroom car must not show through translucent cards: Track select, Versus lobby
  P3/P4 cards, locked cup cards (backdrop focus per screen, or opaque cards). Screenshots.
- **B37** Cup end podium: each car stands on its block (per-car foot offset, not one constant).
- **Small:** a tied Time Trial split shows a neutral "±0.00" (not red "+0.00"); the "Couldn't save"
  toast must not cover the Settings help header.
- **Verify:** the keyboard/pad scripts still pass (`tools/scripts/{keyboard_only,gamepad_only,smoke}.script`
  — smoke ×3 —, `tools/scripts/qa/{kb_modes,pad_modes}.script`); before/after screenshots of
  R6, R7, B36, B37.

## T22c — memory growth in long sessions (R3)
**Owned:** `crates/rc_game/src/soak.rs`, `crates/recharged/**` (diagnostics only),
`tools/scripts/qa/**` (new scripts), `target/agent-data/T22c/**`; the soak lines of the
"Automation" section of `docs/design/rc_game.md`, `docs/design/automation.md` (soak section).
- **Goal:** find what keeps growing (T21: RSS 4.31 → 4.63 GB between min 12 and 30 of `--soak 30`
  at 1600×900, ≈ +20 MB/min, while the census of entities/resources/assets stays flat), and fix it
  if it is ours. Time-box the investigation to about 3 hours of wall time.
- **Method:** build your binary once and copy it. A 60-min soak (T21's seed
  `1792365836540779723`, 1600×900) sampling `/proc/<pid>/smaps_rollup` every minute
  (RssAnon / RssFile / RssShmem); heap profiles at ≈ 10, 30, 60 min (T20c used jemalloc's heap
  profiler: `target/agent-data/T20c/heap.sh`); compare allocation sites between profiles. Suspects:
  bevy-react's JS heap or retained nodes, glyph caches, wgpu/driver buffers, kira, per-race
  resources that grow (logs, histories, ghosts, records).
- **Outcome:** a fix if the growth is in ReCharged code (SendMessage `main` if it is outside your
  ownership, with the exact change); if it is in a dependency (bevy-react, Bevy, wgpu, the driver),
  the evidence and a ready-to-paste entry for `docs/upstream-notes.md` in your report. If useful,
  add RSS (from `/proc/self/statm`) to the soak's periodic log line.
