import assert from "node:assert/strict";
import { test } from "node:test";

import type { UiMarkerShape } from "../bevy";
import { markerPoints } from "./markers.ts";

const SHAPES: UiMarkerShape[] = ["circle", "triangle", "square", "diamond", "star", "cross", "pentagon", "hexagon"];

test("every marker shape fits its box and the eight are different", () => {
  const seen = new Set<string>();
  for (const shape of SHAPES) {
    const points = markerPoints(shape);
    assert.equal(points.length % 2, 0);
    assert.ok(points.every((v) => v >= 0 && v <= 100), shape);
    seen.add(points.join(","));
  }
  assert.equal(seen.size, SHAPES.length);
  assert.equal(markerPoints("star").length, 20, "five points and five notches");
  assert.deepEqual(markerPoints("diamond").slice(0, 2), [50, 2.5], "y grows downward, the tip on top");
});
