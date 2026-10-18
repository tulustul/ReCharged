# T01 — Foundation (walking skeleton)

**Goal:** create the workspace, the complete `rc_core` contract crate, crate skeletons for every
subsystem, the headless test kit, the game binary with automation flags, the session lifecycle
in a minimal `rc_game`, and a bevy-react UI package that builds and shows a placeholder menu —
so that subsystem agents can work in parallel on a running game.

**Read first:** `docs/PRACTICES.md`; `docs/ARCHITECTURE.md` (all); GDD §3–§15 (tables for data);
all of `docs/tech/*.md` (Bevy §0 traps, §1 cargo setup, §2–4, §9, §17; rapier setup & headless;
kira setup; bevy-react §1–2, §8, §9).

## Owned paths
`Cargo.toml`, `Cargo.lock`, `.cargo/`, `rust-toolchain`-free (use stable), `crates/*` (skeleton
for all crates; full implementation of `rc_core`, `rc_testkit`, `recharged`, minimal `rc_game`,
minimal `rc_ui`, minimal `rc_camera`), `ui/` (scaffold + placeholder), `assets/data/catalog.ron`,
`assets/CREDITS.md` (create), `tools/ci.sh`, `docs/design/rc_core.md`.

## Deliverables
1. **Workspace** (resolver 3, edition 2024, members `crates/*`), one `[workspace.dependencies]`
   table with every approved dependency:
   - `bevy` 0.20 with `default-features = false` and features `["3d", "ui", "jpeg", "debug"]`
     (+ anything the digests show is needed; never `audio`);
   - `bevy_rapier3d` 0.37 **without** the `parallel` feature (determinism), debug render
     available behind a cargo feature of the binary;
   - `bevy_kira_audio` 0.27 (ogg + wav), `bevy-react` 0.8.1, `serde`, `ron`, `serde_json`, `ts-rs`
     (version bevy-react requires).
   - Profiles: `dev` opt-level 1 + `debug = "line-tables-only"`; `[profile.dev.package."*"]`
     opt-level 3 + `debug = false`; `release` thin LTO.
   - A `dev` cargo feature on the binary for `bevy/dynamic_linking` + `bevy/file_watcher` **only if
     it builds and runs with bevy-react** (verify; otherwise document why not).
2. **All crates of ARCHITECTURE §2** with manifests exactly per §2.1 (declare every dependency a
   crate will need now, so later tasks rarely touch manifests), each with the `//!` crate doc
   (responsibilities, inputs, outputs, must-nots from §5) and its top-level plugin
   (`CorePlugin`, `TrackPlugin`, `VehiclePlugin`, `GameInputPlugin`, `WeatherPlugin`,
   `RacePlugin`, `AiPlugin`, `ItemsPlugin`, `GameCameraPlugin`, `GraphicsPlugin`, `VfxPlugin`,
   `GameAudioPlugin`, `GameFlowPlugin`, `GameUiPlugin`). Stubs register their `LoadingTasks` key
   only if they will load something and mark it done immediately (so the gate works today).
3. **`rc_core` complete** per ARCHITECTURE §3–§4, split into modules by area (`catalog`, `race`,
   `vehicle`, `items`, `input`, `weather`, `track`, `ui`, `flow`, `state`, `schedule`, `rng`,
   `paths`, `interp`, `settings`, `consts`):
   - states (`AppState` incl. `Unloading`; computed `RaceSession` and `RaceAttempt` as
     **hand-written** `ComputedStates` impls (no derive exists) with
     `ALLOW_SAME_STATE_TRANSITIONS = false`; sub-states `RacePhase` (default `Setup`) and
     `PauseState`);
   - system sets configured & chained in `CorePlugin` (FixedUpdate `SimSet`s gated on
     `PauseState::Running`; `FixedLast` `CoreSet::Snapshot`; Update chain
     `FlowSet → InterpolateSet → CameraSet → VisualSet → AudioSet → UiSet`; PreUpdate
     `InputSet::Collect`);
   - `Interpolated` + `PoseHistory` snapshot system (snaps `prev = curr` on `Teleported`);
     `VisualOf` relationship with `Visual` as its `linked_spawn` target;
   - every component / message / resource / enum / constant of §4 (incl. `LoadingTasks` with
     build-time `register` + the `LoadingDone` message, `AutomationConfig`, `UserPaths`,
     `LobbyState`, `TrackSummary`, `Teleported`,
     `GameCommand` lives in rc_game — not here);
   - catalog types + `Catalog::load` with validation; `Settings` + defaults (GDD §15) with RON
     round-trip; `SurfaceKind::props()` (GDD §5.2); `Effect` types (GDD §5.5) as data (the
     effects *system* belongs to rc_items); keyed RNG `rng::draw` + helpers + `RngStream`;
     `paths` (assets dir & user-data dir resolution incl. CLI/env overrides).
   - Every pub item documented. Unit tests: rng determinism & independence of streams, catalog
     validation (incl. "every cup race uses content unlocked no later than the cup"), settings
     round-trip, surface table sanity, paths resolution precedence, pose interpolation maths.
4. **`rc_testkit`**: headless app builder (MinimalPlugins, transform, states, rapier without
   debug render in the fixed schedule, `CorePlugin`, manual time at `SIM_HZ`, `app.finish();
   app.cleanup();`, FixedUpdate ambiguity detection = warn), helpers (`step_seconds`,
   `start_race(RaceConfig)`, `restart`, `next_race`, `quit_to_menu`, `count_entities::<F>()`
   excluding resource entities). Integration tests (in `crates/rc_testkit/tests/` or a crate that
   uses it): **lifecycle** (start → restart ×2 → next race → menu, **plus leaving the session
   while still in `Setup` — via quit and via a load error**: session entities spawned by a dummy
   loader and attempt entities spawned by a dummy spawner in `OnEnter(RaceAttempt)` come back to
   baseline, exactly one session at a time, attempt entities recreated on restart) and **pause**
   (120 paused frames change nothing; race ticks frozen).
5. **`recharged` binary**: window "ReCharged" 1600×900; `AssetPlugin` rooted at
   `UserPaths.assets_dir`; rapier configured for the fixed schedule (gravity from `rc_core`); all
   plugins; `ReactPlugins` with the bundle path (dev: workspace `ui/dist/app.js`; release:
   exe-relative) and `hot_reload(cfg!(debug_assertions))`; ECS error handler that logs; CLI parsing
   (std only) of **every** flag in ARCHITECTURE §8 into `AutomationConfig` (unknown flag → usage +
   exit 2). Implement now: `--user-data-dir`, `--assets-dir`, `--unlock-all`, `--exit-after`,
   `--screenshot*`, `--export-bindings`, `--quick-race` (+ its options → `GameCommand::StartRace`
   skipping the menu). Others parsed and logged once as "not implemented yet".
6. **Minimal `rc_game`**: Boot (Catalog + default Settings; no persistence yet — T07) → Menu;
   `GameCommand` message with at least `StartRace`, `Restart`, `NextRace`, `QuitToMenu`,
   `Pause`, `Resume`, `QuitGame`; the full transition table of ARCHITECTURE §3.1 incl. `Unloading`,
   restart via `RacePhase::Setup`, the `LoadingTasks` gate (rc_game sole runtime writer, fed by
   `LoadingDone` messages; min display 1 s, 0 with automation),
   pause per §3.1 (no `Time<Virtual>` pause). A placeholder phase driver may advance
   Setup → Intro → Countdown → Racing on timers until rc_race exists (clearly marked
   `// ponytail: placeholder until T06`).
7. **Minimal `rc_ui`**: module per namespace; channels `app.state` (event), `app.snapshot`
   (request), `ui.ready`, `ui.error`, `race.start`, `race.restart`, `race.quit`, `race.resume`,
   `app.quit` (messages); `register_bindings(app)` shared with the `export_bindings` example
   (`cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`) and `--export-bindings`.
   Log an error if `ui.ready` has not arrived 15 s after start (automation runs exit non-zero).
8. **`ui/`**: scaffold with `npx bevy-react init ui` (0.8.1, matching the crate), generate
   `src/bevy.ts`, then a placeholder `App.tsx` with an ErrorBoundary (→ `ui.error`), `ui.ready` on
   mount, an app-state store seeded by `app.snapshot` and updated by `app.state`: Menu → title
   "ReCharged" + buttons "Test drive" (`race.start` for `test_oval`) and "Quit"; Loading →
   "Loading…"; InRace → "Race (HUD placeholder)" + "Restart" and "Back to menu". Mouse only (real
   navigation comes with T04/T05). `npm test` (`node --test`) with one real test of the store.
9. **Minimal `rc_camera`**: single-view UI host — a full-window 3D backdrop camera marked
   `IsDefaultUiCamera` in every state (menus/loading/race) until T04 builds the real rig (game
   cameras will be session-scoped and re-bound per attempt — ARCHITECTURE §3.1).
10. **`assets/data/catalog.ron`**: the complete content of GDD v2 — cars (§4.1, 4 base paints each
    + one paint per unlockable set of §9), tracks (§7.1 with default laps and §7.2 allowed presets,
    plus a hidden dev track `test_oval`), cups (§3.1 table, normalized), rivals (§8.3 with per-class
    cars), unlocks (§9). The catalog test validates every cross-reference.
11. **`tools/ci.sh`**: `cargo fmt --all --check`, clippy `-D warnings` (workspace, all targets),
    `cargo test --workspace`, and in `ui/`: typecheck, test, build. Non-zero on any failure.
12. `docs/design/rc_core.md` (one page: contract overview, deviations) and `assets/CREDITS.md`
    (header + table, initially empty).

## Acceptance criteria
- `./tools/ci.sh` passes; the lifecycle and pause tests pass.
- `cargo run -p recharged -- --user-data-dir target/agent-data/T01 --exit-after 12 --screenshot-at 8 --screenshot <scratchpad>/t01-menu.png`
  exits 0 and the screenshot shows the React menu (look at it).
- `cargo run -p recharged -- --user-data-dir target/agent-data/T01 --quick-race test_oval --exit-after 10`
  logs Boot → Loading → InRace (Setup → … → Racing) and exits 0; clicking Restart in a manual run
  re-enters Setup without reloading.
- No subsystem logic beyond stubs (no physics, no track building) — later tasks own that.
- Report the full-build time and the `target/` size after the build.
