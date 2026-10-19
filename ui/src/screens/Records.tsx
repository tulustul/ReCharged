// UI-15 Records: per layout best lap / race with car and medal; trophies; Golden Batteries; stats.
import { useState } from "react";

import type { UiLayoutRecord } from "../bevy";
import { BatteryIcon, MedalIcon, TrophyIcon } from "../components/icons";
import { List } from "../components/lists";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { Tabs } from "../components/Tabs";
import { formatDuration, formatKm, formatTime, layoutLabel } from "../format";
import { useDebug } from "../hooks";
import { cycle } from "../nav/focus";
import { back } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { catalog, profile, records } from "../state/stores";
import { trophyOf } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { hint, medalName, trophyName, word } from "../strings/common";
import { cupSelect, records as strings } from "../strings/menu";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

const TABS = ["tracks", "trophies", "batteries", "stats"] as const;
const ROW = 64;

type Line = { track: string; name: string; reverse: boolean; mirror: boolean; record?: UiLayoutRecord };

export function Records() {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const recs = useStore(records);
  const [tab, setTab] = useState(0);
  const switchTab = (delta: number) => {
    sfx("hover");
    setTab((t) => cycle(t, delta, TABS.length));
  };
  useDebug("tab", (id) => setTab(Math.max(0, TABS.indexOf(id as (typeof TABS)[number]))));
  const lines: Line[] = (cat?.tracks ?? [])
    .filter((t) => p?.unlocked.tracks.includes(t.id))
    .flatMap((t) =>
      [
        [false, false],
        [true, false],
        [false, true],
        [true, true],
      ]
        .filter(([rev, mir]) => (!rev || p?.unlocked.reverse) && (!mir || p?.unlocked.mirror))
        .map(([reverse, mirror]) => ({
          track: t.id,
          name: t.name,
          reverse,
          mirror,
          record: recs?.layouts.find((r) => r.track === t.id && r.reverse === reverse && r.mirror === mirror),
        })),
    );
  const nav = useFocusNav({ rows: { count: TABS[tab] === "tracks" ? lines.length : 0, cols: 1 } }, { region: "rows", index: 0 }, {
    back,
    prevTab: () => switchTab(-1),
    nextTab: () => switchTab(1),
  });
  return (
    <Screen
      title={strings.title}
      headerRight={<Tabs tabs={TABS.map((id) => ({ id, label: strings.tabs[id] }))} active={tab} onSelect={(i) => switchTab(i - tab)} />}
      hints={[
        { action: "prevTab", label: hint.tabs, onPress: () => switchTab(1) },
        { action: "back", label: hint.back, onPress: back },
      ]}
    >
      <Panel style={{ flexGrow: 1, gap: space.s }} padding={28}>
        {TABS[tab] === "tracks" && <TracksTab lines={lines} index={nav.focus.index} onHover={(i) => nav.hover("rows", i)} />}
        {TABS[tab] === "trophies" && (
          <node style={{ flexDirection: "row", gap: space.l, flexGrow: 1, alignItems: "center", justifyContent: "center" }}>
            {(cat?.cups ?? []).map((cup) => {
              const trophy = trophyOf(p, cup.id);
              return (
                <node key={cup.id} style={{ width: 340, height: 360, flexDirection: "column", alignItems: "center", justifyContent: "center", gap: space.m, borderRadius: radius.l, backgroundColor: "rgba(4, 8, 20, 0.4)" }}>
                  <TrophyIcon size={120} color={trophy ? color[trophy] : "rgba(255, 255, 255, 0.12)"} />
                  <text style={text.h3}>{cup.name.toUpperCase()}</text>
                  <text style={{ ...text.label, color: trophy ? color[trophy] : color.textFaint }}>{trophy ? trophyName[trophy] : cupSelect.noTrophy}</text>
                </node>
              );
            })}
          </node>
        )}
        {TABS[tab] === "batteries" && (
          <node style={{ flexDirection: "column", gap: space.l, flexGrow: 1, justifyContent: "center", alignItems: "center" }}>
            <node style={{ flexDirection: "row", gap: space.l }}>
              {(cat?.tracks ?? []).map((t) => {
                const found = !!p?.goldenBatteries.includes(t.id);
                return (
                  <node key={t.id} style={{ width: 240, height: 260, flexDirection: "column", alignItems: "center", justifyContent: "center", gap: space.m, borderRadius: radius.l, backgroundColor: "rgba(4, 8, 20, 0.4)" }}>
                    <BatteryIcon size={96} color={found ? color.gold : "rgba(255, 255, 255, 0.12)"} />
                    <text style={{ ...text.label, textAlign: "center" }}>{t.name}</text>
                    <text style={{ ...text.small, color: found ? color.gold : color.textFaint }}>{found ? strings.found : strings.missing}</text>
                  </node>
                );
              })}
            </node>
            <text style={text.body}>{strings.batteriesHint}</text>
          </node>
        )}
        {TABS[tab] === "stats" && <StatsTab />}
      </Panel>
    </Screen>
  );
}

function TracksTab({ lines, index, onHover }: { lines: Line[]; index: number; onHover: (i: number) => void }) {
  const cars = useStore(catalog)?.cars ?? [];
  const carName = (id: string) => cars.find((c) => c.id === id)?.name ?? id;
  const header = (label: string, width: number | string) => <text style={{ ...text.caps, width }}>{label.toUpperCase()}</text>;
  return (
    <>
      {!lines.some((l) => l.record) && <text style={{ ...text.small, margin: { left: 20, bottom: space.s }, color: color.textDim }}>{strings.empty}</text>}
      <node style={{ flexDirection: "row", padding: { horizontal: 20 }, gap: space.m }}>
        {header(strings.layout, 460)}
        {header(strings.bestLap, 420)}
        {header(strings.bestRace, 420)}
        {header(strings.medal, 200)}
      </node>
      <List index={index} rows={lines.length} rowHeight={ROW} gap={4} height={620}>
        {lines.map((l, i) => (
          <button
            key={`${l.track}-${l.reverse}-${l.mirror}`}
            onPointerHover={() => onHover(i)}
            style={{
              height: ROW,
              flexShrink: 0,
              flexDirection: "row",
              alignItems: "center",
              gap: space.m,
              padding: { horizontal: 20 },
              borderRadius: radius.m,
              backgroundColor: i === index ? "rgba(25, 217, 255, 0.12)" : i % 2 ? "rgba(40, 62, 120, 0.22)" : color.clear,
            }}
          >
            <text style={{ ...text.label, width: 460, color: l.record ? color.text : color.textFaint }}>{`${l.name} · ${layoutLabel(l.reverse, l.mirror, word)}`}</text>
            <text style={{ ...text.label, width: 420, color: l.record?.bestLap ? color.text : color.textFaint }}>
              {l.record?.bestLap ? `${formatTime(l.record.bestLap.ms)}  ·  ${carName(l.record.bestLap.car)}` : word.none}
            </text>
            <text style={{ ...text.label, width: 420, color: l.record?.bestRace ? color.text : color.textFaint }}>
              {l.record?.bestRace ? `${formatTime(l.record.bestRace.ms)}  ·  ${carName(l.record.bestRace.car)}` : word.none}
            </text>
            <node style={{ width: 200, flexDirection: "row", gap: 8, alignItems: "center" }}>
              {l.record?.medal ? (
                <>
                  <MedalIcon size={26} color={color[l.record.medal]} />
                  <text style={{ ...text.label, color: color[l.record.medal] }}>{medalName[l.record.medal]}</text>
                </>
              ) : (
                <text style={{ ...text.label, color: color.textFaint }}>{word.none}</text>
              )}
            </node>
          </button>
        ))}
      </List>
    </>
  );
}

function StatsTab() {
  const s = useStore(profile)?.stats;
  const tiles: [string, string][] = s
    ? [
        [strings.stats.races, String(s.races)],
        [strings.stats.wins, String(s.wins)],
        [strings.stats.podiums, String(s.podiums)],
        [strings.stats.distance, formatKm(s.distanceKm)],
        [strings.stats.itemsUsed, String(s.itemsUsed)],
        [strings.stats.hitsLanded, String(s.hitsLanded)],
        [strings.stats.timePlayed, formatDuration(s.timePlayedSecs)],
      ]
    : [];
  return (
    <node style={{ flexGrow: 1, display: "grid", gridTemplateColumns: "repeat(4, 1flex)", gap: space.l, alignContent: "center" }}>
      {tiles.map(([label, value]) => (
        <node key={label} style={{ height: 190, flexDirection: "column", justifyContent: "center", gap: 10, padding: 28, borderRadius: radius.l, backgroundColor: "rgba(4, 8, 20, 0.4)" }}>
          <text style={{ ...text.caps }}>{label.toUpperCase()}</text>
          <text style={{ ...text.hero, fontSize: 64, color: color.cyanHi }}>{value}</text>
        </node>
      ))}
    </node>
  );
}
