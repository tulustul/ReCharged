# GDD Review 2 (delta)
Verdict: APPROVED WITH REQUIRED CHANGES

Scope: only the review-1 blocking items B1–B9, plus anything the v2 revision broke.
Reviewed: `GDD.md` v2, `ARCHITECTURE.md` v2, `PRACTICES.md` v2, `docs/tech/*.md`.

**Condition.** Two small fixes, N1 and N2 below, are required. Each is a few sentences in the
docs plus one test case. Once they are in the docs and in the T01/T04/T07 briefs, the GDD
package is **approved**; no review 3 is needed. Everything else in this file is optional.

**Verification.**
- I re-ran the Bevy 0.20 state probe against the v2 lifecycle: `Unloading`, computed
  `RaceSession`/`RaceAttempt` with `ALLOW_SAME_STATE_TRANSITIONS = false`, and restart via `Setup`.
- I checked the new PRACTICES "Bevy 0.20 traps" against the registry sources. `On<Add<T>>`
  (lifecycle events are generic, `bevy_ecs-0.20.0/src/lifecycle.rs:352`), `IsResource`,
  `set_if_different`, `If<T>` and `WorldAssetRoot`/`WorldInstanceReady` all exist as stated.

## Per-item status

| Item | Status | Where it is fixed | Remaining |
|---|---|---|---|
| B1 Session lifecycle | **partially** | ARCH §3.1 (states, transition table, `LoadingTasks` gate, cleanup rules), §6.2, §8 testkit; PRACTICES §4; GDD Pillar 5 | **N1** below: attempt entities leak when a session ends during `Setup` (reproduced); pseudo-code doesn't compile |
| B2 Pause freezes UI | resolved | ARCH §3.1 "Pause"; PRACTICES §4 "Time & determinism" | related new issue **N2** |
| B3 Contract ambiguities | resolved | ARCH §1.4, §3.2 (Drivers/Assist/Effects sets), §3.4 RNG, §4.2 `Controller::Human{autopilot}`, §4.3, §4.5 (`RespawnCar{kind}`, `DrainCharge`), §4.6, §5.5, §5.7; GDD §5.5 effects table, §5.1 cap | minor N5 |
| B4 Menu input → React | resolved | ARCH §4.5 `UiNav`, §5.3, §5.13 (`ui.nav`, logical `nav/`, per-slot contexts); GDD §12.2–12.3; PRACTICES §8 | minor N9 |
| B5 Scale vs lap length | resolved | GDD §4.2, §5.1, §7.1 (lap lengths, `default_laps`, width rules); PRACTICES §4 units | — |
| B6 Content tables | resolved | GDD §3.1 cups (all enum values; every weather kind and time used; unlock order holds with Wrench Works), §7.2 presets and Golden Battery, §8.3 rivals, §5.2 surfaces, §9; ARCH §4.1 (`allowed_*`, `default_laps`, `CupRace.laps`, `ClassCars`, `Unlockable`) | — |
| B7 Track data model | resolved | ARCH §5.1 (`dir`, ribbon styles, `ground`, `offroad_margin`, mirror without mesh mirroring, elevation plot); GDD §5.4, §7.2–7.3 | minor N8 |
| B8 Complete-game flows | resolved | GDD §3.2–3.3, §12.3, §13, §14 (UI-20/21/22), §15, §19; ARCH §5.12 `GameCommand` | — |
| B9 Practices conflicts | resolved | PRACTICES §1 (single-editor files, per-area strings), §2 (`--user-data-dir`, `--unlock-all`), §7; ARCH §4.7 paths, §8 | fonts: accepted (see below) |

**Fonts (B9).** Accepted as an orchestrator decision. Review 1 was wrong to call
"original or CC0" a user mandate: it was in the project description, not in the user's quoted
requirements. Shipping the SIL OFL 1.1 licence file next to each font (PRACTICES §7) is
sufficient.

**Accepted with conditions.**
- The split-screen UI overlay rig (ARCH §5.8) has only been GPU-verified with plain bevy_ui. ARCH
  already requires a screenshot-verified bevy-react prototype first, with a documented fallback.
  Keep that order.
- TAA only in single view: consistent with the digest.

## Required changes

### N1 — Attempt entities spawned in `Setup` sit outside their own scope (part of B1)
**Location:** ARCH §3.1 (pseudo-code and "Restart"), §4.3 ("Spawned by rc_race in
`OnEnter(RacePhase::Setup)`"), §5.5, §6.1; brief T01 (lifecycle test).

**Problem.** `RaceAttempt` is absent during `Setup`, and that is what makes restart work. But
racers, the ghost car, the Golden Battery, pickups and cameras are spawned *in*
`OnEnter(RacePhase::Setup)` while the scope is absent. If the session ends before the phase
leaves `Setup` (a quit in the same frame as a restart, an error raised during Setup, or the M1
placeholder phase driver), `RaceAttempt` never exits and those entities leak into the Menu.

Probe output for `InRace (Setup) → Unloading → Menu`: tracks 0, **racers 8**.

Spawning in `OnEnter(RaceAttempt)` instead is clean on every path: restart ×2, next race via
`Unloading`, quit during Setup (racers 0 in Menu).

**Fix.**
- Replace the rule with: "Attempt entities are spawned in `OnEnter(RaceAttempt)`.
  `OnEnter(RacePhase::Setup)` only resets attempt resources." Change §3.1, §4.3, §5.5 and §6.1
  accordingly.
- Add "leave the session while in `Setup`" (quit and load error) to the rc_testkit lifecycle
  test and to T01.
- **The B1 mechanism must also be exact.** `ComputedStates` has no derive in Bevy 0.20 (only
  `States` and `SubStates`, `bevy_state_macros-0.20.0/src/lib.rs`). So `#[derive(ComputedStates)]`
  in §3.1 does not compile, and the flag must live in a hand-written impl:
  ```rust
  impl ComputedStates for RaceAttempt {
      type SourceStates = RacePhase;
      const ALLOW_SAME_STATE_TRANSITIONS: bool = false;
      fn compute(p: RacePhase) -> Option<Self> { (p != RacePhase::Setup).then_some(Self) }
  }
  ```
  `RaceSession` is the same, with `SourceStates = AppState`.
- Add this trap to `bevy-0.20-notes.md` §0/§4. That digest is the agents' API ground truth and
  never mentions `ALLOW_SAME_STATE_TRANSITIONS`.

### N2 — Pressing Confirm on "Resume" also fires the held item
**Location:** ARCH §3.2 (PreUpdate "edges latched until a fixed tick consumes them"), §5.3;
brief T04.

**Problem.** Driving edges are latched in PreUpdate regardless of pause state, and every
`SimSet` is gated while paused. The A press (gamepad) or Enter press (keyboard B) that clicks
Resume is therefore also latched as `use_item`. The first running tick consumes it and fires the
item.

**Fix.** Add to §5.3:
- "rc_input latches driving edges only while `PauseState::Running` and no menu or dialog is
  open."
- "It clears all latched edges on entering `Running` and on `OnEnter(RacePhase::Setup)`."

Add a unit test: pause → Confirm → resume → no `use_item`.

## Optional notes (non-blocking)
- **N3 — Visual proxies: lifetime and teleports** (ARCH §3.3).
  - Make `VisualOf`/`Visual` a Bevy relationship with `#[relationship_target(..., linked_spawn)]`.
    One-to-one is supported in 0.20. A proxy then despawns with its sim entity, for example a
    rocket that explodes mid-attempt. Otherwise proxies leak.
  - Teleports (respawn, prop reset on Setup, spawn) must snap `prev = curr`. `PoseHistory` has a
    single writer (core), so the snapshot system needs a `Teleported` marker or should read
    `CarRespawned`. Otherwise the model streaks across the track for one frame.
- **N4 — UI host churn on restart** (ARCH §5.8). In 1P the attempt-scoped `GameCamera` is the
  `IsDefaultUiCamera`, so every restart despawns the UI host. Make `GameCamera`s session-scoped
  and re-bind them by slot in `OnEnter(RaceAttempt)`. That avoids host switching and bloom/AA
  history resets.
- **N5 — `LoadingTasks` has several writers** (ARCH §4.6), which contradicts the single-writer
  rule in PRACTICES §4. Use `LoadingDone(&'static str)` messages with rc_game as the sole writer,
  or list it as a second exception.
- **N6 — MSAA vs SSAO** (ARCH §5.8). SSAO needs `Msaa::Off` (`bevy_pbr-0.20.0/src/ssao/mod.rs:506`),
  and SSAO stays on at 2P, so the split-screen mapping must never choose MSAA while SSAO is on.
  Prefer TAA → SMAA. Note that Bevy's per-camera default is `Msaa::Sample4`.
- **N7 — Reset timing** (ARCH §4.3/§5.3/§5.5). Say who times the reset hold: `reset_held` as a
  bool timed by rc_race, or seconds held from rc_input. Clean up the "via rc_game? No:" sentence
  in §5.3.
- **N8 — `dir` on control points** (ARCH §5.1). Dropping control points in Reverse changes the
  loop's shape. Limit `dir` to point flags and features (gaps, ramps, props, shortcuts), and
  model a different path as a shortcut.
- **N9 — Stop `UiNav` while racing.** D-pad steering otherwise sends `ui.nav` Left/Right repeats
  during races. Suppress `UiNav` while `RacePhase::Racing` and not paused.
- **N10 — Small text fixes.**
  - `AutomationConfig` readers should include rc_input (`--input-script`).
  - PRACTICES "No bundles" → "no built-in `*Bundle` types". `impl Bundle` and `#[derive(Bundle)]`
    still exist, and the digest itself uses `impl Bundle`.
