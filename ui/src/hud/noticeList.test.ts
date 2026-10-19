import assert from "node:assert/strict";
import { test } from "node:test";

import { LANDED_MERGE_MS, MAX_NOTICES, landedNotice, withNotice, type Notice } from "./noticeList.ts";

test("hits by one item within a second collapse into one counting line", () => {
  let list: Notice[] = [];
  let id = 1;
  const hit = (slot: number, item: "oilSlick" | "sparkRocket", victim: string, now: number) => {
    const notice = landedNotice(list, id++, slot, item, victim, now);
    list = withNotice(list, notice);
    return notice;
  };
  const first = hit(0, "oilSlick", "Dash", 0);
  assert.equal(first.text, "Oil Slick hit Dash!");
  const second = hit(0, "oilSlick", "Nova", 400);
  assert.equal(second.id, first.id, "the same line");
  assert.equal(second.text, "Oil Slick hit 2 racers");
  assert.equal(hit(0, "oilSlick", "Penny", 400 + LANDED_MERGE_MS).text, "Oil Slick hit 3 racers", "measured from the latest hit");
  assert.equal(list.length, 1);

  assert.equal(hit(0, "sparkRocket", "Rusty", 1500).text, "Spark Rocket hit Rusty!", "another item: its own line");
  assert.equal(hit(1, "oilSlick", "Dash", 1500).text, "Oil Slick hit Dash!", "another player's view");
  const late = hit(0, "oilSlick", "Glitch", 3000);
  assert.notEqual(late.id, first.id, "a later hit starts a new line");
  assert.equal(late.text, "Oil Slick hit Glitch!");
});

test("each view keeps its newest notices", () => {
  let list: Notice[] = [];
  for (let id = 1; id <= 5; id++) list = withNotice(list, { id, slot: 0, tone: "info", text: `n${id}` });
  list = withNotice(list, { id: 9, slot: 1, tone: "info", text: "other" });
  assert.deepEqual(
    list.filter((n) => n.slot === 0).map((n) => n.id),
    [3, 4, 5],
  );
  assert.equal(list.filter((n) => n.slot === 0).length, MAX_NOTICES);
  // Re-adding a line moves it to the end instead of duplicating it.
  list = withNotice(list, { id: 4, slot: 0, tone: "info", text: "again" });
  assert.deepEqual(
    list.filter((n) => n.slot === 0).map((n) => n.text),
    ["n3", "n5", "again"],
  );
});
