// Design tokens (GDD §14): dark navy frosted glass, electric cyan + volt yellow, bold italic display
// type. Every colour, font, size and spacing used by the UI comes from here.

export const color = {
  bg: "#050913",
  bgDeep: "#02040a",
  navy: "#0b1226",
  /** Frosted glass fill (single view, over `backdropFilter`). */
  glass: "rgba(12, 20, 42, 0.66)",
  /** Translucent fill without backdrop blur (split-screen overlays). */
  glassFlat: "rgba(9, 15, 32, 0.86)",
  glassHi: "rgba(30, 48, 92, 0.55)",
  glassBorder: "rgba(120, 175, 255, 0.18)",
  line: "rgba(140, 180, 255, 0.12)",
  lineStrong: "rgba(140, 180, 255, 0.28)",
  scrim: "rgba(2, 4, 10, 0.62)",
  text: "#eef2fb",
  textDim: "#9aa7c6",
  textFaint: "#5f6b8a",
  ink: "#04101d",
  cyan: "#19d9ff",
  cyanHi: "#8af0ff",
  cyanMid: "#0fa8e6",
  cyanDeep: "#0a3d5c",
  cyanGlow: "rgba(25, 217, 255, 0.35)",
  volt: "#f2e10f",
  voltHi: "#fff27a",
  voltGlow: "rgba(242, 225, 15, 0.35)",
  danger: "#ff5468",
  success: "#3ddc84",
  warning: "#ffb020",
  gold: "#ffd34d",
  silver: "#cfd8e6",
  bronze: "#e0915a",
  platinum: "#b8f3ff",
  clear: "rgba(0, 0, 0, 0)",
} as const;

/** Player slot colours P1–P4 (rc_core `SLOT_COLORS`): cyan, yellow, magenta, green. */
export const slotColor = ["#00d9ff", "#ffdb1a", "#ff33cc", "#40e64d"] as const;

/** Registered font families (rc_ui `with_fonts`); the body font is the default (no family). */
export const font = { display: "Display" } as const;

/** Font sizes in UI px (menus are authored at 1920×1080). */
export const size = {
  hero: 150,
  title: 68,
  h2: 44,
  h3: 32,
  menu: 40,
  button: 28,
  body: 23,
  small: 19,
  tiny: 16,
} as const;

export const space = { xs: 4, s: 8, m: 16, l: 24, xl: 40, xxl: 64 } as const;

export const radius = { s: 6, m: 10, l: 16, pill: 999 } as const;

/** Stacking of overlays (nav layers use the same order). */
export const layer = { screen: 0, overlay: 10, dialog: 20, toast: 30 } as const;

/** Outer margins of a screen. */
export const frame = { x: 88, top: 64, bottom: 56 } as const;
