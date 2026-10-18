# ReCharged — Engineering Practices

Mandatory for every agent and contributor. A task brief may **narrow** scope or add rules, but
it can never override §1 (ownership), §2 (build & run hygiene) or §7 (assets & licences).
Read this **before** writing code.

---

## 0. Before you start
1. Read `docs/GDD.md` (the sections your task cites), `docs/ARCHITECTURE.md` (§1–4 always,
   plus your subsystem's section), and this file.
2. Read the API digests relevant to you in `docs/tech/`: `bevy-0.20-notes.md` (start with its
   §0 "traps"), `rapier-notes.md`, `kira-notes.md`, `bevy-react-notes.md`. Bevy 0.20,
   bevy_rapier3d 0.37, bevy_kira_audio 0.27 and bevy-react 0.8.1 are **newer than your training
   data**: never write API calls from memory when a digest or the source can confirm them.
3. Ground truth for any API is the source: `~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/`.
   Working Bevy 0.20 + bevy-react examples: `/home/tul/Projects/bevy-react/examples/`
   (read-only — never modify anything outside this repository).
4. Find what already exists before adding anything: `rc_core` contracts, helpers in your crate,
   `rc_testkit`. Re-implementing something a crate away is the most common defect.

## 1. Ownership & collaboration
- You own only the paths listed in your task brief. **Do not edit files you do not own.** Need a
  change elsewhere? Put the exact request (type/field/function/file) under *Requests* in your
  report, or — if it blocks you — send it to the orchestrator (`SendMessage` to `main`) and keep
  working on something else meanwhile.
- **Shared files have a single editor.** Only the orchestrator, or the one agent whose brief
  explicitly names them, edits:
  - `crates/rc_core/**` — a brief may grant additive edits in one named `rc_core` module
    (e.g. `rc_core/src/vehicle.rs`); additive = new items or new fields with defaults, never
    renames/removals; everything must keep compiling;
  - the workspace `Cargo.toml`, `Cargo.lock`, `.cargo/`, `tools/ci.sh`;
  - `ui/src/bevy.ts` (generated — only the rc_ui owner regenerates it), `ui/package.json`;
  - `assets/CREDITS.md` and `assets/data/catalog.ron` — put credits/catalog changes in your report.
- New files in a shared directory (e.g. `tools/blender/lib/`) carry your area prefix
  (`<env>_*.py`, `<crate>_*`); check that a path does not exist before creating it, and never
  overwrite a file you did not create.
- UI strings live in `ui/src/strings/<area>.ts`, one file per area/owner.
- Never run commands that touch files you do not own (`cargo fmt` without `-p`, mass
  search-and-replace, `git checkout/reset/stash/clean`).
- **Do not commit.** The orchestrator reviews and commits. Read-only git (`status`, `diff`,
  `log`) is fine.
- Finish the task to its acceptance criteria, or report precisely what is missing and why.

## 2. Build, run & test workflow
| Do | Command |
|---|---|
| iterate | `cargo check -p <crate>` (add `--tests` / `--examples` when relevant) |
| test | `cargo test -p <crate>` |
| lint | `cargo clippy -p <crate> --all-targets -- -D warnings` |
| format | `cargo fmt -p <crate>` (only your crates) |
| UI | in `ui/`: `npm run typecheck`, `npm test`, `npm run build` |
| TS bindings (rc_ui owner only) | `cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts` |
| run the game | `cargo run -p recharged -- --user-data-dir target/agent-data/<task-id> <flags> --exit-after <secs>` |

- All builds share the workspace `target/` dir. "Blocking waiting for file lock" means another
  agent is building — wait; never kill other agents' processes, never set a different
  `CARGO_TARGET_DIR`, never `cargo clean`, never `cargo update` (disk and time are limited).
- **Every game run** passes `--user-data-dir target/agent-data/<task-id>` (never touch the user's
  real profile in `~/.config`) and `--exit-after`; use `--unlock-all` to reach locked content.
  Tests write only to temporary directories. Make sure no process you started keeps running.
- A game run opens a window on the user's desktop: keep runs short and purposeful.
- Screenshots and scratch outputs go to the scratchpad or `target/`, never into `assets/` unless
  they are deliverables.
- Before reporting done: `check`, `test`, `clippy` and `fmt` are clean for your crates.

## 3. Simplicity first
- The smallest change that fully solves the task wins. No speculative abstractions: no trait with
  one implementation, no generic "framework" for one use, no config for a value that never
  changes, no scaffolding "for later".
- Prefer: existing code in this repo → std → Bevy built-ins → approved dependencies → new code.
- Delete dead code instead of commenting it out.
- When you deliberately cut a corner with a known ceiling, mark it:
  `// ponytail: <the limitation>, <the upgrade path>`.

## 4. Rust & Bevy conventions
**Structure**
- Each crate exposes one top-level plugin (`TrackPlugin`, `VehiclePlugin`, …) in `lib.rs`, plus
  the public types others need. `lib.rs` starts with a `//!` doc: responsibilities, inputs
  (components/resources/messages read), outputs (written/produced), and what it must never do.
- One module per feature; aim for ≤ 400 lines per file. Systems are small (≤ ~60 lines) and do one
  thing; logic lives in pure functions next to them.
- Plugin names never clash with Bevy's (`GameInputPlugin`, not `InputPlugin`).

**ECS**
- Put every system in the `rc_core` set for its stage (ARCHITECTURE §3.2). Gameplay logic runs in
  `FixedUpdate` only; presentation in `Update`.
- **One writer per component/resource** (ARCHITECTURE §4 tables). The only exception: the
  steering-assist pass may change `CarControl.steer` of human racers in `SimSet::Assist`.
- Cross-crate communication: `rc_core` components, resources and **messages**. Observers
  (`On<Add<Racer>>`) decorate entities spawned by other crates and handle local immediate
  reactions — not a hidden cross-crate call mechanism.
- Components are plain data; derive `Component, Debug, Clone` (+ `Default`/`PartialEq` when useful).
  Use required components for defaults. Construct `rc_core` structs with `..Default::default()`
  when they have many fields so additive changes don't break you.
- Use run conditions (`in_state`, `resource_exists`, `resource_changed`) and change filters.
- Lifetimes follow the session lifecycle (ARCHITECTURE §3.1): attempt entities are **spawned in
  `OnEnter(RaceAttempt)`** (never in `OnEnter(Setup)`) with `DespawnOnExit(RaceAttempt)`; session
  entities `DespawnOnExit(RaceSession)`; menu entities `DespawnOnExit(AppState::Menu)`. Reset attempt
  resources in `OnEnter(RacePhase::Setup)`, remove session resources in
  `OnEnter(AppState::Unloading)`. Never leak across races.
- Loaders report completion with a `LoadingDone(key)` message; only rc_game writes `LoadingTasks`.
- Teleporting an `Interpolated` entity (spawn, respawn, reset) → send `Teleported { entity }`.

**Time & determinism**
- Gameplay clocks count fixed ticks; never use `Time<Virtual>` pausing, `Time<Fixed>::elapsed()`
  or wall time for gameplay. Pause = gated `SimSet`s (ARCHITECTURE §3.1).
- Never read `ButtonInput`/gamepad state in `FixedUpdate` — read rc_input's latched intents.
- Randomness only through `rc_core::rng::draw(seed, tick, stream, key)`; add a new `RngStream`
  variant for a new purpose.
- Systems in the same set that touch the same `ResMut`/component must be explicitly ordered.
- Only rc_vehicle (and rapier) write a rapier body's `Transform`; presentation uses visual proxies.

**Bevy 0.20 traps** (details in `bevy-0.20-notes.md` §0)
- No built-in `*Bundle` types (`impl Bundle` / `#[derive(Bundle)]` still exist); spawn tuples of
  ≤ 15 items (nest for more); the same component twice in one spawn panics — `.insert()` overrides.
- `ComputedStates` has no derive: hand-write the impl, and set
  `const ALLOW_SAME_STATE_TRANSITIONS: bool = false` for scope states (default `true` re-fires
  exit/enter when the computed value doesn't change).
- Buffered events are `Message`s (`MessageWriter/Reader`, `add_message`); `Event` is for observers;
  lifecycle observers are `On<Add<T>>`, `On<Remove<T>>`, `On<Discard<T>>`.
- Scenes: `WorldAssetRoot` / `WorldInstanceReady` (not `SceneRoot`). Custom shaders are **WESL**.
- Resources live on entities: `Query<Entity>` matches them — filter `Without<IsResource>` and never
  count "all entities" in tests.
- `NextState::set` re-enters the same state; use `set_if_different` unless you mean a restart.
- Headless tests call `app.finish(); app.cleanup();` before the first `update()`.
- Missing `Res<T>` panics: use `Option<Res<T>>`/`If<..>` for optional resources.

**Errors & logging**
- Runtime systems must not panic. Expected absence → `let Ok(x) = q.single() else { return };`,
  `Option<Single<..>>`, or a fallible system returning `Result` with `?`. No `unwrap()` in runtime
  code; `expect("why this cannot fail")` only for true invariants or startup-time programmer errors.
- Bad data files: fail at load with a precise message (file, field); in the running game surface
  it as an error dialog and return to the menu (GDD §19), never a panic.
- Log lifecycle events with `info!`, details with `debug!`. Never log every frame; use
  `warn_once!`/`error_once!` for recurring problems.

**Numbers & data**
- No magic numbers in systems: tuning values live in the subsystem's RON data file, or in named
  `const`s with the unit in the name or doc (`const ORB_SPEED_UPS: f32 = 24.0; // units/s`).
- Units: world units (1 u = 22.5 cm; car ≈ 2.0 u), seconds, radians internally (degrees only in
  data files). Y up, **−Z forward**, +X right.
- No `unsafe`, no global mutable statics, no threads of your own.

## 5. Testing
- Pure logic gets unit tests in the same file (`#[cfg(test)] mod tests`): splines, racing line,
  lap/standings rules, odds, steering maths, input mapping, unlock/points rules, serialization,
  mixing maths, layout maths. Test edge cases (start-line wrap-around, reverse/mirror, empty
  inputs, ties).
- ECS behaviour gets headless integration tests in `crates/<crate>/tests/` using `rc_testkit`
  (dev-dependency, **only from `tests/`**): no window, GPU or audio device; manual time;
  deterministic.
- Every data file is loaded and validated by a test of its owning crate.
- Test names state the behaviour: `lap_counts_only_after_all_checkpoints`.
- Keep the default suite fast (each test ideally < 2 s). Long simulations (full races on every
  track, soak) are `#[ignore]` with a comment saying how to run them; QA runs them.
- Never weaken or delete a failing test to get green — fix the code or report the problem.
- UI: pure TS logic (formatting, navigation model, stores) gets `*.test.ts` run by `npm test`
  (`node --test`, no extra dependencies).

## 6. Performance
- No allocations per frame in hot paths (reuse buffers via `Local<Vec<_>>` or components).
- No O(n²) over unbounded sets; O(racers²) with ≤ 8 racers is fine.
- Never load assets per frame; load during `Boot`/`Loading`, keep handles in resources.
- Share meshes and materials (clone handles) so Bevy can batch; bounded particle counts.
- Respect the budgets in ARCHITECTURE §9; measure (`--bench`, frame-time diagnostics) when in doubt.

## 7. Assets & licences
- Assets are original (made by our generators) or **CC0**. Exception: **fonts may be SIL OFL 1.1**
  (ship the licence file next to the font). Nothing with an unclear licence, nothing imitating a
  real brand (no real logos, trademarks or platform button logos). Put every external source in
  your report (what, author, URL, licence) — the orchestrator maintains `assets/CREDITS.md`.
- Generators in `tools/` are the source of truth (Blender Python, Python + numpy/PIL, ffmpeg).
  Re-running a generator reproduces its output. Deliver script + output.
- **Look at what you make**: render previews (Blender) or plots and inspect them before
  delivering. Visual quality is a requirement. Audio: you cannot listen — verify with waveform /
  spectrogram plots, loudness and loop-seam checks, and sound-design knowledge.
- Models: glTF binary (`.glb`), metallic-roughness PBR, applied transforms, real scale per GDD §4.2
  (props = real size × 4.44). Blender: front toward **+Y**, Z up, origin at the ground-contact
  centre; export with "+Y Up" so the front ends up at Bevy **−Z**. Budgets: car ≤ 25 k tris, props
  ≤ 5 k (hero ≤ 15 k). Naming per ARCHITECTURE §7. Follow `docs/design/art-bible.md`.
- Textures: ≤ 1024² (hero ground materials ≤ 2048²), PNG or KTX2; sRGB for colour, linear for
  normal/roughness/AO.
- Audio: OGG Vorbis, 44.1/48 kHz; SFX mono unless stereo matters; seamless loops; peaks ≤ −1 dBFS;
  music around −14 LUFS.
- File names `snake_case`; ids in data files match file names.

## 8. UI (React / bevy-react)
- TypeScript strict; function components and hooks only.
- Talk to Bevy only through the generated `ui/src/bevy.ts` client; never edit it by hand.
- **Navigation comes only from the `ui.nav` event** (+ pointer events). Never subscribe to raw
  `gamepadInput`/`keyDown` for navigation; never give menu buttons `tabIndex`.
- Every store fetches a **snapshot request on mount** (events are not buffered) and lives in an
  hmr-safe singleton on `globalThis`; every effect has a cleanup (hot reload re-runs effects).
- Never `setState` per frame. Per-frame visuals use shared values driven from Rust; HUD text comes
  from the batched `race.hud` event (≤ 10 Hz).
- No `<anchor>` in the race HUD (it follows only the UI camera); 3D name tags are rc_vfx's job.
- Split-screen panels use translucent fills (no `backdropFilter`); single-view menus may use it.
- Every interactive element is reachable and operable with gamepad, keyboard and mouse, shows
  hover/focus/press states and plays a UI sound cue.
- Strings in `ui/src/strings/<area>.ts`; colours, fonts, spacings and motion from `ui/src/theme/`.
- Components ≤ ~250 lines; screens are compositions of components; an ErrorBoundary wraps screens.
- Must look right at 1280×720, 1920×1080 and in 4-way split-screen; honour the HUD scale setting.
- `npm run typecheck`, `npm test` and `npm run build` pass before reporting done.

## 9. Dependencies
Approved — nothing else without explicit orchestrator approval (ask in your report):
- Rust: `bevy` 0.20 (workspace features: default features **off**, collections `3d` + `ui` +
  what the workspace lists — never re-enable Bevy's `audio`), `bevy_rapier3d` 0.37 (no `parallel`),
  `bevy_kira_audio` 0.27, `bevy-react` 0.8.1, `serde` (+derive), `ron`, `serde_json`, `ts-rs` (as
  bevy-react requires), `thiserror` (if a custom asset loader needs it).
- npm: what the bevy-react 0.8.1 template installs (React, bevy-react, TypeScript, types).
- Tools: Python 3 stdlib + numpy + PIL, Blender 5.2 (bpy/bmesh/mathutils), ffmpeg, ImageMagick.

## 10. Documentation
- `//!` crate docs and `///` docs on every public item of `rc_core` and each crate's public API.
  Comments explain *why*, not *what*.
- Each subsystem keeps a short design note in `docs/design/<crate>.md`: decisions, tuning
  rationale, how to test/verify, known limitations. Keep it current; no essays.

## 11. Task report (your final message)
```
## Summary            what was built, in 3–6 bullets
## Files              created / modified (paths)
## Contracts          rc_core additions/changes (exact items), new data files & formats
## Tests              commands run and results (counts), what they cover
## Verify             how the orchestrator can see it working (commands, flags, screenshots)
## Known issues       limitations, `ponytail:` notes, TODOs, risks
## Requests           changes needed in files you do not own; credits; dependency requests; questions
```
