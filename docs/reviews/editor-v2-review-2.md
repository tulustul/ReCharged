# Editor v2 (P5–P10) — design review 2 (delta)
Verdict: **APPROVED WITH CHANGES.** P5 and P6 may start now. Before P7 and P9 start, the doc must
take in findings 1 and 2: a few sentences each, with no design change.

Scope: the amendments for review 1's findings 1–14, and anything they broke. I read D6, D10, the
Interaction table and notes, Placement mode, Data changes, Incremental rebuilds, Settings, the
P6–P10 rows and Risks, and checked them against the code at `2cf223f`.

## Status of review 1

| # | Status | Notes |
|---|---|---|
| 1 Track key | resolved | def minus props/rows/battery/labels + `bounds()`; `geometry_hash` kept for records only |
| 2 Source indices, unreachable items | resolved | `PlacedProp::source`, decor index, dimmed markers; test (a) covers a `dir: Rev` prop |
| 3 Single writer | resolved, with new findings 1 and 4 | the drafts carry the preview, and the game crates move their own tagged entities |
| 4 `TrackPreview` | resolved, with new finding 4 | generation check, failure path, `TrackSummary` re-insert |
| 5 Once-per-session state | resolved | `RoomPiece` rebuild; the user's reload list. See new finding 3 for how a reload gets triggered |
| 6 Mirrored lights | resolved, with new finding 5 | — |
| 7 Placement mode | resolved, with new finding 6 | — |
| 8 Keys | resolved | G/R/C: no letter clashes with WASDQE (`view.rs:340-342`); Ctrl-up rule, cancel-first rule, P listed |
| 9 Equivalence tests | resolved | split into (a) rc_track headless ECS and (b) rc_render pure plan |
| 10 Settings | resolved | `read_settings` inserted before Boot (`boot_saves` skips, `flow.rs:132`); `--user-data-dir` covered |
| 11–14 | resolved | — |

## Findings

**1. major: putting the live preview inside the drafts makes things rebuild every frame of a
drag, and sun/fill drags have no preview path at all.**
- `TrackDraft.preview` / `EnvironmentDraft.preview` (`rc_editor.md:241`) mark each draft as
  changed every frame of a drag. Every system that diffs on a changed draft then re-runs its diff
  each frame. That covers rc_track, decor, and rc_render's env copy.
- The env copy matters most. Once rc_render copies the draft into `ActiveEnvironment.def`, that
  resource is changed, so `make_env_map` regenerates the 128² cubemap on the CPU every frame
  (`lighting/room.rs:17,147`) and re-uploads it.
- Sun and fill are not entities with a `Source`, so `Option<(Source, Transform)>` cannot express
  them. `apply_lights` re-poses the key and fill lights from `Look` every frame
  (`lighting/mod.rs:331-353`), so a pose written onto those light entities would be overwritten
  anyway. The doc does not say how the sun rotates live.

*Fix:*
- Move the preview out of the drafts into separate editor-written resources:
  `rc_track::DragPreview(Option<(Source, Transform)>)` and
  `rc_render::EnvPreview(Option<EnvPreviewKind>)`, where the kind is a light pose or a
  sun/fill (elevation, azimuth).
- The drafts then change only on commit.
- rc_render applies a sun/fill preview to the `EnvLight::Key`/`Fill` transforms after
  `apply_lights`. The env map is regenerated once, on release.
- Copy the draft into `ActiveEnvironment.def` only when the def actually differs.

**2. major: P6 ships gizmo drags with no live feedback.** P6 says "Commits still reload (P4
path)" (`rc_editor.md:389`), but the preview poses only arrive in P7. The P4 path only draws the
dragged handle's sphere (`view.rs:378-434`). In P6, a decor rotate or scale, or a prop move,
would therefore show nothing until release, then a ≈ 0.7 s reload. Reload-per-commit is
otherwise fine for P6: selection is by document index, the camera pose survives, and the tint
clones die with their entities.

*Fix (lazy):* move "preview poses applied by rc_track/rc_render" (finding 1's resources) from P7
into P6. It does not depend on the diff work, and P6's "move/rotate/scale" done-when needs it.
P7 keeps `TrackPreview`, the diffs and the tests. Also move the "a prop drag is real-time"
done-when item from P7 to P6.

**3. minor: the doc does not say how an edit decides between an in-place update and a reload.**
- Today every accepted change bumps `wanted`, and `reload` turns `built != wanted` into
  `ChangeTrack` (`lib.rs:272-273,600-601`).
- From P7 that must happen only for edits on the reload list. Undo and redo across such an edit
  must reload too.
- The Architecture note "Generation counters (`wanted`/`built`) stay" (`rc_editor.md:70`) now
  misleads.

*Fix:* one sentence. After any accepted document change (edit, undo or redo), compare the
reload-list fields (`practicals`, `fixture_materials`, `shadow_distance`, `sky`, and the catalog's
`environment` / `indoor` / `music`, plus time and weather fallback). If they differ, bump
`wanted`, which reloads; otherwise the change applies in place.

**4. minor: two placement-of-definition gaps.**
- `TrackPreview` is called "editor-written" but rc_render reads it. It has to live in rc_track,
  next to `TrackDraft`. The same applies to finding 1's resources: rc_track and rc_render.
- The doc says the editor keeps ARCHITECTURE §3.3 ("only rc_vehicle and rapier write a rapier
  body's `Transform`"). But rc_track already writes dynamic prop bodies in `reset_props`
  (`spawn.rs:256-264`), and the preview adds a second such write.

*Fix:* say where each resource lives. Add a Requests line asking the orchestrator to amend
ARCHITECTURE §3.3 so it names rc_track (prop reset and editor preview, both with `Teleported`).

**5. minor: there is nothing to re-bake for the mirrored-lights fix.**
- The only mirror volumes are Aisle Rush's (untracked; `git ls-files` shows only `normal_*.irv`).
- Aisle Rush's `environment.ron` has no point or spot lights. Toy Town, the only map with them,
  has no mirror volume.
- So "Mirror volumes baked before the fix are re-baked" (`rc_editor.md:214`) and the P9 row
  describe work that does not exist.

*Fix:* say "none today; any mirror bake of a map with environment lights must come after the
fix", and drop the re-bake from P9.

**6. minor: placement-mode edge cases.**
- Only the left and right panes count as UI (`rc_editor.md:188-191`). A release over the toolbar
  or the error toast would place an item under them.
- "A second click on a pane cancels" also catches a click on another entry (the user wants to
  switch assets), on the search box, or on a pane scrollbar.
- While armed, the press of the placing click in the view must not also run the normal
  select/clear pick.
- The ghost must load its glb through rc_render's `load_scene` (`content.rs:124`, currently
  `pub(crate)`). The same file requested with different loader settings conflicts ("Every glb
  load goes through here").

*Fix:*
- Count the toolbar and toast rects as UI too, or test "any `ReactInteraction` node under the
  cursor" via the `HoverMap`.
- A click on another entry re-arms with that asset; a click elsewhere in a pane cancels.
- Armed suppresses picking.
- Make `rc_render::load_scene` public for the ghost.
