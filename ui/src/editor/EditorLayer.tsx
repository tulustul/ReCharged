// The track editor's screen while editing (rc_editor): the toolbar (name, a stale bake, save
// state, undo/redo, save, session time and weather, test-drive layout, test drive), the left pane
// (tabs), the error toast under the toolbar, and the right pane (the selection's fields and
// actions; none when nothing is selected). Test drives show the game's race UI instead.
import type { EditorStatus } from "../bevy";
import { bevy } from "../bevy";
import { editorStrings as words } from "../strings/editor";
import { color, space } from "../theme/tokens";
import { command, Field, panel, Radio, select, solid, Tool, ui } from "./controls";
import { LeftPane } from "./LeftPane";
import { SwapPicker } from "./Library";

export function EditorLayer({ status }: { status: EditorStatus }) {
  return (
    <node style={{ width: "100%", height: "100%", flexDirection: "column", padding: space.s, gap: space.s }}>
      <Toolbar status={status} />
      <node style={{ flexGrow: 1, flexDirection: "row", alignItems: "flexStart", gap: space.s, minHeight: 0 }}>
        <LeftPane status={status} />
        <node style={{ flexGrow: 1, flexDirection: "column", alignItems: "center" }}>
          {status.error !== null && (
            // hoverStyle: the toast owns the pointer (a placing click on it does not place).
            <node style={{ ...solid, padding: { horizontal: space.m, vertical: space.s }, maxWidth: 700, borderColor: color.danger }} hoverStyle={{}}>
              <text style={ui.text}>{status.error}</text>
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
    <node style={{ ...solid, flexDirection: "row", alignItems: "center", gap: space.xs, padding: { horizontal: space.m, vertical: space.xs } }} hoverStyle={{}}>
      <node style={{ flexDirection: "row", alignItems: "center", gap: space.s, flexGrow: 1 }}>
        <text style={ui.caps}>{words.title}</text>
        <text style={ui.title}>{status.name}</text>
      </node>
      {status.staleBake.length > 0 && status.baking === null && (
        <text style={{ ...ui.text, color: color.warning, margin: { right: space.s } }}>{words.bakeStale}</text>
      )}
      <text style={{ ...ui.text, color: status.baking !== null ? color.cyan : status.dirty || status.loading ? color.warning : color.textDim, margin: { right: space.s } }}>{state}</text>
      <Tool label={words.undo} onPress={() => command({ kind: "undo" })} disabled={!status.canUndo} />
      <Tool label={words.redo} onPress={() => command({ kind: "redo" })} disabled={!status.canRedo} />
      <Tool label={words.save} onPress={() => command({ kind: "save" })} />
      <node style={{ width: space.m }} />
      <Radio value={o.time} options={status.times} onPick={(time) => setOptions({ time })} />
      <node style={{ width: space.s }} />
      <Radio value={o.weather} options={status.weathers} onPick={(weather) => setOptions({ weather })} />
      <text style={{ ...ui.dim, margin: { left: space.s } }}>{words.driveAs}</text>
      <Tool label={words.reverse} hot={o.reverse} onPress={() => setOptions({ reverse: !o.reverse })} />
      <Tool label={words.mirror} hot={o.mirror} onPress={() => setOptions({ mirror: !o.mirror })} />
      <Tool label={words.drive} hot onPress={() => command({ kind: "drive" })} />
    </node>
  );
}

function Inspector({ status }: { status: EditorStatus }) {
  const sel = status.selection;
  if (!sel) return null;
  const item = sel.item;
  const kind = item.kind;
  const insertable = kind === "point" || kind === "vertex" || kind === "pathPoint";
  const duplicable = ["prop", "itemRow", "ground", "vertex", "shortcut", "pathPoint", "decor", "light"].includes(kind);
  // The track, a time's sun and its look always exist.
  const deletable = kind !== "track" && kind !== "sun" && kind !== "look";
  return (
    <node style={{ ...panel, width: 400, maxHeight: "100%", overflowY: "scroll" }} hoverStyle={{}}>
      <text style={ui.title}>{sel.title}</text>
      {sel.asset !== null && <SwapPicker key={JSON.stringify(item)} asset={sel.asset} />}
      {sel.fields.map((f) => (
        <Field
          key={`${kind}.${f.key}`}
          field={f}
          onSet={(value, live) => bevy.editor.set({ item: null, key: f.key, value, live })}
        />
      ))}
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.xs, margin: { top: space.xs } }}>
        {insertable && <Tool label={words.insert} onPress={() => command({ kind: "insert" })} />}
        {kind === "point" && <Tool label={words.jumpHere} onPress={() => command({ kind: "add", what: "gap" })} />}
        {item.kind === "vertex" && <Tool label={words.wholeArea} onPress={() => select({ kind: "ground", index: item.ground })} />}
        {item.kind === "pathPoint" && <Tool label={words.wholeShortcut} onPress={() => select({ kind: "shortcut", index: item.shortcut })} />}
        {duplicable && <Tool label={words.duplicate} onPress={() => command({ kind: "duplicate" })} />}
        <Tool label={words.focus} onPress={() => command({ kind: "focus" })} />
        {deletable && <Tool label={words.remove} onPress={() => command({ kind: "delete" })} />}
      </node>
    </node>
  );
}
