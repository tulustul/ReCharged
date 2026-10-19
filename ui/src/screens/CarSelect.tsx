// UI-8 Car select: a car carousel over the showroom backdrop (`menu.backdrop`), stat bars, class,
// paint picker, lock reasons. Left/right picks the car (or paint), up/down switches rows, Confirm
// starts the race.
import { useEffect } from "react";

import { bevy, type UiCar } from "../bevy";
import { Badge } from "../components/Badge";
import { Caption, StatBar } from "../components/bits";
import { Button } from "../components/Button";
import { LockIcon } from "../components/icons";
import { Carousel } from "../components/lists";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { rgbHex } from "../format";
import { useSeen } from "../hooks";
import { back } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { draft, startGate, startMessage } from "../state/draft";
import { appState, catalog, profile } from "../state/stores";
import { carUnlocked, isFresh, lockText, paintUnlocked } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { className, driveName, hint, paintSetName, word } from "../strings/common";
import { carSelect } from "../strings/menu";
import { race } from "../strings/race";
import { focusTransition } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";
import { HeroCar } from "./MainMenu";

export function CarSelect() {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const d = useStore(draft);
  const showroom = useStore(appState)?.showroom ?? false;
  const cars = cat?.cars ?? [];
  const initial = Math.max(0, cars.findIndex((c) => c.id === d.car && carUnlocked(p, c.id)));
  const maxPaints = Math.max(1, ...cars.map((c) => c.paints.length));
  const start = (car: UiCar | undefined, paint: number) => {
    if (!car || !carUnlocked(p, car.id) || !car.paints[paint] || !paintUnlocked(p, car.paints[paint])) return sfx("error");
    if (!startGate(performance.now())) return;
    sfx("select");
    draft.update((x) => ({ ...x, car: car.id, paint }));
    bevy.race.start(startMessage({ ...d, car: car.id, paint }, [{ slot: 0, car: car.id, paint }]));
  };
  const nav = useFocusNav(
    {
      cars: { count: cars.length, cols: cars.length, wrap: true, down: "paints" },
      paints: { count: maxPaints, cols: maxPaints, wrap: true, up: "cars" },
    },
    // The paint row starts at the draft's paint.
    { region: "cars", index: initial, memory: { paints: cars[initial]?.id === d.car ? d.paint : 0 } },
    {
      confirm: () => start(car, paintIndex),
      back,
    },
  );
  const carIndex = nav.focus.region === "cars" ? nav.focus.index : (nav.focus.memory?.cars ?? initial);
  const car = cars[carIndex];
  const paintIndex = Math.min(nav.focus.region === "paints" ? nav.focus.index : (nav.focus.memory?.paints ?? 0), (car?.paints.length ?? 1) - 1);
  const unlocked = !!car && carUnlocked(p, car.id);

  // The 3D showroom shows the focused car and paint.
  useEffect(() => {
    if (car) bevy.menu.backdrop({ car: car.id, paint: paintIndex, focus: "car" });
  }, [car, paintIndex]);
  useEffect(() => () => bevy.menu.backdrop({ car: draft.get().car, paint: draft.get().paint, focus: "wide" }), []);
  useSeen(car ? { kind: "car", id: car.id } : null, !!car && isFresh(p, { kind: "car", id: car.id }));

  const mode = d.mode === "championship" ? cat?.cups.find((c) => c.id === d.cup)?.name ?? race.modeName.championship : race.modeName[d.mode];
  return (
    <Screen
      title={carSelect.title}
      kicker={mode}
      hints={[
        { action: "move", label: hint.browse },
        { action: "confirm", label: carSelect.start, onPress: () => start(car, paintIndex) },
        { action: "back", label: hint.back, onPress: back },
      ]}
    >
      <node style={{ flexGrow: 1, flexDirection: "row", gap: space.xl, minHeight: 0 }}>
        {car && (
          <Panel style={{ width: 470, gap: space.m, alignSelf: "flexStart" }} padding={30}>
            <node style={{ flexDirection: "row", alignItems: "center", justifyContent: "spaceBetween" }}>
              <text style={{ ...text.h2, fontSize: 48 }}>{car.name.toUpperCase()}</text>
              {isFresh(p, { kind: "car", id: car.id }) && <Badge label={word.new} />}
            </node>
            <node style={{ flexDirection: "row", gap: space.s }}>
              <Pill label={className[car.class]} tone={color.cyan} />
              <Pill label={driveName[car.drive]} tone={color.textDim} />
            </node>
            <StatBar label={carSelect.stats.speed} value={car.stars.speed} />
            <StatBar label={carSelect.stats.accel} value={car.stars.accel} />
            <StatBar label={carSelect.stats.handling} value={car.stars.handling} />
            <StatBar label={carSelect.stats.weight} value={car.stars.weight} tint={color.volt} />
            {!unlocked && (
              <node style={{ flexDirection: "row", gap: 10, alignItems: "center", margin: { top: space.s } }}>
                <LockIcon size={24} color={color.warning} />
                <text style={{ ...text.small, color: color.warning, flexShrink: 1 }}>{lockText(car.unlock, cat)}</text>
              </node>
            )}
          </Panel>
        )}
        <node style={{ flexGrow: 1, flexDirection: "column", alignItems: "center" }}>
          {!showroom && car && <HeroCar car={car} width={430} />}
          {showroom && <node style={{ flexGrow: 1 }} />}
          {car && <Paints car={car} index={paintIndex} focusedRow={nav.focus.region === "paints"} onPick={(i) => nav.hover("paints", i)} />}
        </node>
        <node style={{ width: 300, flexDirection: "column", justifyContent: "flexEnd" }}>
          <Button label={carSelect.start} variant="primary" focused={false} locked={!unlocked} onPress={() => start(car, paintIndex)} style={{ justifyContent: "center" }} />
        </node>
      </node>
      <Carousel
        count={cars.length}
        index={carIndex}
        itemWidth={200}
        width={1744}
        height={170}
        render={(i) => <CarCard car={cars[i]} focused={nav.focus.region === "cars" && i === carIndex} selected={i === carIndex} onPress={() => nav.hover("cars", i)} />}
      />
    </Screen>
  );
}

function Paints({ car, index, focusedRow, onPick }: { car: UiCar; index: number; focusedRow: boolean; onPick: (i: number) => void }) {
  const p = useStore(profile);
  const paint = car.paints[index];
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: 10, margin: { bottom: space.m } }}>
      <node style={{ flexDirection: "row", gap: 12 }}>
        {car.paints.map((paintInfo, i) => {
          const open = paintUnlocked(p, paintInfo);
          const on = i === index;
          return (
            <button
              key={paintInfo.id}
              onClick={() => onPick(i)}
              style={{
                width: 52,
                height: 52,
                borderRadius: radius.pill,
                border: 3,
                borderColor: on ? (focusedRow ? color.cyan : color.text) : "rgba(255, 255, 255, 0.15)",
                backgroundGradient: { type: "linear", angle: 135, stops: [{ color: rgbHex(paintInfo.base) }, { color: rgbHex(paintInfo.base), position: "60%" }, { color: rgbHex(paintInfo.accent), position: "61%" }] },
                transform: { scale: on ? 1.15 : 1 },
                transition: focusTransition,
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {!open && <LockIcon size={22} color={color.text} />}
            </button>
          );
        })}
      </node>
      {paint && (
        <node style={{ flexDirection: "row", gap: 10, alignItems: "center" }}>
          <Caption tint={focusedRow ? color.cyan : color.textDim}>{carSelect.paint}</Caption>
          <text style={text.label}>{paint.name}</text>
          {!paintUnlocked(p, paint) && <text style={{ ...text.small, color: color.warning }}>{carSelect.paintLocked(paintSetName[paint.set])}</text>}
        </node>
      )}
    </node>
  );
}

function CarCard({ car, focused, selected, onPress }: { car: UiCar; focused: boolean; selected: boolean; onPress: () => void }) {
  const p = useStore(profile);
  const unlocked = carUnlocked(p, car.id);
  return (
    <button
      onClick={onPress}
      style={{
        height: 160,
        flexDirection: "column",
        alignItems: "center",
        padding: 8,
        borderRadius: radius.l,
        border: 2,
        borderColor: selected ? (focused ? color.cyan : "rgba(25, 217, 255, 0.4)") : color.glassBorder,
        backgroundColor: selected ? "rgba(18, 34, 72, 0.9)" : "rgba(10, 18, 40, 0.7)",
        transform: { scale: selected ? 1.06 : 0.94 },
        transition: focusTransition,
      }}
    >
      {car.thumbnail && <image src={car.thumbnail} tint={unlocked ? "#ffffff" : "#05070d"} style={{ width: 118, height: 118, imageRendering: "trilinear" }} />}
      <text style={{ ...text.label, fontSize: 19, color: unlocked ? color.text : color.textFaint }}>{car.name}</text>
      {!unlocked && (
        <node style={{ positionType: "absolute", right: 10, top: 10 }}>
          <LockIcon size={20} />
        </node>
      )}
    </button>
  );
}

function Pill({ label, tone }: { label: string; tone: string }) {
  return (
    <node style={{ padding: { horizontal: 12, vertical: 4 }, borderRadius: radius.pill, border: 1, borderColor: tone }}>
      <text style={{ ...text.caps, color: tone, letterSpacing: 1.5 }}>{label.toUpperCase()}</text>
    </node>
  );
}
