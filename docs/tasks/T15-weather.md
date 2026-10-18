# T15 — Weather state (`rc_weather`)

**Goal:** a deterministic weather simulation that every other system reads from `WeatherState`.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §3.4 (RNG), §5.4; GDD §5.2 (weather grip),
§7.2 (allowed presets), §10.

## Owned paths
`crates/rc_weather/**`, `docs/design/rc_weather.md`. **rc_core grant:** additive edits in
`crates/rc_core/src/weather.rs` only.

## Deliverables
1. Plan from `RaceConfig.weather` + the track's indoor flag (`Catalog`): Fixed / Random (allowed
   presets) / Dynamic (one change at 30–70 % of race distance — from the leader's progress vs
   `TrackSummary.length × laps` — to a neighbouring preset, 20 s blend); indoor → `outside_only`
   (no wetness/snow).
2. Per-kind targets (precipitation, cloudiness, fog, wind) blended smoothly; wetness and snow cover
   accumulate (~20 s) and dry/melt slower; gusts in Storm; lightning schedule (Storm) →
   `WeatherEvent::Lightning`; `WeatherEvent::Changed` on transitions.
3. Writes `WeatherState` in `SimSet::Weather`; resets in `OnEnter(Setup)`; removed in
   `OnEnter(Unloading)`; menus get a sensible default (Clear, Noon).

## Tests
Unit: plan determinism per seed; Random respects allowed presets; Dynamic neighbour graph and
timing window; accumulation/drying curves; indoor never wet. Headless: a 3-lap race with Dynamic
weather changes exactly once.

## Acceptance
Tests, clippy, fmt clean; design note with the parameter table per weather kind.
