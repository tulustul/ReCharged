# T17 — Automation & verification tools

**Goal:** the tools that let us prove the Definition of Done without a human: scripted input,
headless races, determinism, benchmarks and medal calibration (ARCHITECTURE §8, GDD §21 DoD).

**Read first:** PRACTICES; ARCHITECTURE §3 (lifecycle, schedules, determinism), §8; GDD §9 (medals),
§21 (DoD 1, 2, 7, 8); design notes `docs/design/{rc_core,rc_input,rc_game,rc_race,rc_ai,rc_vehicle,rc_track}.md`;
`crates/recharged/src/{main,cli,automation}.rs`; `crates/rc_testkit/src/lib.rs` (`SimPlugins`);
`crates/rc_ui/src/debug.rs` (the existing `RECHARGED_UI_SCRIPT` UI hook).

## Owned paths
`crates/recharged/**` (granted for this task), new module `crates/rc_input/src/script.rs` (+ its
one registration line in `rc_input/src/lib.rs`), `assets/data/medals.ron` (written by the
calibration tool), `docs/design/automation.md`. Requests for other crates → `SendMessage` to `main`.

## Deliverables
1. **`--input-script <file>`** (rc_input): a scripted virtual device. A small text/RON format of
   timed steps (wall-clock or race-tick based): driving intents per slot (throttle/brake/steer/drift/
   boost/item/look-back/reset hold), menu `UiNav` actions with a device kind (keyboard / gamepad), and
   pause. Injected at the same layer real devices feed (so bindings, latching, `UiNav` suppression
   rules all apply). Ship example scripts in `tools/scripts/` for DoD-1: "keyboard only" and
   "gamepad only" full flows (Title → Quick Race → race → Pause → Results → Menu; Settings change;
   Versus 2P join + race) and a CI-friendly smoke script.
2. **`--headless-race`**: no window, render or audio — MinimalPlugins + states + rapier + `CorePlugin`
   + the sim plugins + rc_game's flow (no UI), the `--quick-race` options, AI for every racer
   (autopilot humans), run as fast as possible, print `RaceResults` as JSON on stdout, exit 0.
3. **Determinism**: an automated test (in `recharged` or `rc_testkit`) running the same seeded
   headless race twice → identical result JSON; document what is (not) deterministic.
4. **`--calibrate-medals`**: headless, for every catalog track file × Fwd/Rev × mirror: Elite AI in a
   Phantom, items and catch-up off, 3 laps → reference times → write `assets/data/medals.ron` in
   rc_game's format (`reference_secs`), printing a table. Tracks without a file are skipped with a note.
5. **`--bench <scenario>`**: windowed scenarios (`toy_town_1p`, `toy_town_2p`, `toy_town_4p`, plus
   `<track>_<n>p`): autopilot race, warm-up 5 s, measure 20 s, print p50/p95/p99 frame time and fps as
   JSON; non-zero exit if the run failed.
6. Remove implemented flags from `pending_flags`; usage text current; tests for parsing and the
   script format.

## Acceptance
Example scripts drive the full keyboard-only and gamepad-only flows (screenshots at key points
inspected); `--headless-race --quick-race toy_town --ai 7 --seed 1` prints results; determinism
test passes; `medals.ron` regenerated for the tracks that exist; bench numbers for 1P/2P/4P on
Toy Town reported. `docs/design/automation.md` documents every tool.
