import assert from "node:assert/strict";
import { test } from "node:test";

import type { UiProfile, UiTrack } from "../bevy";
import { DEFAULT_DRAFT, maxAi, normalize, oncePer, startMessage, weatherChoices, withTrack } from "./draft.ts";

const track = (over: Partial<UiTrack> = {}): UiTrack => ({
  id: "aisle_rush",
  name: "Aisle Rush",
  environment: "supermarket",
  indoor: true,
  difficulty: 2,
  preview: null,
  defaultLaps: 3,
  times: ["sunset", "night"],
  weather: ["clear"],
  unlock: { kind: "start" },
  ...over,
});

const profile = (over: Partial<UiProfile["unlocked"]> = {}): UiProfile => ({
  unlocked: {
    cars: [],
    tracks: [],
    cups: [],
    reverse: false,
    mirror: false,
    difficulties: ["rookie", "pro"],
    paintSets: ["base"],
    credits: false,
    ...over,
  },
  fresh: [],
  trophies: [],
  goldenBatteries: [],
  goldMedals: 0,
  stats: { races: 0, wins: 0, podiums: 0, distanceKm: 0, itemsUsed: 0, hitsLanded: 0, timePlayedSecs: 0 },
});

test("indoor tracks offer no dynamic weather", () => {
  assert.deepEqual(
    weatherChoices(track()).map((c) => c.kind),
    ["fixed", "random"],
  );
  const outdoor = weatherChoices(track({ indoor: false, weather: ["clear", "rain"] }));
  assert.deepEqual(outdoor[outdoor.length - 1], { kind: "dynamic", start: "clear" });
});

test("normalize keeps options inside the track's presets and unlocks", () => {
  const d = normalize(
    { ...DEFAULT_DRAFT, time: "noon", weather: { kind: "fixed", weather: "storm" }, reverse: true, mirror: true, difficulty: "elite", laps: 12, ai: 9 },
    track(),
    profile({ reverse: true }),
  );
  assert.equal(d.time, "sunset", "noon is not allowed in Aisle Rush");
  assert.deepEqual(d.weather, { kind: "fixed", weather: "clear" });
  assert.equal(d.reverse, true);
  assert.equal(d.mirror, false, "mirror is locked");
  assert.equal(d.difficulty, "pro", "elite is locked");
  assert.equal(d.laps, 9);
  assert.equal(d.ai, 7);
});

test("time trial is three laps, no items, no rivals", () => {
  const d = withTrack({ ...DEFAULT_DRAFT, mode: "timeTrial", items: true }, track({ defaultLaps: 5 }), profile());
  assert.equal(d.laps, 3);
  assert.equal(d.items, false);
  assert.equal(d.ai, 0);
  assert.equal(maxAi("versus", 3), 5);
  const msg = startMessage(d, [{ slot: 0, car: "zip", paint: 1 }]);
  assert.deepEqual([msg.mode.kind, msg.laps, msg.ai, msg.items], ["timeTrial", 3, 0, false]);
});

test("a cup start carries only the cup and the car", () => {
  const msg = startMessage({ ...DEFAULT_DRAFT, mode: "championship", cup: "pro_cup", track: "maple_street" }, [
    { slot: 0, car: "torque", paint: 2 },
  ]);
  assert.deepEqual(msg.mode, { kind: "championship", cup: "pro_cup", race: 0 });
  assert.equal(msg.laps, undefined);
  assert.equal(msg.players[0].car, "torque");
});

test("one Confirm sends race.start once: repeats within the window are dropped", () => {
  const gate = oncePer(1000);
  assert.equal(gate(5000), true);
  assert.equal(gate(5016), false, "the same press seen twice");
  assert.equal(gate(5999), false);
  assert.equal(gate(6000), true, "a new press after the window");
});
