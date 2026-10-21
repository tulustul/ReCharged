// Small controls of the track editor: a tool button, chips (radio and checkbox), a switch, slider
// bars, and one inspector field per `FieldValue` type. Numbers are slider bars over the field's
// range, or dragged left / right when unbounded (Shift: fine); a drag shows live (`editor.set` with
// `live`) and the release commits. Only text fields
// (names, paths, unlock rules) type: click the value, type, Enter commits, Escape cancels.
import { useEffect, useState } from "react";
import type { PointerEventData } from "bevy-react";

import type { EditorCommand, EditorField, EditorItem, FieldRange, FieldValue } from "../bevy";
import { bevy } from "../bevy";
import { useEvent } from "../hooks";
import { stepValue } from "../nav/focus";
import { editorStrings as words } from "../strings/editor";
import { glass, text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

/** The editor's compact text (the game's sizes are for the couch). */
export const ui = {
  text: { fontSize: 14, fontWeight: "500", color: color.text, lineHeight: 1.25 },
  dim: { fontSize: 13, fontWeight: "500", color: color.textDim, lineHeight: 1.25 },
  caps: { ...text.caps, fontSize: 11, letterSpacing: 1.5 },
  title: { ...text.h3, fontSize: 18 },
} as const;

export const command = (c: EditorCommand) => bevy.editor.command({ command: c });
export const select = (item: EditorItem | null) => bevy.editor.select({ item });
// Opaque enough to read over a bright scene.
export const solid = { ...glass, borderRadius: radius.m, backgroundColor: "rgba(8, 14, 30, 0.9)" } as const;
export const panel = { ...solid, padding: space.s, gap: space.xs, flexDirection: "column" } as const;

export function Tool({ label, onPress, hot = false, disabled = false }: { label: string; onPress: () => void; hot?: boolean; disabled?: boolean }) {
  return (
    <button
      disabled={disabled}
      onClick={onPress}
      style={{
        height: 24,
        padding: { horizontal: space.s },
        alignItems: "center",
        justifyContent: "center",
        borderRadius: radius.s,
        border: 1,
        borderColor: hot ? color.voltGlow : color.glassBorder,
        backgroundColor: hot ? color.volt : color.glassHi,
      }}
      hoverStyle={{ borderColor: color.cyan }}
    >
      <text style={{ ...ui.text, fontWeight: "700", color: disabled ? color.textFaint : hot ? color.ink : color.text, lineBreak: "noWrap" }}>{label}</text>
    </button>
  );
}

/** One radio or checkbox option. */
function Chip({ label, on, dim = false, onPress }: { label: string; on: boolean; dim?: boolean; onPress: () => void }) {
  return (
    <button
      onClick={onPress}
      style={{
        height: 22,
        padding: { horizontal: 7 },
        alignItems: "center",
        borderRadius: radius.s,
        border: 1,
        borderColor: on ? color.cyan : color.line,
        backgroundColor: on ? color.cyanDeep : "rgba(4, 10, 24, 0.5)",
      }}
      hoverStyle={{ borderColor: color.cyan }}
    >
      <text style={{ ...ui.dim, color: on ? (dim ? color.textDim : color.text) : color.textDim, lineBreak: "noWrap" }}>{label}</text>
    </button>
  );
}

/** Radio chips: one of `options`. */
export function Radio({ value, options, onPick, dim }: { value: string; options: string[]; onPick: (v: string) => void; dim?: boolean }) {
  return (
    <node style={{ flexDirection: "row", flexWrap: "wrap", gap: 3, flexGrow: 1, flexShrink: 1 }}>
      {options.map((o) => (
        <Chip key={o} label={o} on={o === value} dim={dim} onPress={() => onPick(o)} />
      ))}
    </node>
  );
}

/** Checkbox chips: any of `options`, kept in their order. */
function Checks({ value, options, onChange }: { value: string[]; options: string[]; onChange: (v: string[]) => void }) {
  const flip = (o: string) => onChange(options.filter((x) => (x === o ? !value.includes(o) : value.includes(x))));
  return (
    <node style={{ flexDirection: "row", flexWrap: "wrap", gap: 3, flexGrow: 1, flexShrink: 1 }}>
      {options.map((o) => (
        <Chip key={o} label={o} on={value.includes(o)} onPress={() => flip(o)} />
      ))}
    </node>
  );
}

export function Switch({ value, onChange, dim = false }: { value: boolean; onChange: (v: boolean) => void; dim?: boolean }) {
  return (
    <button onClick={() => onChange(!value)} style={{ flexDirection: "row", alignItems: "center", gap: space.s }}>
      <node style={{ width: 32, height: 18, borderRadius: radius.pill, padding: 3, backgroundColor: value ? (dim ? "#1f6a88" : color.cyan) : "rgba(255, 255, 255, 0.12)" }}>
        <node style={{ width: 12, height: 12, borderRadius: radius.pill, backgroundColor: value && !dim ? color.ink : color.textDim, transform: { translateX: value ? 14 : 0 } }} />
      </node>
      <text style={ui.dim}>{value ? words.on : words.off}</text>
    </button>
  );
}

/** Slider position (0–1) of `v` and back; logarithmic ranges span orders of magnitude. */
const toT = (r: FieldRange, v: number) => (r.log ? Math.log(v / r.min) / Math.log(r.max / r.min) : (v - r.min) / (r.max - r.min || 1));
function fromT(r: FieldRange, t: number) {
  if (!r.log) return stepValue(r.min + t * (r.max - r.min), 0, r.min, r.max, r.step);
  const v = r.min * Math.pow(r.max / r.min, t);
  return Math.min(r.max, Math.max(r.min, Number(v.toPrecision(3))));
}

type BarProps = { label?: string; value: number; range: FieldRange | null; onCommit: (v: number, live: boolean) => void; integer?: boolean; dim?: boolean };

/** Digits a value shows: its step's, or up to 3 when unbounded. */
const digits = (r: FieldRange | null) => (r ? Math.max(0, Math.ceil(-Math.log10(r.step) - 1e-6)) : 3);

/** The bevy-react demos' slider: a bar filled to the value with "label value" centred on it. Over a
 *  range, click or drag sets the value; unbounded (no fill), dragging left / right moves it (0.1 per
 *  pixel, Shift 0.01). Each change is sent live, the release commits; out-of-range values (hand-edited
 *  files) show full or empty. */
function Bar({ label = "", value, range, onCommit, integer = false, dim = false }: BarProps) {
  const [draft, setDraft] = useState<number | null>(null);
  const [from, setFrom] = useState<{ x: number; v: number } | null>(null);
  const shown = draft ?? value;
  const pick = (e: PointerEventData) => {
    let v: number;
    if (range) v = fromT(range, Math.min(1, Math.max(0, e.x)));
    else if (from) {
      const raw = from.v + (e.clientX - from.x) * (integer ? 0.2 : e.shiftKey ? 0.01 : 0.1);
      v = integer ? Math.round(raw) : Math.round(raw * 1000) / 1000;
    } else return;
    if (v !== shown) onCommit(v, true);
    setDraft(v);
  };
  const t = range ? Math.min(1, Math.max(0, toT(range, shown))) : 0;
  const n = digits(range);
  const number = range ? shown.toFixed(n) : String(Number(shown.toFixed(n)));
  return (
    <node
      onPointerDown={(e) => (range ? pick(e) : setFrom({ x: e.clientX, v: value }))}
      onPointerMove={pick}
      onPointerUp={() => {
        // Always: the release also ends the drag in Bevy.
        if (draft !== null) onCommit(draft, false);
        setDraft(null);
        setFrom(null);
      }}
      style={{ flexGrow: 1, flexShrink: 1, flexBasis: 0, height: 20, borderRadius: radius.s, backgroundColor: "rgba(40, 52, 82, 0.85)", cursor: range ? "pointer" : "ewResize" }}
      hoverStyle={{ backgroundColor: "rgba(52, 66, 102, 0.9)" }}
    >
      <node
        style={{
          positionType: "absolute",
          left: 0,
          top: 0,
          width: `${t * 100}%`,
          height: "100%",
          borderRadius: radius.s,
          backgroundGradient: { type: "linear", angle: 180, stops: dim ? [{ color: "#3a5566" }, { color: "#2a3e4c" }] : [{ color: "#1f8ab8" }, { color: "#145e7e" }] },
        }}
      />
      <node style={{ positionType: "absolute", left: 0, top: 0, width: "100%", height: "100%", alignItems: "center", justifyContent: "center" }}>
        <text style={{ ...ui.dim, fontWeight: "600", color: dim ? color.textDim : color.text, lineBreak: "noWrap" }}>{label ? `${label} ${number}` : number}</text>
      </node>
    </node>
  );
}

/** Short integer spans (difficulty, laps) are radio chips. */
const chips = (field: EditorField) => field.value.type === "integer" && field.range !== null && field.range.max - field.range.min <= 9;

const COMPONENTS = { xyz: ["x", "y", "z"], rgb: ["r", "g", "b"] };
// ponytail: colours told by key; a new colour field outside this pattern gets x/y/z labels.
const isColor = (key: string) => /color|clouds_|window/.test(key);
const swatch = (v: number[]) => `rgb(${v.map((c) => Math.round(Math.min(1, Math.max(0, c)) * 255)).join(", ")})`;

/** One bar per component, side by side (and a swatch for colours). */
function Vector({ field, onSet }: { field: EditorField; onSet: (value: FieldValue, live: boolean) => void }) {
  if (field.value.type !== "vector") return null;
  const v = field.value.value;
  const rgb = isColor(field.key) && v.length === 3;
  const names = rgb ? COMPONENTS.rgb : COMPONENTS.xyz;
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: 3, flexGrow: 1, flexShrink: 1 }}>
      {v.map((c, i) => (
        <Bar key={i} label={names[i]} value={c} range={field.range} onCommit={(n, live) => onSet({ type: "vector", value: v.map((x, j) => (j === i ? n : x)) }, live)} />
      ))}
      {rgb && <node style={{ width: 14, height: 20, flexShrink: 0, borderRadius: radius.s, border: 1, borderColor: color.line, backgroundColor: swatch(v) }} />}
    </node>
  );
}

/** Inline text editing (the only typed fields). */
function TextInput({ value, dim, onSet }: { value: string; dim: boolean; onSet: (value: FieldValue) => void }) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState("");
  useEffect(() => {
    if (open) setDraft(value);
  }, [open]);
  useEvent("keyDown", (e) => {
    if (!open) return;
    if (e.key === "Escape") setOpen(false);
    if (e.key === "Enter") {
      onSet({ type: "text", value: draft });
      setOpen(false);
    }
  });
  if (open) return <editableText autofocus value={draft} onChange={setDraft} style={{ ...ui.text, ...valueBox(true) }} />;
  return (
    <button onClick={() => setOpen(true)} style={valueBox(false)} hoverStyle={{ borderColor: color.cyan }}>
      <text style={{ ...ui.text, color: dim ? color.textDim : color.text, lineBreak: "noWrap" }}>{value}</text>
    </button>
  );
}

/** One inspector row: the key, its control, and × for set optional fields. An unset one shows what it inherits, dimmed. */
export function Field({ field, onSet }: { field: EditorField; onSet: (value: FieldValue | null, live: boolean) => void }) {
  const { value } = field;
  const dim = field.optional && !field.set;
  const unset = field.optional && field.set && (
    <button onClick={() => onSet(null, false)} style={{ width: 20, height: 20, flexShrink: 0, alignItems: "center", justifyContent: "center", borderRadius: radius.s }} hoverStyle={{ backgroundColor: color.glassHi }}>
      <text style={ui.dim}>{words.unset}</text>
    </button>
  );
  let control;
  switch (value.type) {
    case "number":
    case "integer": {
      const r = field.range;
      const commit = (n: number, live = false) => onSet({ type: value.type, value: n }, live);
      if (chips(field) && r) {
        const options = Array.from({ length: r.max - r.min + 1 }, (_, i) => String(r.min + i));
        control = <Radio value={String(value.value)} options={options} dim={dim} onPick={(v) => commit(Number(v))} />;
      } else {
        control = <Bar value={value.value} range={r} integer={value.type === "integer"} dim={dim} onCommit={commit} />;
      }
      break;
    }
    case "toggle":
      control = <Switch value={value.value} dim={dim} onChange={(v) => onSet({ type: "toggle", value: v }, false)} />;
      break;
    case "choice":
      control = <Radio value={value.value} options={value.options} dim={dim} onPick={(v) => onSet({ type: "choice", value: v, options: [] }, false)} />;
      break;
    case "flags":
      control = <Checks value={value.value} options={value.options} onChange={(v) => onSet({ type: "flags", value: v, options: [] }, false)} />;
      break;
    case "vector":
      control = <Vector field={field} onSet={onSet} />;
      break;
    case "text":
      control = <TextInput value={value.value} dim={dim} onSet={(v) => onSet(v, false)} />;
      break;
  }
  return (
    <node style={{ flexDirection: "row", alignItems: "flexStart", gap: space.s }}>
      <text style={{ ...ui.dim, color: dim ? color.textFaint : color.textDim, width: 108, flexShrink: 0, padding: { top: 3 } }}>{field.key}</text>
      {control}
      {unset}
    </node>
  );
}

function valueBox(open: boolean) {
  return {
    flexGrow: 1,
    flexShrink: 1,
    minHeight: 22,
    padding: { horizontal: space.s, vertical: 2 },
    borderRadius: radius.s,
    border: 1,
    borderColor: open ? color.cyan : color.line,
    backgroundColor: "rgba(4, 10, 24, 0.5)",
  } as const;
}
