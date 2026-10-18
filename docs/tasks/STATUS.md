# Task status board (orchestrator)

| Task | Scope | Depends on | Status |
|---|---|---|---|
| R1 | Bevy 0.20 / rapier / kira API digests | — | done (`docs/tech/`) |
| R2 | bevy-react usage digest | — | done (`docs/tech/bevy-react-notes.md`) |
| GDD | GDD + architecture + practices, reviews 1–2 | R1, R2 | **approved** (v2.1) |
| T01 | foundation / walking skeleton | GDD | done (53 tests; `rc_pins` build-hygiene crate added) |
| T08 | art bible + 8 car models | GDD | done (phantom wing follow-up running) |
| T09a | SFX synthesis (phase 1 of T09) | GDD | done (73 files, `docs/design/sfx.md`) |
| T10 | music | GDD | done (12 cues, `docs/design/music.md`) |
| T02 | track system + test_oval + toy_town layout | T01 | in progress |
| T03 | vehicle physics | T01 | in progress |
| T04 | input + cameras (split-screen rig first) | T01 | in progress |
| T05 | UI framework + menus | T01 | in progress |
| T07 | game flow, progression, persistence | T01 | in progress |
| T09b | rc_audio system (phase 2 of T09) | T01, T09a | in progress |
| T06 | race rules | T02, T03 | todo |
| T11 | AI drivers | T02, T03 | todo |
| T12 | items | T02, T03 | todo |
| T13 | graphics (render) | T02, T08 | todo |
| T14 | VFX | T03 | todo |
| T15 | weather | T01 | done (12 tests) |
| T05b | race HUD | T05, T06 | todo |
| T16+ | environments & track layouts (2 waves) | T02, T13, art bible | todo |
| QA | playtests, soak, bench, DoD checks | all | todo |

Milestones: M1 walking skeleton (T01–T05 basics) → M2 vertical slice on Toy Town (+2P smoke test)
→ M3 systems complete → M4 content complete → M5 polish & QA.
