# T05b — Race HUD (`ui/src/hud` + `rc_ui` race/hud namespaces)

**Goal:** a clean, modern, highly readable per-player HUD for 1–4 players, driven efficiently.

**Read first:** PRACTICES §8; ARCHITECTURE §5.13; GDD §13 (HUD), §11 (split layouts), §18;
`docs/tech/bevy-react-notes.md` §3 (HUD data flow: shared values + batched events), §8;
`docs/design/ui.md` (T05's framework); `docs/design/rc_race.md`, `rc_items.md`.

## Owned paths
`ui/src/hud/**`, `ui/src/strings/hud.ts`, `crates/rc_ui/src/{race.rs, hud.rs}` (or the namespace
modules T05 created for them), `docs/design/hud.md`. Coordinate `bevy.ts` regeneration with the
rc_ui owner rules (you own those namespaces, so you may regenerate).

## Deliverables
1. Rust: `race.setup` (racers, colours, track outline from `TrackSummary`, laps), batched
   `race.hud` (all local players, ≤ 10 Hz: position, lap, times, best, item, roulette, speed,
   charge, warnings), `race.event` (notifications, hints, lap/finish, lock-on, golden battery,
   race-ends-in), `viewports` (from `PlayerViewports`), `race.snapshot`; shared values for per-frame
   visuals (speed gauge, charge bar, minimap dots) bound via `hud.bind/unbind`, written
   `.after(AnimationSet::Tick).before(AnimationSet::Apply)`.
2. React HUD per viewport rect: position & lap, timers (TT splits with colour **and** sign), item
   slot with roulette, speed (units setting), charge meter (ready glow, spending flash), minimap
   (SVG outline + dots via shared values; 3P → 4th quadrant panel with minimap + standings),
   countdown, notifications, warnings (rocket lock-on arrow, goo fuse, shield timer, wrong way),
   hints, finish banner, "Waiting for other players", "Race ends in…". Translucent styling in
   split-screen; HUD scale setting; compact layout for small viewports.
3. Verify at 1280×720 and 1920×1080, 1–4 players, with screenshots inspected.

## Tests
`npm test` for formatters/layout maths; typecheck/build; Rust compiles and bindings are current.

## Acceptance
All checks pass; screenshots of 1P/2P/3P/4P HUDs in the report.
