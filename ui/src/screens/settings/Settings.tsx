// UI-16 Settings: 5 tabs, changes apply live (`settings.apply`; rc_game persists at most once a
// second). Controls: binding lists per scheme with rebind capture and conflict warnings; Gameplay:
// reset progress behind a double confirmation. Also opened from the pause menu.
import { useEffect, useState } from "react";

import { bevy, type UiBindings, type UiDriveAction, type UiScheme, type UiSettings } from "../../bevy";
import { Button } from "../../components/Button";
import { Cycler, OptionRow, Slider, Toggle } from "../../components/controls";
import { Glyph } from "../../components/Glyph";
import { WarningIcon } from "../../components/icons";
import { List } from "../../components/lists";
import { Panel } from "../../components/Panel";
import { Screen } from "../../components/Screen";
import { Tabs } from "../../components/Tabs";
import { useDebug, useEvent } from "../../hooks";
import { cycle, stepValue } from "../../nav/focus";
import { useDevice } from "../../nav/input";
import { useFocusNav } from "../../nav/useFocusNav";
import { sfx } from "../../sound";
import { refreshSettings } from "../../state/Feeds";
import { openDialog } from "../../state/overlays";
import { bindings as bindingsStore, settings as settingsStore } from "../../state/stores";
import { useStore } from "../../state/useStore";
import { appStrings } from "../../strings/app";
import { hint, word } from "../../strings/common";
import { actionName, schemeName, settingsText } from "../../strings/settings";
import { text } from "../../theme/styles";
import { color, space } from "../../theme/tokens";
import { rebindNotice, rowsOf, TABS, type RebindNotice, type RowDef } from "./rows";

const SCHEMES: UiScheme[] = ["keyboardA", "keyboardB", "gamepad"];
const ROW = 56;

type Item =
  | { type: "row"; row: RowDef }
  | { type: "scheme" }
  | { type: "binding"; action: UiDriveAction }
  | { type: "resetBindings" };


export function Settings({ onBack }: { onBack: () => void }) {
  const device = useDevice();
  const [tab, setTab] = useState(0);
  const s = useStore(settingsStore);
  const bindings = useStore(bindingsStore);
  const [scheme, setScheme] = useState<UiScheme>(device === "gamepad" ? "gamepad" : device === "keyboardB" ? "keyboardB" : "keyboardA");
  const [capture, setCapture] = useState<UiDriveAction | null>(null);
  const [notice, setNotice] = useState<RebindNotice | null>(null);
  useEffect(refreshSettings, []);
  useEvent("input.rebindResult", (r) => {
    setCapture(null);
    if (!r.binding) setNotice({ kind: "cancelled" });
    else if (r.conflict) setNotice({ kind: "conflict", other: r.conflict, swapped: r.previous !== null });
    else setNotice(null);
  });
  useDebug("tab", (id) => setTab(Math.max(0, TABS.indexOf(id as (typeof TABS)[number]))));

  const tabId = TABS[tab];
  const rows = s ? rowsOf(tabId, s) : [];
  const actions = bindings?.schemes.find((x) => x.scheme === scheme)?.actions ?? [];
  const items: Item[] =
    tabId === "controls"
      ? [{ type: "scheme" }, ...actions.map((a) => ({ type: "binding" as const, action: a.action })), { type: "resetBindings" }, ...rows.map((row) => ({ type: "row" as const, row }))]
      : rows.map((row) => ({ type: "row" as const, row }));

  const edit = (next: UiSettings) => {
    settingsStore.set(next);
    bevy.settings.apply(next);
  };
  const switchTab = (delta: number) => {
    sfx("hover");
    setNotice(null);
    setTab((t) => cycle(t, delta, TABS.length));
    nav.setFocus({ region: "rows", index: 0 });
  };
  const adjust = (item: Item | undefined, delta: number): boolean => {
    if (!item || !s) return false;
    if (item.type === "scheme") {
      sfx("toggle");
      setNotice(null);
      setScheme(SCHEMES[cycle(SCHEMES.indexOf(scheme), delta, SCHEMES.length)]);
      return true;
    }
    if (item.type !== "row") return false;
    const row = item.row;
    if (row.locked) {
      sfx("error");
      return true;
    }
    if (row.kind === "toggle") {
      sfx("toggle");
      edit(row.set(s, !row.get(s)));
    } else if (row.kind === "cycle") {
      sfx("toggle");
      edit(row.pick(s, cycle(row.index(s), delta, row.choices.length)));
    } else if (row.kind === "slider") {
      sfx("sliderTick");
      edit(row.set(s, stepValue(row.get(s), delta, row.min, row.max, row.step)));
    } else return false;
    return true;
  };
  const activate = (item: Item | undefined) => {
    if (!item) return;
    if (item.type === "binding") {
      sfx("select");
      setNotice(null);
      setCapture(item.action);
      bevy.input.rebind({ scheme, action: item.action });
    } else if (item.type === "resetBindings") {
      sfx("select");
      setNotice(null);
      bevy.input.resetBindings({ scheme });
    } else if (item.type === "row" && item.row.id === "resetProgress") {
      sfx("select");
      confirmReset();
    } else if (!(item.type === "row" && item.row.kind === "slider")) adjust(item, 1);
  };
  const nav = useFocusNav({ rows: { count: items.length, cols: 1, wrap: true } }, { region: "rows", index: 0 }, {
    back: onBack,
    prevTab: () => switchTab(-1),
    nextTab: () => switchTab(1),
    adjust: (f, delta) => adjust(items[f.index], delta),
    confirm: (f) => activate(items[f.index]),
  });
  const focused = items[nav.focus.index];
  // The action a rebind left without any binding (shown in this scheme) is highlighted.
  const { text: noticeText, unbound } = rebindNotice(notice, scheme, bindings);
  const label = (item: Item | undefined) =>
    !item ? "" : item.type === "row" ? settingsText.rows[item.row.id] : item.type === "scheme" ? settingsText.rows.scheme : item.type === "resetBindings" ? settingsText.rows.resetBindings : [actionName[item.action], ""];
  const [title, desc] = label(focused) || ["", ""];

  return (
    <Screen
      title={settingsText.title}
      headerRight={<Tabs tabs={TABS.map((id) => ({ id, label: settingsText.tabs[id] }))} active={tab} onSelect={(i) => switchTab(i - tab)} />}
      hints={[
        { action: "move", label: hint.change },
        { action: "nextTab", label: hint.tabs, onPress: () => switchTab(1) },
        { action: "back", label: hint.back, onPress: onBack },
      ]}
    >
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <Panel style={{ flexGrow: 1 }} padding={20}>
          <List index={nav.focus.index} rows={items.length} rowHeight={ROW} gap={4} height={660}>
            {s &&
              items.map((item, i) => (
                <OptionRow
                  key={item.type === "row" ? item.row.id : item.type === "binding" ? item.action : item.type}
                  label={label(item)[0]}
                  height={ROW}
                  focused={nav.is("rows", i)}
                  disabled={item.type === "row" && item.row.locked}
                  warn={item.type === "binding" && unbound?.scheme === scheme && unbound.action === item.action}
                  onHover={() => nav.hover("rows", i)}
                  onPress={() => activate(item)}
                >
                  <Control item={item} s={s} scheme={scheme} bindings={bindings} capturing={item.type === "binding" && capture === item.action} onStep={(d) => adjust(item, d)} onSet={edit} onActivate={() => activate(item)} />
                </OptionRow>
              ))}
          </List>
        </Panel>
        <Panel style={{ width: 520, gap: space.m, flexShrink: 0 }} padding={32}>
          <text style={{ ...text.h3, color: color.cyanHi, lineBreak: "wordBoundary" }}>{title.toUpperCase()}</text>
          {desc && <text style={text.body}>{desc}</text>}
          {capture && (
            <node style={{ flexDirection: "column", gap: 8, padding: 18, borderRadius: 10, backgroundColor: "rgba(25, 217, 255, 0.12)" }}>
              <text style={{ ...text.label, color: color.cyanHi }}>{settingsText.capture}</text>
              <text style={text.small}>{settingsText.captureHint}</text>
            </node>
          )}
          {noticeText && !capture && (
            <node style={{ flexDirection: "row", gap: 12, alignItems: "center" }}>
              <WarningIcon size={26} />
              <text style={{ ...text.small, color: color.warning, flexShrink: 1 }}>{noticeText}</text>
            </node>
          )}
        </Panel>
      </node>
    </Screen>
  );
}

function confirmReset() {
  openDialog({
    title: settingsText.resetTitle,
    text: settingsText.resetText,
    icon: "warning",
    actions: [
      {
        label: settingsText.reset,
        tone: "danger",
        run: () =>
          openDialog({
            title: settingsText.resetAgainTitle,
            text: settingsText.resetAgainText,
            icon: "warning",
            actions: [{ label: settingsText.resetConfirm, tone: "danger", run: () => bevy.profile.reset(null) }, { label: appStrings.cancel }],
            cancel: 1,
            focus: 1,
          }),
      },
      { label: appStrings.cancel },
    ],
    cancel: 1,
    focus: 1,
  });
}

type ControlProps = {
  item: Item;
  s: UiSettings;
  scheme: UiScheme;
  bindings: UiBindings | null;
  capturing: boolean;
  onStep: (delta: number) => void;
  onSet: (next: UiSettings) => void;
  onActivate: () => void;
};

/** The control on the right of a row. */
function Control({ item, s, scheme, bindings, capturing, onStep, onSet, onActivate }: ControlProps) {
  if (item.type === "scheme") return <Cycler label={schemeName[scheme]} onStep={onStep} />;
  if (item.type === "resetBindings") return <Button label={settingsText.reset} onPress={onActivate} style={{ height: 44 }} />;
  if (item.type === "binding") {
    if (capturing) return <text style={{ ...text.label, color: color.cyanHi }}>{settingsText.capture}</text>;
    const list = bindings?.schemes.find((x) => x.scheme === scheme)?.actions.find((a) => a.action === item.action)?.bindings ?? [];
    return (
      <node style={{ flexDirection: "row", gap: 10 }}>
        {list.length ? list.map((b, i) => <Glyph key={i} binding={b} size={34} />) : <text style={text.label}>{settingsText.unbound}</text>}
      </node>
    );
  }
  const row = item.row;
  switch (row.kind) {
    case "toggle":
      return <Toggle value={row.get(s)} labels={[word.off, word.on]} onChange={() => onStep(1)} />;
    case "cycle":
      return <Cycler label={row.choices[row.index(s)]?.label ?? ""} dim={row.locked} onStep={onStep} />;
    case "slider":
      return <Slider value={row.get(s)} min={row.min} max={row.max} step={row.step} format={row.format} onChange={(v) => onSet(row.set(s, v))} />;
    case "action":
      return <Button label={settingsText.reset} variant="danger" onPress={onActivate} style={{ height: 44 }} />;
  }
}
