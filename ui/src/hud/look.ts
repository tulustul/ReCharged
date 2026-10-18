// HUD surfaces and type: translucent navy chips (no backdrop blur — the HUD sits over gameplay in
// every layout), display numbers with a drop shadow for bright scenes, spaced small caps.
import type { BevyStyle } from "bevy-react";

import { color, font } from "../theme/tokens";

export const tone = {
  chip: "rgba(5, 10, 24, 0.6)",
  chipStrong: "rgba(5, 10, 24, 0.82)",
  edge: "rgba(140, 180, 255, 0.22)",
  good: color.success,
  bad: color.danger,
  warn: color.warning,
  gold: color.gold,
  info: color.cyan,
  shadow: "rgba(0, 0, 0, 0.55)",
} as const;

/** A translucent panel scaled by `k`. */
export const chip = (k: number, strong = false): BevyStyle => ({
  backgroundColor: strong ? tone.chipStrong : tone.chip,
  border: 1,
  borderColor: tone.edge,
  borderRadius: Math.round(12 * k),
});

/** Big italic numbers and banners. */
export const num = (size: number, tint: string = color.text, weight = "800"): BevyStyle => ({
  fontFamily: font.display,
  fontWeight: weight,
  fontSize: size,
  color: tint,
  lineHeight: 1,
  lineBreak: "noWrap",
  textShadow: { color: tone.shadow, offsetX: 0, offsetY: Math.max(2, Math.round(size / 28)) },
});

/** Small spaced caps labels. */
export const caps = (size: number, tint: string = color.textDim): BevyStyle => ({
  fontSize: size,
  fontWeight: "700",
  letterSpacing: Math.round(size * 0.14),
  color: tint,
  lineBreak: "noWrap",
  textShadow: { color: tone.shadow, offsetX: 0, offsetY: 1 },
});

/** Body-weight text (notices, hints). */
export const label = (size: number, tint: string = color.text): BevyStyle => ({
  fontSize: size,
  fontWeight: "600",
  color: tint,
  lineBreak: "noWrap",
});
