// UI-13 Results: standings (pos, name, car, total, best lap, points), new records / medal / ghost,
// unlocks, and the mode's actions (GDD §3.3).
import type { ReactNode } from "react";

import type { UiResultEntry } from "../../bevy";
import { Badge } from "../../components/Badge";
import { Button } from "../../components/Button";
import { Spinner } from "../../components/bits";
import { MedalIcon } from "../../components/icons";
import { Panel } from "../../components/Panel";
import { Screen } from "../../components/Screen";
import { formatDelta, formatTime, ordinal, rgbHex } from "../../format";
import { useFocusNav } from "../../nav/useFocusNav";
import { sfx } from "../../sound";
import { catalog, session } from "../../state/stores";
import { unlockLabel } from "../../state/unlocks";
import { useStore } from "../../state/useStore";
import { hint, medalName } from "../../strings/common";
import { race, results as strings } from "../../strings/race";
import { text } from "../../theme/styles";
import { color, layer, radius, slotColor, space } from "../../theme/tokens";

export type Action = { label: string; run: () => void; primary?: boolean };

const ROW = 62;

/** The results table and the mode's actions. */
export function Results({ actions }: { actions: Action[] }) {
  const { setup, results } = useStore(session);
  const cat = useStore(catalog);
  const nav = useActions(actions);
  const mode = setup?.mode.kind ?? "quickRace";
  const track = cat?.tracks.find((t) => t.id === setup?.track)?.name ?? "";
  const points = results?.entries.some((e) => e.points !== null) ?? false;
  const me = results?.entries.find((e) => e.slot === 0) ?? results?.entries.find((e) => e.slot !== null);
  const outcome = results?.outcome;
  return (
    <Screen title={strings.title} kicker={`${race.modeName[mode]} · ${track}`} hints={[{ action: "move", label: hint.select }, { action: "confirm", label: hint.confirm, onPress: () => actions[nav.index]?.run() }]}>
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <Panel style={{ flexGrow: 1, gap: 4 }} padding={24}>
          <Row header cells={[strings.col.pos, strings.col.name, strings.col.car, strings.col.time, strings.col.best, points ? strings.col.points : ""]} />
          {results ? (
            results.entries.map((e) => <EntryRow key={e.rank} entry={e} points={points} />)
          ) : (
            <node style={{ flexDirection: "row", gap: space.m, alignItems: "center", padding: space.l }}>
              <Spinner />
              <text style={text.body}>{strings.waiting}</text>
            </node>
          )}
        </Panel>
        <node style={{ width: 480, flexDirection: "column", gap: space.l, flexShrink: 0 }}>
          {me && (
            <Panel style={{ gap: space.s, alignItems: "flexStart" }} padding={30}>
              <text style={text.caps}>{strings.finished.toUpperCase()}</text>
              <text style={{ ...text.hero, fontSize: 120, color: me.rank <= 3 ? color.volt : color.text }}>{ordinal(me.rank)}</text>
              <text style={{ ...text.label, color: color.textDim }}>{formatTime(me.timeMs)}</text>
              <node style={{ flexDirection: "row", flexWrap: "wrap", gap: 8, margin: { top: space.s } }}>
                {outcome?.newBestLap && <Badge label={strings.newBestLap} tone={color.cyan} />}
                {outcome?.newBestRace && <Badge label={strings.newRecord} tone={color.cyan} />}
                {outcome?.ghostSaved && <Badge label={strings.ghostSaved} tone={color.success} />}
              </node>
              {outcome?.medal && (
                <node style={{ flexDirection: "row", alignItems: "center", gap: 12, margin: { top: space.s } }}>
                  <MedalIcon size={44} color={color[outcome.medal]} />
                  <text style={{ ...text.h3, color: color[outcome.medal] }}>{medalName[outcome.medal].toUpperCase()}</text>
                </node>
              )}
              {outcome?.nextMedal && (
                <text style={text.small}>{`${medalName[outcome.nextMedal.medal]}: ${formatTime(outcome.nextMedal.targetMs)} (${formatDelta(outcome.nextMedal.deltaMs)})`}</text>
              )}
              {outcome?.unlocked.map((u) => (
                <text key={JSON.stringify(u)} style={{ ...text.label, color: color.volt }}>{unlockLabel(u, cat)}</text>
              ))}
            </Panel>
          )}
          <node style={{ flexGrow: 1 }} />
          <ActionColumn actions={actions} nav={nav} />
        </node>
      </node>
    </Screen>
  );
}

function EntryRow({ entry, points }: { entry: UiResultEntry; points: boolean }) {
  const cars = useStore(catalog)?.cars ?? [];
  const racer = useStore(session).setup?.racers.find((r) => r.name === entry.name);
  const tint = entry.slot !== null ? slotColor[entry.slot] : racer ? rgbHex(racer.color) : color.textDim;
  const time = entry.finished ? formatTime(entry.timeMs) : `${formatTime(entry.timeMs)} ${strings.projected}`;
  return (
    <Row
      highlight={entry.slot !== null ? tint : undefined}
      cells={[ordinal(entry.rank), entry.name, cars.find((c) => c.id === entry.car)?.name ?? entry.car, time, formatTime(entry.bestLapMs), entry.points !== null && points ? `+${entry.points}` : ""]}
      tint={tint}
    />
  );
}

const WIDTHS = [80, 300, 200, 220, 190, 70];

/** A table row: header, or a racer with its colour bar. */
export function Row({ cells, header, highlight, tint, widths = WIDTHS }: { cells: ReactNode[]; header?: boolean; highlight?: string; tint?: string; widths?: number[] }) {
  return (
    <node
      style={{
        height: header ? 34 : ROW,
        flexShrink: 0,
        flexDirection: "row",
        alignItems: "center",
        gap: space.m,
        padding: { horizontal: 18 },
        borderRadius: radius.m,
        backgroundColor: highlight ? "rgba(25, 217, 255, 0.10)" : color.clear,
        border: highlight ? 1 : 0,
        borderColor: highlight ?? color.clear,
      }}
    >
      {!header && <node style={{ positionType: "absolute", left: 0, top: 12, bottom: 12, width: 5, borderRadius: 3, backgroundColor: tint ?? color.clear }} />}
      {cells.map((cell, i) =>
        typeof cell === "string" ? (
          <text key={i} style={header ? { ...text.caps, width: widths[i] } : { ...(i === 0 ? text.h3 : text.label), fontSize: i === 0 ? 28 : 22, width: widths[i], color: i === 5 ? color.volt : color.text }}>
            {header ? cell.toUpperCase() : cell}
          </text>
        ) : (
          <node key={i} style={{ width: widths[i] }}>
            {cell}
          </node>
        ),
      )}
    </node>
  );
}

/** Focus over a column of action buttons (overlay nav layer: above the race HUD). */
export function useActions(actions: Action[]) {
  const nav = useFocusNav({ actions: { count: actions.length, cols: 1, wrap: true } }, { region: "actions", index: 0 }, {
    confirm: (f) => {
      sfx("select");
      actions[f.index]?.run();
    },
  }, layer.overlay);
  return { index: nav.focus.index, hover: (i: number) => nav.hover("actions", i) };
}

export function ActionColumn({ actions, nav }: { actions: Action[]; nav: ReturnType<typeof useActions> }) {
  return (
    <node style={{ flexDirection: "column", gap: 10 }}>
      {actions.map((a, i) => (
        <Button
          key={a.label}
          label={a.label}
          variant={a.primary ? "primary" : "secondary"}
          focused={i === nav.index}
          onHover={() => nav.hover(i)}
          onPress={() => {
            sfx("select");
            a.run();
          }}
          style={{ justifyContent: "center" }}
        />
      ))}
    </node>
  );
}

