# T07 — Game flow, modes, progression & persistence (`rc_game`)

**Goal:** everything between the menus and the race: modes, cup runs, Versus sessions, Time Trial
ghosts, results processing, unlocks, records, saves — robust and tested.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §3.1 (lifecycle — T01 implemented the skeleton),
§5.12, §6; GDD §3, §8.3 (rival cars), §9, §12.3 (disconnect, focus loss), §15, §19.

## Owned paths
`crates/rc_game/**`, `assets/data/medals.ron` (placeholder values until calibration),
`docs/design/rc_game.md`. **rc_core grant:** additive edits in `crates/rc_core/src/flow.rs` only.

## Deliverables
1. **`GameCommand`** handling (complete list in ARCHITECTURE §5.12) with validation against the
   profile's unlocks (`--unlock-all` overrides in memory).
2. **RaceConfig building** per mode: grid composition (humans + rivals; rival car by class rule GDD
   §8.3; difficulty by cup / option), laps (`default_laps` / cup override / TT 3), weather choice
   within allowed presets, items, catch-up, seed, `golden_battery` (forward layout, single-player,
   not collected), ghost (TT).
3. **Restart semantics** (review 2, N1): restart = `RacePhase::Setup`; owners spawn attempt entities
   in `OnEnter(RaceAttempt)`; any exit from a session (incl. while still in `Setup`, and load
   errors) goes through `Unloading` and must leave no session/attempt entities behind (extend the
   rc_testkit lifecycle test for the paths you add).
4. **Cup runs** (`CupRun`): points 10-8-6-5-4-3-2-1, tie-breaks, restart reverting points, next
   race via Unloading → Loading, cup end with trophy; quit confirmation handled by UI.
5. **Versus**: lobby open/close (`LobbyState`), session scoreboard (`VersusSession`), rematch / next
   track; dropped player → AI takeover.
6. **Results processing** (pure functions, unit tested): records (best lap any SP mode, best race TT
   only, with car and geometry hash; mismatched hash → discard with notice), medals from
   `medals.ron`, trophies, Golden Batteries, stats, unlock evaluation over `Catalog.unlocks` + car/
   track/cup rules, hints seen.
7. **Loading gate**: rc_game is the only runtime writer of `LoadingTasks` (fed by `LoadingDone`).
   **Pause & robustness**: pause/resume per ARCHITECTURE §3.1; window focus loss → pause;
   controller disconnect → pause + dialog event; load errors → Unloading → Menu + error event.
8. **Persistence** in `UserPaths.data_dir`: `settings.ron` (≤ 1 write/s), `profile.ron`,
   `ghosts/<variant>.ghost`; versioned with migrations (pure, tested), atomic writes (temp +
   rename), corrupt → backup + defaults + toast; logs of the last 3 sessions in `logs/`.
9. **Automation**: `--quick-race` options incl. `--mode tt|versus|cup:<id>`, `--unlock-all`, and the
   soak runner (`--soak <minutes>`: chain random races, assert entity counts return to baseline).

## Tests
Unit: unlock evaluation (every rule kind), points & tie-breaks, cup state machine (incl. restart),
records/medals/hash logic, migrations, settings throttling. Headless (rc_testkit): a full cup flow
with placeholder racers; save/load round trip in a temp dir; corrupt file recovery.

## Acceptance
All tests pass; clippy/fmt clean; `docs/design/rc_game.md` documents commands, events and save
format.
