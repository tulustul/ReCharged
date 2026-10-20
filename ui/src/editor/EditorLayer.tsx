// The track editor's screen while editing (rc_editor): the toolbar (name, a stale bake, save
// state, undo/redo, save, session time and weather, test-drive layout, test drive), the left pane
// (tabs), the error toast under the toolbar, and the right pane (the selection's fields and
// actions; none when nothing is selected). Test drives show the game's race UI instead.
import { useState } from "react";

import type { EditorStatus } from "../bevy";
import { bevy } from "../bevy";
import { editorStrings as words } from "../strings/editor";
import { text } from "../theme/styles";
import { color, space } from "../theme/tokens";
import { command, Cycle, Field, panel, select, solid, Tool } from "./controls";
import { LeftPane } from "./LeftPane";

export function EditorLayer({ status }: { status: EditorStatus }) {
  return (
    <node style={{ width: "100%", height: "100%", flexDirection: "column", padding: space.m, gap: space.m }}>
      <Toolbar status={status} />
      <node style={{ flexGrow: 1, flexDirection: "row", alignItems: "flexStart", gap: space.m, minHeight: 0 }}>
        <LeftPane status={status} />
        <node style={{ flexGrow: 1, flexDirection: "column", alignItems: "center" }}>
          {status.error !== null && (
            <node style={{ ...solid, padding: { horizontal: space.m, vertical: space.s }, maxWidth: 700, borderColor: color.danger }}>
              <text style={{ ...text.small, color: color.text }}>{status.error}</text>
            </node>
          )}
        </node>
        {status.selection !== null && <Inspector status={status} />}
      </node>
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
      {status.staleBake.length > 0 && status.baking === null && (
        <text style={{ ...text.label, color: color.warning, margin: { right: space.m } }}>{words.bakeStale}</text>
      )}
      <text style={{ ...text.label, color: status.baking !== null ? color.cyan : status.dirty || status.loading ? color.warning : color.textDim, margin: { right: space.m } }}>{state}</text>
      <Tool label={words.undo} onPress={() => command({ kind: "undo" })} disabled={!status.canUndo} />
      <Tool label={words.redo} onPress={() => command({ kind: "redo" })} disabled={!status.canRedo} />
      <Tool label={words.save} onPress={() => command({ kind: "save" })} />
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

function Inspector({ status }: { status: EditorStatus }) {
  const [active, setActive] = useState<string | null>(null);
  const sel = status.selection;
  if (!sel) return null;
  const item = sel.item;
  const kind = item.kind;
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
        {kind === "point" && <Tool label={words.jumpHere} onPress={() => command({ kind: "add", what: "gap" })} />}
        {item.kind === "vertex" && <Tool label={words.wholeArea} onPress={() => select({ kind: "ground", index: item.ground })} />}
        {item.kind === "pathPoint" && <Tool label={words.wholeShortcut} onPress={() => select({ kind: "shortcut", index: item.shortcut })} />}
        {duplicable && <Tool label={words.duplicate} onPress={() => command({ kind: "duplicate" })} />}
        <Tool label={words.focus} onPress={() => command({ kind: "focus" })} />
        {kind !== "track" && <Tool label={words.remove} onPress={() => command({ kind: "delete" })} />}
      </node>
    </node>
  );
}
