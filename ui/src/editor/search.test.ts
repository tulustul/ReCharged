import assert from "node:assert/strict";
import { test } from "node:test";

import type { LibraryGroup } from "../bevy";
import { assetName, filterGroups, sameAsset } from "./search.ts";

const groups: LibraryGroup[] = [
  {
    title: "Props · toy",
    open: true,
    entries: [
      { name: "Toy bear", asset: { kind: "prop", id: "toy_bear" } },
      { name: "Toy block cube", asset: { kind: "prop", id: "toy_block_cube" } },
    ],
  },
  { title: "Decor · toy", open: true, entries: [{ name: "Bear", asset: { kind: "decor", model: "models/props/toy/bear.glb" } }] },
  { title: "Gameplay", open: true, entries: [{ name: "Item row", asset: { kind: "itemRow" } }] },
];

test("search matches every word, any case, and drops empty groups", () => {
  assert.deepEqual(
    filterGroups(groups, "BEAR").map((g) => g.title),
    ["Props · toy", "Decor · toy"],
  );
  assert.deepEqual(
    filterGroups(groups, "block toy").flatMap((g) => g.entries.map((e) => e.name)),
    ["Toy block cube"],
  );
  assert.equal(filterGroups(groups, "  ").length, 3, "no query: everything");
});

test("the swap picker lists only its own type", () => {
  assert.deepEqual(
    filterGroups(groups, "", "decor").map((g) => g.title),
    ["Decor · toy"],
  );
});

test("assets compare by value and name themselves", () => {
  assert.ok(sameAsset({ kind: "prop", id: "toy_bear" }, { kind: "prop", id: "toy_bear" }));
  assert.ok(!sameAsset({ kind: "prop", id: "toy_bear" }, null));
  assert.equal(assetName(groups, { kind: "prop", id: "toy_bear" }), "Toy bear");
  assert.equal(assetName(groups, { kind: "prop", id: "gone" }), "gone");
});
