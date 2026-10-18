# Backlog (small follow-ups, orchestrator-maintained)

Items found during integration that are not part of a running task's brief. Pick them up in the
QA/polish phase (T18) or when the owning area is touched next.

| # | Item | Area | Source |
|---|---|---|---|
| B1 | `--track-tour` camera flies through pickup volumes (translucent wash) — skip item rows in tour poses or hide pickups from capture cameras | rc_render capture / rc_vfx | T13a review |
| B2 | A human closing the window while pipelines compile can still hit the NVIDIA teardown crash — defer `AppExit` until the pipeline queue is idle | recharged | T13a diagnosis (`ponytail` in main.rs) |
| B3 | Track previews for the remaining tracks once their environments are final (`--capture-previews`) | rc_render capture | T13b |
| B4 | Unused `serde`/`ron` deps in `rc_weather` and `rc_audio` manifests (remove + `rc_pins` refresh) | manifests | T15, T09 |
| B5 | Lint for banked turns whose inner edge sinks below a floor (rule is documented in `rc_track.md` §A5) | rc_track | T16 toy |
| B6 | Human listening pass for SFX and music (only measured so far) | audio | T09, T10 |
| B7 | Upstream note: Bevy 0.20 atmosphere env-map probe panics every frame in our setup (worked around with a generated sky cubemap) | rc_render | T13a |
| B8 | Credits screen content: list tools, generators, the Saira font licence | ui | T05 |
| B9 | Barrier styles for house, workshop and supermarket use the plain sweep — themed barrier blocks like toy/street | rc_render track_visuals | T13b |
| B10 | Finished human keeps car control after crossing the line — hand to the AI (one `AiDriver` insert) | rc_race | T06 |
| B11 | Rockets never re-acquire a lock; explosions/EMP ignore walls; orb uses racing-width edges not barriers | rc_items | T12 |
