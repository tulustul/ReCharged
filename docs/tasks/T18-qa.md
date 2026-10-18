# T18 — QA: playtests, integration checks, bug triage

**Goal:** prove the game is complete against GDD §21 (Definition of Done), find what is broken or
weak, and produce a prioritized, reproducible bug report the orchestrator can dispatch.

**Read first:** GDD (all — you are checking it), ARCHITECTURE §3, §8, PRACTICES, every
`docs/design/*.md` (what each subsystem claims), `docs/design/automation.md` (scripted input,
headless races, bench, medals), `docs/tasks/BACKLOG.md` (known issues — don't re-report them,
but do confirm or update them).

## Owned paths
`docs/qa/**` (report, screenshots index), `tools/scripts/qa/**` (your input scripts). **You do not
fix code** — you report. Exception: trivial data typos may be listed with the exact fix.

## Checks
1. **Build health:** `./tools/ci.sh` from a clean state (report every failure with the exact output).
2. **DoD-1 (input parity):** scripted keyboard-only and gamepad-only runs through Title → each mode →
   race → pause → results → back, plus Settings (every tab, a rebind, a graphics preset change, an
   audio volume change). Screenshot each step; flag dead ends, unreachable controls, missing focus.
3. **DoD-2 (tracks):** `cargo test -p rc_ai --test race -- --ignored every_layout` and the plots of
   all 6 tracks × 4 variants; in-game `--track-tour` of each track forward and reverse; flag visual
   holes, z-fighting, floating/sunken props, unreadable sections, wrong surfaces.
4. **DoD-3 (progression & saves):** a scripted Rookie Cup to the podium → unlocks granted and still
   there after a restart of the app; a Time Trial → ghost saved and raced; records and medals shown;
   settings persist; corrupt a save file → recovery toast.
5. **DoD-4 (split-screen):** 2P (both orientations), 3P, 4P races with mixed devices (script), HUD
   per viewport, per-view precipitation, audio sanity (no errors), pause from any player.
6. **DoD-5 (weather/time):** each weather on an outdoor track and each time of day on every track;
   screenshots; flag anything unreadable or broken.
7. **DoD-6 (audio):** every music cue and SFX category is triggered at least once (logs), no audio
   errors/warnings; note anything that seems missing.
8. **DoD-7 (performance):** `--bench` for 1P/2P/4P on Toy Town and on the heaviest outdoor track,
   **with no other game window open** (check `pgrep -a recharged` first — covered windows get
   throttled); compare with GDD §1.3 targets.
9. **DoD-8 (robustness):** `--soak 20` (chained races, entity baseline), determinism test, no panics
   in any run (grep logs for `panicked`, `ERROR`).
10. **Feel & fun review:** watch autopilot races and your scripted races: pacing, item balance, AI
    aggression, camera, readability of HUD and effects. Concrete suggestions with priority.

## Report
`docs/qa/report.md`: a summary table per DoD item (pass / partial / fail with evidence), then issues
`Q1…Qn`, each with severity (blocker/major/minor/polish), area/owner crate, exact repro (command +
script), expected vs actual, screenshot path, and a suggested fix. Keep screenshots under
`target/qa/` and reference them; put only small curated images (if any) in `docs/qa/img/`.
