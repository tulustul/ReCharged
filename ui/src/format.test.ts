import assert from "node:assert/strict";
import { test } from "node:test";

import { compactNumber, formatDelta, formatDuration, formatKm, formatTime, layoutLabel, ordinal, percent, rgbHex } from "./format.ts";

test("race times show minutes, seconds and milliseconds", () => {
  assert.equal(formatTime(83456), "1:23.456");
  assert.equal(formatTime(5007), "0:05.007");
  assert.equal(formatTime(3_600_000), "60:00.000");
  assert.equal(formatTime(null), "—");
  assert.equal(formatTime(-5, "--"), "--");
  assert.equal(formatTime(999.6), "0:01.000", "rounds to the millisecond");
});

test("deltas carry their sign", () => {
  assert.equal(formatDelta(1204), "+1.204");
  assert.equal(formatDelta(-350), "-0.350");
  assert.equal(formatDelta(0), "+0.000");
});

test("ordinals handle the teens", () => {
  assert.deepEqual([1, 2, 3, 4, 8].map(ordinal), ["1st", "2nd", "3rd", "4th", "8th"]);
  assert.deepEqual([11, 12, 13, 21, 22, 101, 111].map(ordinal), ["11th", "12th", "13th", "21st", "22nd", "101st", "111th"]);
});

test("colours, percentages, durations and distances", () => {
  assert.equal(rgbHex([0, 0.85, 1]), "#00d9ff");
  assert.equal(rgbHex([2, -1, 0.5]), "#ff0080", "clamped");
  assert.equal(percent(0.746), "75%");
  assert.equal(formatDuration(59), "0 min");
  assert.equal(formatDuration(7500), "2 h 05 min");
  assert.equal(formatKm(12.345), "12.3 km");
  const words = { forward: "Forward", reverse: "Reverse", mirror: "Mirror" };
  assert.equal(layoutLabel(false, false, words), "Forward");
  assert.equal(layoutLabel(true, true, words), "Reverse · Mirror");
});

test("diagnostic values stay short", () => {
  assert.equal(compactNumber(0.4237), "0.42");
  assert.equal(compactNumber(61.27), "61.3");
  assert.equal(compactNumber(1840.4), "1840");
  assert.equal(compactNumber(18_420), "18.4k");
  assert.equal(compactNumber(2_100_000), "2.1M");
});
