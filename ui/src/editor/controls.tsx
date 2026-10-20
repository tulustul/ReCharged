// Small controls of the track editor: a tool button and one inspector field per `FieldValue` type.
// Text and number fields edit inline: click the value, type, Enter commits, Escape cancels.
import { useEffect, useState } from "react";

import type { EditorField, FieldValue } from "../bevy";
import { useEvent } from "../hooks";
import { editorStrings as words } from "../strings/editor";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

const fieldText = { ...text.small, color: color.text };

export function Tool({ label, onPress, hot = false, disabled = false }: { label: string; onPress: () => void; hot?: boolean; disabled?: boolean }) {
  return (
    <button
      disabled={disabled}
      onClick={onPress}
      style={{
        height: 36,
        padding: { horizontal: space.m },
        alignItems: "center",
        justifyContent: "center",
        borderRadius: radius.s,
        border: 1,
        borderColor: hot ? color.voltGlow : color.glassBorder,
        backgroundColor: hot ? color.volt : color.glassHi,
      }}
      hoverStyle={{ borderColor: color.cyan }}
    >
      <text style={{ ...text.small, fontWeight: "700", color: disabled ? color.textFaint : hot ? color.ink : color.text, lineBreak: "noWrap" }}>{label}</text>
    </button>
  );
}

/** Cycles through `options` on click (short lists: time, weather). */
export function Cycle({ label, value, options, onChange }: { label: string; value: string; options: string[]; onChange: (v: string) => void }) {
  const next = options[(options.indexOf(value) + 1) % Math.max(options.length, 1)] ?? value;
  return <Tool label={`${label}: ${value}`} onPress={() => onChange(next)} disabled={options.length < 2} />;
}

const round = (v: number) => String(Math.round(v * 1000) / 1000);

function format(value: FieldValue): string {
  switch (value.type) {
    case "number":
    case "integer":
      return round(value.value);
    case "vector":
      return value.value.map(round).join(", ");
    case "text":
    case "choice":
      return value.value;
    case "toggle":
      return value.value ? words.on : words.off;
  }
}

/** Parses typed text back into the field's type; `null` when it does not parse. */
function parse(value: FieldValue, typed: string): FieldValue | null {
  const num = (s: string) => (s.trim() === "" ? NaN : Number(s));
  switch (value.type) {
    case "number":
    case "integer": {
      const n = num(typed);
      return Number.isFinite(n) ? { type: value.type, value: value.type === "integer" ? Math.round(n) : n } : null;
    }
    case "vector": {
      const parts = typed.split(/[,\s]+/).filter((p) => p !== "").map(num);
      return parts.length === value.value.length && parts.every(Number.isFinite) ? { type: "vector", value: parts } : null;
    }
    case "text":
      return { type: "text", value: typed };
    default:
      return null;
  }
}

type FieldProps = {
  field: EditorField;
  /** This field's inline editor is open. */
  active: boolean;
  onActivate: (open: boolean) => void;
  onSet: (value: FieldValue | null) => void;
};

/** One inspector row: the key, the value (click to edit / toggle / pick), and × for set optional fields. */
export function Field({ field, active, onActivate, onSet }: FieldProps) {
  const { value } = field;
  const [draft, setDraft] = useState("");
  const [bad, setBad] = useState(false);
  useEffect(() => {
    if (active) {
      setDraft(format(value));
      setBad(false);
    }
  }, [active]);
  useEvent("keyDown", (e) => {
    if (!active) return;
    if (e.key === "Escape") onActivate(false);
    if (e.key === "Enter") {
      const parsed = parse(value, draft);
      if (parsed === null) return setBad(true);
      onSet(parsed);
      onActivate(false);
    }
  });
  const dim = field.optional && !field.set;
  const shown = (
    <text style={{ ...fieldText, color: dim ? color.textDim : color.text, lineBreak: "noWrap" }}>
      {format(value)}
      {dim && value.type !== "text" ? `  (${words.inherited})` : ""}
    </text>
  );
  let editor;
  if (value.type === "toggle") {
    editor = (
      <button onClick={() => onSet({ type: "toggle", value: !value.value })} style={valueBox(false)} hoverStyle={{ borderColor: color.cyan }}>
        {shown}
      </button>
    );
  } else if (value.type === "choice") {
    editor = (
      <node style={{ flexDirection: "column", flexGrow: 1, gap: space.xs }}>
        <button onClick={() => onActivate(!active)} style={valueBox(active)} hoverStyle={{ borderColor: color.cyan }}>
          {shown}
        </button>
        {active && (
          <node style={{ maxHeight: 260, flexDirection: "column", overflowY: "scroll", gap: 2 }}>
            {value.options.map((option) => (
              <button
                key={option}
                onClick={() => {
                  onSet({ type: "choice", value: option, options: [] });
                  onActivate(false);
                }}
                style={{ padding: { horizontal: space.s, vertical: 2 }, borderRadius: radius.s, backgroundColor: option === value.value ? color.cyanDeep : color.clear }}
                hoverStyle={{ backgroundColor: color.glassHi }}
              >
                <text style={fieldText}>{option}</text>
              </button>
            ))}
          </node>
        )}
      </node>
    );
  } else if (active) {
    editor = (
      <editableText
        autofocus
        value={draft}
        onChange={(d) => {
          setDraft(d);
          setBad(false);
        }}
        style={{ ...fieldText, ...valueBox(true), borderColor: bad ? color.danger : color.cyan }}
      />
    );
  } else {
    editor = (
      <button onClick={() => onActivate(true)} style={valueBox(false)} hoverStyle={{ borderColor: color.cyan }}>
        {shown}
      </button>
    );
  }
  return (
    <node style={{ flexDirection: "row", alignItems: "flexStart", gap: space.s }}>
      <text style={{ ...text.small, color: color.text, width: 150, flexShrink: 0, padding: { top: 4 } }}>{field.key}</text>
      {editor}
      {field.optional && field.set && (
        <button onClick={() => onSet(null)} style={{ width: 30, height: 30, alignItems: "center", justifyContent: "center" }} hoverStyle={{ backgroundColor: color.glassHi }}>
          <text style={{ ...fieldText, color: color.textDim }}>{words.unset}</text>
        </button>
      )}
    </node>
  );
}

function valueBox(open: boolean) {
  return {
    flexGrow: 1,
    flexShrink: 1,
    minHeight: 30,
    padding: { horizontal: space.s, vertical: 3 },
    borderRadius: radius.s,
    border: 1,
    borderColor: open ? color.cyan : color.line,
    backgroundColor: "rgba(4, 10, 24, 0.5)",
  } as const;
}
