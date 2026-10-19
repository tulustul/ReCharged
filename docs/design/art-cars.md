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
lowest point vs the envelope (hard-coded exceptions: hornet's flaps, phantom's wing to 0.56);
thumbnail 512² RGBA.

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
| dumpster | red/black short-bed pickup monster truck: tall boxy cab with window nets, black flares/rockers, angular black/silver slashes, ribbed bed cover, LED fascia + light-bar bumper, exposed A-arms with red coil-overs | Race Red `#BE1621` / charcoal `#1D1D20` | black 6-spoke rims with red ring, chevron tyres | — | 24 146 |
| zip | mini truggy: narrow lime wedge shell between the wheels, cab-forward canopy with black roof, black lower sides with white/grey swooshes and side scoop, red shocks/hubs/rear tower, tubular front bumper, big wing on stands | Lime `#8BD12A` / black `#1D1D20` | black dish rims with red nuts, pin-spike tyres | — | 24 589 |
| torque | rusty wasteland interceptor fastback: steel blower + twin scoops, spiked dozer blade, mesh-caged windows, riveted scrap plates, J stacks above the roof, boot fuel tanks, spiked ducktail, roof harpoon gun (to 0.78) | Rust Bucket `#3F2419` / charcoal `#2E2B28` | rusty steel 8-hole dish rims with spiked hubs, block tyres | — | 24 966 |
| hornet | lifted '30s three-window hot rod coupe: chrome shield grille, stalk headlamps, black pontoon fenders + running boards, tilted spare on the deck, chrome nerf bars, red coil-overs | Hot Rod Yellow `#EAA80A` / gloss black `#1D1D20` | grey steel 8-hole dish rims, block all-terrain tyres | 5 | 24 186 |
| phantom | widebody GT86 time-attack 2+2 coupe: long louvred black bonnet, fastback greenhouse, bolt-on box overfenders, splitter + canards, finned diffuser, swan-neck GT wing (to 0.64), black slash livery | Rally Blue `#1F5FE0` / gloss black `#1D1D20` | gunmetal 10-spoke rims, red calipers, street tyres | 13 | 24 232 |
| volt_gt | P1-style mid-engine hypercar: teardrop canopy into a black roof spine + snorkel, swept smile headlights, huge side scoops, skeletal mesh tail with C-shaped taillights, centre exhaust, finned diffuser, active wing on pylons | Volt Yellow `#F2B705` / carbon `#1E1E22` | grey 10-spoke rims, street tyres | 17 | 23 761 |
| bolt | modern F1: needle nose + 3-element front wing, halo + helmet, airbox, undercut black sidepods, shark fin, big rear wing on swan-neck pylon (to 0.56) over diffuser, red push-rod wishbones | Race Red `#D8262B` / carbon black `#1D1D20` | grey 10-spoke rims with red nuts, fat slicks | 1 | 20 464 |

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

- Concepts: `docs/concept-arts/` (zip = truggy, torque = interceptor, hornet = hot rod, phantom =
  widebody GT, volt_gt = hypercar, bolt = F1, dumpster = RC-X monster truck).
- **Volt GT** rear tyre tops reach the 0.46 body height (allowed by `car-dimensions.md`); they
  show through louvre slots on the haunches. Its fenders are capped at the body height.
- **Bolt**: open wheels rise above the 0.42 body height (allowed).
- **Phantom's wing** uses the `car-dimensions.md` footnote allowance: the body stays within
  0.50, only the dual-element wing on its swan necks rises to 0.56 (hard-coded as `WING_TOP` in
  `cars/phantom.py` and `check_cars.py`), so it floats clear of the deck from the chase camera.
- Arms, hub carriers and shocks are part of `body` (static); when the renderer moves wheels for
  suspension travel the arms do not follow (ends hide inside the rims).
- Bodies may exceed the envelope by ≤ 0.01 u where decals sit on the nose/tail
  (decal offset 0.0035 u).
- Re-running a generator reproduces the same geometry, materials and triangle sets; the glb
  bytes can still differ in the triangle order of a few primitives (Blender's internal face
  order after joining), so an unchanged car may show up as a binary diff.
- Previews are rendered with a neutral studio setup (Khronos PBR Neutral view, -0.5 EV, no
  bloom); in game the `light_*`/`glow` materials bloom and TonyMcMapface grades colours.
