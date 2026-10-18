# T08 — Art bible & car models (Blender)

**Goal:** define the visual language of the game, then build the 8 RC cars — the hero assets the
player looks at all the time. Quality matters more than speed here.

**Read first:** PRACTICES (§7); GDD §1, §4 (cars), §7.1 (environments), §17 (visual direction);
ARCHITECTURE §7 (naming); `docs/design/car-dimensions.md` (binding geometry).

## Owned paths
`docs/design/art-bible.md`, `tools/blender/**`, `assets/models/cars/**`, `assets/ui/cars/**`,
`docs/design/art-cars.md`.

## Deliverables
1. **Art bible** (`docs/design/art-bible.md`, before modelling): the global look (stylised realism,
   saturated toy colours, beveled edges, PBR value ranges for paint/plastic/rubber/metal/chrome,
   emissive levels for HDR + bloom), the car style guide (proportions, detail level, decals by
   geometry or small textures, lights, antenna), and **per-environment palettes & material lists**
   for all six environments (Toy Town, Sofa Circuit, Backyard Bash, Aisle Rush, Wrench Works, Maple
   Street) incl. bevel widths, texture density (px/u) and prop-kit lists (≤ 6 hero props per track +
   ~15 kit props per environment) — the environment artists will follow it later.
2. **Shared Blender library** (`tools/blender/lib/`): scene reset, unit/axis conventions, PBR
   material helpers (named materials), bevel/weighted-normal helpers, parametric wheel (tyre with
   tread pattern variants, rim styles), suspension parts, light meshes, glTF export (+Y up, apply
   modifiers, metallic-roughness), preview renderer (Eevee, studio lighting, 4 angles + ¾ hero
   shot) and thumbnail renderer (transparent background, 512²).
3. **8 car generators** `tools/blender/cars/<id>.py` → `assets/models/cars/<id>.glb` +
   `<id>.wheels.ron` (format in `car-dimensions.md`) + `assets/ui/cars/<id>.png` thumbnail.
   Requirements: exact dimensions/wheel layout from `car-dimensions.md`; origin at the
   ground-contact centre; front toward Blender +Y; nodes `body`, `wheel_fl`, `wheel_fr`, `wheel_rl`,
   `wheel_rr` (wheel origins at wheel centres, spin axis = local X); materials `paint`,
   `paint_accent`, `light_head`, `light_tail`, `light_brake`, plus rubber/plastic/metal/glass/
   chrome; ≤ 25 k triangles; distinct, readable silhouettes matching GDD §4.1 body styles
   (buggy with roll cage & knobbly tyres, monster truck, mini touring, muscle, rally hatch with roof
   scoop & mud flaps, sports coupe with big wing, hypercar with glowing accent strips, futuristic
   open-wheel formula with fins); whip antenna; racing number via geometry or a small texture.
4. **Review loop**: render previews of every car, inspect them critically (proportions, bevels
   catching light, material separation, silhouette from the chase-camera angle) and iterate until
   they look like premium toy RC cars. Start with **sparky** and send its preview path to the
   orchestrator (SendMessage to `main`) before doing the other seven; continue unless told to stop.
5. `docs/design/art-cars.md`: per-car notes, triangle counts, how to regenerate.

## Acceptance
All 8 `.glb` files load (validate with a quick Blender re-import or `gltf` sanity script); wheel
metadata matches `car-dimensions.md` within 0.01 u; previews and thumbnails exist and look good;
art bible complete. Report preview image paths.
