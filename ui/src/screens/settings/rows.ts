// Settings rows as data (pure): what each row shows and how it edits `UiSettings` (GDD §15).
// Picking a quality preset sets the graphics options it governs; changing one of them makes the
// preset Custom (the preset table is UI policy — rc_render only reads the individual options).
import type { UiGraphics, UiQuality, UiSettings } from "../../bevy";
import { percent } from "../../format.ts";
import { difficultyName } from "../../strings/common.ts";
import {
  aaName,
  cameraModeName,
  displayModeName,
  particlesName,
  qualityName,
  settingsText,
  shadowsName,
  speedUnitName,
  splitName,
} from "../../strings/settings.ts";

export type Choice<T> = { value: T; label: string };

type Base = { id: keyof typeof settingsText.rows; locked?: boolean };
export type RowDef =
  | (Base & { kind: "toggle"; get: (s: UiSettings) => boolean; set: (s: UiSettings, v: boolean) => UiSettings })
  | (Base & { kind: "cycle"; choices: Choice<unknown>[]; index: (s: UiSettings) => number; pick: (s: UiSettings, i: number) => UiSettings })
  | (Base & { kind: "slider"; min: number; max: number; step: number; format: (v: number) => string; get: (s: UiSettings) => number; set: (s: UiSettings, v: number) => UiSettings })
  | (Base & { kind: "action" });

export type TabId = keyof typeof settingsText.tabs;
export const TABS: TabId[] = ["gameplay", "graphics", "audio", "controls", "accessibility"];

type PresetFields = Pick<UiGraphics, "shadows" | "antiAliasing" | "ambientOcclusion" | "bloom" | "motionBlur" | "particles" | "volumetricFog">;

export const PRESETS: Record<Exclude<UiQuality, "custom">, PresetFields> = {
  low: { shadows: "off", antiAliasing: "fxaa", ambientOcclusion: false, bloom: false, motionBlur: false, particles: "low", volumetricFog: false },
  medium: { shadows: "low", antiAliasing: "smaa", ambientOcclusion: false, bloom: true, motionBlur: false, particles: "medium", volumetricFog: false },
  high: { shadows: "high", antiAliasing: "taa", ambientOcclusion: true, bloom: true, motionBlur: false, particles: "high", volumetricFog: false },
  ultra: { shadows: "high", antiAliasing: "taa", ambientOcclusion: true, bloom: true, motionBlur: true, particles: "high", volumetricFog: true },
};

export const WINDOW_SIZES: [number, number][] = [
  [1280, 720],
  [1600, 900],
  [1920, 1080],
  [2560, 1440],
];

const choices = <T extends string>(names: Record<T, string>): Choice<T>[] =>
  (Object.keys(names) as T[]).map((value) => ({ value, label: names[value] }));

/** A cycler row over `choices` reading/writing one field. */
function cycle<T>(id: Base["id"], list: Choice<T>[], get: (s: UiSettings) => T, put: (s: UiSettings, v: T) => UiSettings): RowDef {
  return {
    id,
    kind: "cycle",
    choices: list,
    index: (s) => Math.max(0, list.findIndex((c) => c.value === get(s))),
    pick: (s, i) => put(s, list[i].value),
  };
}

const gameplay = (s: UiSettings, g: Partial<UiSettings["gameplay"]>): UiSettings => ({ ...s, gameplay: { ...s.gameplay, ...g } });
const graphics = (s: UiSettings, g: Partial<UiGraphics>): UiSettings => ({ ...s, graphics: { ...s.graphics, ...g } });
/** A preset-governed option: changing it makes the preset Custom. */
const tuned = (s: UiSettings, g: Partial<PresetFields>): UiSettings => graphics(s, { ...g, preset: "custom" });
const audio = (s: UiSettings, a: Partial<UiSettings["audio"]>): UiSettings => ({ ...s, audio: { ...s.audio, ...a } });
const controls = (s: UiSettings, c: Partial<UiSettings["controls"]>): UiSettings => ({ ...s, controls: { ...s.controls, ...c } });
const access = (s: UiSettings, a: Partial<UiSettings["accessibility"]>): UiSettings => ({ ...s, accessibility: { ...s.accessibility, ...a } });

const volume = (id: Base["id"], key: keyof Omit<UiSettings["audio"], "muteUnfocused">): RowDef => ({
  id,
  kind: "slider",
  min: 0,
  max: 1,
  step: 0.05,
  format: percent,
  get: (s) => s.audio[key],
  set: (s, v) => audio(s, { [key]: v }),
});

/** The rows of a tab (the Controls tab adds its binding rows itself). */
export function rowsOf(tab: TabId, s: UiSettings): RowDef[] {
  switch (tab) {
    case "gameplay":
      return [
        cycle("speedUnit", choices(speedUnitName), (x) => x.gameplay.speedUnit, (x, v) => gameplay(x, { speedUnit: v })),
        cycle("cameraMode", choices(cameraModeName), (x) => x.gameplay.cameraMode, (x, v) => gameplay(x, { cameraMode: v })),
        { id: "cameraShake", kind: "toggle", get: (x) => x.gameplay.cameraShake, set: (x, v) => gameplay(x, { cameraShake: v }) },
        { id: "hudScale", kind: "slider", min: 0.75, max: 1.25, step: 0.05, format: percent, get: (x) => x.gameplay.hudScale, set: (x, v) => gameplay(x, { hudScale: v }) },
        { id: "minimap", kind: "toggle", get: (x) => x.gameplay.minimap, set: (x, v) => gameplay(x, { minimap: v }) },
        cycle("split", choices(splitName), (x) => x.gameplay.split, (x, v) => gameplay(x, { split: v })),
        { id: "aiCatchUp", kind: "toggle", get: (x) => x.gameplay.aiCatchUp, set: (x, v) => gameplay(x, { aiCatchUp: v }) },
        cycle(
          "aiDifficulty",
          (["rookie", "pro", "elite"] as const).map((value) => ({ value, label: difficultyName[value] })),
          (x) => x.gameplay.aiDifficulty,
          (x, v) => gameplay(x, { aiDifficulty: v }),
        ),
        { id: "hints", kind: "toggle", get: (x) => x.gameplay.hints, set: (x, v) => gameplay(x, { hints: v }) },
        { id: "resetProgress", kind: "action" },
      ];
    case "graphics": {
      const sizes = WINDOW_SIZES.some(([w, h]) => w === s.graphics.windowSize[0] && h === s.graphics.windowSize[1])
        ? WINDOW_SIZES
        : [...WINDOW_SIZES, s.graphics.windowSize];
      return [
        {
          id: "preset",
          kind: "cycle",
          choices: choices(qualityName).filter((c) => c.value !== "custom" || s.graphics.preset === "custom"),
          index: (x) => Math.max(0, (["low", "medium", "high", "ultra", "custom"] as UiQuality[]).indexOf(x.graphics.preset)),
          pick: (x, i) => {
            const preset = (["low", "medium", "high", "ultra", "custom"] as const)[i];
            return preset === "custom" ? x : graphics(x, { preset, ...PRESETS[preset] });
          },
        },
        cycle("displayMode", choices(displayModeName), (x) => x.graphics.displayMode, (x, v) => graphics(x, { displayMode: v })),
        {
          ...cycle(
            "windowSize",
            sizes.map(([w, h]) => ({ value: `${w}x${h}`, label: `${w} × ${h}` })),
            (x) => `${x.graphics.windowSize[0]}x${x.graphics.windowSize[1]}`,
            (x, v) => graphics(x, { windowSize: v.split("x").map(Number) as [number, number] }),
          ),
          locked: s.graphics.displayMode !== "windowed",
        },
        { id: "vsync", kind: "toggle", get: (x) => x.graphics.vsync, set: (x, v) => graphics(x, { vsync: v }) },
        { id: "fov", kind: "slider", min: 60, max: 100, step: 1, format: (v) => `${Math.round(v)}°`, get: (x) => x.graphics.fovDeg, set: (x, v) => graphics(x, { fovDeg: v }) },
        cycle("shadows", choices(shadowsName), (x) => x.graphics.shadows, (x, v) => tuned(x, { shadows: v })),
        cycle("antiAliasing", choices(aaName), (x) => x.graphics.antiAliasing, (x, v) => tuned(x, { antiAliasing: v })),
        { id: "ambientOcclusion", kind: "toggle", get: (x) => x.graphics.ambientOcclusion, set: (x, v) => tuned(x, { ambientOcclusion: v }) },
        { id: "bloom", kind: "toggle", get: (x) => x.graphics.bloom, set: (x, v) => tuned(x, { bloom: v }) },
        { id: "motionBlur", kind: "toggle", get: (x) => x.graphics.motionBlur, set: (x, v) => tuned(x, { motionBlur: v }) },
        cycle("particles", choices(particlesName), (x) => x.graphics.particles, (x, v) => tuned(x, { particles: v })),
        {
          id: "volumetricFog",
          kind: "toggle",
          get: (x) => x.graphics.volumetricFog,
          set: (x, v) => tuned(x, { volumetricFog: v }),
          locked: s.graphics.preset !== "ultra" && s.graphics.preset !== "custom",
        },
      ];
    }
    case "audio":
      return [
        volume("master", "master"),
        volume("music", "music"),
        volume("sfx", "sfx"),
        volume("engines", "engines"),
        volume("ui", "ui"),
        volume("ambience", "ambience"),
        { id: "muteUnfocused", kind: "toggle", get: (x) => x.audio.muteUnfocused, set: (x, v) => audio(x, { muteUnfocused: v }) },
      ];
    case "controls":
      return [
        { id: "deadZone", kind: "slider", min: 0, max: 0.5, step: 0.02, format: percent, get: (x) => x.controls.steerDeadZone, set: (x, v) => controls(x, { steerDeadZone: v }) },
        { id: "sensitivity", kind: "slider", min: 0.5, max: 2, step: 0.1, format: (v) => `${v.toFixed(1)}×`, get: (x) => x.controls.steerSensitivity, set: (x, v) => controls(x, { steerSensitivity: v }) },
        { id: "vibration", kind: "toggle", get: (x) => x.controls.vibration, set: (x, v) => controls(x, { vibration: v }) },
        {
          id: "vibrationIntensity",
          kind: "slider",
          min: 0,
          max: 1,
          step: 0.1,
          format: percent,
          get: (x) => x.controls.vibrationIntensity,
          set: (x, v) => controls(x, { vibrationIntensity: v }),
          locked: !s.controls.vibration,
        },
      ];
    case "accessibility":
      return [
        { id: "reduceFlashing", kind: "toggle", get: (x) => x.accessibility.reduceFlashing, set: (x, v) => access(x, { reduceFlashing: v }) },
        { id: "colorblindMarkers", kind: "toggle", get: (x) => x.accessibility.colorblindMarkers, set: (x, v) => access(x, { colorblindMarkers: v }) },
        { id: "steeringAssist", kind: "toggle", get: (x) => x.accessibility.steeringAssist, set: (x, v) => access(x, { steeringAssist: v }) },
        { id: "autoAccelerate", kind: "toggle", get: (x) => x.accessibility.autoAccelerate, set: (x, v) => access(x, { autoAccelerate: v }) },
      ];
  }
}
