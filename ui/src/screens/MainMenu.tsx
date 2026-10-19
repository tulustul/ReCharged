// UI-4 Main menu: the four modes, then Garage … Quit (confirm). Focus starts on Championship; the
// panel on the right explains the focused entry and shows progress. Mode entries charge-morph.
import { useEffect, type ReactNode } from "react";
import { interpolate, useSharedValue, withRepeat, withTiming } from "bevy-react";

import { bevy } from "../bevy";
import { Caption } from "../components/bits";
import { Button } from "../components/Button";
import { Hints } from "../components/Hints";
import { BatteryIcon, BoltIcon, MedalIcon, TrophyIcon } from "../components/icons";
import { Panel } from "../components/Panel";
import { ChargeLine } from "../components/Screen";
import { useEnter } from "../hooks";
import { go, type Route } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { draft, type MenuMode } from "../state/draft";
import { openDialog } from "../state/overlays";
import { appState, catalog, profile } from "../state/stores";
import { useStore } from "../state/useStore";
import { appStrings } from "../strings/app";
import { hint, word } from "../strings/common";
import { mainMenu } from "../strings/menu";
import { fill, text } from "../theme/styles";
import { color, frame, space } from "../theme/tokens";

type EntryId = keyof typeof mainMenu.entries;
type Entry = { id: EntryId; mode?: MenuMode; route?: Route };

const ENTRIES: Entry[] = [
  { id: "championship", mode: "championship", route: "cupSelect" },
  { id: "quickRace", mode: "quickRace", route: "trackSelect" },
  { id: "timeTrial", mode: "timeTrial", route: "trackSelect" },
  { id: "versus", mode: "versus", route: "lobby" },
  { id: "garage", route: "garage" },
  { id: "records", route: "records" },
  { id: "howToPlay", route: "howToPlay" },
  { id: "settings", route: "settings" },
  { id: "credits", route: "credits" },
  { id: "quit" },
];
const MODES = 4;

export function confirmQuit() {
  openDialog({
    title: appStrings.quitTitle,
    text: appStrings.quitText,
    actions: [
      { label: appStrings.quit, tone: "danger", run: () => bevy.app.quit(null) },
      { label: appStrings.cancel },
    ],
    cancel: 1,
    focus: 1,
  });
}

export function MainMenu() {
  const p = useStore(profile);
  const cat = useStore(catalog);
  const showroom = useStore(appState)?.showroom ?? false;
  const activate = (entry: Entry) => {
    sfx("select");
    if (entry.id === "quit") return confirmQuit();
    if (entry.mode) draft.update((d) => ({ ...d, mode: entry.mode as MenuMode, cup: null }));
    if (entry.route) go(entry.route, entry.mode ? "charge" : "fade");
  };
  const nav = useFocusNav(
    { items: { count: ENTRIES.length, cols: 1, wrap: true } },
    { region: "items", index: 0 },
    { confirm: (f) => activate(ENTRIES[f.index]), back: confirmQuit },
  );
  const focused = ENTRIES[nav.focus.index];
  const fresh = (kind: string) => !!p?.fresh.some((f) => f.kind === kind);
  const badge: Partial<Record<EntryId, boolean>> = { championship: fresh("cup"), quickRace: fresh("track"), garage: fresh("car") };
  const listEnter = useEnter(-60, 0);
  return (
    <node style={{ ...fill, flexDirection: "row", padding: { left: frame.x, right: frame.x, top: frame.top, bottom: frame.bottom } }}>
      <node style={{ width: 640, flexDirection: "column", gap: space.l }}>
        <Logo />
        <node style={{ ...listEnter, flexDirection: "column", gap: 10, margin: { top: space.s } }}>
          {ENTRIES.map((entry, i) => (
            <node key={entry.id} style={{ margin: { top: i === MODES ? space.m : 0 } }}>
              <Button
                label={mainMenu.entries[entry.id].label}
                variant="menu"
                compact={i >= MODES}
                width={i >= MODES ? 520 : 600}
                focused={nav.is("items", i)}
                badge={badge[entry.id] ? word.new : undefined}
                onHover={() => nav.hover("items", i)}
                onPress={() => activate(entry)}
              />
            </node>
          ))}
        </node>
      </node>
      <node style={{ flexGrow: 1, flexDirection: "column", alignItems: "flexEnd", justifyContent: "flexEnd", gap: space.l }}>
        {!showroom && <HeroCar car={draftCar(cat, p)} />}
        <InfoPanel id={focused.id} />
        <Hints
          items={[
            { action: "move", label: hint.browse },
            { action: "confirm", label: hint.confirm, onPress: () => activate(focused) },
            { action: "back", label: appStrings.quit, onPress: confirmQuit },
          ]}
        />
      </node>
    </node>
  );
}

function Logo() {
  const logo = { ...text.title, fontSize: 64, fontWeight: "900" };
  return (
    <node style={{ flexDirection: "column", gap: 10 }}>
      <node style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
        <BoltIcon size={58} />
        <text style={logo}>
          {appStrings.nameRe}
          <text style={{ ...logo, color: color.cyan }}>{appStrings.nameCharged}</text>
        </text>
      </node>
      <ChargeLine width={260} />
    </node>
  );
}

function InfoPanel({ id }: { id: EntryId }) {
  const p = useStore(profile);
  const cat = useStore(catalog);
  const entry = mainMenu.entries[id];
  const stats: [ReactNode, string, string][] = [
    [<TrophyIcon size={30} />, mainMenu.trophies, `${p?.trophies.length ?? 0}/${cat?.cups.length ?? 0}`],
    [<BoltIcon size={30} color={color.cyan} />, mainMenu.cars, `${p?.unlocked.cars.length ?? 0}/${cat?.cars.length ?? 0}`],
    [<BatteryIcon size={30} />, mainMenu.batteries, `${p?.goldenBatteries.length ?? 0}/${cat?.tracks.length ?? 0}`],
    [<MedalIcon size={30} />, mainMenu.medals, `${p?.goldMedals ?? 0}`],
  ];
  return (
    <Panel style={{ width: 640, gap: space.m }} padding={32}>
      <text style={{ ...text.h2, color: color.cyanHi }}>{entry.label.toUpperCase()}</text>
      <text style={{ ...text.body, color: color.text, minHeight: 64 }}>{entry.text}</text>
      <node style={{ height: 1, backgroundColor: color.line, margin: { vertical: 4 } }} />
      <Caption>{mainMenu.progress}</Caption>
      <node style={{ flexDirection: "row", justifyContent: "spaceBetween" }}>
        {stats.map(([icon, label, value]) => (
          <node key={label} style={{ flexDirection: "column", alignItems: "center", gap: 6, width: 135 }}>
            {icon}
            <text style={{ ...text.h3, fontSize: 28 }}>{value}</text>
            <text style={{ ...text.small, fontSize: 15, textAlign: "center" }}>{label}</text>
          </node>
        ))}
      </node>
    </Panel>
  );
}

/** The car shown when there is no 3D showroom: the draft's, else the first unlocked. */
function draftCar(cat: ReturnType<typeof catalog.get>, p: ReturnType<typeof profile.get>) {
  const cars = cat?.cars ?? [];
  return cars.find((c) => c.id === draft.get().car) ?? cars.find((c) => p?.unlocked.cars.includes(c.id)) ?? cars[0];
}

/** A 2D stand-in for the showroom car, floating gently. */
export function HeroCar({ car, width = 620 }: { car?: { thumbnail: string | null }; width?: number }) {
  const float = useSharedValue(0);
  useEffect(() => {
    float.value = withRepeat(withTiming(1, { duration: 2600, easing: "easeInOut" }), { reverse: true });
  }, [float]);
  if (!car?.thumbnail) return null;
  return (
    <node style={{ flexGrow: 1, alignSelf: "stretch", alignItems: "center", justifyContent: "center" }}>
      <node
        style={{
          positionType: "absolute",
          width: width * 0.9,
          height: width * 0.5,
          backgroundGradient: { type: "radial", stops: [{ color: "rgba(25, 217, 255, 0.18)" }, { color: "rgba(25, 217, 255, 0)" }] },
        }}
      />
      <image
        src={car.thumbnail}
        style={{ width, height: width, imageRendering: "trilinear", transform: { translateY: { animated: interpolate(float, [0, 1], [-8, 8]) } } }}
      />
    </node>
  );
}
