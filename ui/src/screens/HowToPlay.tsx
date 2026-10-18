// UI-20 How to play: controls with the player's own glyphs, charge & boost, items, recovery. Also
// opened from the pause menu (`onBack` returns there).
import { useState } from "react";

import type { UiScheme } from "../bevy";
import { Glyph } from "../components/Glyph";
import { BatteryIcon, BoltIcon, CheckIcon } from "../components/icons";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { Tabs } from "../components/Tabs";
import { useDebug } from "../hooks";
import { cycle } from "../nav/focus";
import { useDevice } from "../nav/input";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { bindings as bindingsStore } from "../state/stores";
import { useStore } from "../state/useStore";
import { hint } from "../strings/common";
import { howTo } from "../strings/howto";
import { actionName, schemeName } from "../strings/settings";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

const TABS = ["controls", "charge", "items", "recovery"] as const;

export function HowToPlay({ onBack }: { onBack: () => void }) {
  const [tab, setTab] = useState(0);
  const device = useDevice();
  const bindings = useStore(bindingsStore);
  const switchTab = (delta: number) => {
    sfx("hover");
    setTab((t) => cycle(t, delta, TABS.length));
  };
  useDebug("tab", (id) => setTab(Math.max(0, TABS.indexOf(id as (typeof TABS)[number]))));
  useFocusNav({}, { region: "none", index: 0 }, {
    back: onBack,
    prevTab: () => switchTab(-1),
    nextTab: () => switchTab(1),
    adjust: (_, delta) => {
      switchTab(delta);
      return true;
    },
  });
  const scheme: UiScheme = device === "gamepad" ? "gamepad" : device === "keyboardB" ? "keyboardB" : "keyboardA";
  const id = TABS[tab];
  const cards: readonly (readonly [string, string])[] = id === "charge" ? howTo.charge : id === "items" ? howTo.items : howTo.recovery;
  const icon = id === "charge" ? <BoltIcon size={34} /> : id === "items" ? <BatteryIcon size={34} color={color.cyan} /> : <CheckIcon size={34} color={color.cyan} />;
  return (
    <Screen
      title={howTo.title}
      headerRight={<Tabs tabs={TABS.map((t) => ({ id: t, label: howTo.tabs[t] }))} active={tab} onSelect={(i) => switchTab(i - tab)} />}
      hints={[
        { action: "nextTab", label: hint.tabs, onPress: () => switchTab(1) },
        { action: "back", label: hint.back, onPress: onBack },
      ]}
    >
      {id === "controls" ? (
        <Panel style={{ flexGrow: 1, gap: space.m }} padding={34}>
          <node style={{ flexDirection: "row", gap: space.m, alignItems: "center" }}>
            <text style={{ ...text.h3, color: color.cyanHi }}>{schemeName[scheme].toUpperCase()}</text>
            <text style={text.small}>{howTo.controlsIntro}</text>
          </node>
          <node style={{ display: "grid", gridTemplateColumns: "repeat(2, 1flex)", columnGap: space.xxl, rowGap: 14 }}>
            {(bindings?.schemes.find((s) => s.scheme === scheme)?.actions ?? []).map((a) => (
              <node key={a.action} style={{ height: 60, flexDirection: "row", alignItems: "center", justifyContent: "spaceBetween", padding: { horizontal: 18 }, borderRadius: radius.m, backgroundColor: "rgba(4, 8, 20, 0.4)" }}>
                <text style={text.label}>{actionName[a.action]}</text>
                <node style={{ flexDirection: "row", gap: 10 }}>
                  {a.bindings.map((b, i) => (
                    <Glyph key={i} binding={b} size={36} />
                  ))}
                </node>
              </node>
            ))}
          </node>
        </Panel>
      ) : (
        <node style={{ flexGrow: 1, display: "grid", gridTemplateColumns: id === "items" ? "repeat(3, 1flex)" : "repeat(2, 1flex)", gap: space.l, alignContent: "start" }}>
          {cards.map(([title, body]) => (
            <Panel key={title} style={{ gap: 10, minHeight: id === "items" ? 150 : 210 }} padding={28}>
              <node style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
                {icon}
                <text style={{ ...text.h3, fontSize: 28 }}>{title.toUpperCase()}</text>
              </node>
              <text style={text.body}>{body}</text>
            </Panel>
          ))}
        </node>
      )}
    </Screen>
  );
}
