import assert from "node:assert/strict";
import { test } from "node:test";

import { allReady, slotStep, syncSlots, type SlotChoice } from "./lobby.ts";

const paints = () => 4;

test("joined slots get their own car; departed slots are dropped", () => {
  let choices = syncSlots({}, [0, 1], 3);
  assert.deepEqual([choices[0].car, choices[1].car], [0, 1]);
  choices = { ...choices, 1: { ...choices[1], ready: true } };
  choices = syncSlots(choices, [1, 3], 3);
  assert.deepEqual(Object.keys(choices), ["1", "3"]);
  assert.equal(choices[1].ready, true, "kept");
  assert.equal(choices[3].car, 0, "slot 3 % 3 cars");
});

test("each slot steps its own car and paint rows, and readies up", () => {
  let c: SlotChoice = { car: 0, paint: 3, row: "car", ready: false };
  c = slotStep(c, "left", 3, (car) => (car === 2 ? 2 : 4)).choice;
  assert.equal(c.car, 2, "wraps to the last car");
  assert.equal(c.paint, 1, "paint clamped to that car's paints");
  c = slotStep(c, "down", 3, paints).choice;
  assert.equal(c.row, "paint");
  c = slotStep(c, "right", 3, paints).choice;
  assert.equal(c.paint, 2);
  const ready = slotStep(c, "confirm", 3, paints);
  assert.deepEqual([ready.choice.ready, ready.cue], [true, "select"]);
  assert.equal(slotStep(ready.choice, "left", 3, paints).choice, ready.choice, "a ready player's choice is locked");
  assert.equal(slotStep(ready.choice, "confirm", 3, paints).choice.ready, false, "confirm again un-readies");
});

test("versus needs two ready players", () => {
  const ready = { car: 0, paint: 0, row: "car" as const, ready: true };
  assert.equal(allReady([0], { 0: ready }), false);
  assert.equal(allReady([0, 2], { 0: ready, 2: { ...ready, ready: false } }), false);
  assert.equal(allReady([0, 2], { 0: ready, 2: ready }), true);
});
