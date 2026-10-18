import assert from "node:assert/strict";
import { test } from "node:test";

import { clampFocus, cycle, enter, followScroll, move, stepValue, type FocusMap } from "./focus.ts";
import { pop, popTo, push, replace, top } from "./router.ts";
import { createNavStack } from "./stack.ts";

const menu: FocusMap = { items: { count: 4, cols: 1, wrap: true } };

test("a wrapping list cycles at both ends", () => {
  let f = { region: "items", index: 0 };
  f = move(menu, f, "up");
  assert.equal(f.index, 3);
  f = move(menu, f, "down");
  assert.equal(f.index, 0);
  assert.equal(move(menu, f, "left"), f, "sideways in a lone list does nothing");
});

test("a grid moves in 2D and clamps into a partial last row", () => {
  const map: FocusMap = { cars: { count: 7, cols: 3 } };
  let f = { region: "cars", index: 1 };
  f = move(map, f, "down");
  assert.equal(f.index, 4);
  f = move(map, f, "down");
  assert.equal(f.index, 6, "row 3 has one item: clamp to it");
  f = move(map, f, "right");
  assert.equal(f.index, 6, "no wrap, no neighbour: stays");
  assert.equal(move(map, { region: "cars", index: 2 }, "right").index, 2);
  assert.equal(move(map, { region: "cars", index: 3 }, "up").index, 0);
});

test("leaving a region enters the neighbour at its remembered index", () => {
  const map: FocusMap = {
    tracks: { count: 6, cols: 2, right: "options" },
    options: { count: 5, cols: 1, left: "tracks" },
  };
  let f = move(map, { region: "tracks", index: 3 }, "right");
  assert.deepEqual([f.region, f.index], ["options", 0]);
  f = move(map, f, "down");
  f = move(map, f, "down");
  f = move(map, f, "left");
  assert.deepEqual([f.region, f.index], ["tracks", 3], "back where it left");
  f = move(map, f, "right");
  assert.deepEqual([f.region, f.index], ["options", 2]);
});

test("empty neighbours are skipped and wrap applies instead", () => {
  const map: FocusMap = {
    row: { count: 3, cols: 3, wrap: true, down: "none" },
    none: { count: 0, cols: 1 },
  };
  assert.equal(move(map, { region: "row", index: 1 }, "down").region, "row");
  assert.equal(move(map, { region: "row", index: 0 }, "left").index, 2);
  assert.equal(move(map, { region: "row", index: 2 }, "right").index, 0);
});

test("enter, clamp and cycle keep indices valid", () => {
  const f = enter({ region: "a", index: 2 }, "b", 3, 9);
  assert.deepEqual([f.region, f.index, f.memory?.a], ["b", 2, 2]);
  const map: FocusMap = { a: { count: 2, cols: 1 }, b: { count: 0, cols: 1 } };
  assert.equal(clampFocus(map, { region: "a", index: 5 }, "a").index, 1);
  assert.equal(clampFocus(map, { region: "b", index: 0 }, "a").region, "a");
  assert.equal(cycle(0, -1, 5), 4);
  assert.equal(cycle(4, 1, 5), 0);
  assert.equal(cycle(2, 7, 5), 4);
  assert.equal(cycle(3, 1, 0), 0);
});

test("only the top-most nav layer receives events", () => {
  const stack = createNavStack<string>();
  const got: string[] = [];
  const screen = stack.add(0, { current: (e) => got.push(`screen:${e}`) });
  const dialog = stack.add(20, { current: (e) => got.push(`dialog:${e}`) });
  // Added after the dialog but lower: must not steal input (effect order after a hot reload).
  stack.add(0, { current: (e) => got.push(`late:${e}`) });
  stack.dispatch("a");
  dialog();
  stack.dispatch("b");
  screen();
  stack.dispatch("c");
  assert.deepEqual(got, ["dialog:a", "late:b", "late:c"]);
});

test("the router never pops its bottom screen", () => {
  let s = push(push(["title"], "main"), "settings");
  assert.equal(top(s), "settings");
  s = pop(s);
  assert.equal(top(s), "main");
  assert.deepEqual(pop(pop(pop(s))), ["title"]);
  assert.deepEqual(replace(s, "garage"), ["title", "garage"]);
  assert.deepEqual(popTo(["title", "main", "cups", "cars"], (r) => r === "main"), ["title", "main"]);
  assert.deepEqual(popTo(["title", "main"], (r) => r === "x"), ["title"]);
});

test("slider steps snap to the grid and clamp", () => {
  assert.equal(stepValue(0.7, 1, 0, 1, 0.05), 0.75);
  assert.equal(stepValue(0.98, 1, 0, 1, 0.05), 1);
  assert.equal(stepValue(0.333, 0, 0, 1, 0.05), 0.35, "a pointer pick snaps");
  assert.equal(stepValue(60, -1, 60, 100, 1), 60);
  assert.equal(stepValue(0.75, 1, 0.75, 1.25, 0.05), 0.8);
});

test("lists scroll to keep the focused row centred, within bounds", () => {
  // 20 rows of 50 px + 10 px gaps in a 300 px viewport: content 1190 px, max offset 890.
  assert.equal(followScroll(0, 50, 10, 20, 300), 0);
  assert.equal(followScroll(10, 50, 10, 20, 300), 475);
  assert.equal(followScroll(19, 50, 10, 20, 300), 890);
  assert.equal(followScroll(3, 50, 10, 4, 300), 0, "content shorter than the viewport never scrolls");
});
