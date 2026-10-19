# Automation & verification tools (design note, T17)

Tools that prove the Definition of Done (GDD §21) without a human. All live in the binary
(`crates/recharged`) except the input script device (`crates/rc_input/src/script.rs`).

| Tool | Proves | Window | Output |
|---|---|---|---|
| `--input-script <file>` | DoD 1: every flow with keyboard only / gamepad only | yes | exit code, `shot` screenshots |
| `--headless-race` | DoD 2, 8: races run end to end, AI only | no | results JSON on stdout |
| determinism test | DoD 8: deterministic replay of a seeded race | no | `cargo test -p recharged` |
| `--calibrate-medals` | GDD §9 medal targets for every layout | no | `assets/data/medals.ron` + table |
| `--bench <track>_<n>p` | DoD 7: frame-time budget | yes | frame-time JSON on stdout |

Exit codes: 0 success, 1 failure (named in the log), 2 usage error. Logs go to stderr, so stdout
carries only the JSON/table.

**Every automated windowed run** (`AutomationConfig::is_automated()`: `--quick-race`,
`--input-script`, `--exit-after`, `--screenshot`, `--bench`, tours, …) opens its window with
`PresentMode::AutoNoVsync` and `WinitSettings` continuous whether focused or not, and compiles
pipelines synchronously. Runs then depend neither on the monitor (a DPMS-blanked screen stalls
vsync'd presentation to ~1 fps) nor on the window having the focus (Bevy throttles an unfocused
window to 60 Hz). rc_render does not re-enable vsync from `Settings` in automated runs. Human runs
keep vsync and Bevy's defaults, compile pipelines asynchronously and defer their exit (window
close, Quit): `crates/recharged/src/quit.rs` holds every `AppExit` and close request back
until rc_render's `PipelineQueue` has been idle for 2 frames (at most 3 s, then it exits anyway
with a warning), so quitting mid-compile cannot drop the GPU device under a compile task (the
NVIDIA exit crash, exit 139). It logs `exit: pipeline queue idle after N s`.

## `--input-script <file>`: a scripted virtual device
**Layer.** Steps press keys of both keyboard schemes in `ButtonInput<KeyCode>` and buttons/sticks
of virtual gamepads (entities with a `Gamepad` component, spawned at startup for every pad the
script names) right after Bevy's `InputSystems` and before `InputSet::Collect` — exactly where
real devices land. So bindings (`Settings.controls`, rebinds included), the roster (single
player owns everything; lobby joins by Confirm), edge latching, the steering ramp, `UiNav`
suppression while racing, pause rules and glyph switching (`LastUsed`) all run unchanged. Keys
are re-pressed every frame (a focus loss releases every key). React never sees raw key events,
only `ui.nav` — as with real devices.

**Format** (one step per line, `#` comments):
```text
[<when>] <step>
when     <secs> after the previous step (wall clock, default 0) | @<tick> once RaceStatus.race_ticks ≥ tick
wait     ui | menu | loading | race | intro | countdown | racing | finished | paused | running
         | screen <name>   the screen React reports (`ui.screen`): a menu route (title, main,
                           trackSelect, carSelect, settings …), loading, race, pause, results,
                           standings, score, changeTrack
nav      <device> up|down|left|right|confirm|back|prevTab|nextTab   tap the fixed menu key (GDD §12.2)
hold     <device> <control> <value>   held until changed: steer -1…1, the others 0…1
tap      <device> <control>           held 0.1 s (not steer)
release  <device>                     let go of everything on the device
shot     <name>                       screenshot → <user-data-dir>/screenshots/<name>.png
exit                                  quit (after pending screenshots), exit 0 if the UI is healthy
device   keyboardA | keyboardB | gamepad (= pad 1) | gamepad2 … gamepad4
control  throttle brake steer drift item boost lookBack reset camera pause
```
- A control presses the device's **first binding** of the action: keys are down from 0.5; pad
  buttons get the value as analog travel (digital from 0.75, Bevy's threshold); sticks get ±value.
  Nav keys: A = WASD/Space/Esc, B = arrows/Enter/Backspace, both Q/E for tabs; pads = D-pad,
  A/B, LB/RB.
- `wait ui` waits for React's `ui.ready` (the binary mirrors `rc_ui::UiStatus` into
  `InputScript::ui_ready`); start every script with it. `wait screen <name>` waits until React
  reports that screen (mirrored into `InputScript::screen`; rc_ui logs every change as
  `UI screen: <name>`): use it after the splash and wherever a step needs a screen to be up,
  instead of guessing a delay — the first frames may stall for seconds while pipelines compile.
- A `nav` step also waits ≥ 4 frames after the previous step: React needs a few frames to show a
  dialog or the pause menu, and at low frame rates (a DPMS-blanked monitor stalls vsync'd
  presentation to ~1 fps; software rendering in CI) wall-clock delays alone outrun it. The smoke
  script passes at ~1 fps.
- Errors at load (`--input-script <file>:<line>: <problem>`) exit with code 2 before a window opens.
- `--exit-after <s>` is the run's timeout: if the script has not finished by then the exit code is
  1 and the log names the step and the screen React showed (`--input-script did not finish; it
  is at line 27: wait menu (screen: pause)`). `RUST_LOG=info,bevy_react_core::js=debug` adds a
  line per `ui.nav` with the screen that got it, and every focus move.
  The rumble warnings gilrs prints for the virtual pads are filtered from the log.

**Writing scripts.** Held driving inputs also act in menus like real ones (keyboard A's throttle W
is nav Up, Space is drift *and* Confirm): `release` before menus or a pause. Keep taps of one key
≥ 0.1 s apart (closer ones merge into one press). Track select: focusing a card previews it (laps
reset to its default); Confirm selects it and puts the focus on Start / Choose car, so Confirm,
Confirm takes the focused track with its options; Up from there reaches every option (from Start:
Time of day, Weather, Items, AI difficulty, Rivals, Laps, Mirror, Layout). Scripts assume default
settings and a fresh profile: give each run its own fresh `--user-data-dir`. Use
`--window <W>x<H>` (i3 floats a fixed-size window at that size). Park the mouse pointer outside
the window: focus follows real pointer motion only, but a hand on the mouse still wins.

**Shipped scripts** (`tools/scripts/`):
| Script | Flow | Time |
|---|---|---|
| `keyboard_only.script` | Title → Settings (Graphics → VSync off, Accessibility → Steering assist on) → Quick Race (Backyard Bash, 1 lap, Dumpster) → drive (throttle, boost, item, look back, camera ×3, reset hold, scheme-B item, drift) → Pause → Resume → finish → Results → Menu → Versus: P1 joins with Space, P2 with Enter, car/ready → track (1 lap) → 2P split-screen race → P2 pauses → Quit → menu | ~2 min |
| `gamepad_only.script` | the same on virtual pads (two pads in Versus; analog trigger and stick values; LB for the tab) | ~2.5 min |
| `smoke.script` | CI: Title → Quick Race → drive 4 s → pause → quit → menu → exit, waiting on each screen (`wait screen`) instead of fixed delays | ~40 s (2 min at 1 fps) |

```sh
rm -rf target/agent-data/kb && cargo run -p recharged -- --user-data-dir target/agent-data/kb \
  --window 1600x900 --input-script tools/scripts/keyboard_only.script --exit-after 400
```
Screenshots `kb_01_main_menu … kb_14_menu` / `pad_01 … pad_14` land in the data dir's
`screenshots/`; `settings.ron` there shows `vsync: false` and `steering_assist: true` afterwards.

**Relation to `RECHARGED_UI_SCRIPT`** (rc_ui): that hook only drives React fixtures (`go`,
`demo`, `tab`) for UI screenshots. Navigation and pausing are scripted here, through the real
input path (what DoD 1 must prove); its old `nav`/`pause` verbs were removed (B27) and are
rejected with a pointer to `--input-script`.

## `--headless-race`
`MinimalPlugins` + transforms + states + the sim plugins (`main.rs::add_sim_plugins`, shared with
the game: core, physics, track, vehicle, weather, race, AI, items) + rc_game's flow — no window,
render, audio, input or UI. `TimeUpdateStrategy::FixedTimesteps(1)`: every update is exactly one
60 Hz tick, as fast as the CPU allows (Toy Town, 8 cars, 3 laps ≈ 83 s of race in ≈ 8 s). Takes
the `--quick-race` options; every human gets the autopilot (rc_race: Elite AI, neutral
personality). rc_game runs the real flow (unlocks, saves into `--user-data-dir`), so locked
content needs `--unlock-all`. The race ends by rc_race's rules: when every human (autopilot
included) has finished, else by the soft-lock guard; rivals still racing get projected times.
```sh
cargo run -p recharged -- --user-data-dir target/agent-data/x --headless-race --quick-race toy_town --ai 7 --seed 1
```
```json
{"variant":{"track":"toy_town","reverse":false,"mirror":false},"mode":"QuickRace","laps":3,"seed":1,
 "geometry_hash":7815978084218388903,"entries":[{"index":0,"name":"Player","car":"sparky","slot":0,
 "rank":1,"finished":true,"time_ticks":4958,"best_lap_ticks":1547}, …]}
```
`entries` are rc_core's `RaceResultEntry` (ranked, ticks at 60 Hz); the Time Trial ghost is left
out. Failures (unknown track, load error, no result after 30 simulated minutes) exit 1.

## Determinism
`a_seeded_headless_race_repeats_exactly` (default suite, ~2 s: test_oval, 3 rivals, 1 lap) and
`a_full_field_on_toy_town_repeats_exactly` (`#[ignore]`, ~20 s: the acceptance race) build the
headless app twice in one process with fresh data dirs and compare the result JSON byte for byte;
the CLI gives identical JSON across processes too (checked with three runs).
- **Deterministic:** the simulation given `RaceConfig` (seed) and the input stream, in headless
  runs: one tick per update, keyed RNG (`rc_core::rng`), ordered sim sets, rapier without
  `parallel`. Same command line + same `settings.ron` (AI difficulty and catch-up feed
  `RaceConfig`) → same JSON; records, ghosts and the Golden Battery state never change results.
- **Not deterministic:** windowed runs — a frame runs 0…n ticks and state transitions apply once
  per frame (GO, phase changes move by a tick), input is sampled per frame, `--input-script` times
  are wall clock; races without `--seed` (rc_game seeds from the clock); wall-clock data (play
  time, session logs, loading-screen timing); other machines/compilers (no cross-platform float
  determinism — rapier's `enhanced-determinism` is off).

## `--calibrate-medals`
For every catalog track with a `data/tracks/<id>.ron`, each layout (forward/reverse × mirror) runs
headless as a Time Trial — rc_game's TT rules give 3 laps, items and catch-up off, no rivals — with
the Phantom on autopilot (Elite), `--unlock-all`, seeds 1, 2, 3, in a private temp data dir (no
ghosts/records reach a profile). The median time (seeds vary the AI's mistakes by ~2 %, about the
gold/platinum gap) becomes `reference_secs["<track>[_rev][_mirror]"]` (2 decimals, rc_game's
format, validated by `rc_game::MedalTable::parse` before writing). Tracks without a file keep their
old entries (so rc_game's coverage test holds); a layout whose autopilot does not finish keeps its
old entry and makes the run exit 1. Prints runs, reference and the four targets per layout. ~5 min.
```sh
cargo run -p recharged -- --user-data-dir target/agent-data/x --calibrate-medals
```
Rerun after changing a track, the AI or the Phantom.

## `--bench <track>_<n>p`
The CLI turns the scenario into `--quick-race <track> --players <n> --autopilot --seed 1
--unlock-all` (other race options such as `--weather` may be added; `--quick-race`/`--players` are
rejected; locked tracks bench like the others) and a 1920×1080 window unless `--window`; the binary forces a scale factor of 1 (the budget means
physical pixels; this desktop's DPI scale would otherwise render 2240×1260) and starts from
`Settings::default()` with vsync off (inserted before rc_game's boot, which keeps it and never
saves it): the same High quality whatever the profile, frame cost rather than the display rate
(no vsync and continuous updates as in every automated run). After GO and a 5 s warm-up it
records `Time<Real>` deltas for 20 s, prints one JSON line and exits; exit 1 if GO does not come
within 180 s, the race ends during the measurement or the UI is unhealthy.
```json
{"scenario":"toy_town_1p","window":[1920.0,1080.0],"frames":3950,"secs":20.0,"fps":197.47,
 "frame_ms":{"p50":4.99,"p95":5.98,"p99":6.82,"max":16.09}}
```
Measure with nothing else running (other agents' games and builds compete for the GPU/CPU) and
the window uncovered.

**Toy Town, RTX 3070, dev profile**, measured while two other games ran (pessimistic; QA re-measures in isolation):
| Scenario | fps | p50 ms | p95 ms | p99 ms |
|---|---|---|---|---|
| toy_town_1p | 197.5 | 4.99 | 5.98 | 6.82 |
| toy_town_2p | 115.1 | 7.17 | 18.18 | 39.67 |
| toy_town_4p | 85.5 | 11.29 | 14.33 | 20.67 |

## `--soak <minutes>` and memory
The soak runner is rc_game's (`docs/design/rc_game.md`, Automation). Its census sees ECS and asset
leaks only. wgpu objects, driver memory and allocator growth show only in RSS. Every census line
therefore also logs `RSS … MB (anon …, file …, shmem …), wgpu <allocated>/<reserved> MB in <n>
allocations, entities <spawned> spawned / <indices> indices, <k> font atlases (<m> MB)`.

Long runs use `tools/scripts/qa/memsoak.sh <name> --window 1600x900 --soak 60 --soak-seed <s>`:
- It runs a pinned binary copy (`BIN=…`, assets and UI bundle snapshots in
  `target/agent-data/T22c/bin`).
- Every minute it samples `/proc/<pid>/status` and `smaps_rollup`; every 5 min it copies `smaps`.
  `smaps_split.py` splits a copy, or the growth between two, by mapping.
- `JEMALLOC=35` also preloads jemalloc with heap dumps (one per 32 GiB allocated, ≈ 20 s).
  `heap_diff.py` symbolizes them (llvm-symbolizer) and prints the growth between two dumps, or
  with `--match <regex>` the timeline of one allocation site.

Reading RSS with the NVIDIA driver: its system-memory allocations are mappings of
`/dev/nvidiactl`, and VmRSS (what `ps` shows) exceeds the page-walk Rss of `smaps_rollup` by
RssShmem. Diff the parts (RssAnon, the `/dev/nvidiactl` mappings, V8's heap below 0x7e00…), not
VmRSS alone. Under memory pressure the kernel also evicts the binary's file pages, so RssFile
drops.

What a long soak shows (T22c, QA R3, 1600×900):
- **Bevy 0.20 bind group leak** (`docs/upstream-notes.md` U5): without the rc_render workaround,
  live wgpu allocations grow by ≈ 1,100 per race and RSS by 22–26 MB/min. With it they stay at
  1.4–2.0 k, and after the first visits RSS is flat to +4 MB/min (≈ 10 min).
- **Entity indices** (U6): `DespawnOnExit` never frees them. The `indices` figure doubles every
  few dozen races (≈ 12 k per race, 2.1 M after 170) while the spawned count stays at ≈ 950; most
  of the remaining ≈ 1 MB/min is the tables sized by the largest index.
- Flat: the JS heap (V8, ≈ 25–35 MB), bevy-react (≈ 12 MB), kira's decoded sounds, and
  `/dev/nvidiactl` (+0.3 MB/min). The driver's command-buffer memory under
  `vkAllocateCommandBuffers` plateaus at ≈ 370 MB within 10 min. The font atlases grow
  65 → 73 → 76 MB at 10 / 30 / 60 min.

## Known limitations
- `@tick` steps fire in the first frame that sees the tick (frame resolution, not tick-exact):
  input is injected where devices are, before the frame's fixed ticks.
- Two taps of one input within 0.1 s merge into one press; a tap on a held key makes no edge.
- `wait` has no timeout of its own; `--exit-after` bounds the run.
- Bench numbers include first-use pipeline compiles after the warm-up (automated runs compile
  pipelines synchronously to avoid the NVIDIA exit crash).
- rc_render's `quality/bench.rs` also logs frame-time percentiles every 5 s during `--bench`.
