// Top-centre notifications of one view ("Hit by Spark Rocket — Mayhem", "Fastest lap!", "Golden
// Battery found!"): newest last, a few at most, each with its item icon and a tone colour.
import { useEnter } from "../hooks";
import { fs } from "./layout";
import { chip, label, tone } from "./look";
import { ItemIcon } from "./itemIcons";
import type { Notice } from "./store";

export function Notices({ list, k }: { list: Notice[]; k: number }) {
  if (list.length === 0) return null;
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: Math.round(8 * k) }}>
      {list.map((n) => (
        <Pill key={n.id} notice={n} k={k} />
      ))}
    </node>
  );
}

function Pill({ notice, k }: { notice: Notice; k: number }) {
  const enter = useEnter(0, -18);
  const accent = tone[notice.tone];
  return (
    <node style={{ ...enter, flexDirection: "row", alignItems: "center", gap: Math.round(12 * k), padding: { left: Math.round(10 * k), right: Math.round(20 * k), vertical: Math.round(7 * k) }, ...chip(k, true) }}>
      <node style={{ width: Math.max(4, Math.round(5 * k)), alignSelf: "stretch", borderRadius: 3, backgroundColor: accent }} />
      {notice.item && <ItemIcon kind={notice.item} size={Math.round(Math.max(36 * k, 26))} />}
      <text style={label(fs(26, k, 18), notice.tone === "gold" ? tone.gold : undefined)}>{notice.text}</text>
    </node>
  );
}
