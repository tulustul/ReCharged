# Race HUD (T05b) — `ui/src/hud` + `rc_ui::hud`, design note

A per-player HUD for 1–4 local players (GDD §13, §11, §18) on T05's framework
(`docs/design/ui.md`). Rust reads the race and sends compact data; React draws it inside each
player's view. Nothing is sent per frame: per-frame visuals are shared values written by Rust.

## Data flow
| What | Channel | Rate |
|---|---|---|
| Racers (grid order: name, car, slot, colour, colour-blind marker), laps, mode, items, ghost, minimap outline and racing edges (0–1, ≤ 240 points each, fitted on the edges), checkpoint gates, the heading-up zoom (track extent ÷ 150 u, ≥ 1), HUD settings (speed unit, HUD scale, minimap, reduce flashing, colour-blind markers) | event `race.setup` | on entering the race and when `RaceConfig`, `TrackSummary` or `Settings` change |
| Race clock, countdown number, "ends in" seconds, standings (grid indices), per local player: position, lap (`max(lap, 1)`, ≤ laps), lap/last/best/finish ms, km/h, charge, boost ready, spending, item, roulette, wrong way, lock-on, goo fuse, shield, lost, respawn countdown, the corner ahead (`nav.rs` `turn_ahead`: the one being driven, else the nearest starting within 2.5 s at the current speed, 30–90 u), next gate | event `race.hud` (`UiHudTick`) | ≤ 10 Hz (`on_real_timer(100 ms)`), only when changed |
| Countdown 3-2-1, GO, perfect start, stall, TT split, lap (best, fastest from lap 2), final lap, finished, Golden Battery, missed checkpoint, automatic respawn (`Rescued`, with the cause), race-ends-in, hints; hits taken (`Hit`) and landed (`Landed`) | event `race.event` (`UiRaceEvent`) | as they happen (local players only, except the global ones) |
| Each local player's view in UI px (`PlayerViewports` ÷ `UiScale`) + the free quadrant in 3P | event `viewports` | when changed |
| All of the above for (re)mounts | request `race.snapshot` → `hud` | on HUD mount |
| Speed (u/s), charge, charging glow (`charge_rate` / 0.3), rocket arrow and `guide` arrow to `RaceProgress::rejoin` (degrees, 0 = ahead, clockwise, in the car's frame), minimap `heading` (degrees clockwise turning the player's camera direction up), a minimap dot per racer (0–1) | shared values bound by `hud.bind` / `hud.unbind` | every frame, `.after(AnimationSet::Tick).before(AnimationSet::Apply)`, compare-before-write |

- Lock-on is a state (any `Projectile { SparkRocket }` whose `ItemTarget` is the player's car) plus
  the arrow angle (the nearest such rocket in the car's frame), not an event: the tick shows it
  while it lasts and survives remounts. The goo fuse is the `Hazard { GooBomb }` targeting the car
  (`ItemTimer`), the shield `StatusEffects.shield`.
- `rc_ui::hud` is a directory module: `mod.rs` (channel types, registration), `view.rs`
  (`HudView` builds setup/tick/viewports/snapshot from the world; `MinimapFit`; the
  `RaceEvent` → `race.event` map), `systems.rs` (push/forward/drive).

## React (`ui/src/hud`)
| File | What |
|---|---|
| `Hud.tsx` | root: feed (snapshot + events), shared values (`values.ts`), one `PlayerHud` per view, the 3P `Panel` |
| `Minimap.tsx`, `mapArt.ts` | the ribbon (one path of quads between the edges, nonzero fill: bridges and crossings stay filled), its edges over a dark halo, ten travel chevrons, the yellow start line (static, rasterized once per size), the next gate in cyan (its own layer); a player's map is heading-up — an outer node rotates by `heading` about the frame's centre, an inner one (`zoom` × size) slides the player's dot there — with the player as an arrow in the middle; the 3P panel's map stays north-up and whole |
| `TurnNote.tsx`, `turnIcon.ts` | the turn indicator: slight / square / hairpin arrow by angle (the same stems rc_render lays on the floor), white / yellow / red by severity, words outside compact views; its slot at the top of the centre column is always reserved (notices never jump) |
| `PlayerHud.tsx` | composition per view: `Standing` (position, lap, clock, lap/best, TT split, lap flash, P-badge in split) top-left, `ItemSlot` (roulette strip) top-right, `Gauges` (speed + battery) bottom-right, `Minimap` bottom-left (1P/2P), centre column: `Notices`, `Warnings`, `Center` (countdown / finish / wrong way / lost — an arrow turning by `guide`, "THIS WAY" or "OFF TRACK" with the respawn countdown / final lap), `Hints` (hint with the action's button, spectate, skip intro, race ends in) |
| `store.ts` | hmr-safe stores: setup, tick, views, and short-lived moments per slot (countdown, notices ≤ 3, banners, splits, lap flashes, hints) cleared by timers; reset on a new attempt |
| `noticeList.ts` | pure notice rules (tested): each view's newest lines; hits the player's item landed collapse into one line per item while they come within 1 s of each other ("Oil Slick hit 3 racers") |
| `markers.ts` | colour-blind marker outlines (tested), the same shapes as rc_vfx's name tags |
| `layout.ts` | pure maths (tested): `hudLayout` (size factor `k = hudScale × max(0.6, √fit)`, compact under 0.75 fit), `fs` (font floor 18 UI px ≈ 12 px at 720p 4-way), fallback views, unit, clock, split, place suffix |
| `look.ts`, `itemIcons.tsx` | translucent navy chips (never `backdropFilter`), display numbers with a drop shadow, caps labels; ten vector item icons (distinct shape and colour) |
| `demo.ts` | screenshot scenes |

- Bars fill with `scaleX` + compensating `translateX` (no relayout); dots and the arrow move by
  transform bindings; the battery flash is an `interpolateColor` loop; nothing animates `opacity`
  on text.
- Accessibility: splits carry sign **and** colour (a tie at the hundredths shown is a neutral
  `±0.00`, `layout.ts` `split`), warnings and notices carry text and icons, the
  own minimap dot is larger with a white ring (others' rings by role), Reduce flashing stops the
  meter flash and warning pulses, HUD scale scales everything big, small labels keep a floor.
- Colour-blind markers (setting): every racer has a marker from rc_core's `RacerMarker` (rc_ui
  sends it with each racer) — a shape by grid index for rivals (● ▲ ■ ◆ ★ ✚ ⬟ ⬢), the player
  number for humans. With the setting on, rivals' minimap dots take their shape (static `<svg>`,
  dark outline), humans' dots show their number, and the 3P standings show the shapes; rc_vfx's
  name tags show the same marker left of the name, so a shape always means the same car.
- Split-screen: each view gets a dark hairline border and a P1–P4 badge; 4P drops the minimap
  (GDD), 3P shows standings + a large minimap in the free quadrant.
- A missed checkpoint (rc_race: a checkpoint's `s` passed outside its gate, once per pass) shows
  the notice "Missed checkpoint — go back" (tone *bad*); the `warnings` demo scene includes it.
- The race layer keeps the HUD under the pause menu and shows results 2.6 s after the race ends
  so the finish banner gets its moment. In-race, the Golden Battery shows in the HUD, not as a
  toast.

## Verification
`RECHARGED_UI_SCRIPT="13.5 demo hud [1-4] racing|warnings|nav|roulette|finish|tt|countdown [markers]"` (`nav`: a hairpin ahead, lost, off the track, wrong way) over a
real race (`--quick-race toy_town [--mode versus --players N] --window 1920x1080`) keeps the real
views, racers and outline and fills the rest; without a race the scene brings its own (fallback
views, sample outline). While `demoMode` is on the feed leaves the HUD stores alone and React
writes the shared values instead of Rust.

## Known limitations
- The roulette strip ignores the real roulette length (1 s): it spins until the item lands.
- Shield/goo bars run down from the first value seen (the UI does not know the item tuning).
- Opponent name tags and 3D lock-on markers are rc_vfx's (GDD §13), not HUD overlays.
