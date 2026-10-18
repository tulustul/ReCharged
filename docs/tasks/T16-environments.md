# T16 — Environments & tracks (one agent per environment)

**Goal:** turn one environment of GDD §7.1 into a finished, beautiful, raceable place: its prop kit
and hero props (Blender), its track layout (Forward + Reverse, mirror-safe), and its prop catalog —
following the art bible.

**Read first:** PRACTICES (§1, §2, §7 especially); GDD §4.2 (scale), §5.2 (surfaces), §6.1 (pickups),
§7 (tracks — your track's row, allowed presets, lap length, default laps), §9 (Golden Battery),
§17; ARCHITECTURE §5.1, §7; **`docs/design/rc_track.md` (the authoring guide — binding)**;
**`docs/design/art-bible.md` §3 (shared rules + your environment's section — binding)**;
`docs/design/art-cars.md` and `tools/blender/lib/` (the shared Blender library the car artist built:
reuse it); an existing layout for reference: `assets/data/tracks/toy_town.ron` + its plots.

## Owned paths (replace `<env>` / `<track>`)
`tools/blender/props/<env>/**`, `assets/models/props/<env>/**`, `assets/data/props/<env>.ron`,
`assets/data/tracks/<track>.ron`, `assets/data/decor/<track>.ron` (physics-free dressing — format at
the top of `crates/rc_render/src/props/decor.rs`; anything a car can touch stays in the prop catalog),
`docs/design/env-<env>.md`. You may **add new modules** under
`tools/blender/lib/` named **`<env>_*.py`** (prefix with your env id so parallel agents never collide;
never edit or overwrite existing modules — other agents use them; check a file doesn't exist before
writing it). `assets/data/props/common.ron`
belongs to the orchestrator: request additions. Environment lighting files
(`assets/data/environments/<env>.ron`) belong to the rendering agent until further notice — put
lighting wishes in your report.

## Deliverables
1. **Props** (`tools/blender/props/<env>/<prop>.py` → `assets/models/props/<env>/<prop>.glb`):
   the hero props (≤ 6) and kit (~15) listed in the art bible for your environment, real size ×
   4.44, art-bible bevels, materials and palette, budgets (kit ≤ 5 k tris, hero ≤ 15 k), origin at
   the ground contact, front toward Blender +Y. Large hero props may be split into pieces when
   drivable surfaces are involved. Render previews and inspect them; iterate until they look like
   giant, real, lovingly made objects.
2. **Prop catalog** `assets/data/props/<env>.ron`: every prop with model path, collider primitives
   (inside the visual mesh; drivable tops get a `Surface`), static/dynamic (+ mass; ≤ 40 dynamic
   bodies per track, light enough that cars barely deflect), light attachments for lamps. Kinds
   prefixed by environment (`<env>_…`), per the guide.
3. **Track layout** `assets/data/tracks/<track>.ron` per GDD §7.1/§7.2 and the authoring guide:
   target lap length and default laps, racing width ≥ 8 u (chokepoints ≥ 5 u, ≤ 2 per lap), the
   signature features listed for your track, jumps/ramps that work in their direction (`dir`),
   4–6 item rows, ≥ 8 checkpoints, a Forward-only Golden Battery on a risky line, ground polygons for
   open floors/lawns, walls/barriers in the environment's style, props placed and dressed so the
   place reads as the real environment at toy scale. **Lint-free in all four variants** (plot tool).
4. **Validation**: plots (top + side) for all four variants inspected; when the AI exists
   (`docs/design/rc_ai.md`), the headless AI 3-lap test passes on both directions; in-game
   screenshots once prop visuals are wired (`--quick-race <track> [--reverse]
   --user-data-dir target/agent-data/T16-<env> --screenshot-at … --exit-after …`) — ask the
   orchestrator if visuals aren't ready yet and continue with props/layout meanwhile.
5. `docs/design/env-<env>.md`: prop list with triangle counts, layout notes (features per
   direction, golden battery location, intended lines), lighting wishes per time of day, known issues.

## Acceptance
All props validated (re-import check like `tools/blender/check_cars.py`), previews inspected; catalog
and track files load (`cargo test -p rc_track` covers every track file); lint-free plots in all
variants; report with preview/plot paths.

## Environments (one agent each)
Props directory ids below are independent of the catalog's `TrackInfo.environment` ids (which key
the lighting files `assets/data/environments/<id>.ron`): toy_town → `toy_store`, sofa_circuit →
`house`, maple_street → `street`, backyard_bash → `garden`, aisle_rush → `supermarket`,
wrench_works → `workshop`.
| Wave | Env id | Track id | Notes |
|---|---|---|---|
| 1 | `toy` | `toy_town` | layout exists (T02) — dress it, replace placeholder props, refine the layout if needed |
| 1 | `house` | `sofa_circuit` | ~400 u, 5 laps; furniture verticality (book-stack ramp onto the coffee table, under the sofa, chair-leg slalom, stair ramp) |
| 1 | `street` | `maple_street` | ~900 u, 3 laps, outdoor; drive under parked giant cars, curbs, lawn shortcut, storm drain |
| 2 | `garden` | `backyard_bash` | ~450 u, 5 laps, outdoor |
| 2 | `supermarket` | `aisle_rush` | ~800 u, 3 laps; icy freezer aisle |
| 2 | `workshop` | `wrench_works` | ~700 u, 3 laps; lifts, oil stains, wash bay |
