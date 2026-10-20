// The track editor's screen while editing (rc_editor): the toolbar (save state, undo/redo, save,
// session time and weather, test-drive layout, test drive), the item list (one section open at a
// time, with its handle layer and an Add button), the inspector of the selection, and the notes
// (errors, build warnings, controls). Test drives show the game's race UI instead.
import { useState } from "react";

import type { EditorAdd, EditorCommand, EditorItem, EditorStatus } from "../bevy";
import { bevy } from "../bevy";
import { editorStrings as words } from "../strings/editor";
import { glass, text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";
import { Cycle, Field, Tool } from "./controls";

const command = (c: EditorCommand) => bevy.editor.command({ command: c });
const select = (item: EditorItem | null) => bevy.editor.select({ item });
// Opaque enough to read over a bright scene.
const solid = { ...glass, backgroundColor: "rgba(8, 14, 30, 0.9)" } as const;
const panel = { ...solid, padding: space.m, gap: space.s, flexDirection: "column" } as const;

export function EditorLayer({ status }: { status: EditorStatus }) {
  return (
    <node style={{ width: "100%", height: "100%", flexDirection: "column", padding: space.m, gap: space.m }}>
      <Toolbar status={status} />
      <node style={{ flexGrow: 1, flexDirection: "row", justifyContent: "spaceBetween", alignItems: "flexStart", minHeight: 0 }}>
        <Items status={status} />
        <Inspector status={status} />
      </node>
      <Notes status={status} />
    </node>
  );
}

function Toolbar({ status }: { status: EditorStatus }) {
  const o = status.options;
  const setOptions = (change: Partial<typeof o>) => bevy.editor.options({ options: { ...o, ...change } });
  const state = status.baking ?? (status.loading ? words.rebuilding : status.dirty ? words.unsaved : words.saved);
  return (
    <node style={{ ...solid, flexDirection: "row", alignItems: "center", gap: space.s, padding: { horizontal: space.l, vertical: space.s } }} hoverStyle={{}}>
      <node style={{ flexDirection: "column", flexGrow: 1 }}>
        <text style={text.caps}>{words.title}</text>
        <text style={text.h3}>{status.name}</text>
      </node>
      <text style={{ ...text.label, color: status.baking !== null ? color.cyan : status.dirty || status.loading ? color.warning : color.textDim, margin: { right: space.m } }}>{state}</text>
      <Tool label={words.undo} onPress={() => command({ kind: "undo" })} disabled={!status.canUndo} />
      <Tool label={words.redo} onPress={() => command({ kind: "redo" })} disabled={!status.canRedo} />
      <Tool label={words.save} onPress={() => command({ kind: "save" })} />
      <Tool label={words.bake} onPress={() => command({ kind: "bake", all: false })} disabled={status.baking !== null} />
      <Tool label={words.bakeAll} onPress={() => command({ kind: "bake", all: true })} disabled={status.baking !== null} />
      <Tool label={words.probes(status.probes)} hot={status.probes === false} onPress={() => command({ kind: "toggleProbes" })} disabled={status.probes === null} />
      <node style={{ width: space.m }} />
      <Cycle label={words.time} value={o.time} options={status.times} onChange={(time) => setOptions({ time })} />
      <Cycle label={words.weather} value={o.weather} options={status.weathers} onChange={(weather) => setOptions({ weather })} />
      <text style={{ ...text.small, margin: { left: space.s } }}>{words.driveAs}</text>
      <Tool label={words.reverse} hot={o.reverse} onPress={() => setOptions({ reverse: !o.reverse })} />
      <Tool label={words.mirror} hot={o.mirror} onPress={() => setOptions({ mirror: !o.mirror })} />
      <Tool label={words.drive} hot onPress={() => command({ kind: "drive" })} />
    </node>
  );
}

type Section = { key: keyof typeof words.sections; count: number; layer?: string; add?: EditorAdd; item: (i: number) => EditorItem };

function sections(s: EditorStatus): Section[] {
  const c = s.counts;
  return [
    { key: "points", count: c.points, layer: "points", item: (index) => ({ kind: "point", index }) },
    { key: "gaps", count: c.gaps, add: "gap", item: (index) => ({ kind: "gap", index }) },
    { key: "ground", count: c.ground, layer: "ground", add: "ground", item: (index) => ({ kind: "ground", index }) },
    { key: "rows", count: c.itemRows, layer: "rows", add: "itemRow", item: (index) => ({ kind: "itemRow", index }) },
    { key: "props", count: c.props, layer: "props", add: "prop", item: (index) => ({ kind: "prop", index }) },
    { key: "shortcuts", count: c.shortcuts, layer: "shortcuts", add: "shortcut", item: (index) => ({ kind: "shortcut", index }) },
    { key: "decor", count: c.decor, layer: "decor", add: "decor", item: (index) => ({ kind: "decor", index }) },
    { key: "battery", count: c.battery ? 1 : 0, add: c.battery ? undefined : "battery", item: () => ({ kind: "battery" }) },
  ];
}

/** The index a selection highlights in a section's chips. */
function selectedIndex(item: EditorItem | undefined, key: Section["key"]): number | null {
  if (!item) return null;
  switch (item.kind) {
    case "point":
      return key === "points" ? item.index : null;
    case "gap":
      return key === "gaps" ? item.index : null;
    case "ground":
      return key === "ground" ? item.index : null;
    case "vertex":
      return key === "ground" ? item.ground : null;
    case "itemRow":
      return key === "rows" ? item.index : null;
    case "prop":
      return key === "props" ? item.index : null;
    case "shortcut":
      return key === "shortcuts" ? item.index : null;
    case "pathPoint":
      return key === "shortcuts" ? item.shortcut : null;
    case "decor":
      return key === "decor" ? item.index : null;
    case "battery":
      return key === "battery" ? 0 : null;
    default:
      return null;
  }
}

function Items({ status }: { status: EditorStatus }) {
  const [open, setOpen] = useState<Section["key"] | null>("points");
  const selected = status.selection?.item;
  return (
    <node style={{ ...panel, width: 330, maxHeight: "100%", overflowY: "scroll" }} hoverStyle={{}}>
      <Tool label={words.track} hot={selected?.kind === "track"} onPress={() => select({ kind: "track" })} />
      <text style={{ ...text.caps, margin: { top: space.s } }}>{words.items}</text>
      {sections(status).map((s) => {
        const shown = s.layer !== undefined && status.layers.includes(s.layer);
        const hot = selectedIndex(selected, s.key);
        return (
          <node key={s.key} style={{ flexDirection: "column", gap: space.xs }}>
            <node style={{ flexDirection: "row", alignItems: "center", gap: space.xs }}>
              <button onClick={() => setOpen(open === s.key ? null : s.key)} style={{ flexGrow: 1, padding: { vertical: 4 } }}>
                <text style={{ ...text.label, color: hot !== null ? color.volt : open === s.key ? color.cyan : color.text }}>{`${words.sections[s.key]}  ${s.count}`}</text>
              </button>
              {s.layer !== undefined && (
                <button onClick={() => command({ kind: "toggleLayer", layer: s.layer! })} style={{ width: 70, padding: { horizontal: space.xs } }}>
                  <text style={{ ...text.small, color: shown ? color.cyan : color.textFaint }}>{shown ? words.show : words.hide}</text>
                </button>
              )}
              {s.add !== undefined && <Tool label={words.add} onPress={() => command({ kind: "add", what: s.add! })} />}
            </node>
            {open === s.key && s.count > 0 && (
              <node style={{ flexDirection: "row", flexWrap: "wrap", gap: 4 }}>
                {Array.from({ length: s.count }, (_, i) => (
                  <button
                    key={i}
                    onClick={() => select(s.item(i))}
                    style={{ minWidth: 34, height: 28, padding: { horizontal: 4 }, alignItems: "center", justifyContent: "center", borderRadius: radius.s, backgroundColor: hot === i ? color.volt : color.glassHi }}
                    hoverStyle={{ borderColor: color.cyan, border: 1 }}
                  >
                    <text style={{ ...text.small, color: hot === i ? color.ink : color.text }}>{String(i)}</text>
                  </button>
                ))}
              </node>
            )}
          </node>
        );
      })}
    </node>
  );
}

function Inspector({ status }: { status: EditorStatus }) {
  const [active, setActive] = useState<string | null>(null);
  const sel = status.selection;
  if (!sel) {
    return (
      <node style={{ ...panel, width: 520 }} hoverStyle={{}}>
        <text style={text.body}>{words.none}</text>
      </node>
    );
  }
  const kind = sel.item.kind;
  const insertable = kind === "point" || kind === "vertex" || kind === "pathPoint";
  const duplicable = ["prop", "itemRow", "ground", "vertex", "shortcut", "pathPoint", "decor"].includes(kind);
  return (
    <node style={{ ...panel, width: 520, maxHeight: "100%", overflowY: "scroll" }} hoverStyle={{}}>
      <text style={text.h3}>{sel.title}</text>
      {sel.fields.map((f) => (
        <Field
          key={`${kind}.${f.key}`}
          field={f}
          active={active === f.key}
          onActivate={(open) => setActive(open ? f.key : null)}
          onSet={(value) => bevy.editor.set({ key: f.key, value })}
        />
      ))}
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.s, margin: { top: space.s } }}>
        {insertable && <Tool label={words.insert} onPress={() => command({ kind: "insert" })} />}
        {kind === "point" && <Tool label={words.addGap} onPress={() => command({ kind: "add", what: "gap" })} />}
        {duplicable && <Tool label={words.duplicate} onPress={() => command({ kind: "duplicate" })} />}
        <Tool label={words.focus} onPress={() => command({ kind: "focus" })} />
        {kind !== "track" && <Tool label={words.remove} onPress={() => command({ kind: "delete" })} />}
      </node>
    </node>
  );
}

function Notes({ status }: { status: EditorStatus }) {
  return (
    <node style={{ flexDirection: "column", alignItems: "flexStart", gap: space.s }}>
      {status.error !== null && <Note tone={color.danger} lines={[status.error]} />}
      {status.lints.length > 0 && <Note tone={color.warning} title={words.lints(status.lints.length)} lines={status.lints} />}
      <node style={{ ...solid, padding: { horizontal: space.m, vertical: space.xs } }}>
        <text style={text.small}>{words.controls}</text>
      </node>
    </node>
  );
}

function Note({ tone, title, lines }: { tone: string; title?: string; lines: string[] }) {
  return (
    <node style={{ ...solid, flexDirection: "column", gap: space.xs, padding: space.m, maxWidth: 1100, borderColor: tone }}>
      {title !== undefined && <text style={{ ...text.caps, color: tone }}>{title}</text>}
      {lines.map((line, i) => (
        <text key={i} style={text.small}>
          {line}
        </text>
      ))}
    </node>
  );
}
