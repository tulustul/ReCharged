// Button glyphs (pure): what to draw for a menu action or a binding on a device family. Our own
// generic shapes: keycaps with labels, gamepad face buttons by position — no platform logos.
import type { UiBinding, UiDeviceKind } from "./bevy";

/** Fixed menu actions (GDD §12.2). */
export type MenuAction = "confirm" | "back" | "prevTab" | "nextTab" | "move" | "pause";

export type Dir = "up" | "down" | "left" | "right";

export type GlyphSpec =
  | { kind: "key"; label: string }
  | { kind: "face"; at: "south" | "east" | "north" | "west" }
  | { kind: "shoulder"; label: string }
  | { kind: "dpad"; dir: Dir | null }
  | { kind: "stick"; side: "L" | "R"; dir: Dir | null; press?: boolean }
  | { kind: "start" }
  | { kind: "select" }
  | { kind: "home" }
  | { kind: "mouse" };

const KEY_LABELS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Escape: "Esc",
  ShiftLeft: "L-Shift",
  ShiftRight: "R-Shift",
  ControlLeft: "L-Ctrl",
  ControlRight: "R-Ctrl",
  AltLeft: "L-Alt",
  AltRight: "R-Alt",
  SuperLeft: "Super",
  Delete: "Del",
  PageUp: "PgUp",
  PageDown: "PgDn",
  NumpadDecimal: "Num .",
  NumpadEnter: "Num Enter",
  NumpadAdd: "Num +",
  NumpadSubtract: "Num -",
  NumpadMultiply: "Num *",
  NumpadDivide: "Num /",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
};

/** A Bevy `KeyCode` name (same as the web's `KeyboardEvent.code`) → keycap label. */
export function keyLabel(code: string): string {
  if (KEY_LABELS[code]) return KEY_LABELS[code];
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad\d$/.test(code)) return `Num ${code.slice(6)}`;
  return code;
}

const BUTTONS: Record<string, GlyphSpec> = {
  South: { kind: "face", at: "south" },
  East: { kind: "face", at: "east" },
  North: { kind: "face", at: "north" },
  West: { kind: "face", at: "west" },
  LeftTrigger: { kind: "shoulder", label: "LB" },
  RightTrigger: { kind: "shoulder", label: "RB" },
  LeftTrigger2: { kind: "shoulder", label: "LT" },
  RightTrigger2: { kind: "shoulder", label: "RT" },
  DPadUp: { kind: "dpad", dir: "up" },
  DPadDown: { kind: "dpad", dir: "down" },
  DPadLeft: { kind: "dpad", dir: "left" },
  DPadRight: { kind: "dpad", dir: "right" },
  LeftThumb: { kind: "stick", side: "L", dir: null, press: true },
  RightThumb: { kind: "stick", side: "R", dir: null, press: true },
  Start: { kind: "start" },
  Select: { kind: "select" },
  Mode: { kind: "home" },
};

/** What to draw for a binding. */
export function bindingGlyph(binding: UiBinding): GlyphSpec {
  switch (binding.kind) {
    case "key":
      return { kind: "key", label: keyLabel(binding.code) };
    case "button":
      return BUTTONS[binding.button] ?? { kind: "key", label: binding.button };
    case "axis": {
      const side = binding.axis.startsWith("Right") ? "R" : "L";
      const horizontal = binding.axis.endsWith("X");
      const vertical = binding.axis.endsWith("Y");
      // Bevy's stick Y is positive up.
      const dir: Dir | null = horizontal
        ? binding.positive
          ? "right"
          : "left"
        : vertical
          ? binding.positive
            ? "up"
            : "down"
          : null;
      return { kind: "stick", side, dir };
    }
  }
}

/** What to draw for a fixed menu action on a device family (`lobby`: per-scheme lobby keys). */
export function actionGlyph(action: MenuAction, device: UiDeviceKind, lobby = false): GlyphSpec {
  if (device === "gamepad") {
    switch (action) {
      case "confirm":
        return { kind: "face", at: "south" };
      case "back":
        return { kind: "face", at: "east" };
      case "prevTab":
        return { kind: "shoulder", label: "LB" };
      case "nextTab":
        return { kind: "shoulder", label: "RB" };
      case "move":
        return { kind: "dpad", dir: null };
      case "pause":
        return { kind: "start" };
    }
  }
  const schemeB = device === "keyboardB";
  switch (action) {
    case "confirm":
      return { kind: "key", label: lobby && !schemeB ? "Space" : "Enter" };
    case "back":
      return { kind: "key", label: lobby ? (schemeB ? "Backspace" : "L-Ctrl") : "Esc" };
    case "prevTab":
      return { kind: "key", label: "Q" };
    case "nextTab":
      return { kind: "key", label: "E" };
    case "move":
      return { kind: "key", label: lobby && !schemeB ? "WASD" : "↑↓" };
    case "pause":
      return { kind: "key", label: "Esc" };
  }
}
