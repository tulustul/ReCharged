# T05 — UI framework & front-end screens (`ui/` + `rc_ui`)

**Goal:** a polished, modern, fully gamepad/keyboard/mouse navigable front-end in React
(bevy-react) wired to Bevy through typed channels. (The race HUD is a later task, T05b.)

**Read first:** PRACTICES (§8 especially); ARCHITECTURE §1–4, §5.13, §5.12 (what rc_game offers);
GDD §3 (modes & flows), §9, §12.2, §14, §15, §18; `docs/tech/bevy-react-notes.md` (all);
`/home/tul/Projects/bevy-react/examples/cyberpunk` (menus, settings, transitions) for patterns.

## Owned paths
`ui/**` (you regenerate `ui/src/bevy.ts`), `crates/rc_ui/**`, `assets/fonts/**`,
`assets/ui/glyphs/**`, `docs/design/ui.md`. **rc_core grant:** additive edits in
`crates/rc_core/src/ui.rs` only. rc_game is being extended in parallel (T07): consume its public
API/messages; if something you need is missing, request it (SendMessage to `main`) and stub
gracefully meanwhile.

## Deliverables
1. **Design system**: theme tokens (GDD §14 style: dark navy frosted glass, electric cyan +
   volt-yellow, bold italic display font), typography with OFL fonts (licence files alongside),
   motion presets (springs/timings), components (Button, Panel, Tabs, List, Grid, Slider, Toggle,
   Select/cycler, Carousel, StatBar, Toast, Dialog, Glyph (own generic glyphs per device kind),
   Badge "NEW", LockOverlay, Spinner). Translucent variants for split-screen.
2. **Navigation** (`nav/`): logical focus model (vertical list, row, grid with column count, tabs),
   screen stack router with Back, per-slot focus contexts (Versus lobby), driven **only** by the
   `ui.nav` event + pointer events; pure TS with `node --test` tests.
3. **Stores** (`state/`): app state, catalog, profile, settings, records, lobby — each seeded by a
   snapshot request on mount, updated by events, hmr-safe singletons.
4. **Screens** (GDD §14): Splash (text only), Title, Main menu (+ quit confirm), Cup select, Cup
   standings, Cup end, Track select (layouts, options within allowed presets), Car select (over the
   showroom backdrop via `menu.backdrop`), Versus lobby (4 slots, per-slot focus, colours P1–P4),
   Versus scoreboard, Garage, Records, How to play, Settings (5 tabs; live apply; rebinding UI with
   capture state and conflict warning; reset progress with double confirm), Credits, Loading, Pause
   (Bevy opens, React closes via `race.resume`), Results (mode actions per GDD §3.3), dialogs and
   toasts (unlocks, errors, controller disconnected). Glitch/charge morph only for Main menu → mode;
   fades elsewhere. UI sound cues via `ui.sound` on focus/confirm/back/toggle/error.
5. **rc_ui (Rust)**: namespaced modules with every channel of ARCHITECTURE §5.13 needed by these
   screens (requests for catalog/profile/settings/records/bindings/lobby snapshots; messages for
   race start/restart/next/quit/resume, lobby open/close, settings apply, rebind, profile reset,
   menu backdrop, ui sound, app quit; events app.state, ui.nav, input.device, lobby.slots,
   loading.progress, toast, profile.unlocked, race.results, cup.standings, versus.score) mapping to
   rc_core messages / rc_game commands; `register_bindings` shared with the `export_bindings`
   example.
6. **Quality bar**: looks great at 1280×720 and 1920×1080; every screen usable with gamepad only,
   keyboard only and mouse only; no dead ends; first focus on the most likely action. Take
   screenshots of every screen (`--user-data-dir target/agent-data/T05`, `--unlock-all` for locked
   content) and inspect them; iterate on layout and polish.
7. `docs/design/ui.md`: structure, navigation model, how to add a screen, channel list.

## Tests
`npm test` (nav model, stores, formatters), `npm run typecheck`, `npm run build`; Rust: channel
registration compiles and `export_bindings` produces a `bevy.ts` identical to the committed one.

## Acceptance
All checks pass; screenshot set in the report; a scripted run (`--input-script` once available, or
manual) navigates Title → Main menu → Quick Race → Track → Car → race start → Pause → Quit.
