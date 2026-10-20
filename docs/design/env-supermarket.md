# Environment — Supermarket (Aisle Rush)

T16 environment note (art bible §3.5, GDD §7.1). Track `aisle_rush`, props directory
`supermarket`, lighting file `assets/maps/aisle_rush/environment.ron` (rendering agent).

> **Editor-owned layout.** Since the track editor (`docs/design/rc_editor.md`), the track and decor
> files are edited with `cargo run -p rc_editor -- aisle_rush`; the layout script below writes them only
> with `--layout`, which replaces every edit made in the editor. Its constants still feed the prop
> builds.

## Files and how to rebuild
| What | Where |
|---|---|
| prop generators (Blender 5.2) | `tools/blender/props/supermarket/<prop>.py`, shared kit `smkit.py` |
| textures (PIL, invented brands, OFL Saira text) | `textures.py` → `tex/packaging.png` (64 products), `tex/signs.png` (256-colour PNGs), `tex/atlas.json` |
| prop catalog (dimensions, colliders, lights) | `catalog.py` → `assets/data/props/supermarket.ron` (43 kinds, 64 glbs incl. decor) |
| layout + decor | `plan.py` → `assets/maps/aisle_rush/track.ron`, `assets/maps/aisle_rush/decor.ron` |
| glb validation (no Blender) | `check.py`: glTF 2.0, embedded textures, budgets, ground contact, colliders inside the mesh |
| everything | `tools/blender/props/supermarket/build.sh [--preview DIR]` (≈ 30 s without previews) |

`catalog.py` is the single source of truth for sizes: the Blender scripts, the layout planner and
the RON all read it. `plan.py` carries a Python copy of rc_track's centripetal Catmull-Rom and
refuses (exit 1) to write a layout where a static prop reaches into the racing corridor
(`corridor_hits`, the jumps excepted).

## Store plan (u; +X east, +Z south, north = back wall)
Room 207 × 170 u (46.6 × 38.3 m), ceiling 22 u (room height of the lighting file); shopfront
glazing on the south wall. North → south: dairy/deli coolers on the back wall, the back aisle, a
block of 8 double-sided gondola rows (7 aisles, run z −55.5…27), the promenade, a bank of 8
checkouts (lanes 1–3 open, 4–8 gated) + the customer-service desk + entrance railing, the lobby.
West: the freezer aisle (wall freezers + a back-to-back freezer island). East: produce (wall racks,
bins, the produce island). Closed aisles are blocked with pallets of stock; a pallet wall splits
the back aisle between aisles A and B, a promo wall separates the U-turn from the checkouts, so
nothing on open floor shortcuts the lap.

## Layout (790 u, 3 laps, reference lap 30.9 s, 11 checkpoints, 5 item rows, 285 props, 39 dynamic)
Forward (clockwise from above):
1. **Lobby** westbound — start/finish, sunset light through the shopfront on the left.
2. **Freezer aisle** north (12 u wide, kick-plate barriers both sides) — the floor is iced over
   (`Ice`, grip 0.45) from z ≈ +42 to −52: braking for T2 happens on ice; wet-floor signs and the
   mop bucket mark both ends; an item row sits on the ice.
3. **Back aisle** east past the dairy coolers → **aisle A** south: the box structure is a **hump**
   this way (up the 8.5° long ramp, over the kicker, down the 10° ramp).
4. **U-turn** (R 16) around the two endcaps with knockable **can pyramids** on its apex.
5. **Aisle B** north: the **box jump** (Fwd gap): 10.4° ramp, carton table, 7° kicker to the
   2.7 u lip, landing on the 18 u down-ramp; 0.64 s air at 30 u/s, lands at 0.8× too.
6. Back aisle east past the deli coolers → **produce** south: S-bends around the produce island
   (fruit tables + three pallets of bottled water).
7. **Golden Battery** (forward only): a narrow box ramp (3.55 u wide) straight off the T7 exit up
   onto the water pallets (3.6 u), battery on top, drop back onto the second bend — a straighter,
   risky line (shortcut `dir: Fwd`).
8. **Checkout chicane**: promenade west, left into lane 2 (8 u), right into the lobby. Lanes 1 and
   3 (6 u) are alternative routes (shortcuts, both directions).

Reverse: the same loop backwards; the jump is in aisle A (Rev gap), aisle B is the hump; the start
is followed quickly by the checkout lanes (three lanes spread the pack); the ice is the exit of
T2 and the braking zone of T1. Mirror layouts are geometrically symmetric (lint-free).

## Props
Hero (≤ 15 k): shelving gondola (4 stocked variants + 4 endcaps), freezer row (3-door glass
freezer, open dairy cooler), checkout lane (8 lane numbers), cardboard-box jump, can pyramid
(catering tins stacked 6-3-1, knockable), shopping trolley.

| Prop | Tris | Notes |
|---|---|---|
| gondola_a / b / c / d | 2 962 / 10 138 / 11 458 / 2 902 | cereal+crackers / cans+pasta / drinks+crisps / household+tea; kick plate behind the barrier |
| endcap_a–d | 3 132 / 4 158 / 2 484 / 1 548 | SPECIAL OFFER header, promo stock |
| freezer | 7 320 | glass doors (blend), LED mullions, glowing liner, FROZEN header; `freezer` kind has a night light, `freezer_dark` not |
| cooler | 4 610 | tiered milk/yogurt/cheese, canopy light; `cooler` / `cooler_dark` |
| checkout_1–8 | ≈ 3 780 | belt, scanner, POS, chair, candy rack, lane light (lit on 1–3) |
| box_jump | 5 048 | flattened printed boxes, carton table; 4 colliders match the mesh |
| can_tomato / beans / peach | 350 | dynamic 0.12, one product per pyramid |
| trolley | 1 292 | static (corral by the entrance, strays) |
| lane_gate, service_desk, railing | 336, 1 666, 448 | front-end line |
| water_pallet, water_ramp | 3 256, 504 | Golden Battery display (Cardboard tops) |
| pallet_stack_a / b, pallet | 1 612, 748 | stretch wrap (blend), corner boards |
| produce_table, produce_bin, produce_rack, crate_stack | 4 952, 2 052, 3 044, 1 272 | fruit as low-poly spheres |
| basket, wet_floor_sign, mop_bucket, box_small, box_large | 1 188, 176, 820, 176, 176 | all knockable |
| exit_sign | 308 | green EXIT glow + two emergency spots at night |
| decor: hanging_sign_1–7 / checkout, ceiling_light_on/off, wall_sign ×5, wall_band ×4, topstock_a/b, paper_bag, entrance_doors | 24–312 | physics-free (`decor/aisle_rush.ron`, 184 items) |

Art-bible kit mapping: cans (3) = the three tins; cereal box, bottle, milk carton = shelf stock
(builders in `smkit.py`, 64 invented products); crate = crate_stack; price sign = shelf price
strips + endcap SPECIAL OFFER boards; basket, pallet, wet-floor sign, mop bucket, cardboard box
(2), fruit (table/bin/rack), bag = paper_bag (decor on the open lanes' bagging shelves), shelf
dividers = gondola uprights/strips.

## Lighting wishes (for `assets/maps/aisle_rush/environment.ron`)
- **Sunset:** keep the warm low sun through the shopfront (it rakes across the lobby and the
  checkouts beautifully); room lamps at ~50 % ("half the tubes off"); my `ceiling_light_on` decor
  rows (route aisles, freezer aisle) carry the cold fluorescent look, so the disc lamp fixtures
  could be hidden (`fixture: 0`) or replaced by them.
- **Night:** room lamps off or ≤ 15 %; the light comes from the props: freezer/cooler glow (cyan
  point lights on every other unit, `cooler`/`freezer` kinds), six EXIT signs with twin
  emergency spots (warm, 450 lm, 50°), the lit ceiling rows (decor, emissive only) and the lane
  lights. Cool blue moonlight through the shopfront. Exposure ~6.
- **Barriers:** the supermarket block style alternates kick plates with blue/green crates; in
  front of freezers and shelves a continuous painted-steel kick plate (`#8E959C`, metallic ≈ 0,
  roughness ≈ 0.45) would read more like a real store (the props' own kick plates sit behind it).
- A material prefix for "lit by day, off at night" (the inverse of `lamp*`) would let the
  ceiling rows switch off at Night.

## Verification
- `python3 tools/blender/props/supermarket/check.py` → 64 models, 0 errors.
- `cargo run -p rc_track --example plot -- aisle_rush [--rev] [--mirror]` → no lints (all four).
- `cargo test -p rc_track` (every track file, every variant, no lints) and
  `cargo test -p rc_render props` (lamps + decor files) pass.
- In game: `cargo run -p recharged -- --user-data-dir target/agent-data/T16-supermarket
  --unlock-all --quick-race aisle_rush [--reverse] [--mirror] [--time night] --autopilot
  --screenshot <png> --screenshot-at 6 --screenshot-every 4 --exit-after 40`; the RC-eye tour:
  `--quick-race aisle_rush --track-tour aisle_rush`.
- `--headless-race`: all eight AI cars set lap times in both directions (forward winner 6296
  ticks, best lap 2018; reverse 5869 / 1927). `--bench aisle_rush_1p`: 154 fps, p95 10.4 ms at
  1080p on the dev machine.

## Known issues
- ponytail: every glb embeds its own copy of the packaging/sign atlas (256-colour PNGs, 180 +
  73 KB on disk, 4 MB VRAM each once decoded; 64 models = 15 MB); a shared external texture
  would need glb post-processing.
- The intro fly-by (radius 18 u) swings through the shopfront wall: the grid stands in the lobby,
  10 u from the glazing (camera-side clamp requested).
- The ceiling rows glow at Night too (decor has no time-of-day switch; "security lighting").
- AI drivers ignore static props; the corridor is kept clear by rc_track's corridor lint, but the alternative
  checkout lanes and the Golden Battery line are for humans (and AI if it reads `shortcuts()`).
- The freezer island's back row (facing a closed aisle) uses the full freezer model (7.3 k tris
  each); a cheap back variant would save ≈ 80 k triangles if profiles ask for it.
