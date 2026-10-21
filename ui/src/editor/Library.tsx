// The track editor's asset library (the Assets tab): a search box and the groups of `editor.library`
// (the track's own environment expanded). Pressing an entry arms placement mode in Bevy (a click
// in the view, or a drag released there, places it; the same entry again or Esc cancels); "Replace"
// swaps the selected prop or decor item for the entry. `SwapPicker` is the right pane's list of the
// selection's own type.
import { useState } from "react";

import type { EditorAsset, EditorStatus, LibraryEntry, LibraryGroup } from "../bevy";
import { bevy } from "../bevy";
import { useStore } from "../state/useStore";
import { editorLibrary } from "../state/stores";
import { editorStrings as words } from "../strings/editor";
import { color, radius, space } from "../theme/tokens";
import { command, Tool, ui } from "./controls";
import { assetName, filterGroups, libraryQuery, sameAsset } from "./search";

const swap = (asset: EditorAsset) => command({ kind: "swap", asset });

export function Library({ status }: { status: EditorStatus }) {
  const library = useStore(editorLibrary);
  const query = useStore(libraryQuery);
  const [open, setOpen] = useState<Record<string, boolean>>({});
  if (!library) return null;
  const searching = query.trim() !== "";
  return (
    <>
      <text style={ui.dim}>{words.libraryHint}</text>
      <Search value={query} onChange={(q) => libraryQuery.set(q)} />
      {filterGroups(library.groups, query).map((g) => {
        const shown = searching || (open[g.title] ?? g.open);
        return (
          <node key={g.title} style={{ flexDirection: "column", gap: 2 }}>
            <button onClick={() => setOpen({ ...open, [g.title]: !shown })} style={{ padding: { vertical: 2 } }} hoverStyle={{ backgroundColor: color.glassHi }}>
              <text style={{ ...ui.caps, color: color.textDim }}>{`${shown ? "-" : "+"} ${g.title} (${g.entries.length})`}</text>
            </button>
            {shown && g.entries.map((e) => <Entry key={e.name + JSON.stringify(e.asset)} entry={e} status={status} />)}
          </node>
        );
      })}
    </>
  );
}

function Search({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: space.s }}>
      <text style={{ ...ui.dim, color: color.textDim }}>{words.search}</text>
      <editableText
        value={value}
        onChange={onChange}
        style={{ ...ui.dim, color: color.text, flexGrow: 1, minHeight: 22, padding: { horizontal: space.s, vertical: 3 }, borderRadius: radius.s, border: 1, borderColor: color.line, backgroundColor: "rgba(4, 10, 24, 0.5)" }}
      />
    </node>
  );
}

function Entry({ entry, status }: { entry: LibraryEntry; status: EditorStatus }) {
  const armed = sameAsset(status.placing, entry.asset);
  const disabled = entry.asset.kind === "battery" && status.counts.battery;
  const selected = status.selection?.asset ?? null;
  const replaceable = selected !== null && selected.kind === entry.asset.kind && !sameAsset(selected, entry.asset);
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: space.xs }}>
      <button
        disabled={disabled}
        onPointerDown={(e) => {
          if (!disabled && (e.button ?? 0) === 0) bevy.editor.place({ asset: entry.asset, at: null });
        }}
        style={{ flexGrow: 1, padding: { horizontal: space.s, vertical: 1 }, borderRadius: radius.s, backgroundColor: armed ? color.volt : color.clear }}
        hoverStyle={{ backgroundColor: armed ? color.volt : color.glassHi }}
      >
        <text style={{ ...ui.dim, color: disabled ? color.textFaint : armed ? color.ink : color.text }}>{armed ? `${entry.name}  ${words.placing}` : entry.name}</text>
      </button>
      {replaceable && <Small label={words.replace} onPress={() => swap(entry.asset)} />}
    </node>
  );
}

function Small({ label, onPress }: { label: string; onPress: () => void }) {
  return (
    <button onClick={onPress} style={{ padding: { horizontal: space.s, vertical: 2 }, borderRadius: radius.s, border: 1, borderColor: color.glassBorder }} hoverStyle={{ borderColor: color.cyan }}>
      <text style={{ ...ui.dim, color: color.textDim }}>{label}</text>
    </button>
  );
}

/** The selected prop's kind or decor item's model, and a searchable list of its type to swap it for. */
export function SwapPicker({ asset }: { asset: EditorAsset }) {
  const library = useStore(editorLibrary);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const groups: LibraryGroup[] = library ? filterGroups(library.groups, query, asset.kind) : [];
  return (
    <node style={{ flexDirection: "column", gap: space.xs }}>
      <node style={{ flexDirection: "row", alignItems: "center", gap: space.s }}>
        <text style={{ ...ui.dim, color: color.text, width: 108, flexShrink: 0 }}>{asset.kind === "prop" ? words.kind : asset.kind === "light" ? words.lightType : words.model}</text>
        <text style={{ ...ui.dim, color: color.text, flexGrow: 1 }}>{library ? assetName(library.groups, asset) : ""}</text>
        <Tool label={open ? words.close : words.swap} hot={open} onPress={() => setOpen(!open)} />
      </node>
      {open && (
        <>
          <Search value={query} onChange={setQuery} />
          <node style={{ maxHeight: 300, flexDirection: "column", overflowY: "scroll", gap: 2 }}>
            {groups.map((g) => (
              <node key={g.title} style={{ flexDirection: "column", gap: 2 }}>
                <text style={{ ...ui.caps, color: color.textDim, margin: { top: space.xs } }}>{g.title}</text>
                {g.entries.map((e) => (
                  <button
                    key={e.name + JSON.stringify(e.asset)}
                    onClick={() => swap(e.asset)}
                    style={{ padding: { horizontal: space.s, vertical: 2 }, borderRadius: radius.s, backgroundColor: sameAsset(e.asset, asset) ? color.cyanDeep : color.clear }}
                    hoverStyle={{ backgroundColor: color.glassHi }}
                  >
                    <text style={{ ...ui.dim, color: color.text }}>{e.name}</text>
                  </button>
                ))}
              </node>
            ))}
          </node>
        </>
      )}
    </node>
  );
}
