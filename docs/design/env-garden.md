# Environment `garden` — Backyard Bash

Back garden at RC scale (art bible §3.4; GDD §7.1: ~450 u, 5 laps, outdoor ★★, all six weathers).
A suburban brick house with a raised timber deck and a slab patio, a lawn with brick-edged raised
beds, a vegetable patch along the south fence and a shed in the corner.

## Files and how to rebuild
| What | Path |
|---|---|
| Layout generator (pure Python) → track file | `tools/blender/props/garden/plan.py` → `assets/data/tracks/backyard_bash.ron` |
| Decor generator (physics-free dressing) | `tools/blender/props/garden/decor.py` → `assets/data/decor/backyard_bash.ron` |
| Prop catalog (hand-written) | `assets/data/props/garden.ron` (22 kinds, prefix `garden_`) |
| Prop generators (Blender 5.2) | `tools/blender/props/garden/*.py` (shared helpers: `gardenkit.py`) → `assets/models/props/garden/*.glb` (43 models, 6.1 MB) |
| Rebuild + validate everything | `tools/blender/props/garden/build.sh` (`PREVIEW=<dir>` adds EEVEE previews) |
| Asset check (no Blender) | `python3 tools/blender/props/garden/check.py` |

`gardenkit.py` reuses `lib/propkit` (bake, budgets, ground check, export, previews) and
`lib/street_props` (outdoor preview stage). Textures are generated tileable albedo (wood grain,
clay, stone, grit, brick bond; 512² over 2 u = 256 px/u) embedded as JPEG; every mesh gets UVs
(a textured material on a UV-less mesh exports an invalid glTF that Bevy rejects). No clearcoat.

## Layout (463 u, reference lap 21.3 s, 5 laps ≈ 1:50)
`style: Invisible` — every surface is a ground polygon: lawn 0 (Grass), patio 0.05 (Stone), dirt
path 0.03 (Dirt), puddles 0.06 (Water), raised beds and sandbox sand 1.0 (Dirt), deck marker 1.8
(Wood, hidden inside `garden_deck`, whose Wood box is the real 2.0 top). Walls (rc_render draws
brick-red edging blocks) only where a bed or the lollipop needs them; lawns are open with 6 u
off-road margin (10 on the deck, 4 in the walled lollipop).

Forward (counter-clockwise from above), with reverse in brackets:
1. **Start** on the east lawn heading north; grid on the straight behind the line (reverse: north
   of it).
2. **Deck**: 15° decking ramp up (Fwd prop) → deck run → left turn on the deck → deck straight →
   **drop off the deck's west edge onto the patio** (Fwd gap, 2 u drop, 0.40 s). *(Reverse: ramp
   up from the patio (Rev prop), right turn, drop off the deck's south edge onto the east lawn 28 u
   before the finish.)*
3. **Flower-pot chicane** on the patio: a 10 u lateral jog (R 8, 45°) between two pot clusters,
   knockable small pots on the apex; low brick border wall on the lawn side.
4. Hairpin (R 13) round a raised corner bed into the **lollipop**: the outbound lane passes
   **under the garden bench** (7 u passage, the lap's only chokepoint; the bench frames are the
   walls there) → the **hose-reel spiral**: a 261° decreasing-radius right-hand loop (R 15 → 12.5
   → 10, entered and left through 40.7° veers) round the island with the hose-reel cart and the
   unrolled hose coiled on the grass → return lane (14 u from the outbound lane, raised bed between).
   *(Reverse: an opening spiral, left-hand.)*
5. West lawn (border bed on the right), SW corner (R 14) — the bed's brick wall runs on round
   the corner's outside and along the dirt path's south edge (kerb to the vegetable patch).
6. **Sandbox jump** on the dirt path: plywood kickers lean on both rims of the 2.4 × 1.4 m sandbox
   (12.6°, lip 1.18 u), each with a 2.5 u lead-in board at half the slope in front (T19a: straight
   onto the 12.6° sheet at 30 u/s the suspension bottomed out, the rear rebounded and cars left
   the lip nose-down — an assisted Phantom flipped there); gap over the sand (dir Both: the same
   props make the jump both ways, 0.62 s air); short jumps land in the sand.
7. **Puddles** (three, wet-mud rims) on the dirt path → SE corner past the shed → start.

**Golden Battery** (forward only): above the sandbox, 3.8 u south of the line at 1.9 u — take the
jump on the outer edge of the kicker.

Item rows (5): start straight, deck straight, inside the spiral, west lawn, sandbox run-up.
Checkpoints 8 (automatic). Dynamic props 17 (≤ 40). Intended lines: carry speed up the deck ramp,
brake on the deck for the turn, land straight on the patio and flick through the jog; the spiral
rewards a late apex and a steady drift (battery charge); keep the throttle on over the sandbox.

## Props (triangles; budgets kit 5 k, hero 15 k)
| Kind / model | Tris | Notes |
|---|---|---|
| **bench** (hero) | 4 284 | hardwood 3-seater 1.9 m, seat 0.45 m; 7.4 u clear between frames, 1.65 u headroom |
| **hose_reel** (hero) | 5 200 | A-frame cart, coiled hose drum, crank; centre of the spiral |
| **sandbox** (hero) | 3 196 | pine boards, cap rail at 1.18 (kicker lip), corner seats, castle, spade |
| **pot_cluster** (hero) | 6 402 | three terracotta pots: box ball, lavender, geranium + petunias |
| **shed** (hero) | 4 052 | overlap cladding, felt roof, green ledged door, window |
| **bbq** (hero) | 1 860 | kettle barbecue (on the deck, inside the turn) |
| deck (structure) | 4 148 | 42 × 36 u boards, fascia, skirting, posts, steps |
| deck_ramp | 624 | decking ramp 10 u wide, 2.0 over 7.5 u (Wood wedge) |
| kicker | 520 | plywood sheet on brick stacks + lead-in board (two Wood wedges, 6.54 u run) |
| edging | 198 | brick-red block = barrier block size, closes open bed ends |
| round_bed | 4 778 | stone ring bed with a Japanese maple (central lawn) |
| veg_bed | 2 200 | timber bed, lettuces, bean-cane wigwam |
| pot_large / pot_medium / pot_small | 1 866 / 2 118 / 504 | geranium / lavender / empty (knockable) |
| watering_can / gnome / bucket | 476 / 848 / 500 | knockable |
| football / tennis_ball | 320 / 80 | knockable |
| porch_light / fairy_lights | 668 / 1 548 | lamp props: `lights` + `lamp_*` materials |
| Decor: house, fence, tree_oak, tree_birch, conifer | 3 968, 880, 4 604, 3 728, 4 828 | backdrop beyond the shell walls |
| Decor: tuft_small/tuft/tuft_large, clover, hosta, daisies, fern, hydrangea, ornamental_grass | 96–4 390 | edge grass, lawn patches, bed planting |
| Decor: hose_spiral, puddle_rim_1–3, stepping_stone, pebbles, leaves | 2 676, 28, 236, 624, 98 | soft dressing |

Decor: 452 items, 0.28 M triangles in all; placed props: 79, 0.09 M (well under the 1.5 M
visible budget even if everything were on screen). Lamp-lit house
windows (`lamp_window`) and the lantern panes glow at night and are dimmed by day by rc_render.

## Lighting wishes (`assets/data/environments/garden.ron`, render agent)
- **All times:** shadowed vertical faces render near-black — the west-facing east fence and the
  shed's west wall at noon and in the morning. More sky/ambient fill would keep them readable.
- **Morning:** low sun from the east over the fence: long shadows across the lawn toward the house;
  dew-fresh, slightly cool fill.
- **Noon:** high sun from the south lights the house front (it faces the garden): bright, saturated
  greens; the patio slabs read well already.
- **Sunset:** warm sun from the west over the west fence (looks lovely now); a touch more orange
  bounce on the brick house.
- **Night:** porch lantern (1200 lm, shadows) by the kitchen door, two fairy-light point lights
  over the deck, glowing kitchen/French-door/bedroom windows; the lawn is very dark — a slightly
  stronger cool moon fill would help the racing line read.
- Wet weather: the puddle polygons (Water) read as dark blue sheets; a lighter, sky-reflecting
  water material would sell them better.

## Validation
- Plots: `cargo run -p rc_track --example plot -- backyard_bash [--rev] [--mirror]` — all four
  variants load with **no lints**.
- `cargo test -p rc_track` (every track file in all four variants) and `cargo test -p rc_render`
  (prop lamps, decor) pass.
- AI: `AI_TRACK=backyard_bash AI_DIFF=Elite AI_CLASS=Elite AI_N=8 AI_LAPS=3 AI_SEED=21 [AI_REV=1]
  [AI_MIRROR=1] cargo test -p rc_ai --test race -- --ignored --nocapture probe` — all 8 finish in
  every variant, worst racer 1.24× (≤ 1.5×), ≤ 1 respawn per racer; seeds 3 and 9 and a Pro field
  also pass (worst 1.37×).
- In game: `cargo run -p recharged -- --user-data-dir target/agent-data/T16-garden --unlock-all
  --quick-race backyard_bash [--reverse] [--mirror] --autopilot --exit-after 40`, and
  `--track-tour backyard_bash` for eye-level shots every 25 u.

## Lessons (AI) and known issues
- Exposed obstacle ends next to a corner exit trap the AI (it slides wide, hits, spins, drives
  back): the bed's wall therefore runs on round the SW corner; the deck turn's run-off is kept clear
  (barbecue inside the turn); the spiral's outer wall runs 8 u along the return lane so its end is
  not at the reverse entry apex; a pocket north of the bench lane is closed by a pot cluster and an
  edging row. The deck's off-road margin is 10 so cars landing wide off the reverse ramp crest do
  not miss the deck-turn checkpoint.
- The open central lawn invites a cut from the return lane to the south path; checkpoints and the
  off-track rule stop it, the round bed only partly blocks it.
- The deck ramps are 15° wedges: cars crest them with a little air; the reverse ramp (from the
  patio) sends fast cars wide on landing (handled by the clear run-off).
- Under the bench the chase camera briefly sees the dark seat underside (a tunnel moment).
- Track preview card `assets/ui/tracks/backyard_bash.png` and medal times are not generated yet.
