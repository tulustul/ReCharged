# Environment `toy` — Toy Town (toy store / playroom)

Owner: T16-toy. Look: art bible §3.1–3.2. Track rules: `rc_track.md`. Files: generators
`tools/blender/props/toy/`, models `assets/models/props/toy/`, catalog `assets/data/props/toy.ron`,
layout `assets/maps/toy_town/track.ron`, decor `assets/maps/toy_town/decor.ron`.

## Regenerate / verify

```sh
tools/blender/props/toy/build.sh                       # all props + validation (~25 s)
tools/blender/props/toy/build.sh --preview DIR bear    # chosen props + EEVEE previews
python3 tools/blender/props/toy/check.py               # validation only (no Blender)
python3 tools/blender/props/toy/supports.py            # re-place the track's support pillars
blender -b -P tools/blender/props/toy/diorama.py -- DIR [--rev] [--views top,overview,chase]
cargo run -p rc_track --example plot -- toy_town [--rev] [--mirror]
cargo run -p recharged -- --user-data-dir target/agent-data/T16-toy --unlock-all \
    --quick-race toy_town [--reverse] --track-tour toy_town --exit-after 120
```

- `check.py`: glb parses, budget (kit 5 k / hero 15 k), named materials, no texture without
  UVs, ground contact at 0 (pillar excepted), every collider inside the visual bounds, decor
  models parse, the track uses known kinds and ≤ 40 dynamic bodies, no orphan glb.
- `supports.py` mirrors `spline.rs` and rewrites the `// supports:begin … end` block: a pillar
  pair under every stretch whose ribbon bottom is > 0.5 u above the mat, per direction (pillars
  under the filled Fwd gap are `Rev`, under the Rev gap `Fwd`), skipping spots that would pierce
  a lower part of the track or the castle (r 10.3). Re-run it after any layout change.
- `diorama.py` builds the dressed track in Blender (ribbon profile + barriers swept along the
  same spline, ground, room box, every prop and decor model) for checks without the game.
- Previews (`propkit.finish`, shared `tools/blender/lib/propkit.py`): hero ¾, front, back, top
  and an RC eye-level shot with the Sparky car for scale, plus a contact sheet.
- Shared toy kit `toykit.py`: palette, materials, procedural textures generated in Blender with
  numpy and embedded in the glb (beech grain, stained-wood grain, stone blocks, pegboard,
  dapples; 512² = 2 u at 256 px/u, box/cylindrical UVs in world units), `extrude` with robust
  concave caps, `bevel_bm` (exact bmesh bevels where the modifier misbehaves after insets).

## Props (real size × 4.44; display-size pieces where a toy store has them)

| Kind | Model | Tris | Notes |
|---|---|---|---|
| `toy_dollhouse` | dollhouse | 14 206 | hero; walk-in display dollhouse, 14 × 10 × 15 u, 9 × 4 u wallpapered drive-through tunnel (its walls are the barriers there); symmetric, a porch on each long side, so mirrored layouts still show one to the room |
| `toy_castle` | castle | 5 204 | hero; play fort inside the helix (r ≤ 8.8): stone-block walls, red/blue cone towers, gatehouse + drawbridge, keep with flag (top 13.9 u < 16 u ceiling) |
| `toy_shelf_a` / `_b` | shelf_a / shelf_b | 8 036 / 5 588 | hero; gondola shelves 5.4 × 9.3 × 2.2 u, pegboard, price strips, header pictogram, seeded stock (windowed boxes, plush, balls) |
| `toy_rocking_horse` | rocking_horse | 3 140 | hero; dapple-grey on plank bow rockers |
| block tower set | block_* | — | hero (knockable): the five dynamic blocks below |
| `toy_block_cube` | block_cube | 2 089 | dynamic 0.35; ABC block 1.4 u: recessed painted fields, raised numbers/shapes, beech grain |
| `toy_block_plank` / `_pillar` / `_roof` / `_arch` | … | 108 / 512 / 80 / 344 | dynamic 0.35 / 0.27 / 0.12 / 0.55; stained jumbo blocks on the 1.4 u grid |
| `toy_block_pile` | block_pile | 712 | static heap (clutter without dynamic bodies) |
| `toy_track_support` | track_support | 1 464 | yellow segmented pillar + blue bracket; origin = bracket top, column 8 u into the floor |
| `toy_track_ramp` | track_ramp | 684 | moulded jump ramp, ToyTrack surface, 2.5 u over 9 u, chevrons |
| `toy_play_table` | play_table | 1 504 | Wood surface; printed little town; GB perch; 1.8 u clearance under it |
| `toy_start_arch` | start_arch | 2 932 | checkered banner across the start line, posts outside the barriers |
| `toy_bear` | bear | 4 360 | giant plush teddy, sitting |
| `toy_ball_pit` | ball_pit | 4 098 | inflatable pool heaped with balls (spilled balls visual only) |
| `toy_box` | toy_box | 832 | packaging with a toy car in its window tray; stacks |
| `toy_robot` | robot | 1 902 | tin robot waving a checkered flag (the starter) |
| `toy_train` | train | 2 428 | wooden pull-along train |
| `toy_stacking_rings`, `toy_duck`, `toy_crayons`, `toy_foam_tiles` | … | 3 680 / 1 928 / 1 436 / 1 608 | small floor toys |
| `toy_cubby` | cubby | 2 300 | low cubby shelf with fabric bins (fits under the 3.5 u window sills) |
| decor: bunting, sign | bunting / sign | 366 / 1 868 | `decor/toy_town.ron`: pennants under the ceiling, hanging aisle signs (origins at the ceiling) |

Forward layout: 249 placements (70 shelves, 25 cubbies, 74 pillars, 29 dynamic blocks) ≈ 0.83 M
triangles if everything were in view (shelves ≈ 0.5 M), + 15 decor items (11.5 k).

## Layout (`toy_town.ron`)

639 u, 3 laps, 9 checkpoints, 5 item rows, reference lap 24.5 s; one chokepoint (dollhouse, 7 u).
Lint-free in all four variants. Changes from the T02 placeholder layout:

- **Raised banked turns.** Banked turns pivot on the centreline, so at y 0.3 their inner half
  (lips and barrier) sank under the play-mat. T1/T3 (20°) are now at y 1.8, T5, the U-turn and
  the hairpin (15°) at 1.35, the helix's first point banks 6° instead of 12°; the climbs happen on
  the straights (≤ 8.5°). Two knee points keep the start straight and the grid level. The end
  points of every raised turn are half banked (T19a), so the roll builds over two segments — the
  AI no longer spins at the bank onset (reverse T1). The U-turn's ends sit at mat level, unbanked.
- **Dressing.** Castle in the helix, start gate, play table + ramp instead of the toy box,
  pillars under every raised stretch, shelves on every wall, cubbies under the windows, displays.

Features per direction:

| | Forward (clockwise) | Reverse |
|---|---|---|
| Jumps | elevated drop: 5° kicker at 6.8 u, 12 u gap, landing 3.5 u lower, 0.69 s air at 30.6 u/s, toward the ball pit | infield hop eastward: 8° ramp to 2.3 u, 8 u gap, 0.60 s air |
| Climbs | infield hump (the Rev jump filled), helix up 6 u | 16° climb up the filled drop, helix down |
| Shortcut | through the U-turn interior past two block gates (Carpet: top speed × 0.92) | same, reversed |
| Golden Battery | on the play table (−5, 3.55, 14): straight from the shortcut entry up the toy-track ramp, across the table, a 2.5 u drop and on to the shortcut exit | none (ramp removed) |
| Knockable blocks | gate ×2 on the shortcut path, ABC pyramid where the shortcut rejoins the exit straight, a stack on the elevated straight (knock it down to the mat), towers at the dollhouse's open north exit | same placements |

Intended lines: the racing line takes the raised banked turns at speed and threads the dollhouse
in the middle; the shortcut is shorter but slower and blocked by blocks on lap 1; the
battery line trades time for the collectible.

## Lighting wishes (`assets/maps/toy_town/environment.ron`, rendering agent)

- **Morning:** keep the low warm east sun raking across the mat through the three east windows;
  the shelves and castle should throw long soft shadows. The dollhouse tunnel and the lower helix
  level (under the upper ribbon) go quite dark: a little more bounce/ambient there.
- **Noon:** bright and even; same wish for the covered spots.
- **Sunset:** the warm west light through the two west windows lights the dollhouse's west side and
  the castle's flags; ceiling spots on.
- **Night:** warm ceiling spots; spots aimed at the castle and the dollhouse would make them
  read as lit displays; cool moonlight through the windows over the cubbies.
- Shadow artefact seen in the tours: the upper helix ribbon casts stair-stepped shadows on the
  lower level (shadow-map resolution/cascade).

## Known issues / limitations

- Pillar brackets are flat: under the 12° helix bank a gap of ≤ 0.2 u shows between bracket
  and ribbon; between pillars the underside of raised banked turns is open (toy-track look).
- No colliders on the rocking horse's legs, the ball pit's spilled balls, the crayons, the flat
  foam tiles or decor (all off the driving line or out of reach).
- Cubbies and wall shelves follow the window spans of `toy_store.ron` (east z −46.4…−20.4,
  −10…16, 26.4…52.4; west −36…−10, 16…42): if the windows move, move them too.
- Shelf rows dominate the triangle count (≈ 0.5 M for 70 shelves); LODs would help split-screen.
- The AI 3-lap test is pending (rc_ai not available yet).
