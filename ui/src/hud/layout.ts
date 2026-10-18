// HUD layout maths and formatters (pure, tested): the per-view size factor, compact mode, fallback
// views for demos, the speed unit, running clocks and Time Trial splits.
import type { UiRect, UiSpeedUnit, UiViewport } from "../bevy";

/** rc_core `MPH_PER_KMH`. */
const MPH_PER_KMH = 0.621;

/** A full single view at 1080p: the size the HUD is authored at, in UI px. */
export const DESIGN = { w: 1920, h: 1080 } as const;

export type HudLayout = {
  /** Size factor of the big elements (1 = full 1080p view at 100 % HUD scale). */
  k: number;
  /** A split-screen-sized view: fewer details, labels keep a readable floor. */
  compact: boolean;
  /** Margin from the view's edges, UI px. */
  pad: number;
};

/** Layout of one player's view: big elements scale with √(view fit) so quarter views stay legible. */
export function hudLayout(rect: { w: number; h: number }, hudScale: number): HudLayout {
  const fit = Math.max(0, Math.min(rect.w / DESIGN.w, rect.h / DESIGN.h));
  const k = hudScale * Math.max(0.6, Math.sqrt(fit));
  return { k, compact: fit < 0.75, pad: Math.round(40 * k) };
}

/** A font size that shrinks with `k` but never below `floor` UI px (≈ 12 px at 720p 4-way). */
export const fs = (base: number, k: number, floor = 18): number => Math.round(Math.max(base * k, floor));

/** Views like rc_camera's split (2P top/bottom, 3–4P quadrants) on the 1080p canvas, for demos. */
export function fallbackViews(slots: number[]): { players: UiViewport[]; panel: UiRect | null } {
  const { w, h } = DESIGN;
  const half = { w: w / 2, h: h / 2 };
  const quads = [
    { x: 0, y: 0 },
    { x: half.w, y: 0 },
    { x: 0, y: half.h },
    { x: half.w, y: half.h },
  ];
  const rects: UiRect[] =
    slots.length <= 1
      ? [{ x: 0, y: 0, w, h }]
      : slots.length === 2
        ? [
            { x: 0, y: 0, w, h: half.h },
            { x: 0, y: half.h, w, h: half.h },
          ]
        : quads.slice(0, slots.length).map((q) => ({ ...q, ...half }));
  const players = slots.map((slot, i) => ({ slot, rect: rects[i] ?? rects[0] }));
  return { players, panel: slots.length === 3 ? { ...quads[3], ...half } : null };
}

/** Speed in the unit setting. */
export const speedIn = (kmh: number, unit: UiSpeedUnit): number => Math.round(unit === "mph" ? kmh * MPH_PER_KMH : kmh);

/** Running time with tenths: `"1:23.4"` (the 10 Hz tick's precision). */
export function clock(ms: number): string {
  const tenths = Math.floor(Math.max(0, ms) / 100);
  const minutes = Math.floor(tenths / 600);
  const seconds = Math.floor((tenths % 600) / 10);
  return `${minutes}:${String(seconds).padStart(2, "0")}.${tenths % 10}`;
}

/** A Time Trial split, always signed: `"-0.42"` (ahead) / `"+0.31"` (behind). */
export function split(secs: number): string {
  return `${secs < 0 ? "-" : "+"}${Math.abs(secs).toFixed(2)}`;
}

/** `2` → `["2", "nd"]`: a place with its suffix drawn smaller. */
export function placeParts(n: number): [string, string] {
  const tens = n % 100;
  const suffix = tens >= 11 && tens <= 13 ? "th" : (["th", "st", "nd", "rd"][n % 10] ?? "th");
  return [String(n), suffix];
}
