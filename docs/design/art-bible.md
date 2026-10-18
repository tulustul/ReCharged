# ReCharged — Art bible

Owner: art (T08). Binding for every model, prop, texture and material in `assets/`. Read with
GDD §1, §4, §7.1, §17 and PRACTICES §7. Car-specific notes live in `art-cars.md`.

---

## 1. Global look — "stylised realism at toy scale"

- **The world is real, the cars are toys.** Environments are believable everyday places at true
  scale (1 u = 22.5 cm, real metres × 4.44); cars are premium 1:10 RC toys: chunky, glossy,
  oversized wheels, every part readable.
- **Shapes over detail.** Big, simple primary forms; generous bevels that catch highlights;
  secondary detail (vents, bolts, panel lines) only where the chase camera can see it. No detail
  smaller than **0.015 u** on cars or **0.05 u** on props (it shimmers at gameplay distance).
- **Saturated toy colours on the cars, grounded colours in the world.** Cars own the high-chroma
  end of the palette; environments stay one step less saturated and one step darker in value so
  cars always pop (GDD §17 readability). Environment accents may be saturated in small areas
  (toys, packaging, signage).
- **Light does the work.** Rich bounce light, soft SSAO contact shadows, bloom only on
  emissives, filmic tonemapping (TonyMcMapface). Every environment has one dominant warm/cool
  light contrast (window vs. lamp, sun vs. sky).
- **Everything is a PBR material** (metallic-roughness). No baked lighting in albedo, no painted
  highlights, no black outlines.

### 1.1 PBR value ranges

Albedo values are sRGB 0–255. Non-metal albedo stays within **30–240** (charcoal → fresh snow);
fully saturated colours keep their brightest channel ≤ 235 so they never clip under sunlight.

| Material class | Albedo (sRGB) | Metallic | Roughness | Clearcoat / coat rough. | Notes |
|---|---|---|---|---|---|
| Car paint, solid | saturated hue, value 120–235 | 0 | 0.30–0.40 | 1.0 / 0.03–0.06 | the "toy gloss"; renderer recolours `paint` |
| Car paint, metallic / pearl | value 90–220 | 0.35–0.6 | 0.32–0.45 | 1.0 / 0.04–0.08 | Elite cars |
| Accent paint | contrasting hue or white/black | as paint | as paint | 1.0 / 0.05 | stripes, roof, wings |
| Decal print (numbers, roundels) | white 235 / black 25 | 0 | 0.35 | 1.0 / 0.05 | geometry decals |
| Satin plastic (chassis, arms, trims) | 22–60 | 0 | 0.45–0.60 | — | the "black RC plastic" |
| Glossy toy plastic (bricks, toys) | saturated, value 140–235 | 0 | 0.15–0.30 | optional 0.3 | Toy Town props |
| Rubber (tyres) | 26–40 | 0 | 0.80–0.92 | — | never pure black |
| Anodised aluminium | tinted, value 120–230 | 1 | 0.25–0.40 | — | shocks, hubs: RC colour accents |
| Bare / brushed metal | 150–210 | 1 | 0.30–0.55 | — | tools, shelves, lifts |
| Chrome | 245–250 | 1 | 0.03–0.10 | — | bumpers, exhausts, nuts — small areas |
| Car glass (opaque tinted) | 12–24, slight blue | 0 | 0.03–0.08 | 1.0 / 0.02 | opaque: no transparency sorting |
| Real glass (environment) | — | 0 | 0.02–0.05 | — | `AlphaMode::Blend`, use sparingly |
| Wood, varnished | 90–200 | 0 | 0.30–0.50 | — | floors, furniture |
| Wood, raw / plywood | 140–210 | 0 | 0.60–0.80 | — | ramps, crates |
| Fabric / carpet | any | 0 | 0.90–1.00 | — | no specular highlights |
| Concrete / asphalt | 45–140 | 0 | 0.80–0.95 | — | wet: roughness × 0.35, albedo × 0.7 |
| Glazed tile / vinyl | 120–235 | 0 | 0.10–0.35 | — | supermarket, kitchen |
| Grass / soil | 40–120 | 0 | 0.70–1.00 | — | |
| Cardboard / paper | 150–200 | 0 | 0.85–0.95 | — | |

### 1.2 Emissive levels (HDR + bloom)

Bevy treats `emissive` as linear luminance **not** scaled by exposure (`emissive_exposure_weight`
= 0, the default). A sunlit white diffuse surface lands near **1.0** after `Exposure::SUNLIGHT`,
so emissive values read directly as "times brighter than a sunlit white wall". glTF stores
`emissiveFactor × KHR_materials_emissive_strength` (Blender: *Emission Color × Strength*).

| Use | Colour (linear) | Strength | Look |
|---|---|---|---|
| Headlight lens (on) | 1.00, 0.92, 0.78 | 6 | white-hot core, soft bloom |
| Tail light (running) | 1.00, 0.06, 0.04 | 2.5 | readable red, little bloom |
| Brake light (on) | 1.00, 0.05, 0.03 | 10 | strong red bloom; renderer idles it at **0.2×** |
| Glow accent strips `glow` (Volt GT, Bolt) | 0.10, 0.85, 1.00 | 5 | electric cyan |
| Indicator / warning | 1.00, 0.45, 0.05 | 4 | amber |
| Environment lamps (bulb / tube) | warm 1.00, 0.80, 0.55 | 8–20 | add a real light next to it |
| Screens, signage, displays | any | 1.5–3 | must not bloom much |
| Pickups / items (VFX owners) | charge colours | 4–12 | brighter = more important |

- Everything above **~1.5** blooms; keep non-hero emissives below that. Never put clearcoat on an
  emissive material (Bevy darkens emission by the coat Fresnel).
- Car glb files store the **"on"** values; `rc_render` scales them (brake idle 0.2×, headlights
  dimmed by day if wanted, ghosts/respawn translucency).

---

## 2. Car style guide

### 2.1 Proportions and silhouette
- Wheels are the star: diameter **45–90 % of the body height**, track as wide as the body,
  wheels flush with or proud of the bodywork by up to 0.07 u. Bodies are slim shells that hug the
  wheels with flared arches.
- Each car must be identifiable **in silhouette from the chase camera** (behind, ~20° down) and
  from the side: give every car one unmistakable feature (roll cage, monster tyres, wing, roof
  scoop, blower, fins, glow strips …) and keep it high and at the rear where the chase camera sees
  it.
- Dimensions and wheel layout are binding (`car-dimensions.md`); model to them exactly.

### 2.2 Construction and detail level
| Tier | What | Rule |
|---|---|---|
| Primary | shell, wheels, cage / wing / scoop | readable at 30 u, defines the silhouette |
| Secondary | bumpers, arches, lights, mirrors, splitters, suspension arms and shocks, exhausts | readable at 8 u |
| Tertiary | stripes, numbers, vents, nuts, panel lines | readable at 3 u; skip anything smaller than 0.015 u |

- **Bevels:** shell edges are built rounded (fillet radius 0.02–0.08 u, 3 segments). Hard-surface
  parts (bumpers, wings, plates, boxes) get an angle-limited bevel of **0.004–0.008 u**, 1–2
  segments (a chamfer reads as a round at gameplay distance and halves the cost), plus
  harden/weighted normals so flat faces stay flat and edges glint. Round parts (tubes,
  cylinders, revolves) get their rounding from the profile, never from a bevel modifier.
- **Suspension is visible:** arms, shock bodies (anodised `metal`), chrome shafts and springs
  where the body leaves them exposed.
- **Decals are geometry** (no textures needed): stripes, number roundels and digits are thin
  conformal meshes 0.002–0.003 u above the paint. If a texture is ever needed: ≤ 256², no real
  brands, logos or trademarks — invented sponsors only.
- **Racing number** on both sides (and the roof or hood where it fits): white roundel or panel,
  black digits in the in-house blocky stencil (made from rounded bars; no font files).
- **Lights:** separate lens meshes 0.003–0.006 u proud of the shell with a black or chrome bezel.
  Headlights `light_head`, tail lights `light_tail` and separate brake segments `light_brake`.
  Cars without real lights (buggy, formula) carry light pods or LED strips so all materials exist.
- **Whip antenna** on every car: a thin black tube tapering from radius 0.007 to 0.003 u, 0.35–0.6 u
  long, with an accent-coloured tip cap; separate child node `antenna` of `body` with its pivot at
  the base so the renderer can sway it.
- **Glass** is opaque, dark and glossy (like a painted RC shell): no interiors, no transparency.

### 2.3 Materials (glTF names are a contract)
| Name | Use | Recoloured at runtime |
|---|---|---|
| `paint` | main body colour | yes (paint choice) |
| `paint_accent` | stripes, accent panels, wing, roof | yes |
| `decal` | white roundels and print | no |
| `light_head`, `light_tail`, `light_brake` | lenses (emissive) | intensity only |
| `glow` | emissive accent strips (Volt GT, Bolt; optional elsewhere) — deliberately not `light_*`, so lights toggle separately | intensity only |
| `glass` | windows, canopies | no |
| `rubber` | tyres | no |
| `rim` | wheel rims (per-car colour) | optional |
| `plastic` | chassis, arms, trims, digits, grilles | no |
| `metal` | anodised suspension parts (per-car tint) | no |
| `chrome` | exhausts, shafts, nuts, bumpers | no |

### 2.4 Structure and budgets
- glb nodes: `body` (+ child `antenna`), `wheel_fl`, `wheel_fr`, `wheel_rl`, `wheel_rr`; wheel
  origins at the wheel centres with identity rotation (spin axis = local X); left wheels are
  mirrored copies so the rim face always points outward.
- Origin at the ground-contact centre, front toward Blender +Y, exported "+Y Up" (front = Bevy −Z).
- **≤ 25 k triangles** per car (the shipped cars use 19–24.6 k; the four wheels take 40–47 %).
  Positions and normals only — no UVs or textures on cars.
- Generators: `tools/blender/cars/<id>.py`; shared helpers in `tools/blender/lib/`.

---

## 3. Environments

### 3.1 Rules shared by all environments
- **Scale:** real metres × 4.44 (1 u = 22.5 cm). Floors may be up to 1.5× real area; furniture and
  props stay true scale (GDD §4.2).
- **Bevel widths** (real-object edge radius, exaggerated for readability):

  | Prop size (largest side) | Bevel | Segments |
  |---|---|---|
  | < 2 u (cans, toys, tools) | 0.01–0.02 u | 2 |
  | 2–10 u (chairs, boxes, tyres) | 0.02–0.05 u | 2–3 |
  | > 10 u (sofas, cars, shelves, walls) | 0.04–0.10 u | 3 |

- **Texture density:** hero ground (track surface around the racing line) **512 px/u**
  (a 1024² tile covers 2 × 2 u; ≤ 2048² for hero ground sets); props near the track **256 px/u**;
  background props and room shell **128 px/u**; skyline / far background ≤ 64 px/u. Textures
  ≤ 1024² otherwise; sRGB colour, linear normal/roughness/AO (PRACTICES §7).
- **Tiling:** break repetition with two-scale detail (macro variation 8–20 u) and decals (stains,
  leaves, dust) rather than larger textures.
- **Budgets per track** (GDD §17): ≤ 6 hero props (≤ 15 k tris each), a shared kit of ~15 props
  per environment (≤ 5 k tris each), ≤ 40 dynamic knockable bodies, ≤ 1.5 M visible triangles.
- **Readability:** the racing surface is mid-value; barriers have a contrasting value band at car
  height; nothing on the track uses a car paint hue at full saturation over a large area.
- **Lights:** every visible lamp has a matching real light; night time uses warm practicals and a
  cool moon/sky fill. Emissive levels per §1.2.

### 3.2 Toy Town — toy store / playroom (indoor ★)
- **Mood:** bright, cheerful, primary colours on a calm warm-white base; soft window light by
  day, warm ceiling spots at night.
- **Palette:** play-mat teal `#2E9C9A`, toy-track orange `#F07A1F`, block red `#D8392B`,
  block blue `#2F6FD6`, block yellow `#F2C230`, warm wall white `#EDE6DA`, beech wood `#C99A63`.
- **Materials:** glossy toy plastic (rough 0.2), orange toy track (rough 0.25, moulded ridges),
  felt play-mat with printed roads (carpet, rough 0.95, 512 px/u), beech wood floor and shelves
  (rough 0.4), cardboard packaging (rough 0.9), painted wall (rough 0.85).
- **Hero props (pick ≤ 6 per layout):** toy-track helix, dollhouse (drive-through), block tower
  set (knockable), giant toy-store shelf unit, play castle / fort, rocking horse.
- **Kit (~15):** wooden blocks (cube, arch, cylinder, triangle), stacking rings, toy-track
  straight / curve / ramp / banked piece, plush bear, toy boxes, crayon pack, ball pit balls,
  train wagon, toy robot, stacking cups, puzzle mat tiles, rubber duck, marble.
- **Barriers:** orange toy-track walls; foam block borders.
- **References:** wooden train sets and orange plastic race-track toys, foam play mats with printed roads, toy-store shelving, nursery lighting (warm, soft, big windows).

### 3.3 Sofa Circuit — family house ground floor (indoor ★)
- **Mood:** cosy lived-in home; warm afternoon window light, cool shadows; lamps at night.
- **Palette:** oak floor `#A9784A`, rug terracotta `#B5553C`, sofa sage `#7E9278`, cushion mustard
  `#D9A532`, kitchen tile cream `#E6DFCF`, wall greige `#CFC5B4`, book spines mixed muted.
- **Materials:** varnished oak boards (rough 0.35, 512 px/u, plank 0.67 u wide), wool rug (rough
  1.0), fabric sofa (rough 0.95), glazed kitchen tile (rough 0.2, grout darker), painted wood
  (rough 0.5), books (paper/cloth rough 0.8), ceramic mugs (rough 0.15).
- **Hero props:** sofa (tunnel underneath), coffee table, book-stack ramp, dining table with
  chairs, staircase, sideboard / TV unit.
- **Kit:** books (3 sizes), magazines, mug, remote control, cushion, slipper, plant pot, floor
  lamp, dining chair, rug edge, toy left on the floor, shoe, laundry basket, bowl, cable.
- **Barriers:** book rows, skirting boards, cushions.
- **References:** Scandinavian family living rooms, oak floors with afternoon sun streaks, low coffee tables, dining chairs from a toy's eye height (macro photography at 5–10 cm).

### 3.4 Backyard Bash — back garden (outdoor ★★)
- **Mood:** sunny suburban garden, saturated greens, warm terracotta, long sunset shadows;
  porch light and fairy lights at night.
- **Palette:** lawn green `#5E9A3A`, soil brown `#6B4A2E`, terracotta `#C0663C`, patio stone
  `#B9B1A3`, deck wood `#8A6142`, hose green `#3FA34D`, flower accents `#E84A6F` / `#F5C542`.
- **Materials:** grass (rough 0.8, blades as cards near the track), dirt (rough 0.95), patio
  stone slabs (rough 0.85), decking (rough 0.6), terracotta pots (rough 0.75), water puddles
  (rough 0.05, dark), sand (rough 0.95), garden-bench wood (rough 0.6).
- **Hero props:** garden bench, hose reel (spiral), sandbox, flower-pot cluster, shed corner,
  barbecue.
- **Kit:** flower pots (3), watering can, trowel, garden gnome, stepping stones, pebbles, leaves,
  fence panel, hose segment, bucket, football, plant clumps, toy spade, brick, tennis ball.
- **Barriers:** brick edging, pots, low fence boards.
- **References:** suburban back gardens in summer, macro photos of lawns and patio slabs, terracotta pot clusters, garden hose reels, golden-hour shadows.

### 3.5 Aisle Rush — supermarket after closing (indoor ★★)
- **Mood:** cold fluorescent light with half the tubes off, glowing freezer cases, deep shadows
  between aisles; sunset light from the entrance.
- **Palette:** vinyl floor off-white `#D9D6CC`, shelf grey `#8E959C`, freezer cyan glow `#7FD8F0`,
  price-tag yellow `#F4D03F`, packaging reds `#D7352B` / greens `#3E9B4F`, checkout belt black.
- **Materials:** polished vinyl tile (rough 0.15, strong reflections, 512 px/u), painted steel
  shelves (rough 0.4), frosted ice (rough 0.1 + white rime), cardboard boxes (rough 0.9), cans
  (brushed metal rough 0.35 with printed labels), glass freezer doors (blend), rubber belt.
- **Hero props:** freezer row, checkout lane, cardboard-box ramp, can pyramid (knockable),
  shelving gondola, shopping trolley.
- **Kit:** cans (3), cereal box, bottle, milk carton, crate, price sign, basket, pallet, wet-floor
  sign, mop bucket, cardboard box (2), fruit, bag, shelf divider.
- **Barriers:** shelf kick-plates, crates, wet-floor signs.
- **References:** supermarkets after closing (half-lit fluorescent rows, glowing freezer aisles), polished vinyl reflections, stacked cardboard pallets; no real brands on packaging.

### 3.6 Wrench Works — auto-repair workshop (indoor ★★★)
- **Mood:** gritty, warm sodium work lights against blue daylight from the roll-up door; oil
  stains; hazy paint booth.
- **Palette:** concrete `#8D8A84`, safety yellow `#F2B705`, tool-chest red `#B8322A`, lift blue
  `#2C5E9E`, rubber mat black `#2A2A2A`, plywood `#C8A46E`, oil sheen dark `#1C1A18`.
- **Materials:** stained concrete (rough 0.85, 512 px/u, oil decals rough 0.2), painted steel
  (rough 0.45), rubber mats (rough 0.9, ribbed), plywood (rough 0.7), chrome tools (rough 0.1),
  tyres (rubber), hazard tape (rough 0.6), wet wash-bay tiles (rough 0.1).
- **Hero props:** car on a lift (drive under), workbench with ramp, tyre stack chicane, tool
  chest, paint booth frame, wash bay.
- **Kit:** tyres (2 sizes), wrench, socket set, oil can, jerry can, jack, creeper board, traffic
  cone, bucket, spare wheel, rag, toolbox, battery, hose, pallet.
- **Barriers:** tyre walls, hazard-striped kerbs, rubber mats.
- **References:** independent auto-repair garages, two-post lifts, red tool chests, oil-stained concrete, roll-up doors with blue daylight, paint-booth haze.

### 3.7 Maple Street — suburban street & front lawns (outdoor ★★★)
- **Mood:** wide open suburban morning, maple reds and oranges, clean asphalt; sodium street
  lamps and porch lights at night.
- **Palette:** asphalt `#4A4C50`, lane paint white `#E8E6DF` / yellow `#E3B341`, sidewalk
  `#B7B2A8`, lawn `#5C8F3C`, maple red `#C2452D`, maple orange `#E07B2E`, house trims white.
- **Materials:** asphalt (rough 0.9, 512 px/u, crack decals), concrete sidewalk (rough 0.85),
  kerb stone (rough 0.8), lawn (rough 0.8), car paint for parked giant cars (as car paint but
  desaturated), rubber, metal storm drain grate (rough 0.5), leaves (decals).
- **Hero props:** parked giant car (drive under), storm drain, street lamp, mailbox and
  driveway, fire hydrant cluster, garden fence corner.
- **Kit:** kerb pieces, leaves pile, trash can, recycling bin, sign post, hydrant, mailbox,
  hedge block, lawn sprinkler, newspaper, bike, cone, manhole cover, garden light, maple tree.
- **Barriers:** kerbs, hedges, parked-car wheels.
- **References:** North-American suburban streets in autumn, maple leaves on asphalt, storm drains and kerbs photographed from ground level, sodium street lamps at night.
