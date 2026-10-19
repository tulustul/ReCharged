# UI — front end (`ui/` React + `rc_ui` bridge), design note

React/TSX rendered to bevy_ui by bevy-react 0.8.1. Rust owns state and rules; React shows them and
sends intents. Everything crosses typed channels generated into `ui/src/bevy.ts`
(`cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`; a test fails when it is stale).

## Structure
| Where | What |
|---|---|
| `crates/rc_ui/src/` | one module per namespace: `app` (state, info, quit), `ui` (ready/error, `ui.screen` (the shown screen: logged, waited on by `--input-script`), `ui.sound` cues with a 40 ms retrigger guard, `ui.stinger`, `ui.nav`, `toast` = rc_game notices held until `ui.ready`; hides the OS cursor after keyboard/pad input until the mouse moves), `input` (devices, bindings, rebind), `menu` (catalog view, `menu.backdrop` → `MenuBackdrop`), `profile` (profile/records/NEW badges), `settings` (mirror of `Settings` without bindings), `lobby`, `race` (commands, `race.snapshot`, `loading.progress`), `results` (`race.results` = `RaceResults` + `RaceOutcome`, `cup.standings`, `versus.score`), `hud` (race HUD: `race.setup`, `race.hud`, `race.event`, `viewports`, `hud.bind/unbind` — `docs/design/hud.md`), `filters` (`chargeSwap` morph, embedded WESL), `debug` (`RECHARGED_UI_SCRIPT`) |
| `ui/src/theme/` | `tokens.ts` (colours, slot colours, sizes, spacing, layers), `styles.ts` (text presets, glass), `motion.ts` (springs, timings) |
| `ui/src/components/` | Button, Panel, Tabs, List/Carousel (`lists.tsx`), Grid/StatBar/LockOverlay/Spinner (`bits.tsx`), OptionRow/Toggle/Slider/Cycler (`controls.tsx`), Dialog, Toast, Glyph, Badge, Hints, Screen (frame), TrackArt, Backdrop, icons |
| `ui/src/nav/` | `focus.ts` regions model, `stack.ts` nav layers, `router.ts` stack ops (pure, tested); `input.ts` (`useNav`, glyph device per slot), `useFocusNav.ts`, `routes.ts` (menu routes + morph transitions) |
| `ui/src/state/` | `store.ts` (tiny external store + `hmrSingleton`), `stores.ts` (all singletons), `Feeds.tsx` (snapshots + events), `draft.ts` (race being set up + option rules), `lobby.ts` (per-slot choices), `overlays.ts` (dialogs, toasts), `unlocks.ts` |
| `ui/src/screens/` | Splash, Title, MainMenu, CupSelect, TrackSelect, CarSelect, Lobby, Garage, Records, HowToPlay, `settings/` (Settings + `rows.ts`), Credits, CupEnd, Loading, `race/` (RaceLayer, Pause, Results, Standings = cup standings + Versus scoreboard) |
| `ui/src/hud/` | the race HUD (T05b, `docs/design/hud.md`) |
| `ui/src/strings/` | every user-facing string, one file per area |
| `ui/src/debug.ts` | scripted verbs (`go`, `demo`) and sample data for screenshots |
| `assets/fonts/` | Saira (variable, upright = body, italic = `"Display"`), SIL OFL 1.1 (`Saira-OFL.txt`); registered by `rc_ui::with_fonts` in the binary; the Credits roll carries the font notice |

`App.tsx` picks the layer by app state: menus (router + Backdrop), Loading, or the RaceLayer
(the HUD — hidden under the pause menu —, pause, post-race flow; results wait 2.6 s after the
race ends so the finish banner shows). Dialogs and toasts are `<root>`s above everything.
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
  the 3D showroom draws the `MenuBackdrop` car). Screens that cover the turntable — the Garage grid,
  the Credits roll, the Cup-end podium — and the card screens it would show through — Cup select,
  Track select, the Versus lobby (translucent cards without blur, QA B36) — set `focus: "room"`
  while mounted (`useRoomBackdrop`): rc_render hides the car, the room stays; the wide shot of the
  draft's car returns after. Main menu, Title and Car select show the car; the frosted panels of
  Records, How to play and Settings blur it. Without the showroom the Backdrop draws navy glows/streaks
  and Main menu / Car select show the car thumbnail (`assets/ui/cars/<id>.png`, only offered when it
  exists; same for track previews). The flag arrives before the 3D view is fully drawn (pipelines
  recompile after a race), so the navy layers stay 1 s and then fade out over the showroom.
- **Title**: the logo, charge bar and tagline sit at the top (`LOGO_TOP`), clear of the car the
  wide shot frames at the screen centre (QA R6); "Press any button" below the turntable.
- **Transitions**: screen changes arm a `morphFilter` only while in flight (`routes.ts`): crossfade
  everywhere, the custom `chargeSwap` (a crackling cyan/volt front) only Main menu → mode.
- **Credits**: a roll over the showroom from `strings/credits.ts` (team, tools, generators, the
  Saira OFL notice); its distance and duration follow the list (57 UI px/s).
- **Cup end**: the top three cars (thumbnails) stand on their 1-2-3 blocks — each image overlaps
  its block by its own transparent margin under the wheels (`UiCar.thumbnailFoot`: rc_ui measures
  the lowest opaque row of each PNG; 10–18 % in the shipped thumbnails, QA B37) and is drawn above
  it — with name and points on the block face.
- **Cup select**: a locked cup dims its race list and shows a lock and the unlock reason in its
  footer (where an unlocked cup shows its best trophy).
- **Splash and toasts** run on Bevy's frame clock (shared-value drivers whose completion ends
  them), not on wall-clock JS timers: when the first frames stall (pipelines compiling
  synchronously in automated runs) the splash and a boot toast ("Save file restored") are still
  seen. Unlock toasts are skipped while Results or Cup end show (their panels list the unlocks).
  Toasts sit on the right edge, halfway down: below the headers, tab bars and side-panel titles
  (the Settings help), above the footer and the Results buttons.
- **Loading**: the progress bar's glow (`boxShadow`) also compiles bevy_ui's box-shadow pipeline
  for the LDR camera split-screen races share, so their HUD glows never compile mid-race.

## Navigation model
- Input: **only** the `ui.nav` event (rc_input `UiNav`) plus pointer events. `Feeds` routes each
  `ui.nav` to the top nav layer (`stack.ts`: highest z — screen 0 < overlay 10 < dialog 20 — then the
  latest), so dialogs shadow screens regardless of effect order or hot reloads. No `tabIndex`
  anywhere (a focused bevy `<button>` would also click on Enter).
- Focus: a screen declares regions (`list`/`row`/`grid` via `cols`, neighbours per direction,
  optional wrap) and `useFocusNav` moves through them; entering a region restores its last index.
  Pointer motion over an item sets the same focus (one highlight), click activates. Focus follows
  only real motion — items use `onPointerHover` (fires on frames the pointer moved over them), never
  `onPointerEnter`, which also fires when a screen appears under a resting cursor (or X11 left
  Bevy a stale position after the window moved): a parked mouse never steals the keyboard/pad
  focus. rc_ui hides the OS cursor after keyboard/pad input until the mouse moves. Left/right on
  adjustable rows (cyclers, toggles, sliders) change the value instead of moving.
- Track select: focusing a card previews it (it becomes the draft's track, laps reset to its
  default; the panel shows its options and records). Confirm on a card selects it and moves the
  focus onto Start / Choose car (Confirm, Confirm is the fast path); Up from there reaches every
  option row; Back or Left returns to the selected card (Right from the last column also enters
  the panel). The same screen serves Time Trial, Versus (P1 picks) and, after a race, Change track
  / Next track with every option (laps, rivals, difficulty, items, weather, time; seeded from the
  session's setup) → `race.nextTrack` → rc_game `ChangeTrack`.
- Screens report themselves to Bevy (`ui.screen`, once per change): the menu route, `loading`,
  `unloading`, or the race layer's `race` / `pause` / `pauseHowToPlay` / `pauseSettings` /
  `results` / `standings` / `score` / `changeTrack`; rc_ui logs it (`UI screen: …`) and keeps it in
  `UiStatus::screen` for `--input-script`'s `wait screen <name>`. Every `ui.nav` is logged at debug
  level with the screen that got it, and every focus move (`RUST_LOG=bevy_react_core::js=debug`).
- Per-slot focus: the Versus lobby keeps a choice per slot (`state/lobby.ts`) and applies each
  `ui.nav` to its `slot`; slot-less events (unassigned devices, shared Esc/Q/E) only leave the
  screen. Joining/leaving is rc_input's (`lobby.slots`). Track select in Versus accepts P1 only.
- Router: a stack of routes in a store; Back pops one level, the bottom never pops. Race exits set
  the stack the mode returns to (`router.set([...])` before `race.quit` / `race.next`).
- Glyphs follow the device of each slot's latest nav (`input.device` `lastUsed` too); mouse users
  get keyboard glyphs; lobby keys per scheme (A: Space/L-Ctrl, B: Enter/Backspace). They are drawn
  (`glyphs.ts` spec → `Glyph.tsx` shapes: keycaps, face buttons by position, shoulders, D-pad,
  sticks; no platform logos), so `assets/ui/glyphs/` holds no images.
- Footer hints: the move action reads "Browse" (`hint.browse`; Settings: "Change", values move
  there), the confirm action names what it does (Select, Confirm, Start race …), so no footer says
  the same word twice (QA R7).
- Cues: hover on every focus move, select on confirm, back, toggle (cyclers/toggles), sliderTick,
  error (locked/invalid), unlock (unlock toasts).

## Data flow
One Confirm starts one race: Car select and the Versus track pick send `race.start` through
`startGate` (`state/draft.ts`), which drops repeats within 1.5 s (a double press or a duplicated
nav event once sent it twice).
Every store is an hmr-safe singleton seeded by a snapshot request when `Feeds` mounts, then kept
current by events; app-state changes re-request what they invalidate (profile/records on menu
entry, `race.snapshot` on loading/finish). Settings and bindings are fetched by their screens;
edits apply live (`settings.apply`; rc_game clamps and saves ≤ 1/s). Rebinding: `input.rebind` →
rc_input capture → `input.rebindResult` → rc_game stores → `bindings.changed`. A conflict swaps:
the other action receives the rebound one's previous binding (`previous`); the notice says
"Swapped with Camera" (the other scheme named when it differs), or, without a previous binding,
"Camera is now unbound" with that row highlighted (`rows.ts` `rebindNotice`). The notice clears on
Reset bindings, a tab change and a scheme change.
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
  `cup.standings`, `versus.score`, `race.setup`, `race.hud`, `race.event`, `viewports`, `debug.act`.
- **Messages**: `ui.ready`, `ui.error`, `ui.screen`, `ui.sound`, `ui.stinger`, `menu.backdrop`, `race.start`,
  `race.restart`, `race.next`, `race.rematch`, `race.nextTrack`, `race.resume`, `race.quit`,
  `lobby.open`, `lobby.close`, `settings.apply`, `input.rebind`, `input.resetBindings`,
  `input.dropPlayer`, `profile.reset`, `profile.seen`, `hud.bind`, `hud.unbind`, `app.quit`.
- **Requests**: `app.snapshot`, `app.info`, `catalog.get`, `profile.get`, `records.get`,
  `settings.get`, `bindings.get`, `lobby.snapshot`, `race.snapshot` (setup + results + cup + versus
  + the HUD's setup/tick/views).

## Automation & screenshots
`RECHARGED_UI_SCRIPT="<secs> <action>; …"` (timed from `ui.ready`) sends each action to React as
a `debug.act` fixture: `go <route> [mode]`, `demo results|cupResults|versusResults|pause|racing|
loading|lobby|cupEnd|unlocks|dialog|menu`, `demo hud [1-4] <scene>` (see `hud.md`), `tab <id>`
(Settings, How to play, Records), e.g. `RECHARGED_UI_SCRIPT="1 go credits" cargo run -p recharged
-- --user-data-dir target/agent-data/<id> --screenshot <png> --screenshot-at 5 --exit-after 8`.
Navigation and pausing go through the real input path with `--input-script`
(`docs/design/automation.md`, e.g. `tools/scripts/keyboard_only.script`; `wait screen <name>`
waits for a screen React reported); `nav`/`pause` steps here are rejected with a pointer to it.
`demo hud … markers` turns the colour-blind markers on for a HUD screenshot.

## Tests
`npm test` (node --test, no deps): focus model, nav layers, router, slider/list maths, stores,
dialogs/toasts, race draft rules (incl. Change track from the session's setup), lobby choices,
settings rows and the rebind notice, HUD notice collapsing and marker outlines, formatters, glyph
mapping. Pure
modules import each other with explicit `.ts` extensions (tsconfig `allowImportingTsExtensions`)
so node can run them. `npm run typecheck`, `npm run build`. Rust: `cargo test -p rc_ui` (mappings,
profile/records/standings views, script parser, UiScale, channel round trip, `bevy.ts` freshness).

## Known limitations
- Track previews do not exist yet: TrackArt draws an environment-coloured card instead.
- Lobby: devices join by button press only (no mouse join, by design).
- An animated `opacity` on a `<text>` rendered a dark box behind the text in this build, so pulses
  animate the text colour instead (`interpolateColor`).
- `boxShadow: []` paints a default black shadow (bevy-react parses the empty list as one default
  shadow): conditional shadows use `undefined`.
- Colour-blind markers show in races only (name tags, minimap, 3P standings); the post-race tables
  keep the colour bar (a rival's grid slot, hence its shape, changes between cup races).
