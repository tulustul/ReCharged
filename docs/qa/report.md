# ReCharged — QA report (T18)

| | |
|---|---|
| Build tested | **071f64d** (`docs: status board`, 2026-10-19 01:09) — the code of `511c59b` + `dc59d63` (`RaceEventKind::MissedCheckpoint` contract). Dev profile (`opt-level` 1 / deps 3). T19a/T19b fixes started after this build and are **not** included. |
| How | One binary built from a clean tree at that commit (`target/qa/bin/recharged`), UI bundle and assets snapshotted (`target/qa/pin/`), sources exported (`target/qa/src/`, `git archive`). Windowed runs went through `tools/scripts/qa/pinned.sh`, which bind-mounts the snapshots over `ui/dist` and `assets` in a private mount namespace, so the T19 agents' edits never leaked into a run. Test binaries (`rc_ai` race test, `recharged` determinism tests, `rc_track` plot tool) were built from the snapshot. Without the pinning every command below is `cargo run -p recharged -- <args>`. |
| Machine | Arch Linux, kernel 7.2.9, Ryzen 7 5800X, 32 GB, RTX 3070 (driver 615.78.08, Vulkan), X11/i3, 2560×1440 @ 60 Hz (DPI scale 1.1667), rustc 1.99.0, node 24.15 |
| Date | 2026-10-19, 01:05–02:50 |

Screenshots and logs: `target/qa/` (index in §6). Small curated images: `docs/qa/img/`.
(Saved by the orchestrator from the QA agent's final message; subagents cannot write report files.)

## 1. Definition of Done (GDD §21) — summary

| # | DoD item | Verdict | Evidence (details in §3) |
|---|---|---|---|
| — | Build health (`tools/ci.sh`) | **PASS** | fmt, clippy `-D warnings`, 557 Rust tests (+9 ignored), bindings fresh, UI typecheck, 33 UI tests, UI build — all green in 58 s on a clean tree |
| 1 | All four modes end to end, keyboard only and gamepad only (scripted) | **PARTIAL** | Every mode, pause, results, post-race flows and all Settings tabs (rebind, preset, volume) pass on keyboard-only and gamepad-only scripts (exit 0). But the Track select options of 4 of 6 tracks cannot be reached without a mouse (**Q1**), a resting mouse pointer hijacks keyboard/pad focus (**Q2**), a conflicting rebind silently unbinds another action (**Q3**); the CI smoke script flaked 1/4 (**Q13**) |
| 2 | 6 tracks × Fwd/Rev × Mirror drivable; AI 3 laps ≤ 1.5× expected, ≤ 2 respawns/lap | **FAIL (1 of 24)** | `every_layout`: 23/24 shipped layouts pass, **wrench_works mirror fails** (Mayhem 131.0 s = 1.56×, 4 respawns) — a respawn trap at the workbench jump (**Q4**). All 24 layouts load and plot lint-free; 12 track tours (fwd + rev) complete with no holes or broken geometry seen |
| 3 | Unlocks, records, medals, ghosts, settings persist across restarts | **PASS** | Rookie Cup (gold) through the real menus → Pro Cup, Torque, Hornet, Wrench Works, Maple Street, Reverse unlocked; TT on Toy Town → Silver, record, ghost saved and raced on Retry; app restarted → all of it (and mph / HUD 125 %) still there, ghost on the grid; corrupt saves backed up with a toast |
| 4 | 2–4 player split-screen, per-player HUD, mixed keyboard + pads | **PASS** | 2P top/bottom and left/right, 3P (minimap/standings quadrant), 4P with keyboard A + B + two pads; per-view rain/snow/storm; pause from P2/P3/P4 on any device; rematch / next track / lobby flows; no audio errors |
| 5 | Weather (6) and time of day (4) selectable, visibly/gameplay distinct | **PASS** | All six weathers on Backyard Bash and Maple Street, all four times on every track; outdoor grip effect measured (Backyard TT +4 % rain, +15 % snow); indoor tracks unaffected. Indoor time of day is weak on Toy Town (**Q8**) |
| 6 | §16 music cues and SFX present, mixed via settings buses | **PASS** | All 12 music files, 59 SFX, 14 ambience loops present; director switched menu theme, race_1–4 and every `_final` variant; sink capture: music bus 0 → silence, mute-when-unfocused works, race mix −13…−17 dBFS RMS; every item and race event that maps to a cue occurred in the logged races (mapping unit-tested); no audio warnings. Stingers/one-shots are not observable in logs (**Q19**); no human listening yet (B6) |
| 7 | §1.3 performance on the RTX 3070 (bench runner) | **PASS** | 1080p High: 1P 185–197 fps (p99 ≤ 8.8 ms); 4P 100–117 fps (p99 ≤ 13.6 ms) — at High, while the target is 50 fps at Medium. Dev profile, isolated runs |
| 8 | Tests/clippy/UI pass; deterministic replay; 30-min soak without panic or entity leak | **PASS** | CI green; both determinism tests pass, 3 CLI runs byte-identical; `--soak 30`: 129 races, 128 menu censuses equal to the baseline, 0 panics, 0 ERROR. RSS plateaus around 4.0–4.5 GB (**Q21**) |

## 2. Issues

Severity: **blocker** (cannot ship / DoD fail with no workaround) · **major** (DoD or GDD requirement broken, real player impact) · **minor** (noticeable, workaround exists) · **polish**. Already-known backlog items are not repeated (status in §4).

| ID | Sev | Area / owner | Summary |
|---|---|---|---|
| Q1 | major | ui (TrackSelect) | Track options (layout, mirror, laps, rivals, AI, items, weather, time) unreachable by keyboard/pad except for the 2 right-column tracks |
| Q2 | major | ui (focus/hover) | A mouse pointer resting in the window steals keyboard/gamepad focus when screens appear |
| Q3 | major | rc_game prefs + ui Settings | Conflicting rebind silently unbinds the other action; notice says "Also bound to …"; notice survives Reset |
| Q4 | major | rc_track respawns + env workshop data + rc_ai recovery | `every_layout` fails on wrench_works mirror: respawn trap before the workbench jump, cars wedge at the ramp-foot walls |
| Q5 | major | rc_vfx tags + ui HUD | "Colour-blind markers" setting does nothing (no reader) |
| Q6 | minor | rc_ai items + items.ron | Goo Bomb explodes on its own carrier in 15/16 uses; Oil Slick 3.4 and Spring Pad 2.75 victims per drop |
| Q7 | minor | rc_ai assist | Steering assist + throttle-only (the kid-friendly path) gets stuck and is very slow |
| Q8 | minor | rc_render lighting / env data | Indoor time of day barely distinct on Toy Town; Sofa Circuit sunset/night are a dark red-brown wash |
| Q9 | minor | rc_render warm-up / rc_game loading | Loading → Intro exceeds the 3 s budget on first loads (up to 3.9 s) |
| Q10 | minor | ui (toasts, Results) | Unlock toasts cover the Results side panel (and duplicate its unlock list) |
| Q11 | polish | rc_ui results + rc_game stats | "PTS +0" column in Quick Race / Time Trial; Time Trial runs count as wins and podiums |
| Q12 | minor | ui (RaceLayer) | Post-race "Change track" offers only Layout + Mirror (GDD §3.3 "Change track/options") |
| Q13 | minor | automation (rc_input script, rc_ui) | CI smoke script flaked 1/4 (stuck at `wait racing`), and nothing in the log says which screen ate the input |
| Q14 | polish | ui + rc_render showroom | Showroom car collides with screens: behind the Garage grid, under the Credits text; podium cars don't sit on the podium |
| Q15 | polish | ui (CupSelect) | Lock overlays on locked cups overlap the race list |
| Q16 | polish | rc_vfx tags | Name tags of nearby rivals get very large and are cut at the screen edge |
| Q17 | polish | recharged (bench) | `--bench` on a locked track fails unless `--unlock-all` is added |
| Q18 | polish | rc_ui / ui (automation) | Boot toasts and the splash expire unseen in automated runs (first-frame stall) |
| Q19 | polish | rc_audio | Stingers and one-shot SFX are not logged — DoD-6 cannot be verified from logs |
| Q20 | polish | rc_render | One 26–33 ms frame in most bench runs after warm-up |
| Q21 | minor | rc_game soak + assets | RSS grows to ≈ 4.4 GB after all tracks were visited; the soak census does not count assets |
| Q22 | polish | catalog | Toy Town / Wrench Works races last ≈ 80–85 s for fast drivers (GDD session 1.5–2.5 min) |

### Q1 — Track options unreachable without a mouse (major, ui)
- **Area:** `ui/src/screens/TrackSelect.tsx` (focus map `tracks: { cols: 3, right: "options" }` + the `useEffect` that makes the focused card the selected track).
- **Repro:** `cargo run -p recharged -- --user-data-dir target/qa/data/tsr --unlock-all --window 1280x720 --input-script tools/scripts/qa/trackselect_reach.script --exit-after 90` → `target/qa/data/tsr/screenshots/tsr_01…04.png` (curated: `docs/qa/img/q1_trackselect_options.jpg`).
- **Expected:** with keyboard or gamepad alone every option of the chosen track can be set (UI-19 "full parity, no unreachable controls"; MODE-2), e.g. Toy Town · Reverse · 1 lap · Rain.
- **Actual:** focusing a card selects it, and the options panel is only the `right` neighbour of the right-most column, so you can only enter it through Backyard Bash (top row) or Maple Street (bottom row), which become the selection on the way (tsr_01 Toy Town selected → tsr_02 the panel shows Backyard Bash's 5 laps). Toy Town, Sofa Circuit, Aisle Rush and Wrench Works can only be raced with their defaults (Forward, no mirror, default laps, 7 rivals, Pro, items on, first weather and time). Same for Time Trial (layout/weather/time), Versus (P1 picks) and the in-race "Change track". The shipped scripts document the workaround ("go top-left, then right ×3").
- **Fix:** decouple *focused card* from *selected track*: Confirm on a card selects it and moves the focus into the options (first row; Start/Choose car stays the last row); `left` from the options returns to the selected card. Alternatively `right` from any column enters the options without changing the selection.

### Q2 — A resting mouse pointer hijacks keyboard/gamepad focus (major, ui)
- **Area:** hover → focus (`useFocusNav().hover`, called from every `onHover`/`onPointerEnter`).
- **Repro:** `tools/scripts/qa/hover_steal.sh <name>` (runs `tools/scripts/qa/hover_steal.script` twice in a 1280×720 window at (0,0): pointer resting at screen (864,591), then parked off-window). Script: Title → Up ×3 → Confirm (= Settings) → Right (speed unit).
- **Expected:** both runs open Settings and switch km/h → mph.
- **Actual:** with the pointer inside the window the **"Quit ReCharged?" dialog** opens instead (`target/qa/data/hover_a_on/screenshots/hs_02*.png`); with it outside the run is correct (`hover_a_off`). Curated: `docs/qa/img/q2_hover_steal_ab.jpg`. The first keyboard-only run (pointer resting over its window) showed the same class of failures: Garage opened on Hornet instead of Sparky, the Settings focus jumped to *Hints* so Right switched hints **off** (saved), and the Graphics "preset" step toggled *Motion blur* instead (preset showed Custom) (`target/qa/data/kb_modes/screenshots/kbm_02, kbm_13, kbm_14`, `…/kb_modes/settings.ron`).
- **Cause (likely):** an entry that appears or slides in under a stationary cursor fires PointerEnter → `hover()` moves the logical focus; keys/pads continue from there. In the A/B run the cursor position Bevy knew was over TIME TRIAL (X11 sends no motion when a window moves under a resting pointer, so it was stale): 3 × Up from Time Trial = Quit. Players who used the mouse once and leave it over the window get this on every screen change (e.g. A on the pause menu opens Quit).
- **Fix:** let hover move the focus only after real pointer motion — remember the last pointer *move* and the last `ui.nav`; ignore PointerEnter unless the pointer moved since the last `ui.nav` / since the screen mounted. Optionally hide the cursor until the mouse moves.

### Q3 — Conflicting rebind silently unbinds the other action (major, rc_game + ui)
- **Area:** `crates/rc_game/src/prefs.rs::apply_rebind`; `ui/src/screens/settings/Settings.tsx` (`input.rebindResult` handler, l. 49–53); `ui/src/strings/settings.ts:73`.
- **Repro:** `cargo run -p recharged -- --user-data-dir target/qa/data/rebind1 --window 1280x720 --input-script tools/scripts/qa/rebind_conflict.script --exit-after 60` → `target/qa/data/rebind1/screenshots/rb_01, rb_02` (curated `docs/qa/img/q3_rebind_conflict.jpg`).
- **Expected:** GDD §12.1 "conflicts are checked … and warned": the player knows what happened — either the two bindings swap or the UI says the other action is now unbound.
- **Actual:** Use item → C takes C away from Camera: the Camera row shows "—" and `settings.ron` has `camera: []` (keyboard A, seen in `kb_modes2/settings.ron`); on the pad Use item → View empties Camera the same way (`padm_18`). The notice says *"Also bound to Camera (Keyboard A)"*, i.e. that both still work. After *Reset bindings* the bindings are restored but the stale notice stays (rb_02, `padm_18b`).
- **Fix:** swap (the conflicting action gets the rebound action's old primary binding) or keep both and warn; if stealing is kept, say "Camera is now unbound" and highlight that row; clear the notice on reset and on tab/scheme change.

### Q4 — `every_layout` fails on Wrench Works mirror: respawn trap at the workbench jump (major)
- **Area:** rc_track respawn placement (`Track::respawn_before`), Wrench Works layout data (`tools/blender/props/workshop/plan.py` → `assets/data/tracks/wrench_works.ron`), rc_ai recovery.
- **Repro:** `cargo test -p rc_ai --test race -- --ignored every_layout` (log: `target/qa/logs/every_layout.log`); trace: `AI_TRACK=wrench_works AI_MIRROR=1 AI_DIFF=Elite AI_CLASS=Elite AI_SEED=21 AI_TRACE=1 AI_TRACE_EVERY=30 cargo test -p rc_ai --test race -- --ignored --nocapture probe` (log: `target/qa/logs/ww_mirror_probe.log`).
- **Expected:** every racer ≤ 1.5 × 83.9 s with ≤ 6 respawns (DoD-2).
- **Actual:** Mayhem (Volt GT) 131.0 s (1.56×), laps 42.9 / 36.2 / 51.9 s, 4 respawns. The trace (s = arc length, lat = lateral offset): after a reset near s ≈ 224 the car respawns at s ≈ 149.4, 52 u before the workbench lip (s ≈ 201, on the ramp approach) → reaches the lip slow, tumbles (`up` 0.09–0.26 at s ≈ 200–204) → lands misaligned → wedges against the barrier where the walls start again at the west ramp foot (point 23, width 10 → lat exactly ±5.01) → reverses ~2 u, drives into the same wall again (twice) → resets → respawns at s ≈ 149 again (a loop, seen 3×). It also sticks at the tyre-stack chicane entry (s ≈ 297–303, lat −5.5). All four Wrench Works variants have 4–5 respawns per race (other tracks 0–2) and lap-1 losses of up to 15 s (forward: Dash 43.6 s, Nova 42.8 s vs 28 s reference), so the margin is thin everywhere on this track.
- **Fix:** (a) no respawn point inside the run-up a car needs to reach 0.8× the jump's takeoff speed (or a checkpoint/respawn on the landing ramp, so failures after the landing do not send cars back before the ramp) — a rc_track lint could flag it; (b) flare or delay the walls at the west ramp foot so a misaligned landing is not funnelled into a corner; (c) rc_ai recovery: reverse with the wheels turned toward the centre (lat → 0) instead of re-entering the same wall. Re-run `every_layout`.

### Q5 — "Colour-blind markers" does nothing (major, accessibility)
- **Area:** rc_vfx `tags.rs` (name tags), ui `ui/src/hud` (minimap dots, 3P standings); rc_ui only mirrors the field (`crates/rc_ui/src/settings.rs`).
- **Repro:** Settings → Accessibility → Colour-blind markers On → any race: nothing changes. `grep -rn colorblind crates ui/src` lists only the rc_core field, the rc_ui mirror and the UI row/string.
- **Expected:** GDD §15/§17/§18 (ACC-1): colour-blind-safe racer markers — shapes/patterns, not only colours — on name tags, minimap and standings.
- **Fix:** when on, a per-racer shape (● ▲ ■ ◆ ★ ✚ … and P1–P4 letters for humans) next to each name tag (atlas row) and as minimap/standings dot shapes.

### Q6 — Goo Bomb is self-harm; oil and spring pads hit whole groups (minor, balance)
- **Area:** rc_ai `items.rs` / `scene.rs` (goo seeking, `goo_clearance_u`), `assets/data/ai.ron`, `assets/data/items.ron`.
- **Repro:** `RUST_LOG=warn,rc_items=debug,rc_race=debug cargo run -p recharged -- --user-data-dir target/qa/data/hli/toy_town --unlock-all --headless-race --quick-race toy_town --ai 7 --seed 5 2> items.log` (also `backyard_bash`, `maple_street`), then `python3 tools/scripts/qa/item_stats.py items.log`.
- **Data** (3 headless 8-car races + the autopilot Rookie Cup log, 313 grants): Goo Bomb used 16×, **15 explosions hit the carrier**, only 7 other victims — a pass practically never happens (rivals keep 2 u extra berth from a carrier, the carrier only aims at the nearest car ahead). Oil Slick 18 drops → 62 hits (3.4 per slick), Spring Pad 12 → 33 (2.75), Storm Strike 7 → 40. Rockets 54 → 27 hits (fine). Hazard hits are a large share of B17's respawns.
- **Fix:** carriers seek any car within ~4 u laterally / 15 u ahead with a small speed bonus and drop the extra berth for followers; AI hazard avoidance re-check (oil on the line within ~20 u → lane change); consider Oil lifetime 25 → 15 s.

### Q7 — Steering assist + throttle only gets stuck (minor, feel, rc_ai `assist.rs`)
- **Repro:** the scripted human in `tools/scripts/keyboard_only.script` and `tools/scripts/qa/kb_modes.script` only holds throttle with Steering assist on.
- **Actual:** Backyard Bash 1 lap: 38.1 s vs winner 24.6 s, "Stuck? Hold Reset" after 8 s (`kb_shipped/screenshots/kb_07`); Sofa Circuit cup race (5 laps): not finished inside the soft-lock window (projected 5:22 vs 1:54, `kb_modes2/…/kbm_29`); Time Trial Backyard: 1:39.9 vs Bronze 1:09.7. GDD pillar "fun within five seconds", audience 8+.
- **Fix:** a stronger assist level (look-ahead steering to the racing line + gentle auto-brake before tight turns), or an "Auto-steer (kids)" option on top of auto-accelerate.

### Q8 — Indoor time of day weak or muddy (minor, rc_render lighting / env RON)
- **Repro:** `tools/scripts/qa/weather_time.sh toy_town:clear:morning toy_town:clear:noon toy_town:clear:sunset toy_town:clear:night` (same for `sofa_circuit`) → `target/qa/shots/wt/`, sheets `target/qa/shots/sheets/wt_tod_*.png` (curated `docs/qa/img/q8_toy_town_times.jpg`).
- **Actual:** Toy Town morning/noon/sunset/night look nearly identical (orange ribbon, same exposure; night is not darker — only the window patch and the headlight glow differ). Sofa Circuit sunset and night are a heavy dark red-brown wash (night has no cool moonlight; low readability of the floor). Wrench Works, Aisle Rush and both outdoor tracks are clearly distinct.
- **Fix:** per-time exposure and grading for indoor envs (darker room at night with lamp pools and a cool fill; less red at sunset), GDD §10.1.

### Q9 — Loading → Intro over the 3 s budget (minor)
- **Data** (`python3 tools/scripts/qa/log_scan.py target/qa/logs/*.log`): 3.88 s (4P Toy Town storm night, `vs4_720p.log`), 3.80 s (Toy Town, cup race 2), 3.0–3.2 s on several first Backyard Bash loads; most loads 0.2–2.5 s. ARCHITECTURE §9: ≤ 3 s. Restarts 0.00–0.07 s (budget 0.5 s) ✓.
- **Note:** the time is the pipeline warm-up; T19b's "intro hitch" item adds warm-up — keep the budget.

### Q10 — Unlock toasts cover the Results panel (minor, ui)
- **Repro:** `dod3_cup.script` (autopilot Rookie Cup) → `target/qa/data/dod3/screenshots/cup_11_r4_results.png`.
- **Actual:** four stacked "Unlocked!" toasts sit on top of the "YOU FINISHED 1st" panel (big "1st", time and the panel's own unlock list are covered/overlapping) — the same unlocks are listed twice.
- **Fix:** on Results/Cup end show unlocks only in the panel (toasts elsewhere), or move toasts below/left of the action column.

### Q11 — "PTS +0" outside cups/Versus; TT runs count as wins (polish)
- **Area:** rc_ui results mapping (points), rc_game stats.
- **Actual:** Quick Race and Time Trial results show a PTS column with "+0" for every racer (`kb_shipped/…/kb_09_results`, `kb_modes2/…/kbm_35_tt_results`); the profile after 4 cup races + 2 solo Time Trials reports races 6 / **wins 6 / podiums 6** (Records → Stats, `cup_28_records_stats`).
- **Fix:** `points: None` unless cup/Versus; count TT runs as races but not wins/podiums.

### Q12 — Post-race "Change track" has no options (minor, ui)
- **Area:** `ui/src/screens/race/RaceLayer.tsx` → `TrackSelect inRace` (only layout rows).
- **Actual:** after Quick Race / Time Trial results, "Change track" offers Layout and Mirror only — no laps, rivals, difficulty, items, weather, time (GDD §3.3 "Change track/options"). Same in Versus "Next track" (`vs3/…/vs3_09_next_track_select`).

### Q13 — Smoke script flake and undiagnosable stalls (minor, automation)
- **Repro:** `cargo run -p recharged -- --user-data-dir <fresh> --window 1600x900 --input-script tools/scripts/smoke.script --exit-after 150` — 1 of 4 runs (`target/qa/logs/smoke.log`, the first after the build) stalled at `wait racing`: all confirm steps ran (2 s apart) but no race was requested; the other runs pass in 25 s.
- **Gap:** the log shows script steps and app states only; which screen consumed or dropped a `nav` is invisible. The fixed delays after the splash are fragile when the first frames stall.
- **Fix:** debug log of route changes and handled `ui.nav` in React (`ui.debug` message) and a `wait screen <route>` script step.

### Q14 — Showroom car collides with screens (polish, ui + rc_render showroom)
- **Evidence:** `docs/qa/img/q10_q11_polish.jpg` — Garage (`kb_modes2/…/kbm_02_garage`): the 3D car stands behind the translucent car grid between the Zip and Torque cards; Credits (`kbm_12`): the roll text runs over the car; Cup end (`dod3/…/cup_13, cup_14`): the 3D cars float above/over the 2D podium blocks and their name labels instead of standing on 1-2-3.
- **Fix:** per-screen backdrop focus: car right of the grid (or hidden) in Garage, dimmed/hidden under Credits, podium poses matching the blocks (or blocks drawn in 3D) on Cup end.

### Q15 — Lock overlays overlap cup race lists (polish, ui `CupSelect.tsx`)
- `cup_15_cup_select`, `rst_03_cup_select`: the "Finish on the podium in …" box sits over races 2–3 while the list text shows around and through it. Dim/blur the list under the lock or put the reason in the footer of the card.

### Q16 — Huge name tags for nearby rivals (polish, rc_vfx `tags.rs`)
- `pad_shipped/…/pad_12_versus_racing`: "Turbo Tom" tag ~10 % of the view height, cut at the right edge; `wt_tod_sofa_circuit` sunset/night: "Nova" dominates the frame. Lower the 2.2 u cap or fade tags closer than ~6 u.

### Q17 — `--bench` on a locked track (polish, recharged `bench.rs`)
- `--bench maple_street_1p` on a fresh profile: `ERROR cannot start race: layout … maple_street … is locked`, exit 1. The bench already ignores the profile's settings; it should imply `--unlock-all`.

### Q18 — Boot toasts/splash expire unseen in automated runs (polish, rc_ui/ui)
- Automated runs compile pipelines synchronously: the first frame appears ≈ 5 s after `ui.ready` (`target/qa/logs/corrupt1.log`), so the 4.5 s "Save file restored" toast and the 2.6 s splash (wall-clock timers) are gone before anything is drawn. In a normal run both show (`target/qa/shots/corrupt_ui2_2.png`). Start toast/splash timers on the first presented frame (or hold them while the frame loop stalls).

### Q19 — Stingers and one-shots are not logged (polish, rc_audio)
- The music director logs loop targets only; `results_jingle`, `podium_fanfare`, `cup_victory` and every SFX one-shot leave no trace, so DoD-6 "every cue triggered" cannot be shown from logs. Add a rate-limited `debug!` per stinger and per one-shot cue (cue id, bus, volume).

### Q20 — One long frame per bench run (polish, rc_render)
- Max frame 26–33 ms in 9 of the 12 bench runs in §3.8 after the 5 s warm-up (p99 stays < 14 ms) — likely a first-use pipeline (pickups/effects); re-check after T19b's intro-hitch fix.

### Q21 — Memory: ≈ 4.4 GB RSS after all tracks were visited (minor)
- `--soak 10 --soak-seed 1792365836540779723`: RSS 1.85 GB in the menu → 4.0 GB at 3.5 min → 4.46 GB at 10 min (`target/qa/logs/soak10_rss.log`); `--soak 30`: 4.03–4.36 GB between minutes 18 and 30 (`soak30_rss.log`). It plateaus (no unbounded growth seen), but it is high for a dev build and the soak census counts entities and resource types only, not `Assets<Mesh/Image/StandardMaterial/AudioSource>`, so an asset leak would pass. Add asset counts to the census; B21 (props embedding their own texture copies) is a likely contributor.

### Q22 — Short races on Toy Town / Wrench Works (polish, catalog)
- Elite autopilot: Toy Town 3 laps 79 s (cup race 2), Wrench Works ≈ 84 s reference; GDD §1.3 one race 1.5–2.5 min. Fine for rookies; consider 4 laps if playtests agree.

## 3. Check details

### 3.1 Build health
`nice ./tools/ci.sh` on a clean tree (`git status` empty, HEAD 511c59b/071f64d, shared target dir — `cargo clean` is not allowed): **CI passed** in 57.8 s. 557 Rust tests passed, 9 ignored (`every_layout`, `lap_table`, `probe`, 5 rc_vehicle tables/probes, `a_full_field_on_toy_town_repeats_exactly`); UI: typecheck, 33 tests, build (esbuild size notes for `vendor.js` 3.0 MB / `app.js` 1.4 MB only). Log: `target/qa/logs/ci.log`.

### 3.2 DoD-1 — input parity (keyboard only / gamepad only)
| Run (script) | Devices | Flow | Result |
|---|---|---|---|
| `tools/scripts/keyboard_only.script` | kb A, kb B | Settings (VSync off, assist) → Quick Race Backyard → every driving action → pause → results → Menu → Versus 2P (A + B) → race → P2 pauses → quit | exit 0, `vsync: false`, `steering_assist: true` saved |
| `tools/scripts/gamepad_only.script` | pad 1, pad 2 | same on pads (analog trigger, stick, LB) | exit 0 |
| `tools/scripts/qa/kb_modes.script` | kb A (+ kb B Back) | Garage (locked car), Records 4 tabs, How to play 4 tabs, Credits, Settings 5 tabs (mph, preset Low, music 60 %, rebind Use item, assist), Championship (locked cup → error cue; Rookie race 1 → pause → How to play / Settings → preset High live → resume → finish → results → standings → Quit cup dialog), Time Trial (finish → Retry vs ghost → pause → quit), Quit dialog → Cancel | exit 0 (run `kb_modes2`; the first run `kb_modes` shows Q2) |
| `tools/scripts/qa/pad_modes.script` | pad 1 | same + Reset bindings | exit 0 |
| `vs2_vertical`, `vs3`, `vs4`, `dod3_cup`, `dod3_restart` | mixed | §3.4–3.5 | exit 0 |
| `tools/scripts/smoke.script` | kb A | CI smoke | 3 / 4 (Q13) |

Settings applied live and saved: `kb_modes2/settings.ron` (`speed_unit: Mph`, `preset: High` after the pause-menu change, `music: 0.6`, `steering_assist: true`); the Low → High change is visible in game (`target/qa/shots/sheets/kbm2_low_vs_high.png`). Glyphs follow the device (keycaps vs pad shapes) in hints, lobby and How to play. Pause menu, dialogs, Cup standings, Versus scoreboard, Garage, Records and Settings were all reachable with focus visible; no dead end found. Quitting a race returns to the screen before it (Car select / Versus track select) — consistent with "Back goes up one level". Screenshots: `target/qa/data/{kb_modes2,pad_modes,kb_shipped,pad_shipped}/screenshots/`.

### 3.3 DoD-2 — tracks
**AI test** (`every_layout`, Elite field of 8, Elite cars, seed 21, items off; `target/qa/logs/every_layout.log`):

| Layout | Ref. 3 laps (s) | Worst ratio | Respawns (sum / worst racer) |
|---|---|---|---|
| toy_town / rev / mirror / rev+mirror | 73.5 / 73.3 / 73.5 / 73.3 | 1.18 / 1.26 / 1.16 / 1.32× | 0 / 0 / 0 / 1 |
| sofa_circuit (4 variants) | 60.4 / 60.2 / 60.4 / 60.2 | 1.00 / 1.08 / 1.00 / 1.13× | 0 |
| backyard_bash (4) | 63.9 / 63.6 / 63.9 / 63.6 | 1.08 / 1.09 / 1.24 / 1.14× | 1 / 2 / 1 / 0 (max 1) |
| aisle_rush (4) | 92.7 / 93.3 / 92.7 / 93.3 | 1.01 / 1.01 / 1.00 / 1.06× | 0 |
| wrench_works (4) | 83.9 / 83.8 / 83.9 / 83.8 | 1.21 / 1.14 / **1.56×** / 1.20× | 5/2 · 4/2 · **5/4** · 5/2 |
| maple_street (4) | 104.5 / 104.7 / 104.5 / 104.7 | 1.11 / 1.06 / 1.06 / 1.12× | 2 / 1 / 2 / 2 (max 2) |
| test_oval (dev, 4) | 32.4 / 32.1 / 32.4 / 32.1 | 1.38 / 1.06 / 1.06 / 1.08× | 0 |

Every racer finished every layout; the only failure is Q4.

**Plots** (`target/qa/bin/plot <track> [--rev] [--mirror] --assets target/qa/pin/assets`, 48 SVG+PNG in `target/qa/plots/`, console `target/qa/logs/plots.log`): all 24 layouts load, **no lints**. Lap lengths vs GDD §7.1: Toy Town 639 (650), Sofa 415 (400), Backyard 463 (450), Aisle 790 (800), Wrench 702 (700), Maple 941 (900) — all within 5 %. 5 item rows each, 8–13 checkpoints, ≥ 1 jump per direction (Backyard 2), air 0.6–0.8 s at target speed.

**Track tours** (`tools/scripts/qa/tours.sh`, 12 runs fwd + rev, contact sheets `target/qa/shots/tours/<track>_<fwd|rev>_sheetN.png`, frames every 25 u): no holes, z-fighting, floating or sunken props seen; surfaces match the GDD (toy ribbon, house wood/rug/tile, garden grass/dirt/patio/deck/puddles, supermarket vinyl + ice aisle, workshop concrete/mats/plywood, street asphalt/lawn/sidewalk). Notes: Wrench Works has cones, tyres and a fuel can right next to the line (rev frames 006/011) — consistent with Q4's stuck points; Maple Street fwd frame 000 starts inside a traffic cone (tour pose only); pickups still wash the tour camera (B1, mild).

### 3.4 DoD-3 — progression & saves
`dod3_cup.script` with `--autopilot` (the Elite AI drives, the script uses the real menus; log with `rc_audio`/`rc_items`/`rc_race` debug: `target/qa/logs/dod3_cup.log`), then `dod3_restart.script` on the same data dir:
- Settings changed (mph, HUD 125 %) → Rookie Cup, 4 races (intro skipped with Confirm in race 2), results → standings → next race ×3 → Cup end (Gold) → unlocks shown on Cup end, Cup select (Pro Cup NEW), main menu progress, Garage, Track select (Maple Street, Wrench Works, Reverse unlocked; Mirror still locked) ✓.
- Time Trial Toy Town in the new Hornet → Silver, "Best lap / New record / Ghost saved", next-medal delta; Retry races the ghost (split vs ghost shown) ✓; Records tabs show it ✓.
- App restart: 1/4 trophies, 5/8 cars, Pro Cup open, Rookie "Best: Gold", TT track card with Silver + best lap/race, mph + HUD 125 % kept, "TIME TRIAL · vs GHOST" with the ghost on the grid and a green −0.26 split ✓ (`target/qa/data/dod3/screenshots/rst_*.png`, `profile.ron`, `ghosts/toy_town.ghost`).
- Corrupt saves (`tools/scripts/qa/corrupt.sh`, `corrupt_ui.sh`): truncated `profile.ron`, garbage `settings.ron`, truncated ghost → files kept as `*.corrupt-<unix s>`, defaults used, two "Save file restored" toasts (normal run, `target/qa/shots/corrupt_ui2_2.png`), ghost discarded with a warning, TT starts cleanly ✓ (Q18 for automated runs).
- Save failure (read-only data dir, `batch_c.sh`): "Couldn't save … Permission denied" toast, the game continues, the next change retries ✓ (`target/qa/data/save_fail/screenshots/`).

### 3.5 DoD-4 — split-screen
| Run | Layout | Devices | Track · weather | Checked |
|---|---|---|---|---|
| `keyboard_only` / `gamepad_only` | 2P top/bottom | kb A + kb B / 2 pads | Backyard · clear | race, P2 pause, quit |
| `vs2_vertical.script` | 2P left/right | kb A + pad | Backyard · rain | per-view rain, P2 (pad) pause → resume, results, scoreboard, rematch, P1 pause → quit |
| `vs3.script` (`--unlock-all`) | 3P + minimap/standings quadrant | kb A + kb B + pad | Maple · snow · morning, then Backyard | per-view snow, P2 (kb B) pause/Backspace resume, results, scoreboard, Next track, P3 (pad) pause → quit |
| `vs4.script` | 4P quadrants, no minimap | kb A + pad + kb B + pad 2 | Backyard · storm | per-view storm, items, P4 (pad 2) pause → resume, results, scoreboard (10/8/2/1 pts), Lobby keeps players, Esc leaves |
| `vs4_720p` (`--quick-race … --players 4 --autopilot`) | 4P at 1280×720 | auto | Toy Town · storm · night | HUD legible at 720p 4-way (`target/qa/shots/vs4_720p_001.png`) |

HUD per viewport (P1–P4 badges in slot colours, position, lap, time, item slot, speed + charge, hints, notifications) ✓; "two players on one keyboard" warning in the lobby ✓; no audio WARN/ERROR in any split run. Screenshots: `target/qa/data/{vs2v,vs3,vs4}/screenshots/`.

### 3.6 DoD-5 — weather & time of day
`tools/scripts/qa/weather_time.sh` (35 cells, autopilot, captures at 13 s and 19 s): `target/qa/shots/wt/<track>_<weather>_<time>_00N.png`, sheets `target/qa/shots/sheets/wt_*.png`.
- Outdoor weather (Backyard Bash, Maple Street): clear (sharp shadows), cloudy (flat, grey), rain (streaks, wet darker ground, splashes), storm (slanted heavy rain), fog (distance fog — fairly light), snow (flakes + white ground cover) — all distinct and readable.
- Times: all four distinct on Backyard Bash, Maple Street (street lamps + headlights at night) and Wrench Works; Aisle Rush sunset vs night distinct; Toy Town / Sofa Circuit see Q8. Indoor "outside weather" (Toy Town storm night, Sofa rain sunset, Wrench storm noon) only changes the window light.
- Gameplay (headless Phantom Time Trial, seed 3, total s): Backyard clear 60.6 · cloudy 60.3 · rain 63.0 · storm 63.3 · fog 60.2 · snow 70.0; Maple clear 104.9 · rain 103.5 · storm 113.5 · fog 100.6 · snow 110.4 (best laps 33.3 → 34.7 storm); Toy Town identical in every weather (indoor) ✓.

### 3.7 DoD-6 — audio
- Files (GDD §16): `assets/audio/music` 12 (menu theme, race_1–4 + `_final`, 3 stingers), `sfx` 59 (every §16.2 item: 3 engines, skid/scrub, 3 rumbles, landing, impacts, boost whoosh/crackle, drift sizzle, charge ready, pickup/roulette, every item sound, countdown/GO, lap chime, final-lap fanfare, finish horn, wrong-way, respawn, Golden Battery, 7 UI cues), `ambience` 14 (room tone, birds, crickets, wind, rain light/heavy + indoor, 3 thunders, supermarket hum, workshop hum + tools).
- Director (debug log, `dod3_cup.log`): Menu → silence on loading → Race1/Race4/Race3/Race2 (cup tracks) → `final_lap: true` crossfade in every race → Menu; restart restarts the loop. All 10 items, hits, respawns (to-track and flip), countdowns, laps, final laps and finishes occurred in the logged races (their cue mapping is unit-tested in rc_audio).
- Output (`tools/scripts/qa/audio_check.sh`, records the sink monitor; RMS dBFS per second): menu −25.7…−27.9 (music 0.7); music bus 0 → −120 (silence: buses work); `mute_unfocused` on with focus → sound, focus moved to another window → −120 within 1 s; Toy Town autopilot race −12.9…−17.0 (engines + SFX + music). `target/qa/audio/*.wav`.
- No device (`ALSA_CONFIG_PATH=/nonexistent`): "no audio device: the game runs muted", smoke passes.
- No audio WARN/ERROR in any run. Gaps: stingers/one-shots unobservable (Q19); B6 (human listening).

### 3.8 DoD-7 — performance
`tools/scripts/qa/bench.sh` (pinned build, 1920×1080 physical, default High, vsync off, autopilot, seed 1; each scenario re-run if another `recharged` process appeared before or during it). JSON: `target/qa/logs/bench-<scenario>*.json`.

| Scenario | fps | p50 ms | p95 ms | p99 ms | max ms |
|---|---|---|---|---|---|
| toy_town_1p | 192.9 | 5.14 | 5.93 | 6.70 | 30.1 |
| toy_town_2p | 155.1 | 6.39 | 7.42 | 7.99 | 26.2 |
| toy_town_3p | 143.4 | 6.89 | 7.99 | 8.81 | 26.4 |
| toy_town_4p | 117.4 | 8.43 | 9.71 | 10.52 | 28.8 |
| backyard_bash_1p | 192.1 | 5.08 | 6.04 | 8.75 | 26.8 |
| backyard_bash_2p | 145.0 | 6.73 | 8.15 | 9.82 | 26.1 |
| backyard_bash_4p | 107.9 | 9.15 | 10.37 | 11.61 | 29.3 |
| maple_street_1p | 196.4 | 5.07 | 5.70 | 6.15 | 7.6 |
| maple_street_2p | 148.3 | 6.73 | 7.64 | 8.75 | 10.9 |
| maple_street_4p | 109.4 | 8.96 | 10.66 | 13.62 | 33.2 |
| maple_street_1p · storm · night | 184.8 | 5.35 | 6.19 | 7.13 | 26.9 |
| maple_street_4p · storm · night | 100.4 | 9.89 | 11.50 | 12.44 | 17.4 |

GDD §1.3: 1080p 60 fps at High in 1P ✓ (×3), 4P ≥ 50 fps at Medium ✓ (≥ 100 fps even at High). Maple Street (941 u, 459 props) is the heaviest outdoor track. B14 (Toy Town slow) is gone. Runs made while other games competed for the GPU were discarded (e.g. backyard_bash_1p 163.7 fps, p99 31.8).

### 3.9 DoD-8 — robustness
- CI green (§3.1). Determinism: `a_seeded_headless_race_repeats_exactly` and `a_full_field_on_toy_town_repeats_exactly` pass (15 s); three CLI headless runs of a stormy 8-car item race (`--quick-race backyard_bash --weather storm --ai 7 --seed 7`) give identical JSON (md5).
- `--soak 30` (seed `1792365836540779723`, `target/qa/logs/soak30.log`): **soak passed: 129 races** (28 cup, 39 quick, 30 TT, 32 Versus; restarts, pauses, next race/track, rematches, quits in Loading / Setup / race), **128 menu returns** equal to the baseline (335 entities, 490 resources), 0 panics, 0 ERROR (WARN only: 9 × "NextRace: the cup race has no result yet" — the soak's own random command — and winit XSETTINGS). Loads 0.2–3.2 s, restarts ≤ 0.04 s. `--soak 10` with the same seed passed too (43 races). Memory: Q21.
- Other robustness checks: load errors (truncated track + bad `items.ron` field via `--assets-dir target/qa/broken-assets`): automated runs exit 1 by design; in a normal run (`tools/scripts/qa/load_errors_ui.sh`) a "Something went wrong … You're back in the menu" dialog with file:line:col appears and the game returns to Car select ✓ (`target/qa/shots/load_errors_ui1_12.png`). Window focus loss in a race auto-pauses ✓ (`tools/scripts/qa/focus_loss.sh`, `target/qa/shots/focus1_24.png`). Corrupt saves / save failure / no audio device: §3.4, §3.7.
- Panics: none in 70+ logs (`grep panicked`). Recurring upstream WARNs at startup: winit "error setting XSETTINGS", Bevy "Couldn't get swap chain texture after configuring. Cause: 'Outdated'".

### 3.10 Feel & fun review
From stills and logs of ~40 autopilot races (cup, quick, Versus, weather matrix) and the scripted human runs.

| Pri | Observation | Suggestion |
|---|---|---|
| high | Goo Bomb punishes whoever gets it (Q6); hazards hit 3 cars each → chains of "Oil Slick hit X/Y/Z!" notifications, cars spinning in packs | Q6 fixes; cap notifications about others' hits at 1 line ("Oil Slick hit 3 racers") |
| high | Keyboard/pad menu flow has two traps (Q1, Q2) that a new player meets in the first minute | fix before any playtest |
| medium | Assist + throttle (the "kid mode") can't get round (Q7); "Stuck? Hold Reset" is the main feedback | stronger assist level |
| medium | Wrench Works is the hardest to drive cleanly — props on the line and the bench jump (Q4); every AI loses time there | widen the ramp-foot exit, fewer props on the line |
| medium | Name tags too big up close (Q16); with 8 cars bunched the view gets busy | smaller cap, fade near |
| low | HUD reads well at 720p 4-way (big position, item icon, charge "BOOST READY"); finish/standing screens are clear; the charge meter + hints teach the boost | keep |
| low | Pacing: races 80–115 s for fast drivers; the Elite autopilot won every Rookie Cup race comfortably (e.g. Aisle Rush by 11 s) — difficulty for humans needs a human playtest | user playtest |
| low | Fog is light; storm reads mostly as heavier rain (lightning flashes not captured in stills) | denser fog at distance; check storm in motion |
| low | Camera: chase framing good on all tracks; look-back still black against walls (B23, in T19) | — |

## 4. Backlog status (`docs/tasks/BACKLOG.md`)
| # | Status at 071f64d |
|---|---|
| B1 | **Confirmed, mild**: tour frames still pass close to cubes (toy_town fwd 001/016) |
| B2, B4, B5, B8, B10–B13, B15–B17, B19, B23–B25, B27 | In T19 (not re-verified). Observed in passing: B23 look-back black on the Backyard deck (`kb_06_look_back`); B8 — the Credits screen already lists team/tools/engine (content), but its layout over the car is Q14 |
| B3 | **Resolved**: previews for all six tracks exist and show in Track select / Time Trial |
| B6 | Open (no human listening); objective output and bus checks in §3.7 |
| B7 | Upstream, n/a |
| B9 | **Resolved for the workshop** (yellow/black tyre-wall blocks); house/supermarket use furniture/shelves as edges — fine in tours |
| B14 | **Resolved**: Toy Town 193 fps 1P / 117 fps 4P in isolation |
| B18 | **Done** — result: Q4 (wrench_works mirror fails) |
| B20 | Done (not re-checked) |
| B21 | Still open (supermarket props 15 MB); see Q21 |
| B22 | Not reproducible in-game: Aisle Rush allows only Sunset/Night, where lit ceiling rows are plausible |
| B26 | **Resolved**: `rc_render/src/quality/bench.rs` no longer exists |

## 5. QA tooling (`tools/scripts/qa/`)
| File | Purpose |
|---|---|
| `pinned.sh` | runs `target/qa/bin/recharged` with `ui/dist` + `assets` pinned to the tested commit (private mount namespace); waits while the user plays (a `recharged` without a `target/` user-data dir) |
| `kb_modes.script`, `pad_modes.script` | DoD-1: every screen, Settings tabs, Championship, Time Trial, dialogs — keyboard only / gamepad only |
| `vs2_vertical.script`, `vs3.script`, `vs4.script` | DoD-4 split-screen with mixed devices |
| `dod3_cup.script`, `dod3_restart.script` | DoD-3 Rookie Cup + TT (run with `--autopilot`), then the app restart |
| `trackselect_reach.script`, `hover_steal.script` + `hover_steal.sh`, `rebind_conflict.script` | repros for Q1, Q2, Q3 |
| `corrupt.sh` + `corrupt.script`, `corrupt_ui.sh`, `load_errors.script`, `load_errors_ui.sh`, `save_fail.script`, `focus_loss.sh` | robustness (normal-run variants use `RECHARGED_UI_SCRIPT` + `timeout` + `import`, since automated runs exit on load errors and skip focus pause by design) |
| `smoke_shots.script` | smoke flow with a screenshot per step |
| `bench.sh` | DoD-7 scenarios, re-run when another game process appears |
| `weather_time.sh`, `tours.sh`, `sheets.sh` | DoD-5 matrix, DoD-2 tours, contact sheets |
| `audio_check.sh` | DoD-6 sink capture per settings case |
| `batch_a.sh` … `batch_d.sh` | the batches as run (3 windows in parallel, placed with `place.sh`, pointer parked off-window — see Q2) |
| `log_scan.py`, `item_stats.py`, `rss.sh` | panics/WARN/ERROR + load/restart times; item statistics; memory sampling |

Every script assumes a fresh `--user-data-dir` and default settings (pass `--unlock-all` where noted).

## 6. Evidence index
| What | Where |
|---|---|
| Run logs (stderr) | `target/qa/logs/<run>.log`; session logs in each data dir's `logs/` |
| Script screenshots | `target/qa/data/<run>/screenshots/` (`kb_modes2`, `pad_modes`, `kb_shipped`, `pad_shipped`, `vs2v`, `vs3`, `vs4`, `dod3` (`cup_*`, `rst_*`), `tsr`, `hover_a_{on,off}`, `rebind1`, `corrupt1`, `save_fail`) |
| Contact sheets | `target/qa/shots/sheets/` |
| Weather / time matrix | `target/qa/shots/wt/` |
| Track tours | `target/qa/data/tour/<track>_<dir>/captures/`, sheets `target/qa/shots/tours/` |
| Track plots | `target/qa/plots/` (+ `target/qa/logs/plots.log`) |
| Bench JSON | `target/qa/logs/bench-*.json` |
| AI test / probe | `target/qa/logs/every_layout.log`, `target/qa/logs/ww_mirror_probe.log` |
| Soak | `target/qa/logs/soak30.log`, `soak10.log`, `*_rss.log` |
| Audio captures | `target/qa/audio/*.wav` |
| Curated images | `docs/qa/img/` |

## 7. Fix status (orchestrator-maintained)
| Issue | Status |
|---|---|
| Q1, Q2, Q3, Q5, Q10–Q18, notification collapse | fixed in T20b (4247177) |
| Q4 | fixed in T19a (3d53d14): every_layout 28/28, wrench_works worst 1.19×, ≤ 2 respawns per car |
| Q6 | improved in T19a (oil/pad 15 s, softer launches, goo aiming); goo self-hits still high → T19a round 2 (B34) |
| Q7 | fixed in T19a (assist auto-brake); pace too generous → T19a round 2 (B35) |
| Q22 | Toy Town and Wrench Works default to 4 laps (8132efd) |
| Q8, Q9, Q19, Q20, Q21 | T20c in progress |
