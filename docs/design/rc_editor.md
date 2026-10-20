# rc_editor — track editor (design note, proposal)

A separate executable for creating and editing tracks, test driving them, and baking their indirect
lighting. Status: **agreed scope, not started.**

## Decisions
| # | Decision | Consequence |
|---|---|---|
| D1 | **The editor is the source of truth** for track layouts | `tracks/<id>.ron` and `decor/<id>.ron` become editor-owned. The `plan.py` track sections are retired one track at a time (§Migration). Blender scripts keep building prop models and catalogs. |
| D2 | **UI in bevy-react** | Second bundle `ui/dist/editor.js` reusing `ui/src/components` and `theme`. TS types via ts-rs. |
| D3 | **Irradiance volumes only; no lightmaps** (maybe later) | Bake indirect diffuse into one `IrradianceVolume` per track variant × time of day. Direct light stays real-time. |
| D4 | **No geometry or draw-call "compile" step** | The QA bench is well inside budget (1P 185–204 fps, 4P ≈ 105 fps at High, RTX 3070). Revisit only if a measurement says so (e.g. Iris Xe). |

## Architecture
```
crates/recharged      lib: game app assembly (moved out of main.rs) ─┬─ bin recharged (game, as today)
                                                                      └─ bin rc_editor  (+ EditorPlugin)
crates/rc_editor      EditorPlugin: editing session, tools, undo, save, test drive, bake
ui/src/editor/        React editor UI → ui/dist/editor.js
```
- **The game, unchanged, renders the edited track.** Editing runs a normal track session
  (`Loading` → `InRace`) with no racers, the race clock held, and an editor fly camera with an
  orthographic top-down toggle. The room shell, props, decor, ground, lighting and weather all
  come from the existing plugins, so nothing is duplicated.
- **In-memory overrides.** `rc_track` and rc_render's decor loader read an optional
  `TrackDefOverride` / `DecorOverride` resource before falling back to the file. The editor sets
  them, so test drives and rebuilds use unsaved edits. Disk is written only on save.
- **Rebuild on commit, preview while dragging.** A finished edit (mouse release, field change)
  reloads the session: Unloading → Loading takes about 0.25 s. While dragging, the editor draws
  the centreline and edges with gizmos from rc_track's pure spline code.
- **Test drive:** F5 puts the player's car on the grid (the `--quick-race` path with the override)
  in the chosen variant, time and weather. Esc returns to editing at the same camera position.
- **Undo:** a stack of `(TrackDef, DecorDef)` snapshots, one per committed edit. The defs are
  small, so no command pattern.

## Editing scope ("all track properties")
| Area | Interaction |
|---|---|
| Control points | select, move (3D handles and top-down), insert or delete; panel: width, bank, surface, walls, offroad margin, optional **`label`** (new field: keeps `plan.py`'s `// 2: T1` notes after the switch) |
| Track settings | style, start, kill height, wall height, bounds |
| Ground polygons | vertex edit, surface |
| Props / decor | place from the `PropCatalog` list (filtered by environment + common), move, yaw, `dir` |
| Gaps, shortcuts, item rows, golden battery | handles on the centreline / in the world + panels, `dir` |
| Catalog entry | `TrackInfo` for the track (name, environment, difficulty, allowed weather and times, hidden) |
| Lints | `Track::lints()` + rc_track analysis, live in a panel. Click to focus the problem. |

**New track:** a dialog asks for id, name, environment and style. It writes a minimal oval (like
`test_oval`), an empty decor file and a `hidden: true` catalog entry.

**Saving** uses pretty RON in the existing field order, with `implicit_some`. Comments are lost on
the first save, which is why `label` exists.

## Migration off `plan.py` (D1)
1. Port `plan.py`'s corridor check (no static obstacle in the racing corridor) to `rc_track::analysis`
   as a lint, so every track gets it, not just generated ones.
2. Per track, when it is next touched: open it in the editor and move the comment labels into
   `label`. Save, then delete the layout and decor writer from that environment's `plan.py`.
   Remove the `GENERATED` header.

## Irradiance bake (D3)
**Constraints found in the code:**
- Binding arrays are off on every adapter (upstream-notes U7, the Intel Mesa hang). Without them,
  Bevy uses **only the irradiance volume closest to the view**, so we use **one volume per track**
  covering the drivable bounds.
- Shader priority is lightmap > irradiance volume > environment map for *diffuse*. Inside the
  volume, the env map's diffuse is skipped, but its specular stays. So the bake must include sky
  light, which also gives sky occlusion under tables and shelves.
- The sun and window light are **not mirrored** in the mirror variant. Flipping a volume would put
  the bounce on the wrong side, so mirror gets its own bake. Reverse shares the forward bake (only
  `dir`-filtered features differ).
- That gives **8 volumes per track**: {normal, mirror} × {Morning, Noon, Sunset, Night}.

**Baker** (an editor command plus a headless `rc_editor --bake <track>`), using the game renderer
with no custom ray tracer:
1. Load the variant at the time of day, Clear weather, no cars, dynamic props at rest.
2. Probe grid: track bounds horizontally, from the floor to the highest drivable point + 6 u. 2 u
   spacing by default (the car is 2 u long), with a per-track override.
3. Per probe: render a small (16²) HDR cubemap with exposure fixed and no tonemapping (scene light
   units). Convolve it to the 6 ambient-cube values (cosine-weighted irradiance per axis).
4. Probes inside solid geometry (mostly back faces seen) are marked invalid and filled from valid
   neighbours, so solid props don't leak darkness.
5. Optional second pass with pass 1's volume active gives a second bounce.
6. Pack the result into Bevy's layout: a 3D texture of (Rx, 2Ry, 3Rz), RGB9E5.

**Files:** `assets/data/lighting/<track>/<normal|mirror>_<time>.irv`, a raw header (dims, bounds)
plus RGB9E5 texels. It is read by rc_render into an `Image`, so no ktx2 feature is needed. At 2 u
spacing Aisle Rush is about 104×6×85 probes, ≈ 1.3 MB per volume and ≈ 10 MB per track.

**Runtime (rc_render):**
- If the file exists, spawn the `LightProbe + IrradianceVolume` scaled to its bounds.
- Weather scales `intensity` by the look's sky/key factor (Cloudy, Rain, Storm, Fog).
- Lightning and headlights stay real-time only.
- With no file, lighting is the same as today, so tracks can be baked one at a time.

## Phases
| Phase | Delivers | Done when |
|---|---|---|
| P1 Shell | shared app assembly, `rc_editor` binary, editor bundle, editing session + fly cam, overrides, open/save, F5/Esc | open Toy Town, save with no edits → identical file except comments; drive and return |
| P2 Tools | everything in "Editing scope", undo, lints, new-track dialog, `label` | build a new track from scratch and race it in the game |
| P3 Migration | corridor lint; first track moved off `plan.py` | `plan.py` no longer writes that track; CI green |
| P4 Bake | baker, `.irv` loader, weather scaling | Sofa Circuit baked; visible occlusion under furniture; bench within budget on the 3070 **and** runs on the Iris Xe |

## Risks / to verify first
- **A session with zero racers:** check rc_race, rc_camera, rc_ai and the HUD tolerate it. The
  fallback is one parked, hidden racer.
- **bevy-react with two bundles from one `ui/`:** check that `bevy-react build` supports a second
  entry. Otherwise use a second package dir.
- **Bake time:** ~50 k probes × 6 views. Bevy renders shadow cascades per view, so measure on one
  track before committing to 2 u spacing. The budget is ≤ 10 min per track (8 volumes) on the
  3070.
- **The Iris Xe path:** a single irradiance volume without binding arrays has not been tried there
  (see upstream-notes U7).
- **Repo size:** ≈ 10 MB of volumes per track.
