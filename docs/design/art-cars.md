# Car models — notes (T08)

Owner: art. Look and rules: `art-bible.md` §2. Binding geometry: `car-dimensions.md`.

## Regenerate / verify

```sh
tools/blender/build_cars.sh                      # all 8 cars + validation (~2 min)
tools/blender/build_cars.sh --preview /tmp/prev sparky zip   # chosen cars + preview renders
blender -b -P tools/blender/cars/<id>.py -- [--preview DIR]  # one car
python3 tools/blender/check_cars.py              # validation only (no Blender needed)
```

Each generator writes `assets/models/cars/<id>.glb`, `<id>.wheels.ron` and the 512² transparent
thumbnail `assets/ui/cars/<id>.png`; `--preview DIR` adds EEVEE studio renders (hero ¾, side,
front, low chase camera, top) and a contact sheet `<id>_sheet.png`. Re-running a generator
reproduces the same model (see the triangle-order note below).

`check_cars.py` checks: glb parses; scene roots `body`, `wheel_fl/fr/rl/rr` (+ `antenna` under
`body`); wheel nodes translation-only at the table layout (±0.01 u) and equal to `wheels.ron`;
wheel mesh radius/width; required materials; ≤ 25 k triangles; body length/width/height and
lowest point vs the envelope; thumbnail 512² RGBA.

## Structure (all cars)

- Scene roots `body`, `wheel_fl`, `wheel_fr`, `wheel_rl`, `wheel_rr`; `antenna` is a child of
  `body` with its pivot at the antenna base (for sway). Wheels have identity rotation (spin axis
  = local X); left wheels are mirrored meshes so rims face outward. Tyre tops are exactly at
  the wheel radius, tyre width = wheel width (sidewall bulge adds ≤ 4 %).
- Materials (glTF names): `paint`, `paint_accent`, `decal`, `light_head`, `light_tail`,
  `light_brake`, `glass`, `rubber`, `rim`, `plastic`, `metal`, `chrome`, and `glow` on Volt GT and
  Bolt. Emissive "on" levels per art bible §1.2 (head 6, tail 2.5, brake 10, glow 5).
- Wheels take 40–47 % of each car's triangles (tyre ≈ 1.6–2.0 k incl. tread, rim ≈ 0.7–0.9 k).
- Positions + normals only (no UVs/textures). Stripes, numbers, windows and lamps are
  conformal geometry decals.
- Enclosed bodies are section lofts (`lib/shell.py`) with real wheel wells; fenders lift
  automatically to clear each arch. Width/height follow the body envelope, wheels excluded
  (overall width = track + wheel width).

## Cars

| Car | Look / signature | Default paint / accent | Rims, tyres | Number | Triangles |
|---|---|---|---|---|---|
| sparky | buggy: slim cowl, yellow roll cage with 4-lamp roof bar, driver, exposed motor and gearbox, big rear wing, long arms + gold coil-overs | electric blue `#1E5EE6` / yellow `#F2C318` | white 6-spoke, knobby pins | 7 | 23 476 |
| dumpster | monster truck: pickup cab + dumpster-bin bed (black lid, hazard chevrons), roof light bar, chrome bull bar + stacks, portal axles, 4-links, coil-overs | dumpster green `#257A34` / hazard yellow `#F2BE12` | chrome 5-spoke beadlock, chevron lugs | 13 | 24 121 |
| zip | mini touring sedan: box flares, white twin stripes, wing on stands, splitter, diffuser, black tail panel | turquoise `#12B5CF` / white `#F4F4F2` | silver 8-spoke, street | 3 | 22 673 |
| torque | muscle fastback: haunches over fat rear tyres, chrome supercharger + scoop, quad lamps, chrome bumpers, ducktail, twin pipes | candy red `#C8161E` / white `#F4F4F0` | chrome 5-spoke, white-letter street tyres | 88 | 24 610 |
| hornet | rally hatch: black roof + bonnet stripe, roof scoop, roof spoiler, white mud flaps behind every wheel, 4 rally lamps, skid plate | hornet yellow `#EAA80A` / black `#1D1D20` | white 8-spoke, gravel blocks | 5 | 24 295 |
| phantom | low GT coupe: wide haunches, full-width dual-element wing on swan necks with tall endplates, canards, diffuser, slim LED lamps | violet metallic `#3F1F80` / carbon black `#1E1E22` | gold 6-spoke, street | 9 | 22 243 |
| volt_gt | hypercar: teardrop canopy with black roof, side intakes, louvred engine cover and haunches, cyan `glow` DRL band, rocker strips, rear band, cover lines | pearl white `#E9EDF0` / black `#17181B`, glow cyan | graphite turbine, street | 1 | 22 405 |
| bolt | futuristic formula: needle nose, canopy + airbox, sidepods, shark fin with glow edge, wide front wing, twin-fin rear wing, exposed wishbones | hot magenta `#E01E78` / black `#17181B`, glow cyan | black turbine, slicks with glowing sidewall rings | 0 | 18 954 |

Suggested 4 base paints per car (base / accent) for `catalog.ron`:

| Car | 1 (default) | 2 | 3 | 4 |
|---|---|---|---|---|
| sparky | `#1E5EE6` / `#F2C318` | `#D8262B` / `#F2F2F0` | `#7BC62D` / `#1D1D20` | `#F07A1F` / `#1E5EE6` |
| dumpster | `#257A34` / `#F2BE12` | `#F07A1F` / `#1D1D20` | `#6A3FB5` / `#9BE22E` | `#8E959C` / `#D8262B` |
| zip | `#12B5CF` / `#F4F4F2` | `#F2F2F0` / `#D8262B` | `#9BE22E` / `#1D1D20` | `#F25C9A` / `#F4F4F2` |
| torque | `#C8161E` / `#F4F4F0` | `#1D1D20` / `#F07A1F` | `#2347B8` / `#F4F4F0` | `#E0A21A` / `#1D1D20` |
| hornet | `#EAA80A` / `#1D1D20` | `#F2F2F0` / `#2347B8` | `#C8161E` / `#F2F2F0` | `#14868C` / `#F07A1F` |
| phantom | `#3F1F80` / `#1E1E22` | `#B9BEC4` / `#C8161E` | `#14245C` / `#D4A63C` | `#2E5E3A` / `#1E1E22` |
| volt_gt | `#E9EDF0` / `#17181B` | `#17181B` / `#E9EDF0` | `#1F5FE0` / `#17181B` | `#B9BEC4` / `#17181B` |
| bolt | `#E01E78` / `#17181B` | `#F07A1F` / `#17181B` | `#F2F2F0` / `#1F5FE0` | `#9BE22E` / `#17181B` |

## Deviations and notes

- **Hornet's mud flaps** hang below the 0.16 chassis clearance (to 0.10). They are the car's
  signature, flexible, and not part of the chassis; the validator whitelists them.
- **Volt GT** rear tyre tops reach the 0.46 body height (allowed by `car-dimensions.md`); they
  show through louvre slots on the haunches. Its fenders are capped at the body height.
- **Bolt**: open wheels rise above the 0.42 body height (allowed).
- **Phantom's wing** is capped by the 0.50 body height, so from the low chase camera it reads as
  a full-width dark bar with corner endplates rather than a towering wing. Raising phantom's
  height to ~0.56 in `car-dimensions.md` would let it stand on taller swan necks.
- Arms, hub carriers and shocks are part of `body` (static); when the renderer moves wheels for
  suspension travel the arms do not follow (ends hide inside the rims).
- Bodies may exceed the envelope by ≤ 0.01 u where decals sit on the nose/tail
  (decal offset 0.0035 u).
- Re-running a generator reproduces the same geometry, materials and triangle sets; the glb
  bytes can still differ in the triangle order of a few primitives (Blender's internal face
  order after joining), so an unchanged car may show up as a binary diff.
- Previews are rendered with a neutral studio setup (Khronos PBR Neutral view, -0.5 EV, no
  bloom); in game the `light_*`/`glow` materials bloom and TonyMcMapface grades colours.
