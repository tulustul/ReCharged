# Environment `street` — Maple Street (T16)

Owner: T16 street. Look: art bible §3.1 + §3.7. Track rules: `docs/design/rc_track.md`.
Suburban street & front lawns in autumn: a residential street with a cul-de-sac, a boulevard
with a median (Birch Lane), front yards, parked full-size cars, maples. 940 u lap (GDD ~900),
3 laps, outdoor ★★★, surfaces Asphalt / Grass / Stone (+ Wood ramps), all six weathers.

> **Editor-owned layout.** Since the track editor (`docs/design/rc_editor.md`), the track and decor
> files are edited with `cargo run -p rc_editor -- maple_street`; the layout script below writes them only
> with `--layout`, which replaces every edit made in the editor. Its constants still feed the prop
> builds.

## Regenerate / verify

```sh
tools/blender/props/street/build.sh [--preview DIR]   # plan + all props + check (~1 min)
python3 tools/blender/props/street/plan.py            # layout, catalog, decor only
python3 tools/blender/props/street/check.py           # validate glbs against the catalog
cargo run -p rc_track --example plot -- maple_street [--rev] [--mirror]
blender -b -P tools/blender/props/street/<script>.py -- [--preview DIR] [--only NAME] [--draft]
```

**Generated files — edit the generators, not the outputs:**
`assets/maps/maple_street/track.ron`, `assets/data/props/street.ron` and
`assets/maps/maple_street/decor.ron` come from `plan.py` (street plan, race line, dressing,
decor) and `catalog.py` (prop dimensions and colliders, shared with the Blender scripts).
`check.py` checks every kind: glb present and parsing, triangle budget (kit 5 k / hero 15 k),
colliders inside the mesh bounds, `lamp*` bulb material on lit kinds, decor models present,
placed kinds in the catalog, ≤ 40 dynamic.

Blender scripts (`tools/blender/props/street/`): `streetkit.py` (palette, material table,
profile sweeps, full-size decals/glass, road wheel, block lettering), `kerbs.py`,
`parked_suv.py`, `yard_ramps.py`, `storm_drain.py`, `street_lamp.py`, `house.py`, `maple.py`,
`kit_flat.py`, `kit_small.py`. Shared driver: `lib/propkit.py` (toy agent) for bake/checks/
export; `lib/street_props.py` adds the outdoor preview stage and custom preview cameras.

## Street plan (u; 1 u = 22.5 cm)

| Level | Height | What |
|---|---|---|
| road | 0 | one Asphalt polygon over the whole area (minus the drain dish) |
| drain dish floor | −2.05 | under the storm-drain prop (respawn/drivable checks) |
| lawns | 0.6 | Grass blocks behind the kerb backs, median, island |
| sidewalks | 0.62 | Stone strips 8 u wide behind 8 u planting strips |
| driveways | 0.64 | Asphalt, from the aprons to the garages |
| kerb tops | 0.65 | barrier kerbs (vertical face) — props |
| raised yard | 3.0 | Grass plateau in stone retaining walls; kickers to 3.8 |

Maple Street: kerbs at z = ±20, from x = −100 to a cul-de-sac bulb (centre x = 260, kerb R 46,
island R 14). Birch Lane: boulevard south from the T junction, roadways 28 u, median x ±8 with
rounded noses (z 40 and 154), **rolled (mountable) kerbs** on its outer edges (curb hops);
Maple keeps barrier kerbs. Corner radii 16.

## Lap (forward = start eastbound on Maple)

1. Start straight (line at x 90, grid 66–87) → **under a parked SUV lengthwise** (6 u between
   its wheels: chokepoint 1 of 1, cones mark the entry). The line sits at x 90 so that the
   evenly spaced checkpoint gates stay out of the curb-hop S-bend (step 5).
2. Cul-de-sac: clockwise around the planted island (radius 30), out over the driveway apron.
3. Lawn → **east plank** (two plywood sheets on cinder blocks, 14.9°, forward only) onto the
   raised yard → kicker → **jump off the west end** (5.7° kicker, 3.2 u drop, 0.62 s air).
4. Sidewalk west; the corner lot's front yard is walled (rc_render draws the barrier as kerb
   stones) with two gates: the **lawn shortcut** cuts the corner through them (grass, a maple,
   a bike and a sprinkler on the way); the main line turns on the corner paving. The shortcut
   path continues ~12 u along the sidewalk beyond both gates so the checkpoint keep-out zone
   puts the neighbouring gates where every car is still on the main line.
5. Birch east sidewalk → right turn, **curb hop** down the rolled kerb straight **under an SUV
   parked across the line** (between its axles, 9 u wide) → left onto the roadway.
6. **Storm-drain dip** in the median gutter (24 × 18 u dish, 2 u deep, 11° slopes: compression
   and a kick out) → U-turn around the median nose (road closed beyond: barricades) → Birch
   north → right into Maple.

Reverse: the same loop; the **west plank** (reverse only) climbs onto the yard and the car
**jumps off the east end** (Rev gap, 0.62 s air); the curb hop is climbed; the bulb is driven
counter-clockwise. Mirror variants mirror positions/yaws (prop meshes are not mirrored).

| Feature | Forward | Reverse |
|---|---|---|
| tunnel under the parked SUV (Maple) | ✓ | ✓ |
| plank up / jump off | east plank / west edge | west plank / east edge |
| apron (cul-de-sac exit) | down onto the lawn | up from the lawn |
| curb hop + SUV across (Birch) | down the rolled kerb | up the rolled kerb |
| storm-drain dip | ✓ | ✓ |
| lawn shortcut (corner lot) | ✓ | ✓ |
| Golden Battery | in the drain inlet (median kerb, dip bottom) | — |

Item rows (5): start straight, cul-de-sac ring, raised yard, Birch east before the dip, Birch
west. 13 checkpoints, 59–60 respawn points. Reference lap 34.8 s (reverse 34.9 s).

**AI (DoD-2):** `AI_TRACK=maple_street AI_DIFF=Elite AI_CLASS=Elite AI_N=8 AI_LAPS=3 AI_SEED=<s>
[AI_REV=1] [AI_MIRROR=1] cargo test -p rc_ai --test race -- --ignored --nocapture probe` — seeds 21,
7 and 3 × all four variants (T19a): every car finishes, worst 1.22× the expected time (limit 1.5×),
at most 2 respawns per car and race (limit 6). The corridor stays inside the barrier kerbs
everywhere (an earlier version let the racing line graze the north kerb after the tunnel and the NW
fillet: cars spun out in the mirrored reverse).

**Intended lines:** hug the north parking lane into the SUV tunnel; swing wide (north) into
the bulb and clip the island; cross the apron diagonally; stay straight from the plank to the
lip; the shortcut saves ~16 u but is grass with two 9 u gates; after the curb hop, brake for
the tight left (R 11, `HOP_TURN_R`); the dip throws the car up on exit — land straight for the
U-turn.

## Props (47 kinds, ~460 placements, 22 dynamic)

| Kind | Tris | Notes |
|---|---|---|
| `street_parked_suv_a/b/c` (hero) | 14 696 | generic family crossover (no brand), slate blue / burgundy / silver; floor ≥ 1.8 u, 9.2 u between the axles, 6.1 u between the wheels |
| `street_storm_drain` (hero) | 1 647 | dish + grate + kerb inlet + fish marker; deep kerb included |
| `street_lamp` (hero) | 720 | 28 u cobra head, `lamp_sodium` bulb, spot light 60 000 lm |
| `street_house_a/_m`, `street_house_b/_m` | 4 804 / 3 216 | colonial and ranch, colour variants, porch light |
| `street_hydrant_cluster` (hero), `street_hydrant` | 1 368 / 732 | red with white bonnet, yellow bollards |
| `street_fence_corner` (hero), `street_fence` | 2 836 / 1 024 | white pickets, climbing rose |
| `street_kerb_4/2/1`, `street_kerb_rolled` | 252 / 320 | stones + gutter pan |
| `street_kerb_arc_*` (6, from the plan) | 428–3 538 | exact arcs, ≤ 80 u per piece |
| `street_apron` | 566 | driveway apron with flares (14°) |
| `street_plank_ramp`, `street_kicker`, `street_retaining_wall` | 2 784 / 304 / 1 848 | the raised yard |
| `street_maple_a/b` | 4 342 | 44–50 u autumn maples (trunk collides) |
| `street_hedge`, `street_mailbox`, `street_trash_can`, `street_recycling_bin` | 600 / 568 / 596 | |
| `street_sign_stop`, `street_sign_street` | 488 / 3 468 | block-letter STOP, MAPLE ST / BIRCH LN |
| `street_barricade`, `street_cone` (dynamic 0.25) | 260 / 240 | amber flasher light |
| `street_leaves`, `street_leaf_pile` | 1 058 / 4 730 | maple-leaf scatter (flat), soft 0.5 u bump (Grass; the dome is 0.42–0.5 u high like its collider cap, so cars ride on the leaves) |
| `street_manhole`, `street_paint_dash/bar/stop` | 920 / 12 | flush, drivable |
| `street_newspaper` (dynamic 0.08), `street_sprinkler`, `street_garden_light`, `street_bike` | 132 / 192 / 96 / 1 328 | |

Decor (`assets/maps/maple_street/decor.ron`): 9 houses and 13 maples beyond the bounds, standing
at the edge-lawn height (0.6 u) on Grass lawn plates that `plan.py` lays 120 u beyond the bounds
(`build_beyond`; Maple Street and Birch Lane leave between them at road level). Physics never
reaches them.
Visible-triangle estimate for the whole place: ≈ 0.5 M.

Conventions: materials are plain PBR (no textures yet); clearcoat only on the parked cars'
paint and glass (art bible car paint); bulbs named `lamp*` (rc_render dims them by day and adds
the catalog `lights` at night). Collider rotations use `rot: (0, yaw, 0)` (catalog helpers take
`yaw=`).

## Lighting wishes (for `assets/maps/maple_street/environment.ron`)

- **Morning:** low warm sun from the east-south-east, long shadows across Maple Street, cool
  blue sky fill; slight haze so the maples glow against it.
- **Noon:** high neutral sun, short shadows; keep the canopy shade under the maples readable
  (not black), ambient ≥ 0.35 of the sun.
- **Sunset:** sun low in the west down Maple Street (backlit maples, orange rim on the cars),
  warmer grade; porch lights on at 20 %.
- **Night:** cool moonlight fill, the 10 sodium street lamps carry the road (warm pools,
  ≈ 60 000 lm equivalent, spot 110°), porch lights and garden lights warm; one hero shadow
  caster at most (the lamp over the start).
- Ground materials: Asphalt mid-grey with crack decals, Stone light concrete slabs, Grass
  autumn green with leaf litter; wet: darker and glossier, leaves stay matte.

## Verified in game

`cargo run -p recharged -- --user-data-dir target/agent-data/T16-street --unlock-all
--quick-race maple_street [--reverse] [--mirror] [--time Night] --autopilot --exit-after 80`:
forward and reverse laps by day and at night (28 lamp lights), the SUV tunnel, the cul-de-sac
apron, both plank/jump ends, the curb hop under the parked SUV, the storm-drain dip with the
Golden Battery in its inlet (forward only; rc_render's outdoor inner floor sits under the
dish).

## Known issues

- Barriers (the corner lot's wall, `wall_left`) are drawn by rc_render as kerb-stone blocks
  1.2 u tall; a low garden wall/hedge style would suit better.
- Props are untextured PBR (plywood, siding, bark read as flat colours up close).
- Maple canopies are lumpy icosphere clusters (stylised); leaf cards would need alpha.
- Reverse: the left turn right after climbing the curb hop is still the slowest corner (R 11,
  widened from 9 in T19a: the autopilot scrubbed to ~7 km/h on the planting strip; forward it
  is the right turn into the hop).
