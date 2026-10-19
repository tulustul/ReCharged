# T20 — QA fixes (two agents, disjoint ownership)

Fix the issues of `docs/qa/report.md` (QA of build 071f64d) that are not gameplay — Q4, Q6 and Q7
went to T19a, which still owns rc_race, rc_items, rc_ai, rc_camera, rc_track and the workshop/street
data. Each item: root-cause fix, a test where the logic is non-trivial, the design note of the
touched area updated, and a line in your final report (the orchestrator edits BACKLOG.md and the QA
report). The QA scripts in `tools/scripts/qa/` reproduce most issues (`trackselect_reach.script`,
`hover_steal.sh`, `rebind_conflict.script`, `weather_time.sh`, `bench.sh`, `rss.sh`, `log_scan.py` …);
run them against the live tree with `cargo run -p recharged -- …`, not `pinned.sh`, which runs the
old QA binary.

**Read first:** PRACTICES; ARCHITECTURE §1–5 and §8–9; `docs/qa/report.md` (your items in full);
the design notes of the areas you touch; `docs/tech/bevy-react-notes.md` (T20b) or
`docs/tech/bevy-0.20-notes.md` (T20c).

## T20b — UI, flow & accessibility
**Owned:** `ui/**`, `crates/rc_ui/**`, `crates/rc_game/**` except `src/soak.rs`, `crates/rc_input/**`,
`crates/recharged/**`, `crates/rc_render/src/showroom/**`, `crates/rc_vfx/src/tags.rs`, additive edits
in `crates/rc_core/**`.
- **Q1** Track select: Confirm on a track card selects it and moves the focus into its options panel,
  onto the Start / Choose-car button (so Confirm, Confirm keeps the fast path); Up from there reaches
  every option row; Back or Left returns to the selected card. Focus can still preview cards while
  browsing. The same applies in Time Trial, Versus and the post-race "Change track" (**Q12**: that
  screen gets the full options — laps, rivals, difficulty, items, weather, time — not only layout).
- **Q2** Hover may move the logical focus only after real pointer motion since the last `ui.nav`
  or since the screen mounted (a resting cursor under a newly mounted entry must not steal it).
  Hide the OS cursor after keyboard/pad input until the mouse moves.
- **Q3** Rebind conflicts **swap**: the conflicting action receives the rebound action's previous
  binding, and the notice says so ("Swapped with Camera"). If there was no previous binding, the
  other action becomes unbound; the notice says "Camera is now unbound" and that row is highlighted.
  The notice clears on Reset bindings and on tab or scheme change.
- **Q5** Colour-blind markers: when on, every racer gets a shape marker (●▲■◆★✚⬟⬢ by grid index;
  humans P1–P4) next to its name tag, on its minimap dot and in the standings. One mapping in rc_core
  shared by rc_vfx tags and rc_ui, so the shapes always agree.
- **Q10** No unlock toasts on Results / Cup end (their panels list the unlocks); toasts elsewhere
  unchanged.
- **Q11** PTS column only in Cup and Versus; Time Trial runs count as races but not as wins or
  podiums in the profile stats.
- **Q13** Diagnosable scripted runs: a debug log of route changes and handled `ui.nav` (React →
  tracing), and a `wait screen <route>` step in the `--input-script` host; use it in
  `tools/scripts/smoke.script` instead of fixed delays after the splash. Run smoke 4× in a row: 4/4.
- **Q14** Showroom per screen: in the Garage the car stands clear of the card grid (or is hidden);
  hidden or dimmed under Credits; on Cup end the podium cars stand on the 1-2-3 blocks.
- **Q15** Locked cups: dim the race list under the lock, and put the unlock reason in the card footer.
- **Q16** Name tags: lower the size cap and fade tags closer than about 6 u.
- **Q17** `--bench` implies `--unlock-all`.
- **Q18** Splash and boot-toast timers start at the first presented frame.
- **Feel:** notifications about other racers' hits collapse into one line per item within about 1 s
  ("Oil Slick hit 3 racers").
- After all changes: `./tools/ci.sh` passes; re-run QA's keyboard-only/pad-only scripts
  (`tools/scripts/{keyboard_only,gamepad_only}.script`, `tools/scripts/qa/{kb_modes,pad_modes}.script`)
  and update them where the flows changed (Q1).

## T20c — rendering, performance & audio
**Owned:** `crates/rc_render/**` except `src/showroom/**`, `crates/rc_vfx/**` except `src/tags.rs`,
`crates/rc_audio/**`, `crates/rc_game/src/soak.rs`, `assets/data/environments/**`, `assets/shaders/**`.
- **Q9 / B31** Loading → Intro ≤ 3 s (ARCHITECTURE §9). Measure dev and release builds; if over,
  warm up the track-independent pipelines (items, effects, UI, cars) asynchronously in the menu after
  boot, so that Loading compiles only what the track adds. Report before/after for first and repeat
  loads, 1P and 4P.
- **Q20 / B28 / B29** No first-use pipeline compiles mid-race: the bench's single 26–33 ms frame,
  night-lamp shadow pipelines, and the split-screen overlay's sprite/blit pipelines at GO. The
  diagnostics line at GO and `RUST_LOG=rc_render=debug` tell you what compiled.
- **Q8** Indoor time of day is clearly distinct on Toy Town and Sofa Circuit (GDD §10.1): darker
  room at night with lamp pools and a cool moonlight fill, a warm but readable sunset (no dark
  red-brown wash), a bright noon; floor readability first. Before/after contact sheets via
  `tools/scripts/qa/weather_time.sh`.
- **Fog** reads as fog (denser at distance, still drivable), and storms have visible lightning in
  motion.
- **Q21** Memory: add `Assets<Mesh/Image/StandardMaterial/AudioSource>` counts to the soak census
  (they must return to baseline in the menu); find what dominates RSS after visiting all tracks
  (`tools/scripts/qa/rss.sh`), and drop main-world copies of GPU-only assets
  (`RenderAssetUsages::RENDER_WORLD`) wherever nothing reads them on the CPU. Report RSS before/after.
- **B30** Rain splashes sit on the ground under them (no extrapolated plane far from the car).
- **Q19** rc_audio: a rate-limited `debug!` per stinger and per one-shot cue (cue, bus, volume).
- **B1** (optional) Tour shots stay clear of item cubes.
- After all changes: `./tools/ci.sh` passes; re-run `tools/scripts/qa/bench.sh` (isolated: no other
  recharged process) — no regressions against the QA table.
