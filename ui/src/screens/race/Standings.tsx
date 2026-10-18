// UI-6 Cup standings (race list with your results, standings with animated point gains) and UI-22
// Versus scoreboard (session points per player), each with the mode's actions.
import { useEffect, useState } from "react";

import { Caption } from "../../components/bits";
import { ChevronIcon } from "../../components/icons";
import { Panel } from "../../components/Panel";
import { Screen } from "../../components/Screen";
import { layoutLabel, ordinal, rgbHex } from "../../format";
import { spring } from "../../theme/motion";
import { catalog, session } from "../../state/stores";
import { useStore } from "../../state/useStore";
import { hint, word } from "../../strings/common";
import { standings as strings, versus as vs } from "../../strings/race";
import { text } from "../../theme/styles";
import { color, radius, slotColor, space } from "../../theme/tokens";
import { ActionColumn, Row, useActions, type Action } from "./Results";

/** "+10" badges slide in a moment after the table appears. */
function useReveal(delay = 450) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setShown(true), delay);
    return () => clearTimeout(timer);
  }, [delay]);
  return shown;
}

function Gain({ value, shown }: { value: number; shown: boolean }) {
  return (
    <node
      style={{
        padding: { horizontal: 10, vertical: 2 },
        borderRadius: radius.pill,
        border: 1,
        borderColor: value > 0 ? color.voltGlow : color.clear,
        transform: { translateX: shown ? 0 : -24, scale: shown ? 1 : 0.6 },
        transition: { transform: spring.bouncy },
      }}
    >
      <text style={{ ...text.label, color: value > 0 ? color.volt : color.clear }}>{strings.gained(value)}</text>
    </node>
  );
}

export function CupStandings({ actions }: { actions: Action[] }) {
  const cup = useStore(session).cup;
  const cat = useStore(catalog);
  const nav = useActions(actions);
  const shown = useReveal();
  const info = cat?.cups.find((c) => c.id === cup?.cup);
  const me = cup?.rows.find((r) => r.slot === 0);
  return (
    <Screen title={strings.title} kicker={info?.name} hints={[{ action: "move", label: hint.select }, { action: "confirm", label: hint.confirm, onPress: () => actions[nav.index]?.run() }]}>
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <node style={{ width: 520, flexDirection: "column", gap: space.l, flexShrink: 0 }}>
        <Panel style={{ gap: space.s }} padding={26}>
          <Caption>{strings.races}</Caption>
          {(info?.races ?? []).map((race, i) => {
            const track = cat?.tracks[race.track];
            const pos = me?.positions[i];
            const current = cup ? i === cup.played - 1 : false;
            const next = cup ? i === cup.played && !cup.finished : false;
            return (
              <node
                key={i}
                style={{
                  height: 70,
                  flexDirection: "row",
                  alignItems: "center",
                  gap: space.m,
                  padding: { horizontal: 16 },
                  borderRadius: radius.m,
                  border: 1,
                  borderColor: current ? color.cyan : next ? color.voltGlow : color.clear,
                  backgroundColor: current ? "rgba(25, 217, 255, 0.10)" : "rgba(4, 8, 20, 0.35)",
                }}
              >
                <text style={{ ...text.h3, width: 34, color: color.textDim }}>{`${i + 1}`}</text>
                <node style={{ flexDirection: "column", flexGrow: 1 }}>
                  <text style={text.label}>{track?.name ?? ""}</text>
                  <text style={{ ...text.small, fontSize: 15 }}>{layoutLabel(race.reverse, race.mirror, word)}</text>
                </node>
                {next ? (
                  <ChevronIcon size={26} color={color.volt} />
                ) : (
                  <text style={{ ...text.h3, color: pos ? (pos <= 3 ? color.volt : color.text) : color.textFaint }}>{pos ? ordinal(pos) : "—"}</text>
                )}
              </node>
            );
          })}
        </Panel>
        <node style={{ flexGrow: 1 }} />
        <ActionColumn actions={actions} nav={nav} />
        </node>
        <Panel style={{ flexGrow: 1, gap: 4 }} padding={24}>
          <Row header cells={["#", strings.table, "", strings.points, ""]} widths={COLS} />
          {(cup?.rows ?? []).map((r, i) => (
            <Row
              key={r.name}
              widths={COLS}
              highlight={r.slot !== null ? slotColor[r.slot] : undefined}
              tint={r.slot !== null ? slotColor[r.slot] : rgbHex(r.color)}
              cells={[`${i + 1}`, r.name, cat?.cars.find((c) => c.id === r.car)?.name ?? r.car, `${r.points}`, <Gain value={r.gained} shown={shown} />]}
            />
          ))}
        </Panel>
      </node>
    </Screen>
  );
}

const COLS = [60, 420, 220, 120, 140];

export function VersusScore({ actions }: { actions: Action[] }) {
  const score = useStore(session).versus;
  const nav = useActions(actions);
  const shown = useReveal();
  return (
    <Screen title={vs.title} subtitle={score ? vs.subtitle(score.races) : undefined} hints={[{ action: "move", label: hint.select }, { action: "confirm", label: hint.confirm, onPress: () => actions[nav.index]?.run() }]}>
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0, alignItems: "flexStart" }}>
        <node style={{ flexGrow: 1, flexDirection: "column", gap: space.m }}>
          {(score?.rows ?? []).map((r, i) => (
            <Panel key={r.slot} style={{ flexDirection: "row", alignItems: "center", gap: space.l, height: 120, border: 2, borderColor: slotColor[r.slot] }} padding={26}>
              <text style={{ ...text.hero, fontSize: 64, width: 120, color: i === 0 ? color.volt : color.textDim }}>{ordinal(i + 1)}</text>
              <text style={{ ...text.hero, fontSize: 64, color: slotColor[r.slot], width: 140 }}>{word.player(r.slot)}</text>
              <node style={{ flexGrow: 1 }} />
              <node style={{ flexDirection: "column", alignItems: "flexEnd", width: 160 }}>
                <Caption>{vs.wins}</Caption>
                <text style={{ ...text.h2 }}>{`${r.wins}`}</text>
              </node>
              <node style={{ flexDirection: "column", alignItems: "flexEnd", width: 200 }}>
                <Caption>{vs.points}</Caption>
                <text style={{ ...text.h2, color: color.cyanHi }}>{`${r.points}`}</text>
              </node>
              <node style={{ width: 120 }}>
                <Gain value={r.gained} shown={shown} />
              </node>
            </Panel>
          ))}
        </node>
        <node style={{ width: 340, flexShrink: 0 }}>
          <ActionColumn actions={actions} nav={nav} />
        </node>
      </node>
    </Screen>
  );
}
