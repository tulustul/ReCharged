import assert from "node:assert/strict";
import { test } from "node:test";

import { actionGlyph, bindingGlyph, keyLabel } from "./glyphs.ts";

test("key codes become short keycap labels", () => {
  assert.equal(keyLabel("KeyW"), "W");
  assert.equal(keyLabel("Digit7"), "7");
  assert.equal(keyLabel("ArrowLeft"), "←");
  assert.equal(keyLabel("ShiftLeft"), "L-Shift");
  assert.equal(keyLabel("Numpad0"), "Num 0");
  assert.equal(keyLabel("NumpadDecimal"), "Num .");
  assert.equal(keyLabel("Space"), "Space");
  assert.equal(keyLabel("F5"), "F5", "unknown codes pass through");
});

test("gamepad bindings use generic shapes", () => {
  assert.deepEqual(bindingGlyph({ kind: "button", button: "South" }), { kind: "face", at: "south" });
  assert.deepEqual(bindingGlyph({ kind: "button", button: "RightTrigger2" }), { kind: "shoulder", label: "RT" });
  assert.deepEqual(bindingGlyph({ kind: "axis", axis: "LeftStickX", positive: false }), {
    kind: "stick",
    side: "L",
    dir: "left",
  });
  assert.deepEqual(bindingGlyph({ kind: "axis", axis: "RightStickY", positive: true }), {
    kind: "stick",
    side: "R",
    dir: "up",
  });
  assert.deepEqual(bindingGlyph({ kind: "button", button: "Other(9)" }), { kind: "key", label: "Other(9)" });
});

test("menu actions follow the device and the lobby schemes", () => {
  assert.deepEqual(actionGlyph("confirm", "gamepad"), { kind: "face", at: "south" });
  assert.deepEqual(actionGlyph("back", "keyboardA"), { kind: "key", label: "Esc" });
  assert.deepEqual(actionGlyph("back", "mouse"), { kind: "key", label: "Esc" }, "mouse users get keyboard glyphs");
  assert.deepEqual(actionGlyph("confirm", "keyboardA", true), { kind: "key", label: "Space" });
  assert.deepEqual(actionGlyph("back", "keyboardA", true), { kind: "key", label: "L-Ctrl" });
  assert.deepEqual(actionGlyph("confirm", "keyboardB", true), { kind: "key", label: "Enter" });
  assert.deepEqual(actionGlyph("back", "keyboardB", true), { kind: "key", label: "Backspace" });
});
