# T21 — Final regression QA (Definition of Done on the final build)

**Goal:** confirm the final build meets GDD §21's Definition of Done, that every fix since the first
QA pass (`docs/qa/report.md`, build 071f64d) holds, and that nothing regressed. Report only; no code
fixes (trivial data typos may be listed with the exact fix).

**Read first:** `docs/qa/report.md` (the previous pass, its scripts and numbers — reuse them),
GDD §1.3 and §21, ARCHITECTURE §8–9, `docs/design/automation.md`, `docs/tasks/BACKLOG.md`,
`docs/tasks/T19-backlog-fixes.md`, `docs/tasks/T20-qa-fixes.md`.

**Owned:** `tools/scripts/qa/**` (new scripts are fine; keep the old ones working), `target/qa2/**`.
Subagents can't write report `.md` files: return the report as your final message (the orchestrator
saves it as `docs/qa/final-report.md`). No other agent is editing the tree during this pass; build
once and test that build (record `git log -1 --oneline`).

## Checks
1. **Build health:** `./tools/ci.sh` on a clean tree.
2. **DoD-1:** `tools/scripts/{keyboard_only,gamepad_only,smoke}.script` (smoke ×4 in a row),
   `tools/scripts/qa/{kb_modes,pad_modes,trackselect_reach,rebind_conflict}.script`,
   `tools/scripts/qa/hover_steal.sh` (A/B). Confirm Q1, Q2, Q3, Q12, Q13 fixed.
3. **DoD-2:** `cargo test -p rc_ai --test race -- --ignored every_layout`; plots of all layouts
   lint-free; spot-check tours of Toy Town, Wrench Works, Backyard Bash and Maple Street (the tracks
   whose data changed).
4. **DoD-3:** `dod3_cup.script` + `dod3_restart.script`, corrupt-save recovery (Q18: the toast is
   seen), records/stats (Q11), no toasts over Results (Q10).
5. **DoD-4:** `vs2_vertical`, `vs3`, `vs4` scripts; colour-blind markers on (Q5) in a 3P run;
   name tags (Q16).
6. **DoD-5:** `tools/scripts/qa/weather_time.sh` — every time of day on every track, every weather
   on Backyard Bash and Maple Street; compare Toy Town / Sofa Circuit times with the old sheets (Q8),
   fog and storm lightning.
7. **DoD-6:** every music cue, stinger and SFX category appears in the logs (Q19); no audio
   warnings.
8. **DoD-7:** `tools/scripts/qa/bench.sh` isolated (no other `recharged` process before or during a
   run), compared with the old table; Loading → Intro times, first and repeat loads (Q9, ≤ 3 s);
   late pipelines at GO (the info line) = 0 (Q20).
9. **DoD-8:** `--soak 30` with RSS sampling (`rss.sh`) and the asset census (Q21); both determinism
   tests; no `panicked` / `ERROR` in any log.
10. **Spot checks of the T19 fixes:** missed-checkpoint notice (B15), finished human handed to the
    AI (B10), look-back camera against a wall (B23), credits screen (B8), quitting mid-compile
    (B2: window close 1 s after start exits cleanly), assisted throttle-only lap (Q7: finishes,
    no medal pace).
11. **Feel:** a few autopilot races per track with items on; anything that looks wrong or
    unfun now.

Check `pgrep -a recharged` before every windowed run (the user may be playing; never open a window
over a session that isn't an automated agent run). Use `--user-data-dir target/qa2/data/<run>`.

## Report
A DoD table (pass / partial / fail with evidence), a fix-verification table (each Q1–Q22 and each
T19/T20 item: verified / not verified / regressed), then new issues `R1…Rn` in the format of the
first report (severity, area, repro, expected vs actual, screenshot, suggested fix).
