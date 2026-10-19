// Option controls: OptionRow (a focusable labelled row), Toggle, Slider and Cycler (Select). Left /
// right on a focused row adjusts its value (the screen's nav handler calls the row's adjust); the
// pointer clicks the controls directly.
import type { ReactNode } from "react";
import type { PointerEventData } from "bevy-react";

import { ChevronIcon, LockIcon } from "./icons";
import { stepValue } from "../nav/focus";
import { focusTransition, spring } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, size } from "../theme/tokens";

type RowProps = {
  label: string;
  focused: boolean;
  onHover: () => void;
  onPress?: () => void;
  disabled?: boolean;
  /** Needs attention (e.g. an action left unbound by a rebind). */
  warn?: boolean;
  children?: ReactNode;
  height?: number;
};

/** A settings/options row: label on the left, its control on the right. */
export function OptionRow({ label, focused, onHover, onPress, disabled, warn, children, height = 62 }: RowProps) {
  return (
    <button
      onPointerHover={onHover}
      onClick={onPress}
      style={{
        height,
        flexShrink: 0,
        flexDirection: "row",
        alignItems: "center",
        gap: 16,
        padding: { left: 26, right: 18 },
        borderRadius: radius.m,
        border: 1,
        borderColor: focused ? "rgba(25, 217, 255, 0.55)" : warn ? "rgba(255, 176, 32, 0.6)" : color.clear,
        backgroundColor: focused ? "rgba(25, 217, 255, 0.12)" : warn ? "rgba(255, 176, 32, 0.1)" : color.clear,
        transition: focusTransition,
      }}
    >
      <node
        style={{
          positionType: "absolute",
          left: 8,
          top: 16,
          bottom: 16,
          width: 4,
          borderRadius: 2,
          backgroundColor: focused ? color.volt : color.clear,
        }}
      />
      <text style={{ ...text.label, color: disabled ? color.textFaint : warn ? color.warning : focused ? color.cyanHi : color.text, flexGrow: 1 }}>{label}</text>
      {disabled && <LockIcon size={20} color={color.textFaint} />}
      {children}
    </button>
  );
}

/** On/off switch. */
export function Toggle({ value, onChange, labels }: { value: boolean; onChange: (v: boolean) => void; labels: [string, string] }) {
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: 14 }}>
      <text style={{ ...text.label, color: value ? color.text : color.textDim, width: 54, textAlign: "right" }}>{value ? labels[1] : labels[0]}</text>
      <button
        onClick={() => onChange(!value)}
        style={{
          width: 64,
          height: 34,
          borderRadius: radius.pill,
          padding: 4,
          backgroundColor: value ? color.cyan : "rgba(255, 255, 255, 0.12)",
          transition: { backgroundColor: { duration: 140 } },
        }}
      >
        <node
          style={{
            width: 26,
            height: 26,
            borderRadius: radius.pill,
            backgroundColor: value ? color.ink : color.textDim,
            transform: { translateX: value ? 30 : 0 },
            transition: { transform: spring.snappy },
          }}
        />
      </button>
    </node>
  );
}

type SliderProps = {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  format: (v: number) => string;
  width?: number;
};

/** A horizontal slider; drag or click the track, or step it with left/right. */
export function Slider({ value, min, max, step, onChange, format, width = 300 }: SliderProps) {
  const t = Math.min(1, Math.max(0, (value - min) / (max - min || 1)));
  const pick = (e: PointerEventData) => onChange(stepValue(min + e.x * (max - min), 0, min, max, step));
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: 18 }}>
      <node onPointerDown={pick} onPointerMove={pick} style={{ width, height: 30, flexDirection: "column", justifyContent: "center" }}>
        <node style={{ width, height: 8, borderRadius: 4, backgroundColor: "rgba(60, 90, 160, 0.45)" }}>
          <node style={{ width: t * width, height: 8, borderRadius: 4, backgroundGradient: { type: "linear", angle: 90, stops: [{ color: color.cyanMid }, { color: color.cyan }] } }} />
        </node>
        <node
          style={{
            positionType: "absolute",
            left: t * width - 11,
            top: 4,
            width: 22,
            height: 22,
            borderRadius: radius.pill,
            backgroundColor: color.text,
            border: 3,
            borderColor: color.cyan,
          }}
        />
      </node>
      <text style={{ ...text.label, width: 76, textAlign: "right" }}>{format(value)}</text>
    </node>
  );
}

/** "‹ value ›" cycler (the brief's Select). */
export function Cycler({ label, onStep, width = 260, dim = false }: { label: string; onStep: (delta: number) => void; width?: number; dim?: boolean }) {
  const arrow = (dir: "left" | "right") => (
    <button
      onClick={() => onStep(dir === "left" ? -1 : 1)}
      style={{ width: 36, height: 36, borderRadius: radius.s, alignItems: "center", justifyContent: "center", backgroundColor: color.clear }}
      hoverStyle={{ backgroundColor: "rgba(255, 255, 255, 0.1)" }}
    >
      <ChevronIcon dir={dir} size={20} color={dim ? color.textFaint : color.cyan} />
    </button>
  );
  return (
    <node style={{ width, flexDirection: "row", alignItems: "center" }}>
      {arrow("left")}
      <text style={{ ...text.label, fontSize: size.body, color: dim ? color.textFaint : color.text, flexGrow: 1, textAlign: "center" }}>{label}</text>
      {arrow("right")}
    </node>
  );
}
