import assert from "node:assert/strict";
import { test } from "node:test";

import type { UiBindings, UiSettings } from "../../bevy";
import { PRESETS, rebindNotice, rowsOf, type RowDef } from "./rows.ts";

const settings: UiSettings = {
  gameplay: { speedUnit: "kmh", cameraMode: "chase", cameraShake: true, hudScale: 1, minimap: true, split: "horizontal", aiCatchUp: true, aiDifficulty: "pro", hints: true },
  graphics: {
    preset: "high",
    displayMode: "windowed",
    windowSize: [1600, 900],
    vsync: true,
    fovDeg: 90,
    shadows: "high",
    antiAliasing: "taa",
    ambientOcclusion: true,
    bloom: true,
    motionBlur: false,
    particles: "high",
    volumetricFog: false,
  },
  audio: { master: 1, music: 0.7, sfx: 0.9, engines: 0.8, ui: 0.8, ambience: 0.7, muteUnfocused: true },
  controls: { steerDeadZone: 0.12, steerSensitivity: 1, vibration: true, vibrationIntensity: 1 },
  accessibility: { reduceFlashing: false, colorblindMarkers: false, steeringAssist: false, autoAccelerate: false },
};

const row = (tab: Parameters<typeof rowsOf>[0], id: string, s = settings): RowDef => {
  const found = rowsOf(tab, s).find((r) => r.id === id);
  assert.ok(found, `${tab}/${id}`);
  return found;
};

test("every tab lists rows; gameplay offers reset progress", () => {
  for (const tab of ["gameplay", "graphics", "audio", "controls", "accessibility"] as const) {
    assert.ok(rowsOf(tab, settings).length > 0, tab);
  }
  assert.equal(row("gameplay", "resetProgress").kind, "action");
});

test("a quality preset sets the options it governs; tuning one makes it custom", () => {
  const preset = row("graphics", "preset");
  assert.equal(preset.kind, "cycle");
  if (preset.kind !== "cycle") return;
  const low = preset.pick(settings, 0);
  assert.equal(low.graphics.preset, "low");
  assert.deepEqual(
    { ...low.graphics, preset: undefined, displayMode: undefined, windowSize: undefined, vsync: undefined, fovDeg: undefined },
    { ...PRESETS.low, preset: undefined, displayMode: undefined, windowSize: undefined, vsync: undefined, fovDeg: undefined },
  );
  const bloom = row("graphics", "bloom", low);
  assert.equal(bloom.kind, "toggle");
  if (bloom.kind !== "toggle") return;
  const tuned = bloom.set(low, true);
  assert.equal(tuned.graphics.preset, "custom");
  assert.equal(tuned.graphics.bloom, true);
  // FOV is not governed by presets.
  const fov = row("graphics", "fov");
  if (fov.kind === "slider") assert.equal(fov.set(settings, 75).graphics.preset, "high");
});

test("locks follow other settings", () => {
  assert.equal(row("graphics", "volumetricFog").locked, true, "Ultra only");
  const borderless = { ...settings, graphics: { ...settings.graphics, displayMode: "borderless" as const } };
  assert.equal(row("graphics", "windowSize", borderless).locked, true, "no window size in borderless");
  const noRumble = { ...settings, controls: { ...settings.controls, vibration: false } };
  assert.equal(row("controls", "vibrationIntensity", noRumble).locked, true);
});

test("cyclers and sliders read and write their field", () => {
  const units = row("gameplay", "speedUnit");
  if (units.kind !== "cycle") return assert.fail("cycle row");
  assert.equal(units.index(settings), 0);
  assert.equal(units.pick(settings, 1).gameplay.speedUnit, "mph");
  const music = row("audio", "music");
  if (music.kind !== "slider") return assert.fail("slider row");
  assert.equal(music.get(settings), 0.7);
  assert.equal(music.set(settings, 0.25).audio.music, 0.25);
  assert.equal(music.format(0.25), "25%");
  const size = row("graphics", "windowSize");
  if (size.kind !== "cycle") return assert.fail("cycle row");
  assert.deepEqual(size.pick(settings, size.choices.length - 1).graphics.windowSize, [2560, 1440]);
});

test("a conflicting rebind says it swapped, or that the other action is unbound", () => {
  const bindings = (camera: number): UiBindings => ({
    schemes: [{ scheme: "keyboardA", actions: [{ action: "camera", bindings: Array.from({ length: camera }, () => ({ kind: "key", code: "KeyX" })) }] }],
  });
  const camera = { scheme: "keyboardA", action: "camera" } as const;
  assert.deepEqual(rebindNotice({ kind: "conflict", other: camera, swapped: true }, "keyboardA", bindings(1)), { text: "Swapped with Camera", unbound: null });
  assert.deepEqual(rebindNotice({ kind: "conflict", other: camera, swapped: false }, "keyboardA", bindings(0)), {
    text: "Camera is now unbound",
    unbound: camera,
  });
  assert.equal(rebindNotice({ kind: "conflict", other: camera, swapped: false }, "keyboardA", bindings(1)).text, "Removed from Camera");
  assert.equal(rebindNotice({ kind: "conflict", other: camera, swapped: true }, "keyboardB", bindings(1)).text, "Swapped with Camera (Keyboard A)", "other scheme named");
  assert.deepEqual(rebindNotice({ kind: "cancelled" }, "keyboardA", null), { text: "Rebind cancelled", unbound: null });
  assert.deepEqual(rebindNotice(null, "keyboardA", null), { text: null, unbound: null });
});
