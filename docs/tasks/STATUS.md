# Task status board (orchestrator)

| Task | Scope | Depends on | Status |
|---|---|---|---|
| R1 | Bevy 0.20 / rapier / kira API digests | — | done (`docs/tech/`) |
| R2 | bevy-react usage digest | — | done (`docs/tech/bevy-react-notes.md`) |
| GDD | GDD + architecture + practices, reviews 1–2 | R1, R2 | **approved** (v2.1) |
| T01 | foundation / walking skeleton | GDD | done (53 tests; `rc_pins` build-hygiene crate added) |
| T08 | art bible + 8 car models | GDD | done |
| T09a | SFX synthesis (phase 1 of T09) | GDD | done (73 files, `docs/design/sfx.md`) |
| T10 | music | GDD | done (12 cues, `docs/design/music.md`) |
| T02 | track system + test_oval + toy_town layout | T01 | done (39 tests) |
| T03 | vehicle physics | T01 | done (58 tests, stars ±1 %) |
| T04 | input + cameras (split-screen rig first) | T01 | done (rig B overlay; 67 tests) |
| T05 | UI framework + menus | T01 | done (all screens; HUD = T05b) |
| T07 | game flow, progression, persistence | T01 | done (54 tests) |
| T09b | rc_audio system (phase 2 of T09) | T01, T09a | done (in-race listen pending integration) |
| T06 | race rules | T02, T03 | done (27 tests) |
| T11 | AI drivers | T02, T03 | done (52 tests; every layout passes) |
| T12 | items | T02, T03 | done (40 tests) |
| T13 | graphics (render) | T02, T08 | done (T13a + T13b) |
| T14 | VFX | T03 | done (37 tests) |
| T15 | weather | T01 | done (12 tests) |
| T05b | race HUD | T05, T06 | done |
| T16 | environments & tracks (2 waves) | T02, art bible | all six done (toy, house, workshop, street, supermarket, garden) |
| T17 | automation tools (input script, headless race, determinism, medals, bench) | T06, T11 | done |
| T18 | QA playtests, soak, bench, DoD checks, bug triage | all | done (`docs/qa/report.md`: DoD 1 partial, 2 fail 1/24, 3–8 pass; Q1–Q22) |
| T19a | backlog fixes — gameplay (race, items, AI, camera, lint, street data) + QA Q4, Q6, Q7 | T18 parallel | done (every_layout 28/28, worst 1.43×; round 2: B32–B35 in progress) |
| T19b | backlog fixes — presentation, UI, infrastructure | T18 parallel | done |
| T20b | QA fixes — UI, flow, accessibility (Q1–Q3, Q5, Q10–Q18) | T18 | done |
| T20c | QA fixes — rendering, performance, memory, audio logs (Q8, Q9, Q19–Q21, B28–B31) | T18 | in progress |

Milestones: M1 walking skeleton (T01–T05 basics) → M2 vertical slice on Toy Town (+2P smoke test)
→ M3 systems complete → M4 content complete → M5 polish & QA.
