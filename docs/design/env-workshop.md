# Wrench Works — auto-repair workshop (T16, env `workshop`)

Owner: environment artist (T16 workshop). Look: art bible §3.6. Track: GDD §7.1 (`wrench_works`,
indoor ★★★, ~700 u, 3 laps). Lighting file `assets/data/environments/workshop.ron` belongs to
rendering (wishes below).

> **Editor-owned layout.** Since the track editor (`docs/design/rc_editor.md`), the track and decor
> files are edited with `cargo run -p rc_editor -- wrench_works`; the layout script below writes them only
> with `--layout`, which replaces every edit made in the editor. Its constants still feed the prop
> builds.

## Files and regeneration
| What | Path |
|---|---|
| Layout generator (track + decor + catalog) | `tools/blender/props/workshop/plan.py` → `assets/data/tracks/wrench_works.ron`, `assets/data/decor/wrench_works.ron` |
| Prop catalog (dims, colliders, lights) | `tools/blender/props/workshop/catalog.py` → `assets/data/props/workshop.ron` |
| Prop generators (one per glb) | `tools/blender/props/workshop/<name>.py` → `assets/models/props/workshop/<name>.glb` |
| Shared kit (palette, materials, textures, preview stage, tyres) | `tools/blender/props/workshop/wkit.py`, `liftkit.py`, `shelfkit.py` |
| Generic full-size cars (shared lib, brand-free) | `tools/blender/lib/workshop_car.py` (hatch, saloon presets) |
| Validation | `tools/blender/props/workshop/check.py` |

```sh
python3 tools/blender/props/workshop/plan.py            # track + decor + catalog
for f in tools/blender/props/workshop/[a-z]*.py; do case $f in */wkit.py|*/liftkit.py|*/shelfkit.py|*/catalog.py|*/plan.py|*/check.py) ;; *) blender -b -P $f -- --preview /tmp/prev ;; esac; done
python3 tools/blender/props/workshop/check.py           # glb parse, budgets, ground contact, colliders in bounds
cargo run -p rc_track --example plot -- wrench_works [--rev] [--mirror]
```
`--preview DIR` renders the propkit shots on a workshop stage (warm sodium key, cool door fill,
concrete) plus RC-eye shots for the heroes. Textures are procedural (numpy → packed PNG in the
glb): plywood veneer, butcher block, ply edges, timber, tyre tread/sidewall normal map, hazard
stripes, glazed tiles, roller-door slats, pegboard.

## Props (triangles from `check.py`)
**Heroes (6, ≤ 15 k):** `workshop_lift_a` 13 627 (two-post lift + generic hatchback raised high,
sills 6.9 u), `workshop_lift_b` 13 893 (lift + generic saloon at wheel-work height, sills 4.3 u),
`workshop_workbench` 4 036 (butcher-block island 24 × 8 u, top 4.0 u, plywood kicker each end;
drivable, Wood), `workshop_tool_chest` 2 476, `workshop_paint_booth` 4 842 (drive-through,
angled light boxes, paint stands, 4 night spot lights), `workshop_wash_bay` 2 740 (tiled splash
walls + spray gantry).

**Kit (22, ≤ 5 k):** ramp 784 (plywood, 4.5 u over 21.7 u, Wood), tyre_stack2 2 880,
tyre_stack4 4 816, tyre 1 568*, wheel 2 488*, cone 276*, bucket 716*, oil_can 456*,
jerry_can 636*, toolbox 544*, battery 252*, drain_pan 412*, creeper 624*, wrench 496*, jack 692,
oil_drum 1 368, pallet 880 (Wood top), tyre_changer 2 204, wheel_balancer 2 028,
engine_stand 896, engine_crane 660, hanging_lamp 248 (neutral-cool fluorescent batten at
14.85 u, tubes always lit, a 72° spot light at night: 22 over the lanes) (* = dynamic, masses
0.08–1.2).

**Decor (physics-free, `assets/data/decor/wrench_works.ron`):** roller_door 1 008,
tyre_rack 4 884, shelf_a 3 504, shelf_b 3 728, tool_wall 1 164, compressor 1 452,
safety_signs 640 (pictograms only), roof_truss 1 128, bay_marking 16, walkway_line 2, stains 180, start_line 70 (chequered tape —
Invisible tracks get no drawn start line).

Scene total ≈ 0.36 M triangles; 123 placed props (31 dynamic), 61 decor items (T19a: the chicane
entry lost 4 tyre stacks, the S lane 2 cones).

## Layout
Room 176 × 120 u (x −88…88, z −60…60), concrete floor (Asphalt). Invisible ribbon; walls are
rc_track barriers (themed hazard blocks by rc_render) except where the tyre stacks, the booth
and the wash-bay walls take over. 702 u, 10 checkpoints, 43 respawns, reference lap 28.0 s.

Forward (clockwise), lane by lane:
1. **Start / N lane (east):** grid in front of the two roller doors; flat out under the tails of
   the hatch (high) and the saloon (low, a wheel off); oil drips under each car, drain pans and a
   creeper at the edges. Item row before the lifts.
2. **E1 U-turn (right, r 14):** oil slick on the apex (slide), the red tool chest on the island
   tip.
3. **M lane (west):** corridor narrows to 8 u, plywood ramp up (11.7°), crest hop onto the bench
   top (item row), kicker → **jump** off the west end (30 u/s, 0.8 s air) onto the far ramp.
   Falling off the side drops into an enclosed island → respawn. The lane walls stop 7 u short of
   both ramp feet (T19a, Q4): a car landing a little off line, in either direction, runs out on
   open floor instead of into a wall; a respawn point lies 6 u past each landing (rc_track).
4. **W U-turn (left):** oil on the outside of the exit.
5. **S lane (east):** **tyre-stack chicane** (three apexes, 8 u, ~17 u/s; its tyre walls start 5 u
   in, `CHICANE_OPEN`, so cars leaving the W U-turn wide are not caught on the first stack).
   **Golden Battery** (forward only): in the tyre alley behind the first apex — in through a gap in
   the north tyre wall, along a 3.4 u alley, out through the next gap (slow, tight; declared as no
   shortcut, it is longer than the line).
6. **E2 U-turn (right):** oil spill on the S3 exit.
7. **S3 lane (west):** **wash bay** (Water, −15 % top speed) then the **paint booth**
   (drive-through, wider inside).
8. **SW corner, W lane (north) over rubber mats** (Carpet), NW corner, start.

Reverse: the same loop backwards; the jump goes off the east end of the bench (Rev gap); the
alley exists without the battery. Mirror swaps sides (prop meshes are not mirrored).

Knockable dressing (drain pans, creeper, wheel, jugs, jerry cans, cones, tyres) stands behind the
lane walls (`DRESS_BEHIND` 1.5 u) or, where the walls are open, just outside the racing width:
cars knocked it onto the line or tripped over it (QA, Q4). Islands (enclosed by lane walls) hold
the lift columns, the tyre changer, wheel balancer, engine
on a stand and crane, drums, pallets and tall tyre stacks (statics with colliders: a car falling
off the bench can land there).

**AI** (rc_ai `probe` / `every_layout`, 8 Elite AI in Elite cars, 3 laps, T19a): seeds 21, 7
and 3 × all four variants — every car finishes, worst 1.19× the expected time (seed 7 mirror),
at most 2 respawns per car and race (QA before T19a, seed 21: 1.56× — over the 1.5× limit — and
4 respawns: the walls at the ramp feet, the chicane entry and knocked dressing).

## Lighting wishes (rendering)
- **All times:** sodium work lights warm (≈ 1.0, 0.72, 0.42) against cool door daylight; walls
  light grey-cream so the space reads; the booth stays cool-white inside (its prop lights spawn
  at night); a soft haze volume in the booth (x −24, z 42, 34 × 16 × 12 u).
- **Morning:** low cool-blue daylight through the north doors, a warm shaft through the east
  windows onto the E1/E2 U-turns.
- **Noon:** brightest door light, neutral grade.
- **Sunset:** long orange shafts across the N lane start; truss shadows on the walls (already
  lovely in game).
- **Night:** doors dark blue (already great); sodium grid plus my neutral-cool fluorescent
  battens over the lanes; the booth glows. In game the night grade is very orange: a cooler fill
  or less warm temperature would let the battens and the blue doors read as the cool contrast.
- Requested (sent): mullion-free roller-door openings at x 6 / 32 (north) and 27 (south) —
  done by rendering; hazard barrier style — done.

## Known issues
- The centreline dips ≈ 0.2 u at the ramp feet (Catmull-Rom); harmless (respawn height only).
- Both ramp crests give a small hop before the bench top (permanent kickers at both ends keep
  the data identical in both directions).
- Cars on lifts are generic shell lofts at 1/9 scale scaled up: soft faceting on the bonnet,
  simplified wheels (16 segments) to fit the hero budget with the lift.
- Island props are statics with simple box/cylinder colliders (reachable only by falling off the
  bench); decor has no colliders by design.
