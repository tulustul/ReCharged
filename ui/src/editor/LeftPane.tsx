// The track editor's left pane: Assets (add props, decor, item rows, the battery until the library
// exists), Track (track settings, add jump / ground area / shortcut, lints) and Lighting (probes,
// bake). The open tab lives in Bevy (`editor.tab`): the Track tab shows the layout handles.
import type { EditorAdd, EditorStatus, EditorTab } from "../bevy";
import { bevy } from "../bevy";
import { editorStrings as words } from "../strings/editor";
import { text } from "../theme/styles";
import { color, space } from "../theme/tokens";
import { command, panel, select, Tool } from "./controls";

const TABS: EditorTab[] = ["assets", "track", "lighting"];
const add = (what: EditorAdd) => command({ kind: "add", what });

export function LeftPane({ status }: { status: EditorStatus }) {
  return (
    <node style={{ ...panel, width: 330, maxHeight: "100%", overflowY: "scroll" }} hoverStyle={{}}>
      <node style={{ flexDirection: "row", gap: space.xs }}>
        {TABS.map((tab) => (
          <Tool key={tab} label={words.tabs[tab]} hot={status.tab === tab} onPress={() => bevy.editor.tab({ tab })} />
        ))}
      </node>
      {status.tab === "assets" && <Assets status={status} />}
      {status.tab === "track" && <Track status={status} />}
      {status.tab === "lighting" && <Lighting status={status} />}
      <text style={{ ...text.small, color: color.textFaint, margin: { top: space.m } }}>{words.controls}</text>
    </node>
  );
}

function Header({ label, tone = color.textDim }: { label: string; tone?: string }) {
  return <text style={{ ...text.caps, color: tone, margin: { top: space.s } }}>{label}</text>;
}

function Assets({ status }: { status: EditorStatus }) {
  return (
    <>
      <text style={text.small}>{words.library}</text>
      <Header label={words.addHeader} />
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.s }}>
        <Tool label={words.addProp} onPress={() => add("prop")} />
        <Tool label={words.addDecor} onPress={() => add("decor")} />
        <Tool label={words.addRow} onPress={() => add("itemRow")} />
        <Tool label={words.addBattery} onPress={() => add("battery")} disabled={status.counts.battery} />
      </node>
    </>
  );
}

function Track({ status }: { status: EditorStatus }) {
  const lints = status.lints;
  return (
    <>
      <Tool label={words.track} hot={status.selection?.item.kind === "track"} onPress={() => select({ kind: "track" })} />
      <Header label={words.addHeader} />
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.s }}>
        <Tool label={words.addGap} onPress={() => add("gap")} />
        <Tool label={words.addGround} onPress={() => add("ground")} />
        <Tool label={words.addShortcut} onPress={() => add("shortcut")} />
      </node>
      <text style={text.small}>{words.trackHint}</text>
      {/* Lints name a distance along the track, not an item: text only. */}
      <Header label={lints.length > 0 ? words.lints(lints.length) : words.noLints} tone={lints.length > 0 ? color.warning : color.textDim} />
      {lints.map((line, i) => (
        <text key={i} style={text.small}>
          {line}
        </text>
      ))}
    </>
  );
}

function Lighting({ status }: { status: EditorStatus }) {
  const baking = status.baking !== null;
  return (
    <>
      <text style={text.label}>{words.previewing(status.options.time)}</text>
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.s }}>
        <Tool label={words.probes(status.probes)} hot={status.probes === false} onPress={() => command({ kind: "toggleProbes" })} disabled={status.probes === null} />
        <Tool label={words.bake} onPress={() => command({ kind: "bake", all: false })} disabled={baking} />
        <Tool label={words.bakeAll} onPress={() => command({ kind: "bake", all: true })} disabled={baking} />
      </node>
      {status.baking !== null && <text style={{ ...text.small, color: color.cyan }}>{status.baking}</text>}
      {status.staleBake.length > 0 && <text style={{ ...text.small, color: color.warning }}>{words.staleVolumes(status.staleBake)}</text>}
    </>
  );
}
