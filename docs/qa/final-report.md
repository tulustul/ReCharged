# ReCharged — final regression QA report (T21)

| | |
|---|---|
| Build tested | **98fd0d4** (`docs: T21 brief notes after T20c …`). Its code is that of **8b1bd4b** (T20c), the last code commit. Built once from a clean tree (`git status` empty) on 2026-10-19 at 07:04. Dev profile (`cargo build -p recharged`, md5 `eb9161ac…`), copied to `target/qa2/bin/recharged`. A release copy (`--release`, md5 `d270c691…`, `target/qa2/bin/recharged-release`) was used only for four extra bench/load rows. The UI bundle came from `./tools/ci.sh`. |
| Machine | Same as T18: Arch Linux, kernel 7.2.9, Ryzen 7 5800X (16 threads), 32 GB, RTX 3070 8 GB (driver 615.78.08, Vulkan), X11/i3, 2560×1440 at DPI 1.1667, rustc 1.99.0, node 24.15. |
| When | 2026-10-19, 07:03–08:50 local. Nobody else edited the tree. Before every window `live.sh` checked `pgrep recharged`: no user session was seen and no wait or retry was ever needed. |
| How | **Runner:** `tools/scripts/qa/live.sh` runs the copied binary with the tree's `assets/` and `ui/dist`; no pinning was needed. Every run used `--user-data-dir target/qa2/data/<run>`.<br>**1. Timing block first, fully isolated** (no other `recharged`, no cargo; `bench.sh` checks before and during each run): `bench.sh`; T20c's measurement build for menu-flow loads; human-mode runs of the tested build.<br>**2. Functional block:** at most 2 windows (queue A at 0,0; queues B and C at 1067,600; pointer parked off-window), with headless/CPU checks running beside it.<br>**3. Solo stage:** human-mode captures, the audio sink capture, and an isolated 10-min soak.<br>**Human-mode runs** (no automation flag: vsync, async pipelines, menu warm-up) were driven by `tools/scripts/qa/human.sh`. It sends real key presses with xdotool, only while the game window has focus, and stops the game with SIGTERM.<br>Batch logs: `target/qa2/logs/{t21_timing,queue_a,queue_b,queue_c,headless,final}.out`. |

All evidence is under `target/qa2/` (index in §8). In total: 175 logs and 0 panics.

## 1. Definition of Done (GDD §21)

| # | DoD item | Verdict | Evidence |
|---|---|---|---|
| — | Build health (`tools/ci.sh`, clean tree) | **PASS** | fmt, clippy `-D warnings`, **601 Rust tests passed** (11 ignored), bindings fresh, UI typecheck, **39 UI tests**, UI build. Took 24.7 s because the build was cached. Log: `target/qa2/logs/ci.log`. |
| 1 | All four modes, keyboard only and gamepad only (scripted) | **PASS** | All exit 0:<br>• `keyboard_only` and `gamepad_only` (Quick Race + 2P Versus)<br>• `kb_modes` and `pad_modes` (every menu screen, all 5 Settings tabs, a Championship race with pause → Settings, Time Trial + ghost retry, dialogs)<br>• `smoke` 4/4<br>• `trackselect_reach`, `rebind_conflict`, `hover_steal` A/B<br>Q1, Q2, Q3, Q12 and Q13 are fixed. Throttle-only + assist now finishes everything: Sofa 5-lap cup race in 121.5 s / 128.7 s. |
| 2 | 6 tracks × Fwd/Rev × Mirror; AI 3 laps ≤ 1.5× expected, ≤ 2 respawns/lap | **PASS** | `every_layout` **28/28** (161 s). Worst shipped layout: maple_street mirror at 1.16×; test_oval rev+mirror at 1.43×. At most 2 respawns per car. Wrench Works 0.99–1.09× with ≤ 1 respawn (T18: 1.56× and 4 respawns). All 28 plots have no lints. Tours (fwd + rev) of Toy Town, Wrench Works, Backyard Bash and Maple Street (the tracks whose data changed) are clean. |
| 3 | Unlocks, records, medals, ghosts, settings persist | **PASS** | Rookie Cup gold through the real menus. The unlocks appear in the Results panel and on Cup end (no toasts).<br>TT Toy Town in the Hornet: Silver, record, ghost saved, retried against the ghost.<br>After an app restart all of it persisted (mph, HUD 125 %, trophy, cars, cups, record, ghost; split −0.28 vs the ghost).<br>Stats: races 6, wins 4, podiums 4. Corrupt saves recover with visible toasts (automated and human mode). A read-only data dir shows "Couldn't save" and the game keeps going. |
| 4 | 2–4P split-screen, per-player HUD, mixed devices | **PASS** | • 2P top/bottom (both shipped scripts)<br>• 2P side-by-side (kb A + pad, rain)<br>• 3P + standings/minimap quadrant (kb A + kb B + pad, Maple snow → Next track)<br>• 4P (kb A + pad + kb B + pad 2, storm; P4 pauses; scoreboard 10/8/5/2; Lobby keeps players)<br>• 4P at 720p (HUD legible)<br>• Colour-blind shapes in 3P |
| 5 | Weather (6) and time of day (4) distinct | **PASS** | 35-cell matrix. The six weathers are clearly distinct on both outdoor tracks; fog now reads as fog; storm lightning was caught in a dense capture. Toy Town and Sofa Circuit now have four distinct times (Q8 fixed).<br>Gameplay effect (headless Phantom TT, seed 3):<br>• Backyard: clear 60.0 / rain 63.9 / storm 63.4 / snow 70.9 s<br>• Maple: 100.9 / 111.3 / 106.4 / 107.2 s<br>• Toy Town identical in clear and storm (indoor) |
| 6 | §16 music cues and SFX present, mixed via buses | **PASS** (notes) | Cue log (Q19):<br>• one-shots: **45/46**<br>• stingers: `results_jingle` + `cup_victory`<br>• thunder: `thunder_1` + `thunder_2`<br>• music director: Menu, Silence, Race1–4 and every `final_lap` variant<br>`golden_battery_jingle` and `podium_fanfare` were not reached by automation (see §4); `thunder_3` cannot play in a Storm preset (**R4**).<br>Sink capture: menu −25.6…−28.8 dBFS, music bus 0 → −120 (buses work), race −12.4…−21.9 dBFS. The no-device run is muted and passes smoke. No rc_audio warnings. |
| 7 | §1.3 performance on the RTX 3070 (bench) | **PASS** | 1080p High, dev profile, isolated:<br>• 1P: 185–204 fps, p99 ≤ 6.6 ms, max ≤ 14.9 ms<br>• 4P: 105.6–115.6 fps at High, while the target is 50 fps at Medium<br>The max frame is now 7–15 ms in every row (T18: 26–33 ms).<br>Menu-flow loads: 0.25–0.32 s with T20c's measurement build, 0.95 s with the tested build in human mode. Restarts ≤ 0.07 s. "Race start: 0 pipelines" in every menu flow. |
| 8 | Tests/clippy/UI; deterministic replay; 30-min soak, no panic/leak | **PASS** (note) | CI green. Both determinism tests pass (20.4 s), and 3 CLI runs (Backyard, storm, 8 cars, items) give identical JSON (md5 `c5f2675b…`).<br>`--soak 30` (T18 seed): **130 races, 129 menu censuses equal to the baseline**. The census now includes 70 images, 334 meshes, 256 materials and 63 audio sources. 0 panics; no ERROR except those caused on purpose.<br>RSS reaches 4.63 GB at 30 min and keeps creeping up (**R3**). An isolated 10-min soak reproduces T20c's ≈ 2.8–3.0 GB. |

## 2. Performance

**Bench** (`tools/scripts/qa/bench.sh`, 1920×1080 physical, Settings::default() = High, vsync off, autopilot, seed 1, isolated). JSON: `target/qa2/logs/bench-*.json`. The bench only runs at High, so the 4P rows are stricter than the GDD's "Medium".

| Scenario | T21 fps | p99 ms | max ms | T18 fps / p99 / max |
|---|---|---|---|---|
| toy_town_1p | 204.1 | 6.63 | 13.9 | 192.9 / 6.70 / 30.1 |
| toy_town_2p | 160.0 | 7.44 | 9.9 | 155.1 / 7.99 / 26.2 |
| toy_town_3p | 150.4 | 7.69 | 11.1 | 143.4 / 8.81 / 26.4 |
| toy_town_4p | 115.6 | 10.72 | 12.3 | 117.4 / 10.52 / 28.8 |
| backyard_bash_1p | 195.9 | 6.44 | 9.2 | 192.1 / 8.75 / 26.8 |
| backyard_bash_2p | 145.1 | 9.00 | 10.9 | 145.0 / 9.82 / 26.1 |
| backyard_bash_4p | 112.0 | 11.02 | 14.1 | 107.9 / 11.61 / 29.3 |
| maple_street_1p | 197.7 | 6.60 | 14.9 | 196.4 / 6.15 / 7.6 |
| maple_street_2p | 150.2 | 8.43 | 10.5 | 148.3 / 8.75 / 10.9 |
| maple_street_4p | 107.6 | 11.41 | 14.3 | 109.4 / 13.62 / 33.2 |
| maple_street_1p storm night | 185.0 | 6.44 | 13.3 | 184.8 / 7.13 / 26.9 |
| maple_street_4p storm night | 105.6 | 11.48 | 14.3 | 100.4 / 12.44 / 17.4 |
| *release* toy_town_1p / 4p | 206.9 / 132.7 | 6.35 / 9.22 | 8.1 / 10.8 | — |
| *release* backyard_bash_1p / 4p | 198.1 / 112.3 | 6.11 / 10.66 | 7.4 / 13.8 | — |

The numbers match T20c's final table (`target/agent-data/T20c/logs/bench_final8.md`) within ±3 %. There are no regressions.

**Load times** (`python3 tools/scripts/qa/log_scan.py <log>`)

| Path | Build | Loading → InRace | Notes |
|---|---|---|---|
| From the menu after the warm-up: Quick Race Toy Town 1P | T20c measurement build (the brief's method) | **0.32 s** | race start: 0 pipelines, worst frame 27 ms (`t_menu_loads.log`) |
| … then the first outdoor load, Backyard Bash | same | **0.25 s** | 0 pipelines, worst 9 ms |
| … repeat load of Backyard Bash | same | **0.25 s** | 0 pipelines, worst 10 ms |
| … 4P Versus | same | **0.27 s** | 0 pipelines, worst 25 ms (`t_menu_loads_vs4.log`) |
| From the menu, **tested build, human mode** (vsync on, real keys after `menu warm-up done in 11.9 s`) | tested dev build | **0.95 s** | 0 pipelines after the warm-up, worst frame 20 ms (`t_human_race.log`); the gap is vsync plus async compile |
| Menu warm-up, human mode | tested dev build | — | 6 stages, 187 pipelines in 11.9 s; menu frames > 25 ms: 34, 28, 30 ms at splash/title only (`t_human_idle.log`) |
| Cold `--quick-race` (synchronous, no menu warm-up; not the budget) | dev / release | 3.8–5.3 s / 4.3–5.4 s | bench logs. T20c's note says 3.7–4.8 s in release; Backyard 1P release took 5.39 s. |
| Repeat loads, automated | dev | median 0.35 s, p90 0.43 s | 208 soak loads; first visits up to 4.3 s |
| Restart (Setup → Countdown) | dev | ≤ 0.07 s in every run | 294 soak restarts, max 0.04 s (budget 0.5 s) |

**RSS** (`rss.sh`)

| Run | Samples |
|---|---|
| `--soak 30`, 1600×900, beside one other game window (T18 seed) | 2.44 GB at 50 s → 4.00 GB at 6 min → 4.31 GB at 12 min → 4.40 GB at 20 min → 4.54 GB at 25 min → **4.63 GB at 30 min**. At 21 min: RssAnon 2.55 GB, RssFile 1.12 GB, RssShmem 0.86 GB. (T18: 4.03–4.36 GB between min 18 and 30.) |
| `--soak 10`, 1280×720, isolated (T20c's conditions) | 2.25 GB at 55 s → 2.67 GB at 5 min → **2.81 GB peak at 10 min**; 44 races, 43 censuses equal (T20c: 2.99 GB) |

## 3. Fix verification

| Item | Status | Evidence |
|---|---|---|
| Q1 Track options reachable by keyboard/pad | **verified** | Toy Town set to Reverse, 1 lap, Cloudy, Sunset with the keyboard; Back returns to the card and keeps the options (`target/qa2/shots/sheets/tsr.png`). |
| Q2 Resting pointer steals focus | **verified** | `hover_steal.sh`: with the pointer inside the window and parked outside, both runs open Settings and switch km/h → mph; `settings.ron` has Mph in both (`target/qa2/shots/hover_ab.png`). The cursor hiding is not visible in game-rendered captures. |
| Q3 Rebind conflict | **verified** | Swap plus "Swapped with Camera"; Reset bindings restores the defaults and clears the notice (`rebind1` rb_01/02, `kbm_18`, `padm_18`/`18b`). |
| Q4 Wrench Works mirror respawn trap | **verified** | `every_layout`: all four Wrench variants 0.99–1.09×, 0–1 respawn per car (`target/qa2/logs/every_layout.log`). |
| Q5 Colour-blind markers | **verified** | With the setting on, the standings show ◆★✚⬟⬢ per rival and P1–P3 for humans; minimap dots use the same shapes; the 1P name tag reads "▲ Mayhem" (`target/qa2/shots/sheets/q5_standings_ab.png`, `target/qa2/shots/cb1p_tag_zoom.png`). |
| Q6 Goo self-harm; oil/pad victims | **partly** | **Goo Bomb:** the user caught by their own never-passed bomb in 5/23 explosions in headless races (22 %) and 9/25 in windowed races (36 %); T18 was 15/16.<br>**Oil Slick:** 2.5 victims per drop headless, **3.0 per drop in windowed 8-car races** (T18: 3.4).<br>**Spring Pad:** 1.5 / 2.25 per drop (T18: 2.75).<br>See **R1** (`target/qa2/logs/item_stats.txt`, `tools/scripts/qa/hazard_start.py`). |
| Q7 Assist + throttle-only | **verified** | Assist probe (3 laps): 1.13–1.29× the reference lap, 0 respawns, 0 "Hold Reset" hints on all six tracks (`assist_*.log`).<br>Keyboard Backyard 1 lap: 29.15 s, 3rd (T18: 38.1 s + stuck).<br>Sofa cup race finished (121.5 s / 128.7 s).<br>TT Backyard 1:17.3, no medal (Bronze 1:08.85). |
| Q8 Indoor time of day | **verified** | Toy Town and Sofa Circuit: four distinct times; dark room with lamp pools at night; warm, readable sunset (`target/qa2/shots/sheets/q8_toy_town_new_vs_old.jpg`, `q8_sofa_new_vs_old.jpg`, `target/qa2/shots/sheets/wt/`). |
| Q9 Loading → Intro ≤ 3 s | **verified** | 0.25–0.32 s from the menu (measurement build); 0.95 s with the tested build in human mode (§2). |
| Q10 Toasts over Results | **verified** | Race-4 Results and Cup end list the unlocks in the panel; no toasts (`dod3/cup_11`, `cup_14`). |
| Q11 PTS outside cups/Versus; TT counted as wins | **verified** | No PTS column in Quick Race/TT results (`kb_09`, `padm_35`, `cup_22`). Stats after 4 cup races + 2 TT runs: 6 races / 4 wins / 4 podiums (`cup_28`). |
| Q12 Post-race Change track options | **verified** | Versus Next track (`vs3_09`) and Quick Race Change track (`qr_change` qct_04–07): layout, mirror, laps, rivals, AI, items, weather, time are all reachable. |
| Q13 Smoke flake, diagnosability | **verified** | Smoke 4/4 (≈ 27 s each). The log shows every `UI screen:` change and every handled `ui.nav` with its screen. |
| Q14 Showroom vs screens | **verified** | Car hidden in Garage and Credits (`kbm_02`, `kbm_12`); podium cars stand on the 1-2-3 blocks (`cup_14`, `podium/pod_03`). B37's few-pixel offset remains. |
| Q15 Locked cup overlays | **verified** | Reason in the card footer, race list dimmed (`kbm_21`, `padm_21`). |
| Q16 Huge name tags | **verified** | Tags are small and hidden when close (`vs3_05`, `vs4_720p_001`, cb1p, Sofa matrix cells). |
| Q17 `--bench` on a locked track | **verified** | `--bench maple_street_1p` on a fresh profile without `--unlock-all` → exit 0 (`q17_bench`). |
| Q18 Boot toasts/splash unseen | **verified** | Splash plus two "Save file restored" toasts visible in an automated run (`corrupt1` cor_01–03) and in human mode (`target/qa2/shots/corrupt_ui1_2..4.png`). |
| Q19 Stingers/one-shots not logged | **verified** | `audio cue …` lines; coverage in DoD-6 (`tools/scripts/qa/audio_cues.py`). |
| Q20 Long frame per bench run / compiles after GO | **verified** | Bench max frame 7.4–14.9 ms in all 16 rows. Race start: 0 pipelines in menu flows and in cold 1P runs. No compile frame after GO in any of the 35 matrix cells (rc_render=debug). Cold automated split-screen still compiles 2 at Intro start, as documented. |
| Q21 Memory / asset census | **verified** (census) / **partly** (RSS) | The census counts assets and all 172 checks were equal. The isolated soak reproduces T20c's figure. At 1600×900 the RSS creeps up for 30 min (**R3**). |
| Q22 Short races | **verified** | Elite autopilot, default laps: Toy Town 4 laps 101–103 s, Wrench Works 4 laps 123 s, Sofa 5 laps 98–108 s, Backyard 5 laps 109 s, Aisle Rush 3 laps 98–114 s, Maple 3 laps 112 s. |
| Intro hitch (T19b) | **verified** | 0 pipelines after the loading warm-up in cold 1P runs. Exception: non-clear weather compiles one tonemapping pipeline at Intro start (**R5**). |
| Notification collapse (T20b) | **verified** | Single lines such as "Oil Slick hit 6 racers" and "Spark Rocket hit Rusty!". |
| Fog and storm lightning (T20c) | **verified** | Fog is dense at distance (`target/qa2/shots/sheets/wt_*_weather_small.jpg`). A bolt and sky flash were caught in a dense night-storm capture (`target/qa2/shots/sheets/lightning.jpg`, frames ms_019 and ms_050). |
| B1 Tour camera through cubes | **verified** | Tour frames stay clear of the cubes (`target/qa2/shots/tours/`). |
| B2 Quit mid-compile | **verified** | Window closed (i3 kill = WM_DELETE) 1 s and 3 s after start → exit 0, `exit: pipeline queue idle after 2.08 s / 0.12 s` (`t_human_close*.log`). |
| B4 Unused deps | **verified** | rc_weather and rc_audio manifests have no serde/ron. |
| B5 Banked-edge lint | **verified** | Lint plus test `banked_turns_whose_inner_edge_sinks_into_the_floor_are_linted`; 28 layouts are lint-free. |
| B8 Credits content | **verified** | Team, tools, generators (Blender, numpy, ffmpeg), Saira OFL notice (`kbm_12`, `ui/src/strings/credits.ts`). |
| B10 Finished human handed to the AI | **verified** | `rc_race: the AI drives the finished human's car` appears in every finished-human race log (`kb_shipped`, `pad_shipped`, `kb_modes`, `pad_modes`, `dod3`, `vs2v`). |
| B11 Rocket re-lock, EMP/explosions vs walls, orb edges | **verified** (tests) | rc_items tests `a_rocket_that_loses_its_lock_locks_onto_the_next_car_in_its_cone`, `an_emp_does_not_reach_through_a_wall`. Rockets: 103 fired → 84 hits in windowed races. |
| B12 Wheel contact points | **verified** (code/tests) | `WheelState.contact`/`normal` exist; rc_vfx skid/splash tests pass. Marks on ramps were not inspected close up. |
| B13 Soft particles | **not verified** visually | No hard edges were seen in the dust puffs in the feel stills; there was no dedicated check. |
| B15 Missed-checkpoint notice | **verified** | "Missed checkpoint — go back" on Maple Street with the car in the south lane past the next gate; position drops while the distance is frozen (`spot_b15_maple` m15_b_05s, `target/qa2/shots/b15_notice_zoom.png`). Backyard attempts could not miss a gate because the player starts on pole. |
| B16 AI shortcuts / line of sight | **verified** (tests) | rc_ai `only_shortcuts_that_save_time_are_planned`, `a_shortcut_is_decided_once_per_approach_and_dropped_after_it`. |
| B17 Item-race respawns | **verified** | 18 headless 8-car item races: 0.073 to-track respawns per racer-lap (Toy Town 0.094; target ≤ 0.12; T18: 0.14–0.33). |
| B19 Maple Street data | **partly** | The changes are in the data; tours are clean (leaf piles, curbs). The leaf-pile collider height was not measured. |
| B23 Look-back against a wall | **verified** | Nose-first against the Toy Town barrier, the look-back boom swings up and looks down at the car (`spot_b23/b23_06`). Against the garden fence the view is close but not black (`spot_b23_deck`). |
| B24 race.start debounce | **verified** | UI unit test; no `StartRace ignored` WARN in 175 logs. |
| B25 TT outliers | **verified** | Toy Town TT over 8 seeds: 76.1–77.2 s (Sparky), no outliers. Phantom seeds 1–3: 72.37 / 71.50 / 72.87 s; the median equals `medals.ron` (72.37). |
| B27 UI-script nav verbs removed | **verified** | `RECHARGED_UI_SCRIPT="2 nav confirm…"` is rejected with a pointer to `--input-script`. |
| B28 Night-lamp shadow compiles | **verified** | Night cells with 3–28 lamp lights: 0 compile frames after GO. Storm-night bench max 13–14 ms. |
| B29 Split-screen compiles at GO | **verified** | 2P/4P menu flows: race start 0 pipelines. Cold automated 2–4P: 2 at Intro start (documented). |
| B30 Grounded rain splashes | **verified** (tests) | `splashes_lie_on_the_ground_plane`, `far_splashes_snap_to_the_track_surface_under_them`; rain cells show splashes at the wheels. |
| B31 Load budget | **verified** | Same evidence as Q9. |
| B32 Toy Town reverse T1 spin | **verified** | toy_town rev 1.05× with 0 respawns; no TT outliers. |
| B33 Backyard kicker flips assisted Phantom | **verified** | Throttle-only + assist Backyard TT (keyboard and pad) and the 3-lap assist probe: 0 respawns or flips. |
| B34 Goo Bomb on its carrier | **verified** | Same evidence as the Goo part of Q6. |
| B35 Assist too generous | **verified** | Throttle-only runs are 1.21–1.33× the medal reference on every track, so no medals. |
| B36, B37 | **still open** | B36 is also visible through the Versus lobby P3/P4 cards and the locked cup cards (`kb_shipped/kb_10`, `kbm_21`). B37's offset is a few pixels (`cup_14`). |

## 4. Gaps

These were not reached by automation; none of them is a defect:
- **`golden_battery_jingle`.** Batteries sit off the racing line, and 0/30 Elite-autopilot headless races collected one (`target/qa2/logs/golden_battery_probe.txt`). The cue mapping is in `rc_audio/src/shots.rs:77`.
- **`podium_fanfare`.** It plays only for a silver/bronze Cup end (`ui/src/screens/CupEnd.tsx:34`). Three one-race cup attempts with a throttle-only player finished 4th, 6th and 8th.
- **Not re-run:** `focus_loss.sh` and `load_errors_ui.sh` use the `RECHARGED_UI_SCRIPT` nav verbs that B27 removed. They are superseded by `human.sh race` and `human.sh error`, which pass:
  - focus loss → `window lost focus: pausing` and the pause menu (`target/qa2/shots/t_human_race_04_focus_lost.png`);
  - a load error → the error dialog with file:line:col, then back in Car select (`target/qa2/shots/t_load_error_0*.png`).

## 5. New issues

| ID | Sev | Area | Summary |
|---|---|---|---|
| R1 | minor | rc_ai items + rc_items + rc_game grid | Hazards dropped from the front hit the bunched field (up to 7 of 7 rivals in one Oil Slick 3.4 s after GO); Oil Slick still averages ≈ 3 victims per drop |
| R2 | minor | rc_ai traffic | Rivals queue behind a car standing still on the grid: 5 of 7 held 3–8 s, the last one ≈ 20 s |
| R3 | minor | memory (soak census) | RSS keeps creeping up in long sessions (4.31 → 4.63 GB between min 12 and 30 at 1600×900) while the census stays flat |
| R4 | polish | rc_audio ambience | `thunder_3` can never play in a Storm preset |
| R5 | polish | rc_render warm-up | Every non-clear weather compiles one tonemapping pipeline right after the loading warm-up (Intro start) |
| R6 | polish | ui Title / showroom | The title logo and tagline sit on top of the showroom car |
| R7 | polish | ui Garage, TrackSelect | Footer hints read "↑↓ Select · Enter Select" |

### R1 — Hazards from the front wreck the bunched pack (minor, balance/feel)
- **Area:**
  - rc_ai item use (when an AI drops Oil Slick / Spring Pad);
  - rc_items (oil victims);
  - `crates/rc_game/src/race_setup.rs` grid order ("local players first", so a human always starts on pole);
  - `assets/data/items.ron` odds (1st band: Oil 30 %, Spring 20 %).
- **Repro:** any windowed autopilot race with items, e.g. `QA_RUN=tools/scripts/qa/live.sh tools/scripts/qa/run2.sh feel_toy_town 0 0 --unlock-all --quick-race toy_town --autopilot --ai 7 --seed 11 --exit-after 175` with `RUST_LOG=info,rc_items=debug`.
- **Expected:** a dropped hazard catches one or two careless followers. GDD §6.2 counterplay is "steer around".
- **Actual:**
  - In `feel_toy_town.log`, GO is at 06:04:01.3. The pole-sitting Player draws an Oil Slick from the first cube row at +3.1 s and drops it at +3.4 s. It hits all 7 rivals by +5.3 s, plus 2 re-hits after respawns.
  - `vs4_720p`: one slick hits 6 racers at ≈ 2.5 s (`target/qa2/shots/vs4_720p_001.png`).
  - Aisle Rush matrix cells show "Oil Slick hit 6 racers" at ≈ 6 s (`target/qa2/shots/wt/aisle_rush_clear_*_001.png`).
  - Cup races 1 and 4 show "Oil Slick hit 3 racers" at 4.9 s (`target/qa2/data/dod3/screenshots/cup_01_r1_racing.png`, `cup_10_r4_racing.png`).
  - Over all windowed races: Oil Slick 35 drops → 105 hits. 9 of the 21 hazard drops made in the first 10 s hit ≥ 3 racers (`python3 tools/scripts/qa/hazard_start.py target/qa2/logs/{dod3,vs*,feel_*,…}.log`).
  - Because humans always start on pole, a human who draws a 1st-band hazard at the first row can take out the whole field within seconds of GO.
- **Fix (pick one or more):**
  - No hazard grants or drops in the first ≈ 8–10 s, or until the first cube row is behind the leader.
  - A slick loses its grip effect after N victims (e.g. 3).
  - AI drops Oil only when a single follower is close, not a pack.
  - Product decision: start local players mid-grid or at the back in Quick Race/cups (the GDD does not specify a grid position).

### R2 — Rivals queue behind a car standing still on the grid (minor, rc_ai)
- **Area:**
  - `crates/rc_ai/src/traffic.rs::choose_lane`: when no side is free it returns `follow_speed: Some(blocker.speed)`, which is 0 for a parked car;
  - `pace.rs` (follow brake);
  - `side_bounds` (cars alongside pin each other).
- **Repro:** `QA_RUN=tools/scripts/qa/live.sh tools/scripts/qa/run2.sh spot_idle 0 0 --window 1280x720 --input-script tools/scripts/qa/spot_idle_start.script --exit-after 120`. Toy Town Quick Race; the player does not drive for 20 s.
- **Expected:** rivals steer around a stopped car within 1–2 s (GDD §8.1 overtaking). This matters for children who miss GO and for cars stopped after a spin.
- **Actual:** the HUD goes 1st → 2nd at 1 s, 3rd at 8 s, 6th at 15 s, 8th only at 20 s. Look-back frames show 4–5 rivals stopped behind the parked car at 3 s and 8 s (`target/qa2/shots/sheets/spot_idle.jpg`, `target/qa2/data/spot_idle/screenshots/idle_0{3,5,7,9}_*_back.png`). The human-mode run showed the same: 2nd/8 at 7.8 s with six rivals behind (`target/qa2/shots/t_human_race_03_racing.png`).
- **Cause (likely):** each queued AI sees the parked car as a blocker it cannot pass, because the "free side" test is vetoed by the other queued cars near the blocker and by cars alongside. It then follows at speed 0, so the grid drains one car at a time.
- **Fix:**
  - Never "follow" a car slower than ≈ 2 u/s: treat it like a hazard (`dodge` on the roomier side), or ignore other followers of the same blocker in `free()`.
  - Add an rc_ai race test: a parked car on pole with 7 AI on the grid; all must be past within ≈ 3 s.

### R3 — RSS creeps up in long sessions (minor, memory)
- **Area:** the soak census counts entities, resources, meshes, images, materials and audio sources, but not what grows. Candidates: the bevy-react JS heap, font atlas sets (excluded by design), wgpu/driver caches.
- **Repro:** `RSS_EVERY=10 QA_RUN=tools/scripts/qa/live.sh tools/scripts/qa/run2.sh soak30 1067 600 --soak 30 --soak-seed 1792365836540779723`.
- **Expected:** flat RSS once every track has been visited (the census proves no ECS/asset leak).
- **Actual:**
  - 30-min soak at 1600×900, beside one other game window: 4.31 GB (12 min) → 4.63 GB (30 min), about +20 MB/min, still rising. RssAnon went from 2.43 GB at 11 min to 2.55 GB at 21 min (`target/qa2/logs/soak30.rss`).
  - Isolated 1280×720 soak10: plateau 2.67–2.68 GB between 5 and 9 min, then 2.81 GB at 10 min (`target/qa2/logs/soak10_720p.rss`). T20c's own 10-min run also crept from 2.83 to 2.99 GB.
  - T18 reached 4.03–4.36 GB under conditions like this pass's 1600×900 run. The fix-status line "4.46 → 2.97 GB" compares different conditions.
- **Fix:** run a 60-min isolated soak that logs RssAnon/RssFile/RssShmem every minute. Add the bevy-react JS heap size, `FontAtlasSet` count and wgpu buffer totals to the soak diagnostics, and fail the soak on a sustained RSS slope.

### R4 — `thunder_3` never plays in a Storm (polish, rc_audio)
- **Area:**
  - `crates/rc_audio/src/ambience.rs::thunder_for` picks variant 0 at intensity ≥ 0.7, variant 1 at ≥ 0.35, else variant 2;
  - `crates/rc_weather/src/sim.rs`: `STRIKE_INTENSITY` is (0.5, 1.0) and Storm lightning is 1.0.
- **Actual:** strike intensity is always ≥ 0.5 in a Storm, so only `thunder_1` (8 times) and `thunder_2` (2 times) played in all storm runs. `thunder_3` (the distant rumble) can only play during a Dynamic Rain↔Storm blend.
- **Fix:** map variants within the real range (e.g. ≥ 0.83 / ≥ 0.67 / else), or choose randomly weighted by intensity.

### R5 — Weather tonemapping variant compiled after the warm-up (polish, rc_render)
- **Evidence:** in every non-clear matrix cell (Backyard and Maple cloudy/rain/storm/fog/snow, Toy Town storm night, Wrench storm noon), `visuals: new pipeline tonemapping pipeline …` appears about 40 ms after `pipelines warm after 9 frames`. The race-start line then reports 1 pipeline (`target/qa2/logs/wt/*.log`, `RUST_LOG=rc_render=debug`). Clear cells report 0.
- **Impact:** it happens at the start of the Intro, not after GO, so Q20 holds. In human mode it can cost the Intro's first frame(s).
- **Fix:** give the loading warm-up camera the session weather's tonemapping/grading key, or add the weather tonemapping variants to the menu warm-up.

### R6 — Title logo covers the showroom car (polish, ui)
- **Evidence:** `target/qa2/shots/t_human_idle_idle.png`, `target/qa2/shots/corrupt_ui1_3.png`, `target/qa2/data/corrupt1/screenshots/cor_03_3s.png`. The Title uses the default `BackdropFocus::Wide`, which frames the car at screen centre under the logo, the progress bar and "Small cars. Big world. Full charge."
- **Fix:** use `BackdropFocus::Room` on the Title, or offset the car or camera below the logo.

### R7 — Duplicate "Select" footer hints (polish, ui)
- **Evidence:** `ui/src/screens/Garage.tsx:48-49` and `ui/src/screens/TrackSelect.tsx:119-120` use `hint.select` for both the `move` and the `confirm` action, so the footer reads "↑↓ Select · Enter Select" (`target/qa2/data/kb_modes/screenshots/kbm_02_garage.png`, `target/qa2/data/tsr/screenshots/tsr_01_toy_town_focused.png`).
- **Fix:** add a move label to `ui/src/strings/common.ts` (e.g. `browse: "Browse"`) and use it for the `move` hint.

**Smaller observations (not filed):**
- Laps cycles from 1 back to the maximum on Left (a cycler design choice).
- A tied TT split shows a red "+0.00".
- The "Couldn't save" toast covers the Settings help header (`target/qa2/shots/sheets/save_fail.jpg`).
- The Elite autopilot holds a Goo Bomb for 70 s when nobody is near; that is reasonable behaviour.

## 6. Feel review

Sources: ≈ 30 windowed races (6 feel races, one per track with items; cup, Versus and matrix races) and the scripted human runs.

| Pri | Observation | Suggestion |
|---|---|---|
| high | The opening seconds are decided by the first cube row (R1). With the human on pole and AI rivals waiting behind a stopped car (R2), a family player either wrecks or blocks the whole field at GO. | R1 + R2 |
| medium | The Elite autopilot (as the player) leads every feel race from the pole and finishes alone; leading Pro rivals is easy. Sessions are 98–123 s, inside GDD's 1.5–2.5 min (Q22 fixed). | A human playtest of difficulty; consider the grid position (R1) |
| medium | Throttle-only + assist is now fun, not stuck: it finishes every track at 1.13–1.29× the reference, with no stuck hints. It even took a cup podium (2nd on Sofa vs Rookie AI) but earns no TT medals. | Keep |
| low | Items read well: one-line notices, the lock-on warning, the visible Storm Strike bolts, the shield bubble. The Goo Bomb now mostly gets passed on. | — |
| low | Visuals: indoor nights, fog and storm now carry mood. Wrench Works night is very dark around the oil spill at the first corner, but the barriers keep the track readable. | — |
| low | The camera handles walls now (B23); chase and look-back framing are good on all tracks. | — |

## 7. Tooling (`tools/scripts/qa/`, for the orchestrator to commit)

**New files**

| File | Purpose |
|---|---|
| `live.sh` | Runner for the tested build; `QA_BIN` picks another binary |
| `run2.sh` | One run: fresh data dir, logs, window placement, optional RSS sampling |
| `t21_timing.sh`, `t21_queue_a.sh`, `t21_queue_b.sh`, `t21_queue_c.sh`, `t21_headless.sh`, `t21_final.sh` | The batches as run |
| `human.sh` | Human-mode runs: idle, close, race, error |
| `menu_loads.script`, `menu_loads_vs4.script` | Menu-flow loads, restart and repeat load |
| `spot_b15.script`, `spot_b15_maple.script`, `spot_b23.script`, `spot_b23_deck.script`, `spot_idle_start.script`, `spot_wrongway.script`, `spot_podium.script`, `spot_podium5.script`, `qr_change_track.script` | Spot checks |
| `audio_cues.py` | Cue coverage from logs |
| `hazard_start.py` | Victims per hazard drop, early vs late |
| `wt_sheets.sh` | Weather/time contact sheets, including old vs new |

**Changed files** (defaults unchanged, so the T18 workflow still works)
- `bench.sh`, `weather_time.sh`, `tours.sh`, `hover_steal.sh`, `corrupt.sh`, `corrupt_ui.sh`, `audio_check.sh`: new `QA_RUN` / `QA_OUT` parameters.
- `bench.sh`: `BENCH_TAG`.
- `weather_time.sh` and `tours.sh`: `QA_XY`.
- `item_stats.py`: adds a Goo outcome line.
- `focus_loss.sh` and `load_errors_ui.sh` no longer work against this build (B27 removed the nav verbs); `human.sh` replaces them.

## 8. Evidence index

| What | Where |
|---|---|
| CI, build | `target/qa2/logs/ci.log`, `build.log`; binaries and md5 in `target/qa2/bin/` |
| Bench | `target/qa2/logs/bench-*.json` / `.log` |
| Loads, human mode, B2 | `target/qa2/logs/t_menu_loads*.log`, `t_human_{idle,race,close,close3}.log`, `target/qa2/shots/t_human_*.png` |
| Script runs | `target/qa2/data/<run>/screenshots/`, `target/qa2/logs/<run>.log`; contact sheets in `target/qa2/shots/sheets/` |
| every_layout, plots | `target/qa2/logs/every_layout.log`, `target/qa2/plots/`, `target/qa2/logs/plots.log` |
| Tours | `target/qa2/shots/tours/` |
| Weather/time matrix | `target/qa2/shots/wt/`, `target/qa2/shots/sheets/wt/` (incl. `*_old_vs_new.png`), `target/qa2/logs/wt/`, `target/qa2/logs/weather_grip.txt` |
| Items, assist, TT spread | `target/qa2/logs/{item_stats.txt,items_*.log,assist_*.log,tt_spread_*.json}` |
| Determinism | `target/qa2/logs/determinism.log`, `det_cli*.json` |
| Soaks | `target/qa2/logs/soak30.{log,rss}`, `soak10_720p.{log,rss}` |
| Audio | `target/qa2/audio/*.wav`, `target/qa2/logs/audio_*.log` |
| Feel races | `target/qa2/shots/feel/`, `target/qa2/shots/sheets/feel_*.jpg` |

## 9. Verdict

**Shippable against the Definition of Done.** All eight DoD items pass on 98fd0d4.

Every first-pass issue is fixed except two that are only partly done: Q6 (hazards still hit groups) and Q21 (RSS still creeps). There are no blockers and no majors. All 0 panics in 175 logs, and the only ERRORs come from deliberately broken inputs.

**Before a public release:**
- **R2** (rivals queue behind a stopped car) and **R1** (start-of-race hazard pile-ups): both shape the first seconds of a family player's race.
- **R3:** confirm whether the slow RSS growth stops or continues past 30 min.

R4–R7 are polish, along with the still-open B36/B37 and B6 (a human listening pass).

## 10. Fix status (orchestrator-maintained, after T22)
| Issue | Status |
|---|---|
| R1 | fixed in T22a (410edaa): players start behind the AI field (GDD §3.2); Oil Slick / Spring Pad are used up after 3 cars. 72 headless 8-car item races: Oil 2.89 → 1.85 victims per drop, drops in the first 10 s 4.48 → 2.68 per drop and at most 3 (was 9); Spring 1.90 → 1.66 |
| R2 | fixed in T22a: a stopped or crawling non-AI car is steered around like a hazard, never followed. Car parked on pole: Toy Town ≤ 2.3 s, Test Oval ≤ 1.5 s, Backyard ≤ 2.6 s to pass (was 8.8–9.2 s and up to 20 s); Sofa Circuit pole is ≤ 1.1 s on 3 of 6 seeds, 4–8 s on the others (`docs/design/rc_ai.md`, known limitations) |
| R3 | fixed in T22c (b953855): Bevy 0.20 keeps the GPU-preprocessing bind groups of every view that ever rendered (`docs/upstream-notes.md` U5); `rc_render/src/preprocess_fix.rs` prunes them. RSS slope after first visits +22–26 MB/min → +1–4 MB/min (40-min A/B), flat over a 25-min run of the final tree; the rest is Bevy's `DespawnOnExit` index leak (U6, ≈ 1 MB/min, documented). Note: `VmRSS` counts the NVIDIA mappings twice (≈ 0.6–0.9 GB of the 4.6 GB QA saw) |
| R4 | fixed in T22b (c7ce373): the thunder variants split the Storm strike intensities in thirds |
| R5 | fixed in T22b: every look keeps the same grading stages in use, so one tonemapping pipeline serves every environment, time and weather; race start 0 pipelines in all 8 non-clear cells |
| R6 | fixed in T22b: logo, bar and tagline at the top of the Title; the car shows below |
| R7 | fixed in T22b: "Browse" move hint |
| B36, B37 | fixed in T22b: Room focus on Cup select / Track select / Versus lobby; per-car podium foot offsets measured from the thumbnails |
| Tied split, save toast | fixed in T22b: neutral "±0.00"; toasts sit halfway down the right edge |
| Q6 (partly in T21) | closed by R1 |
| Q21 (partly in T21) | closed by R3 |

## 11. Final verification of the shipped build (after T22)
Build 9d85d3d (code of 6761e4e), dev profile, isolated runs (`target/final/`):
- `./tools/ci.sh`: pass (614 Rust tests, 39 UI tests, clippy, fmt, bindings, UI build).
- Bench (1080p High): 1P 177–193 fps (max frame ≤ 15.2 ms), 2P 145–157, 3P 143, 4P 104–120 (max ≤ 14.1 ms),
  storm night 1P 182 / 4P 104. Backyard Bash 1P is 177 fps (T21: 196): the players now start
  behind the field, which the camera sees from the start. Every row is far above the targets (60 / 50 fps).
- Input scripts: `smoke` ×2, `keyboard_only`, `gamepad_only`, `kb_modes`, `pad_modes`: all exit 0.
- `every_layout`: 28/28 pass. Determinism: both tests pass.
