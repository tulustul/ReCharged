// Shared style presets built from the tokens: text styles (no inheritance in bevy-react — spread
// one on every <text>) and surfaces.
import type { BevyStyle } from "bevy-react";

import { color, font, radius, size } from "./tokens";

const display = (fontSize: number, fontWeight = "800"): BevyStyle => ({
  fontFamily: font.display,
  fontWeight,
  fontSize,
  color: color.text,
  lineHeight: 1.05,
  lineBreak: "noWrap",
});

export const text = {
  hero: display(size.hero, "900"),
  title: display(size.title),
  h2: display(size.h2),
  h3: display(size.h3, "700"),
  menu: display(size.menu, "800"),
  button: display(size.button, "700"),
  label: { fontSize: size.body, fontWeight: "600", color: color.text, lineBreak: "noWrap" },
  body: { fontSize: size.body, fontWeight: "400", color: color.textDim, lineHeight: 1.4 },
  small: { fontSize: size.small, fontWeight: "500", color: color.textDim, lineHeight: 1.3 },
  /** Small spaced caps: section headers, table headers. */
  caps: {
    fontSize: size.tiny,
    fontWeight: "700",
    color: color.textDim,
    letterSpacing: 2.5,
    lineBreak: "noWrap",
  },
  /** Numbers in tables (right-aligned columns). */
  number: { fontSize: size.body, fontWeight: "600", color: color.text, lineBreak: "noWrap" },
} satisfies Record<string, BevyStyle>;

/** Frosted glass: blurs the 3D frame behind (single view only). */
export const glass: BevyStyle = {
  backgroundColor: color.glass,
  backdropFilter: { name: "blur", params: { radius: 22 } },
  border: 1,
  borderColor: color.glassBorder,
  borderRadius: radius.l,
};

/** Translucent fill for split-screen overlays (no 3D frame to blur on the overlay camera). */
export const glassFlat: BevyStyle = {
  backgroundColor: color.glassFlat,
  border: 1,
  borderColor: color.glassBorder,
  borderRadius: radius.l,
};

/** Fills the parent. */
export const fill: BevyStyle = {
  positionType: "absolute",
  left: 0,
  top: 0,
  right: 0,
  bottom: 0,
};

/** Any hover style makes a node interactive: it then owns the pointer (modal scrims). */
export const ownsPointer: BevyStyle = {};
