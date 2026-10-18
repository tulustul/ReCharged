# Environment: family house — Sofa Circuit (T16)

Owner: environment artist (T16, `house`). Binding: art bible §3.1 + §3.3, `rc_track.md` (authoring
guide). Files: `tools/blender/props/house/**` → `assets/models/props/house/*.glb`,
`assets/data/props/house.ron`, `assets/data/tracks/sofa_circuit.ron`,
`assets/data/decor/sofa_circuit.ron`, this note.

## Regenerate / verify

```sh
tools/blender/props/house/build.sh [--preview DIR] [prop ...]   # all props + validation (~6 min)
blender -b -P tools/blender/props/house/<prop>.py -- [--preview DIR]
python3 tools/blender/props/house/check.py     # validation only (no Blender)
cargo run -p rc_track --example plot -- sofa_circuit [--rev] [--mirror]
cargo test -p rc_track                          # loads every track file in all four variants
AI_TRACK=sofa_circuit AI_DIFF=Elite cargo test -p rc_ai --test race -- --ignored --nocapture probe
```

`housekit.py` holds the palette, the material table, the procedural textures (oak grain, fabric
weave, subway tiles: tileable, 512 px per 2 u, packed into the glb as JPEG), the shapes (rounded
boxes for upholstery, swept bands for throws and cables, tapered legs, books) and the indoor
preview stage (oak boards, warm low window sun). Models are baked, budget-checked and exported by
the shared `lib/propkit.finish`; `done()` box-maps any part without UVs first (a textured
material on a mesh without UVs exports `texCoord: -1`, which Bevy rejects). Multi-prop scripts:
`bookcase.py` (a, b), `counter.py` (sink, hob), `clutter.py`, `hall.py`.

`check.py` checks every catalog kind: the glb parses, triangles within budget, ground contact
(y = 0, hanging pendants excepted), every collider inside the model's bounds (+0.12 u), no
negative texCoord, dynamic props have a mass; and that every decor model exists.

## Props (real size × 4.44)

| Kind | Tris | Notes |
|---|---|---|
| **sofa** (hero) | 5 740 | sage three-seater on tall tapered oak legs, 1.75 u clear underneath (the tunnel; legs stylised tall so the 1.1 u monster truck fits), dark dust cover = tunnel ceiling, mustard/rust pillows, knitted throw |
| **coffee_table** (hero) | 2 000 | square oak table 5.55 × 1.78 u, shelf with books and a bowl; drivable top (Wood); collider solid below the top (a car's nose wedged under the shelf in testing) |
| **book_ramp** (hero) | 8 772 | 12 slanted atlases on stepped book stacks + two magazines at the toe; covers lie exactly on the wedge (15.3°) |
| **dining_table** (hero) | 1 988 | 2.6 m oak table laid with a runner, fruit, flowers, plates; 2.8 u clear under the apron |
| **staircase** (hero) | 5 954 | 2 steps to a 5 × 4.8 u landing at 2.3 u (open sides), 12 steps to the ceiling, white panelled cupboard with door, runner, spindle balustrade |
| **sideboard** (hero) | 1 696 | walnut, slatted doors, brass pulls (TV unit and dining divider) |
| tv | 208 | 55" off, glossy black screen |
| armchair | 3 036 | mustard lounge chair (pair facing each other across the jump) |
| bookcase, bookcase_b | 2 080 / 2 212 | low white bookcases, two book arrangements: the room dividers |
| dining_chair | 1 120 | oak, cream pad: slalom gates |
| bench | 1 292 | white storage bench, cushions (seals the space under the table) |
| plank | 136 | 80 cm plywood strip onto the stair landing, grip tape, chalk arrow, toe shim |
| kitchen_island | 2 804 | navy shaker cabinets, quartz top, two stools |
| counter, counter_hob | 3 339 / 2 484 | shaker base units, oak worktop, subway tiles; sink + wall cabinets / hob, oven, hood |
| fridge | 1 588 | steel, kids' drawings |
| wall_spine, wall_hall | 4 192 / 2 288 | interior walls (11.5 u): kitchen door, pictures, radiator, clock / two cased openings, mirror, coats |
| plant_pot, floor_lamp, pendant | 1 258 / 600 / 530 | fiddle-leaf fig; lamps carry `lights` |
| console, shoe_rack | 1 380 / 1 496 | hall console with table lamp (`lights`), shoe bench |
| cable | 1 016 | extension cable across lane B: a 0.06 u bump |
| book_small/medium/large, magazines, mug, remote, cushion, slipper, shoe, laundry_basket, bowl, toy_truck | 188–1 948 | knockable clutter (dynamic, mass 0.04–0.3) |
| front_door, doormat, rug_fringe | 632 / 116 / 852 | decor only (`data/decor/sofa_circuit.ron`) |

Hero props 6, kit 31 (+3 decor models), all within budget; the placed props of the track total
≈ 0.13 M triangles. Textures: oak, walnut, plywood grain, fabric weaves, subway tiles (512²).

## Layout: `sofa_circuit.ron` (414.8 u, 5 laps, Invisible ribbon)

House interior x −57..56, z −27..37 (bounds = shell walls, 11.5 u ceiling). Floors: oak in the hall
and living room and the family corner, cream tile in the kitchen (incl. under the dining table),
two terracotta rugs (Carpet, 0.05 u). Rooms are about 1.5× real area (the lap needs it);
furniture is true scale. The loop is an S-meander of four east–west lanes plus an east return:

| Lane (forward) | Room | Feature |
|---|---|---|
| C, west | kitchen-diner, start/finish at x = −3 (grid on C's dining end) | along the bench and the island |
| U CD | kitchen | right hairpin around the island (tile: drifty) |
| D, east | kitchen-diner → family corner | **chair slalom**: narrows to 6 u and weaves on R = 8 arcs around three chairs standing in the lane (chokepoint 2) |
| NE turn, return south | family corner → living room | **tunnel under the sofa** (front to back = back to front in reverse) |
| SE turn | living room, rug, in front of the TV | |
| A, west | living room → hall | **coffee-table jump**: book ramp up, over the table between the two armchairs (chokepoint 1, 6 u), land in the hall |
| U AB | hall | right hairpin around the foot of the stairs; **stair ramp** shortcut up the plank onto the landing |
| B, east | hall → living room (behind the bookcases) | cable bump, toys |
| U BC | living → kitchen-diner | left hairpin round the end of the spine wall |

- **Jumps** (both directions, `dir` Fwd / Rev): forward climbs the east book ramp (Fwd prop), the
  gap runs from the table's east edge to the west ramp's foot; reverse mirrors it with the west
  ramp (Rev prop). Takeoff 27.6 u/s, 0.77 s air, landing in both directions past the landing
  point at 1.0× and 0.8× (plot side views). Fast cars fly over the whole table.
- **Stair ramp**: a plank leans on the two bottom steps; from the U AB apex you can go up it, across
  the landing and drop 2.3 u off the far side onto B (forward) or A (reverse). Not faster than
  the hairpin; risky. **Golden Battery** (forward) sits on the landing at (−29.1, 3.3, 20).
- **Item rows**: C (kitchen), D before the slalom, the return before the sofa, A before the jump
  approach, B in the living room. 8 checkpoints (automatic), 24–25 respawn points.
- **Separators** between parallel lanes are furniture (stairs, bookcase rows, island + table +
  bench + sideboard row, spine wall); barriers only where a gap would let you cut a hairpin
  (the plank pocket, the island-to-table gap on D).
- **Dynamic props**: 19 of 40 (books, cushions, slippers, shoes, mug, remote, magazines, toy
  trucks, laundry basket, dog bowl), placed on and beside the lines; restored on restart.
- Reference lap 20.1 s; headless 8 Elite AI, 3 laps: 0.92–1.05× reference, 0 respawns (forward).

## Lighting wishes (`environments/house.ron`, owned by rendering)

- The patio windows belong to the living room's south wall (x ≈ 8..34; the current South 0.7 =
  x 11..33 is right) and a smaller one to the hall's south wall (South 0.3 is fine).
- **West window: move it into the kitchen**, centre 0.22 (z ≈ −13), width ~16: at 0.5 it
  straddles the interior spine wall (z = 5) and sits above the hall's shoe rack; the front door
  (decor) is at z = 20 on that wall.
- East window (0.4) is fine (open-plan family corner / living room).
- Night: the practical lights are the three floor lamps, the hall console lamp and three pendants
  over the island and the dining table (catalog `lights`); a warm, low key with cool moonlight
  through the patio windows sells "lamps at night". Sunset: long warm streaks across the oak from
  the south-west windows onto the jump.

## Known issues

- The sofa's legs are stylised tall (39 cm clear) so every car fits through the tunnel.
- The stairs run up into the ceiling plane (no stairwell opening in the shell).
- Rooms are ~1.5× real area; the house is large (25 × 14 m at true scale) to fit 400 u.
- Book ramps: the collider wedge is smooth, the visual has slanted covers on stepped stacks;
  wheels may dip ~0.1 u into the toe magazines.
- The racing line ignores the dynamic clutter; rc_ai treats it as traffic-free floor.
