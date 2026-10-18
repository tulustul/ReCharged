# UI — front end (`ui/` React + `rc_ui` bridge), design note

React/TSX rendered to bevy_ui by bevy-react 0.8.1. Rust owns state and rules; React shows them and
sends intents. Everything crosses typed channels generated into `ui/src/bevy.ts`
(`cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`; a test fails when it is stale).

## Structure
| Where | What |
|---|---|
| `crates/rc_ui/src/` | one module per namespace: `app` (state, info, quit), `ui` (ready/error, `ui.sound` cues with a 40 ms retrigger guard, `ui.stinger`, `ui.nav`, `toast` = rc_game notices held until `ui.ready`), `input` (devices, bindings, rebind), `menu` (catalog view, `menu.backdrop` → `MenuBackdrop`), `profile` (profile/records/NEW badges), `settings` (mirror of `Settings` without bindings), `lobby`, `race` (commands, `race.snapshot`, `loading.progress`), `results` (`race.results` = `RaceResults` + `RaceOutcome`, `cup.standings`, `versus.score`), `filters` (`chargeSwap` morph, embedded WESL), `debug` (`RECHARGED_UI_SCRIPT`) |
| `ui/src/theme/` | `tokens.ts` (colours, slot colours, sizes, spacing, layers), `styles.ts` (text presets, glass), `motion.ts` (springs, timings) |
| `ui/src/components/` | Button, Panel, Tabs, List/Carousel (`lists.tsx`), Grid/StatBar/LockOverlay/Spinner (`bits.tsx`), OptionRow/Toggle/Slider/Cycler (`controls.tsx`), Dialog, Toast, Glyph, Badge, Hints, Screen (frame), TrackArt, Backdrop, icons |
| `ui/src/nav/` | `focus.ts` regions model, `stack.ts` nav layers, `router.ts` stack ops (pure, tested); `input.ts` (`useNav`, glyph device per slot), `useFocusNav.ts`, `routes.ts` (menu routes + morph transitions) |
| `ui/src/state/` | `store.ts` (tiny external store + `hmrSingleton`), `stores.ts` (all singletons), `Feeds.tsx` (snapshots + events), `draft.ts` (race being set up + option rules), `lobby.ts` (per-slot choices), `overlays.ts` (dialogs, toasts), `unlocks.ts` |
| `ui/src/screens/` | Splash, Title, MainMenu, CupSelect, TrackSelect, CarSelect, Lobby, Garage, Records, HowToPlay, `settings/` (Settings + `rows.ts`), Credits, CupEnd, Loading, `race/` (RaceLayer, Pause, Results, Standings = cup standings + Versus scoreboard) |
| `ui/src/strings/` | every user-facing string, one file per area |
| `ui/src/debug.ts` | scripted verbs (`go`, `demo`) and sample data for screenshots |
| `assets/fonts/` | Saira (variable, upright = body, italic = `"Display"`), SIL OFL 1.1 (`Saira-OFL.txt`); registered by `rc_ui::with_fonts` in the binary |

`App.tsx` picks the layer by app state: menus (router + Backdrop), Loading, or the RaceLayer
(HUD stand-in until T05b, pause, post-race flow). Dialogs and toasts are `<root>`s above everything.
Each menu screen sits in an ErrorBoundary keyed by route (reports `ui.error`, offers Back).

## Look
Authored at **1920×1080 UI px**; rc_ui writes `UiScale = min(w/1920, h/1080)` of the window's
logical size, so 1280×720 and 1920×1080 render the same layout (other aspects grow the canvas along
the long side). Window-space rects (e.g. `PlayerViewports`) must be divided by `UiScale`.
Dark navy glass (`backdropFilter` blur on a few big panels only), electric cyan focus fills with a
volt charge bar, Saira Italic display type, springy focus scale. Translucent compositing is in
linear light, so translucent fills are navy-tinted (white-tinted ones read grey).
- **Split-screen**: App provides `SplitContext` when a race has 2+ local players; `Panel` then uses
  a translucent fill (the overlay UI camera has no 3D frame to blur).
- **Showroom**: `AppStatus.showroom` mirrors rc_core `ShowroomReady` (inserted by rc_render while
  the 3D showroom draws the `MenuBackdrop` car). Without it the Backdrop draws navy glows/streaks
  and Main menu / Car select show the car thumbnail (`assets/ui/cars/<id>.png`, only offered when it
  exists; same for track previews). The flag arrives before the 3D view is fully drawn (pipelines
  recompile after a race), so the navy layers stay 1 s and then fade out over the showroom.
- **Transitions**: screen changes arm a `morphFilter` only while in flight (`routes.ts`): crossfade
  everywhere, the custom `chargeSwap` (a crackling cyan/volt front) only Main menu → mode.

## Navigation model
- Input: **only** the `ui.nav` event (rc_input `UiNav`) plus pointer events. `Feeds` routes each
  `ui.nav` to the top nav layer (`stack.ts`: highest z — screen 0 < overlay 10 < dialog 20 — then the
  latest), so dialogs shadow screens regardless of effect order or hot reloads. No `tabIndex`
  anywhere (a focused bevy `<button>` would also click on Enter).
- Focus: a screen declares regions (`list`/`row`/`grid` via `cols`, neighbours per direction,
  optional wrap) and `useFocusNav` moves through them; entering a region restores its last index.
  Pointer hover sets the same focus (one highlight), click activates. Left/right on adjustable rows
  (cyclers, toggles, sliders) change the value instead of moving.
- Per-slot focus: the Versus lobby keeps a choice per slot (`state/lobby.ts`) and applies each
  `ui.nav` to its `slot`; slot-less events (unassigned devices, shared Esc/Q/E) only leave the
  screen. Joining/leaving is rc_input's (`lobby.slots`). Track select in Versus accepts P1 only.
- Router: a stack of routes in a store; Back pops one level, the bottom never pops. Race exits set
  the stack the mode returns to (`router.set([...])` before `race.quit` / `race.next`).
- Glyphs follow the device of each slot's latest nav (`input.device` `lastUsed` too); mouse users
  get keyboard glyphs; lobby keys per scheme (A: Space/L-Ctrl, B: Enter/Backspace). They are drawn
  (`glyphs.ts` spec → `Glyph.tsx` shapes: keycaps, face buttons by position, shoulders, D-pad,
  sticks; no platform logos), so `assets/ui/glyphs/` holds no images.
- Cues: hover on every focus move, select on confirm, back, toggle (cyclers/toggles), sliderTick,
  error (locked/invalid), unlock (unlock toasts).

## Data flow
Every store is an hmr-safe singleton seeded by a snapshot request when `Feeds` mounts, then kept
current by events; app-state changes re-request what they invalidate (profile/records on menu
entry, `race.snapshot` on loading/finish). Settings and bindings are fetched by their screens;
edits apply live (`settings.apply`; rc_game clamps and saves ≤ 1/s). Rebinding: `input.rebind` →
rc_input capture → `input.rebindResult` (conflict warning) → rc_game stores → `bindings.changed`.
NEW badges come from `Profile::new_unlocks`; focusing an item sends `profile.seen`.

## Adding a screen
1. Strings in `strings/<area>.ts`.
2. `screens/X.tsx` composing `Screen` + components; focus with `useFocusNav(map, initial,
   { confirm, back, ... })`; give every focusable `focused`/`onHover`/`onPress`; hints with
   clickable actions (mouse parity).
3. Add the route to `nav/routes.ts` and `ScreenFor` in `App.tsx`; navigate with `go(route)`.
4. New data: a channel in the right rc_ui module (+ `register`), regenerate `bevy.ts`, a store in
   `stores.ts` fed by `Feeds.tsx` (snapshot request + event).
5. A `go x` / `demo x` verb in `debug.ts` if it needs sample data, then screenshot it.

## Channels (final)
- **Events**: `app.state`, `ui.nav`, `input.device`, `input.rebindResult`, `bindings.changed`,
  `lobby.slots`, `loading.progress`, `toast`, `profile.changed`, `profile.unlocked`, `race.results`,
  `cup.standings`, `versus.score`, `debug.act`.
- **Messages**: `ui.ready`, `ui.error`, `ui.sound`, `ui.stinger`, `menu.backdrop`, `race.start`,
  `race.restart`, `race.next`, `race.rematch`, `race.nextTrack`, `race.resume`, `race.quit`,
  `lobby.open`, `lobby.close`, `settings.apply`, `input.rebind`, `input.resetBindings`,
  `input.dropPlayer`, `profile.reset`, `profile.seen`, `app.quit`.
- **Requests**: `app.snapshot`, `app.info`, `catalog.get`, `profile.get`, `records.get`,
  `settings.get`, `bindings.get`, `lobby.snapshot`, `race.snapshot` (setup + results + cup + versus;
  T05b extends it for the HUD).
- T05b adds `race.hud`, `race.event`, `viewports`, `hud.bind/unbind` in a `hud` module.

## Automation & screenshots
`RECHARGED_UI_SCRIPT="<secs> <action>; …"` (timed from `ui.ready`): `nav <action> [slot|-]
[device]` sends a real `ui.nav`, `pause` sends `GameCommand::Pause`, anything else is a `debug.act`
for React: `go <route> [mode]`, `demo results|cupResults|versusResults|pause|racing|loading|lobby|
cupEnd|unlocks|dialog|menu`, `tab <id>` (Settings, How to play, Records). Example acceptance run
(Title → Main → Quick Race → Track → Car → race → Pause → Quit):
`RECHARGED_UI_SCRIPT="3.2 nav confirm; 4.8 nav down; 5.6 nav confirm; 7.6 nav confirm; 9.4 nav
confirm; 16 pause; 18 nav down; 18.4 nav down; 18.8 nav down; 19.2 nav down; 20 nav confirm; 21 nav
left; 21.6 nav confirm" cargo run -p recharged -- --user-data-dir target/agent-data/T05 --exit-after 26`.

## Tests
`npm test` (node --test, no deps): focus model, nav layers, router, slider/list maths, stores,
dialogs/toasts, race draft rules, lobby choices, settings rows, formatters, glyph mapping. Pure
modules import each other with explicit `.ts` extensions (tsconfig `allowImportingTsExtensions`)
so node can run them. `npm run typecheck`, `npm run build`. Rust: `cargo test -p rc_ui` (mappings,
profile/records/standings views, script parser, UiScale, channel round trip, `bevy.ts` freshness).

## Known limitations
- The race HUD is a status chip until T05b; per-frame HUD values will use shared values.
- Track previews do not exist yet: TrackArt draws an environment-coloured card instead.
- Lobby: devices join by button press only (no mouse join, by design).
- An animated `opacity` on a `<text>` rendered a dark box behind the text in this build, so pulses
  animate the text colour instead (`interpolateColor`).
