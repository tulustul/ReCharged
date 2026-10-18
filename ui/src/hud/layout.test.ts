import { test } from "node:test";
import assert from "node:assert/strict";

import { clock, fallbackViews, fs, hudLayout, placeParts, speedIn, split } from "./layout.ts";

test("a full 1080p view is the design size; quarter views shrink but stay legible", () => {
  assert.deepEqual(hudLayout({ w: 1920, h: 1080 }, 1), { k: 1, compact: false, pad: 40 });
  const quarter = hudLayout({ w: 960, h: 540 }, 1);
  assert.ok(Math.abs(quarter.k - Math.SQRT1_2) < 1e-9 && quarter.compact);
  assert.equal(hudLayout({ w: 1920, h: 540 }, 1).compact, true, "2P top/bottom");
  assert.equal(hudLayout({ w: 1920, h: 1080 }, 1.25).k, 1.25, "HUD scale setting");
  assert.equal(fs(16, quarter.k), 18, "labels keep their floor");
  assert.equal(fs(120, quarter.k), 85);
});

test("fallback views follow the camera split and leave the 4th quadrant to 3 players", () => {
  assert.deepEqual(fallbackViews([0]).players[0].rect, { x: 0, y: 0, w: 1920, h: 1080 });
  assert.deepEqual(fallbackViews([0, 1]).players[1].rect, { x: 0, y: 540, w: 1920, h: 540 });
  const three = fallbackViews([0, 1, 2]);
  assert.deepEqual(three.players[2].rect, { x: 0, y: 540, w: 960, h: 540 });
  assert.deepEqual(three.panel, { x: 960, y: 540, w: 960, h: 540 });
  assert.equal(fallbackViews([0, 1, 2, 3]).panel, null);
});

test("formatters", () => {
  assert.equal(speedIn(100, "kmh"), 100);
  assert.equal(speedIn(100, "mph"), 62);
  assert.equal(clock(83_456), "1:23.4");
  assert.equal(clock(5_000), "0:05.0");
  assert.equal(split(-0.4249), "-0.42");
  assert.equal(split(0.31), "+0.31");
  assert.equal(split(0), "+0.00");
  assert.deepEqual(placeParts(2), ["2", "nd"]);
  assert.deepEqual(placeParts(8), ["8", "th"]);
  assert.deepEqual(placeParts(11), ["11", "th"]);
});
