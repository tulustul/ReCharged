# rc_game — app flow, modes, progression, persistence (design note)

`GameFlowPlugin` owns everything between the menus and the race: the lifecycle of ARCHITECTURE
§3.1, the loading gate, pause, `RaceConfig` per mode, cup runs, Versus sessions, results →
profile, saves and the soak runner. Systems sit in `FlowSet`, ordered by an internal `Step` chain:
`Signals` (focus, disconnects, soak) → `Progress` (results, batteries, hints, stats) →
`Transitions` (commands, gate, unloading) → `Saves`. Race phases are driven by rc_race (T06).
Progress runs before Transitions, so a restart sent in the frame a result arrives reverts it.

## Modules
| Module | Contents |
|---|---|
| `lib` | `GameCommand`, `RaceRequest` + `PlayerCar`, `GameNotice`, plugin |
| `flow` | boot (catalog, medals, saves), loading gate, `Unloading`, pause & robustness signals |
| `commands` | `StartRace` per mode, restart/rematch/next race/next track/quit/drop player, lobby, `--quick-race` |
| `race_setup` | pure `RaceRequest` → `RaceConfig` and unlock validation |
| `profile` | `Profile`, `Content`, `Trophy`, `Medal`, records, stats, pure unlock rules |
| `records` | pure results → stats, records (geometry hash), medals |
| `medals` | `MedalTable` from `assets/data/medals.ron` |
| `series` | pure points & standings; `CupRun`, `VersusSession` |
| `results` | systems: `RaceOutcome`, Golden Battery, hints, live stats, stale records, profile commands |
| `prefs` | settings edits, rebinding, clamping |
| `save` | files, envelopes, migrations, atomic writes, throttle, ghosts, session logs |
| `soak` | `--soak` runner |

## Commands (`GameCommand`)
| Command | Valid in | Effect |
|---|---|---|
| `StartRace(RaceRequest)` | Menu | validate unlocks → `RaceConfig` → Loading; starts/continues the series (below); closes the lobby |
| `Restart` | InRace | `RacePhase::Setup` + Running; reverts the race's cup/Versus points |
| `Rematch` | InRace | Versus: next race of the session on the same layout, no reload; else = `Restart` |
| `NextRace` | InRace | cup: next race (only once the current race has a result; after the last → Menu); others: same setup again (new seed) → Unloading → Loading |
| `NextTrack(variant)` | InRace, not cups | same options on another layout → Unloading → Loading (Versus counts a new race) |
| `QuitToMenu` | Loading, InRace | Unloading → Menu; an unfinished cup is dropped |
| `Pause` / `Resume` | InRace (pause: Intro/Countdown/Racing only) | `PauseState`; resume is idempotent |
| `OpenLobby` / `CloseLobby` | Menu / any | `LobbyState::open`; closing also ends the `VersusSession` |
| `DropPlayer(slot)` | InRace, Versus | slot's `Controller::Human { autopilot: true }` in `RaceConfig` (later attempts) + `PlayerDropped` (rc_race adds `AiDriver` now) |
| `ApplySettings(Box<Settings>)` | any | clamped (`prefs::sanitize`), saved ≤ 1/s |
| `ResetBindings(scheme)` | any | defaults of one scheme |
| `ResetProgress` | Menu | profile, hints, ghosts erased (React confirms twice) |
| `MarkSeen(content)` | any | clears a NEW badge |
| `QuitGame` | any | `AppExit` (pending saves flushed in `OnAppExitSystems`) |

Also consumed: `PauseRequest` (rc_input), `WindowFocused { focused: false }` → pause (skipped when
`AutomationConfig::is_automated()`: window managers often never focus automated windows),
`DeviceEvent::Disconnected` in a race → pause + `GameNotice::ControllerDisconnected`,
`RebindResult` → the captured binding becomes the action's primary binding and is removed from
the conflicting action, `LoadingDone` / `LoadingFailed`.

## Session data and lifetimes
| Resource | Lifetime |
|---|---|
| `RaceConfig` | `StartRace` → menu; replaced for each next race. Changed between attempts only by rc_game: new TT ghost after a best, Golden Battery collected (`golden_battery = false`), dropped player (autopilot), stale ghost removed — readers re-read it per attempt |
| `RaceOutcome` | results processed (`RacePhase::Finished` + `RaceResults`, once per attempt) → `OnEnter(Setup)` / `OnEnter(Unloading)` |
| `CupRun` | cup `StartRace` → quit/load error (unfinished) or the next `StartRace` (finished: kept for the Cup end screen) |
| `VersusSession` | first Versus `StartRace` → `CloseLobby` or a non-Versus `StartRace` (survives Lobby → race) |
| `Profile`, `Settings`, `MedalTable`, `HintsSeen`, `LobbyState`, `Catalog` | app |

## Race setup per mode (`race_setup`)
- Grid: local players first (slot order, slot colours; "Player" alone, P1–P4 otherwise), then
  rivals in a seeded order (`RngStream::RaceSetup`, Fisher–Yates), so the seed picks the field in
  Quick Race / Versus and the grid order everywhere. Same car twice → distinct base paints.
- Rival class: the cup's (Rookie/Pro/Elite; Elite+ cups field Elite cars), else the best human car
  (Secret = Elite). Difficulty: the cup's, else the request's (must be unlocked; Elite+ never),
  else the settings default clamped down to the best unlocked one.
- Laps: TT 3; cup race override or `default_laps`; Quick/Versus the request (1–9) or default.
  Weather/time: cup race; else the request when allowed on the track (Dynamic outdoor only), else
  the first allowed preset / Noon. Items: cups on, TT off, else the request (default on).
  Catch-up: setting, never in TT. Seed: request or clock; next races chain `rng::draw`.
- `golden_battery`: single-player (one human, not Versus), not reverse, not collected yet.
- `ghost`: TT only, `ghosts/<variant>.ghost` when present.
- Validation: players per mode (Quick 1–4, Versus 2–4, TT/cup 1), unique slots, known and
  unlocked cars, paints (base or unlocked set), cup or layout (track + Reverse + Mirror),
  explicit difficulty. Rejections → `GameNotice::Error`, the game stays in the menu.

## Results processing (once per attempt)
1. Positions → series: cups record every racer, Versus the players (`RacerKey`), each race
   replacing an earlier result of the same race; `Restart` truncates it (points revert).
   Points 10-8-6-5-4-3-2-1; standings tie-break by the latest race, then the one before; missing =
   last. Last cup race → `final_position`, trophy (1st Gold / 2nd Silver / 3rd Bronze), best kept.
2. Stats of the slot-0 player: races, wins, podiums, distance (`RaceProgress::total_distance`);
   items used / hits landed count live from `ItemEvent` / `HitEvent`; play time is real time.
3. Records (single-player only), keyed `variant_key` (`toy_town[_rev][_mirror]`): best lap any
   mode, best race + medal TT only, each with the car and the geometry hash. Times from another
   hash are discarded (medals stay) with `GameNotice::RecordsDiscarded`; this is also checked on
   `OnEnter(InRace)` against `TrackSummary` so a stale ghost never spawns.
4. TT new best → `ghosts/<variant>.ghost` and `RaceConfig.ghost` (Retry races it).
5. Unlocks: every catalog rule (cars, tracks, cups, `Catalog.unlocks`) satisfied for the first
   time → `Profile.unlocked` + `new_unlocks` + `GameNotice::Unlocked`. Earned unlocks stay even if
   progress is later discarded; `--unlock-all` opens everything in memory and earns nothing.
   Also evaluated silently at boot and announced on every return to the menu.
6. `RaceOutcome` + profile save. Golden Batteries (player in a single-player race) and hints save
   immediately; `HintsSeen` mirrors `Profile.hints_seen` for rc_race.

## Save format (`UserPaths::data_dir`)
| File | Content | Written |
|---|---|---|
| `settings.ron` | `(version: 1, data: Settings)` | at most once per second after a change; flushed on exit |
| `profile.ron` | `(version: 1, data: Profile)` | results, battery, hints, reset, MarkSeen, menu return, exit |
| `ghosts/<variant>.ghost` | `(version: 1, data: GhostData)` | new TT best |
| `logs/session-<unix ms>.log` | session log (`session_log_layer`) | the binary's `LogPlugin::custom_layer`; 3 kept |

- Atomic: write `<stem>.tmp`, `sync_all`, rename. Failures → `GameNotice::SaveFailed`; the next
  save writes the whole file again (retry).
- Unparsable or newer files → renamed `<name>.corrupt-<unix s>`, defaults used,
  `GameNotice::SaveRestored` (written in the first frame: rc_ui holds notices until `ui.ready`). A
  broken ghost is deleted.
- Versions: field additions are serde defaults; a bump adds a `match` arm in
  `save::parse_versioned` with a pure, tested migration. Version 0 = the bare struct without an
  envelope (what `Settings::to_ron` wrote) → migrates by unwrapping.

## Automation
- `--quick-race <track> [--mode tt|versus|cup:<id>]`: Versus defaults to 2 players; a cup starts at
  its race on that track (exact layout first). Locked content needs `--unlock-all`.
- `--soak <minutes>` (+ `AutomationConfig::soak_seed` to replay a logged seed; its CLI flag
  `--soak-seed` is requested from the binary's owner): random requests (tracks with data files; cups use theirs,
  so load errors are exercised), random actions every 0.5–5 s (restart, pause, next race, next
  track, rematch, quit; quit during Loading / Setup; ≤ 4 per session). At each settled menu (2 s)
  a census — resource type names and the number of entities, excluding UI/React elements,
  gamepads, Bevy's `PreviousState`s and the series — must equal the first census after a session
  that reached the race. A difference logs exactly what differs (resources, entities per
  component signature) and exits 1. Runs automatically with `--unlock-all` semantics and never
  exits on load errors.

## Tests
`cargo test -p rc_game`: unit — unlock rules (every rule kind, unlock-all, announce once,
earned unlocks stay), points & tie-breaks, series record/revert, cup trophy, setup per mode
(grid, classes, paints, seeds, presets, golden battery, ghost, dropped players, difficulty
fallback, rejections), records/medals/hash discard, medal table (+ data file covers every layout),
saves (round trip, v0 migration, newer/broken, corrupt backup, ghosts), settings throttle,
rebinding, clamping, log rotation, census diff. Headless (`tests/`, private temp data dirs,
placeholder racers in `tests/common`): lifecycle table, quit in Setup, restart+quit same frame,
load errors, pause/disconnect/focus (incl. automated), finished races unpausable, locked content,
Versus scoreboard/rematch/restart/drop/next track/lobby, `--quick-race --mode cup`, a full Rookie
Cup with restart and unlocks, abandoned cups, saves across an app restart (throttle + exit
flush), corrupt files, geometry change, Golden Battery/hints/reset, a 1-minute soak and a leak
the soak must catch.

## Known limitations
- Grid order puts humans on pole (kept from T01; GDD is silent). Change in `race_setup` if
  playtests want humans at the back.
- Race phases are driven by rc_race (T06); rc_game only sets `RacePhase::Setup` (start/restart). `RaceResults` must exist
  no later than entering `Finished` and be removed in `OnEnter(Setup)`.
- Distance counts finished races only; play time is wall time of the app.
- Window settings are applied by rc_render (single writer of the window).
