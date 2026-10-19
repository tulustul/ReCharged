import assert from "node:assert/strict";
import { test } from "node:test";

import { ART, directionMarks, gateLine, ribbonPath, scaled } from "./mapArt.ts";
import { ICON_BOX, turnIcon, turnShape } from "./turnIcon.ts";

/** A square loop of `n` points per side around the centre of the 0–1 square. */
function square(r: number, n: number): number[] {
  const corners = [
    [0.5 - r, 0.5 - r],
    [0.5 + r, 0.5 - r],
    [0.5 + r, 0.5 + r],
    [0.5 - r, 0.5 + r],
  ];
  const out: number[] = [];
  for (let c = 0; c < 4; c++) {
    const [ax, ay] = corners[c];
    const [bx, by] = corners[(c + 1) % 4];
    for (let i = 0; i < n; i++) out.push(ax + ((bx - ax) * i) / n, ay + ((by - ay) * i) / n);
  }
  return out;
}

test("the ribbon is one closed quad per edge point, wrapping round", () => {
  const left = square(0.3, 5);
  const right = square(0.2, 5);
  const path = ribbonPath(left, right);
  const quads = path.split("M").filter(Boolean);
  assert.equal(quads.length, 20);
  assert.ok(quads.every((q) => q.trim().endsWith("Z") && q.split("L").length === 4));
  assert.ok(quads[19].startsWith(`${Math.round(left[38] * ART)} ${Math.round(left[39] * ART)}`), "the last quad closes the loop");
  assert.ok(quads[19].includes(`${Math.round(left[0] * ART)} ${Math.round(left[1] * ART)}`));
  assert.equal(ribbonPath([], []), "");
  assert.equal(ribbonPath(left, right.slice(2)), "", "edges must match");
  assert.deepEqual(scaled([0.5, 0.25]), [500, 250]);
});

test("gates and direction marks sit on the map", () => {
  assert.deepEqual(gateLine([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], 1), [500, 600, 700, 800]);
  assert.equal(gateLine([0.1, 0.2, 0.3, 0.4], 1), null);
  const outline = square(0.25, 10);
  const marks = directionMarks(outline, 8, 40);
  assert.equal(marks.length, 8);
  assert.ok(marks.every((m) => m.length === 8 && m.every((v) => v > 150 && v < 850)));
  // The first mark lies on the top side (y = 250), which runs toward +x: its tip is rightmost.
  const [tipX, tipY, wingX] = marks[0];
  assert.equal(tipY, 250);
  assert.ok(tipX > wingX, "points along the direction of travel");
  assert.deepEqual(directionMarks([0.5, 0.5], 4, 40), []);
});

test("turn icons bend the corner's way and stay in their box", () => {
  assert.equal(turnShape(30), "slight");
  assert.equal(turnShape(90), "square");
  assert.equal(turnShape(178), "hairpin");
  for (const angle of [30, 90, 178]) {
    const right = turnIcon(angle, true);
    const left = turnIcon(angle, false);
    for (const pts of [right.stem, right.head, left.stem, left.head]) {
      assert.ok(pts.every((v) => v >= 0 && v <= ICON_BOX), `${angle}: ${pts}`);
    }
    const tipX = (icon: { head: number[] }) => icon.head[0];
    assert.ok(tipX(right) > ICON_BOX / 2 && tipX(left) < ICON_BOX / 2, `${angle}: right points right`);
    assert.deepEqual(
      left.stem.filter((_, i) => i % 2 === 1),
      right.stem.filter((_, i) => i % 2 === 1),
      "mirrored, not moved",
    );
  }
});
