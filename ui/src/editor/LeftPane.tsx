// The track editor's left pane: Assets (the asset library, `Library.tsx`), Track (track settings,
// add jump / ground area / shortcut, lints) and Lighting (the previewed time's look and the room's
// lamp grid, probes, bake). The open tab lives in Bevy (`editor.tab`): the Track tab shows the
// layout handles.
import { useState } from "react";
import type { BakeQuality, EditorAdd, EditorStatus, EditorTab } from "../bevy";
import { bevy } from "../bevy";
import { editorStrings as words } from "../strings/editor";
import { color, space } from "../theme/tokens";
import { command, Field, panel, select, Tool, ui } from "./controls";
import { Library } from "./Library";

const TABS: EditorTab[] = ["assets", "track", "lighting"];
const add = (what: EditorAdd) => command({ kind: "add", what });

export function LeftPane({ status }: { status: EditorStatus }) {
  return (
    <node style={{ ...panel, width: 320, maxHeight: "100%", overflowY: "scroll" }} hoverStyle={{}}>
      <node style={{ flexDirection: "row", gap: space.xs }}>
        {TABS.map((tab) => (
          <Tool key={tab} label={words.tabs[tab]} hot={status.tab === tab} onPress={() => bevy.editor.tab({ tab })} />
        ))}
      </node>
      {status.tab === "assets" && <Library status={status} />}
      {status.tab === "track" && <Track status={status} />}
      {status.tab === "lighting" && <Lighting status={status} />}
      <text style={{ ...ui.dim, color: color.textFaint, margin: { top: space.s } }}>{words.controls}</text>
    </node>
  );
}

function Header({ label, tone = color.textDim }: { label: string; tone?: string }) {
  return <text style={{ ...ui.caps, color: tone, margin: { top: space.xs } }}>{label}</text>;
}

function Track({ status }: { status: EditorStatus }) {
  const lints = status.lints;
  return (
    <>
      <Tool label={words.track} hot={status.selection?.item.kind === "track"} onPress={() => select({ kind: "track" })} />
      <Header label={words.addHeader} />
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.xs }}>
        <Tool label={words.addGap} onPress={() => add("gap")} />
        <Tool label={words.addGround} onPress={() => add("ground")} />
        <Tool label={words.addShortcut} onPress={() => add("shortcut")} />
      </node>
      <text style={ui.dim}>{words.trackHint}</text>
      {/* Lints name a distance along the track, not an item: text only. */}
      <Header label={lints.length > 0 ? words.lints(lints.length) : words.noLints} tone={lints.length > 0 ? color.warning : color.textDim} />
      {lints.map((line, i) => (
        <text key={i} style={ui.dim}>
          {line}
        </text>
      ))}
    </>
  );
}

function Lighting({ status }: { status: EditorStatus }) {
  const baking = status.baking !== null;
  const look = status.look;
  const [quality, setQuality] = useState<BakeQuality>("low");
  return (
    <>
      <text style={ui.text}>{words.previewing(status.options.time)}</text>
      <text style={ui.dim}>{words.lightingHint}</text>
      {look !== null && (
        <>
          <Header label={look.title} />
          {look.fields.map((f) => (
            <Field
              key={`look.${f.key}`}
              field={f}
              onSet={(value, live) => bevy.editor.set({ item: look.item, key: f.key, value, live })}
            />
          ))}
        </>
      )}
      <Header label={words.bakeHeader} />
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: space.xs }}>
        <Tool label={words.probes(status.probes)} hot={status.probes === false} onPress={() => command({ kind: "toggleProbes" })} disabled={status.probes === null} />
        <Tool label={words.bakeQuality(quality)} hot={quality === "high"} onPress={() => setQuality(quality === "low" ? "high" : "low")} disabled={baking} />
        <Tool label={words.bake} onPress={() => command({ kind: "bake", all: false, quality })} disabled={baking} />
        <Tool label={words.bakeAll} onPress={() => command({ kind: "bake", all: true, quality })} disabled={baking} />
      </node>
      {status.baking !== null && <text style={{ ...ui.dim, color: color.cyan }}>{status.baking}</text>}
      {status.staleBake.length > 0 && <text style={{ ...ui.dim, color: color.warning }}>{words.staleVolumes(status.staleBake)}</text>}
    </>
  );
}
