# T01 — Foundation (walking skeleton)

**Goal:** create the workspace, the complete `rc_core` contract crate, crate skeletons for every
subsystem, the headless test kit, the game binary with automation flags, and a bevy-react UI
package that builds and shows a placeholder menu — so that subsystem agents can work in parallel
on a running game.

**Read first:** `docs/PRACTICES.md`, `docs/ARCHITECTURE.md` (all), GDD §3–§15 tables (for data),
all of `docs/tech/*.md`.

## Owned paths
`Cargo.toml`, `Cargo.lock`, `.cargo/`, `crates/*` (skeleton level for all crates except
`rc_core`, `rc_testkit`, `recharged` which you implement fully), `ui/` (scaffold + placeholder
only), `assets/data/catalog.ron`, `tools/ci.sh`, `docs/design/rc_core.md`.

## Deliverables
1. **Workspace** `Cargo.toml` (resolver 3, edition 2024): members `crates/*`; one
   `[workspace.dependencies]` table for every approved dependency (versions per the digests;
   Bevy features chosen per `docs/tech/kira-notes.md` so that bevy_kira_audio replaces
   bevy_audio, and everything needed for 3D, glTF, PNG/JPEG/KTX2, HDR, tonemapping LUTs, UI as
   bevy-react requires, X11 + Wayland). Profiles: `dev` opt-level 1 + `debug = "line-tables-only"`;
   all dependencies opt-level 3 with `debug = false`; `release` with thin LTO.
2. `.cargo/config.toml` so Bevy resolves `assets/` from the workspace root when run with
   `cargo run -p recharged` (e.g. `[env] BEVY_ASSET_ROOT` relative) — verify it.
3. **All crates of ARCHITECTURE §2**, each with `Cargo.toml` deps exactly per §2.1, a `lib.rs`
   with the `//!` crate doc (responsibilities, inputs, outputs, must-nots from §5) and its
   top-level plugin (empty body or minimal stub). Plugin names: `CorePlugin`, `TrackPlugin`,
   `VehiclePlugin`, `GameInputPlugin`, `WeatherPlugin`, `RacePlugin`, `AiPlugin`, `ItemsPlugin`,
   `GameCameraPlugin`, `GraphicsPlugin`, `VfxPlugin`, `GameAudioPlugin`, `GameFlowPlugin`,
   `GameUiPlugin`.
4. **`rc_core` complete** per ARCHITECTURE §3–§4: states (incl. computed `RaceSession`, sub-states
   `RacePhase`, `PauseState`), system sets configured/chained in `CorePlugin` (FixedUpdate sets
   gated on `InRace` + `Running`), every component / message / resource / enum / constant listed,
   catalog types + `Catalog::load`, `Settings` with defaults from GDD §15, `SurfaceKind::props()`
   from GDD §5.2, `StatusEffects` + the `ApplyEffect` processing system, `rc_core::rng`,
   `rc_core::paths`, collision layer constants, `LoadingProgress`, `AutomationConfig` (parsed
   flags, see 6). Every pub item documented. Unit tests: rng determinism, catalog validation,
   status-effect stacking/expiry maths, settings round-trip through RON, surface table sanity.
5. **`rc_testkit`**: `sim_app()` → headless `App` (MinimalPlugins, transform, states, rapier
   without debug render stepping in `FixedUpdate`, `CorePlugin`, manual time at `SIM_HZ`),
   helpers `step_seconds(&mut App, f32)`, `enter_race(&mut App, RaceConfig)`. Integration test:
   the app reaches `AppState::InRace` and runs 120 fixed ticks.
6. **`recharged` binary**: window "ReCharged" 1600×900; rapier configured (gravity from
   `rc_core`, fixed schedule); all plugins; `ReactPlugins` (bundle `ui/dist/app.js`); CLI
   parsing (std only) of every flag in ARCHITECTURE §8 into `AutomationConfig`. Implement now:
   `--exit-after`, `--screenshot/--screenshot-at/--screenshot-every`, `--export-bindings`,
   `--quick-race` (rc_game builds a `RaceConfig` and skips the menu). Others may be parsed and
   ignored until their subsystem lands (log once). Set the ECS error handler to log (not panic)
   in the binary.
7. **Minimal `rc_game`**: Boot → loads `Catalog` (and default `Settings`) → `Menu`; handles a
   start request → builds `RaceConfig` → `Loading` → waits for `LoadingProgress` empty → `InRace`;
   a quit-to-menu request → `Menu`; `app.quit` → `AppExit`. (Real modes/persistence come in T07.)
8. **Minimal `rc_ui`**: channels `app.state` (event: app state, race phase, paused), `race.start`
   (message: track id, car id, laps, ai count), `race.quit` (message), `app.quit` (message);
   bindings export wired into `--export-bindings`.
9. **`ui/`**: scaffold with `npx bevy-react init ui` (0.8.1, matching the crate), generate
   `src/bevy.ts`, then a placeholder `App.tsx` that switches on `app.state`: Menu → title
   "ReCharged" + buttons "Test drive" (sends `race.start` for track `test_oval`) and "Quit";
   Loading → "Loading…"; InRace → "Race (HUD placeholder)" + a "Back to menu" button. Keep it
   minimal — T05 builds the real UI. Add `npm test` (node `--test`) with one trivial test.
10. **`rc_camera` stub**: spawn the persistent `UiCamera` and a clearing backdrop camera so the
    UI renders in every state.
11. **`assets/data/catalog.ron`**: the complete content of GDD §3.1 (cups), §4.1 (cars, 4 paints
    each + the unlockable paint sets of §9), §7.1 (tracks + a `test_oval` dev track flagged
    hidden), §8.3 (rivals), unlock rules of §9. A test validates every cross-reference.
12. **`tools/ci.sh`**: `cargo fmt --check`, clippy `-D warnings` (all targets, workspace), `cargo
    test --workspace`, and in `ui/`: typecheck, test, build. Exits non-zero on any failure.
13. `docs/design/rc_core.md`: one page — contract overview and any deviation from ARCHITECTURE.

## Acceptance criteria
- `./tools/ci.sh` passes.
- `cargo run -p recharged -- --exit-after 12 --screenshot-at 8 --screenshot <scratchpad>/t01-menu.png`
  exits with code 0 and the screenshot shows the React menu (look at it).
- `cargo run -p recharged -- --quick-race test_oval --exit-after 10` logs Boot → Menu/skip →
  Loading → InRace and exits with code 0.
- No subsystem logic beyond stubs (no physics, no track building) — that belongs to later tasks.
