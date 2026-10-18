// Design tokens (GDD §14: dark navy, electric cyan, volt yellow). T05 grows this into the theme.
export const colors = {
  navy: "#0b1020",
  panel: "#121a33dd",
  panelBorder: "#2a3a6b",
  text: "#e8ecf6",
  textDim: "#9aa6c4",
  cyan: "#19d9ff",
  volt: "#f2e10f",
  danger: "#ff5468",
} as const;

export const space = { s: 8, m: 16, l: 32 } as const;

export const fontSize = { body: 20, button: 22, title: 64, tagline: 22 } as const;
