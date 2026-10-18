// UI-14 Garage: every car (locked ones as silhouettes with their unlock hint), stats and paints.
// Confirm makes the focused car your pick for the next race.
import { useEffect } from "react";

import { bevy, type UiCar } from "../bevy";
import { Badge } from "../components/Badge";
import { Caption, StatBar } from "../components/bits";
import { CheckIcon, LockIcon } from "../components/icons";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { rgbHex } from "../format";
import { useSeen } from "../hooks";
import { back } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { draft } from "../state/draft";
import { catalog, profile } from "../state/stores";
import { carUnlocked, isFresh, lockText, paintUnlocked } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { className, driveName, hint, paintSetName, word } from "../strings/common";
import { carSelect, garage } from "../strings/menu";
import { focusTransition } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

export function Garage() {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const picked = useStore(draft).car;
  const cars = cat?.cars ?? [];
  const pick = (car: UiCar | undefined) => {
    if (!car || !carUnlocked(p, car.id)) return sfx("error");
    sfx("select");
    draft.update((d) => ({ ...d, car: car.id, paint: 0 }));
  };
  const nav = useFocusNav({ cars: { count: cars.length, cols: 4 } }, { region: "cars", index: Math.max(0, cars.findIndex((c) => c.id === picked)) }, {
    confirm: (f) => pick(cars[f.index]),
    back,
  });
  const car = cars[nav.focus.index];
  useEffect(() => {
    if (car && carUnlocked(profile.get(), car.id)) bevy.menu.backdrop({ car: car.id, paint: 0, focus: "car" });
  }, [car]);
  useSeen(car ? { kind: "car", id: car.id } : null, !!car && isFresh(p, { kind: "car", id: car.id }));
  const unlockedCount = cars.filter((c) => carUnlocked(p, c.id)).length;
  return (
    <Screen
      title={garage.title}
      subtitle={garage.subtitle(unlockedCount, cars.length)}
      hints={[
        { action: "move", label: hint.select },
        { action: "confirm", label: hint.select, onPress: () => pick(car) },
        { action: "back", label: hint.back, onPress: back },
      ]}
    >
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <node style={{ flexGrow: 1, display: "grid", gridTemplateColumns: "repeat(4, 1flex)", gridTemplateRows: "repeat(2, 1flex)", gap: space.l }}>
          {cars.map((c, i) => (
            <GarageCard key={c.id} car={c} picked={c.id === picked} focused={nav.is("cars", i)} onHover={() => nav.hover("cars", i)} onPress={() => pick(c)} />
          ))}
        </node>
        {car && <Details car={car} />}
      </node>
    </Screen>
  );
}

function GarageCard({ car, picked, focused, onHover, onPress }: { car: UiCar; picked: boolean; focused: boolean; onHover: () => void; onPress: () => void }) {
  const p = useStore(profile);
  const unlocked = carUnlocked(p, car.id);
  return (
    <button
      onPointerEnter={onHover}
      onClick={onPress}
      style={{
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 4,
        borderRadius: radius.l,
        border: 2,
        borderColor: focused ? color.cyan : color.glassBorder,
        backgroundColor: focused ? "rgba(18, 34, 72, 0.88)" : "rgba(10, 18, 40, 0.66)",
        transform: { scale: focused ? 1.04 : 1 },
        transition: focusTransition,
      }}
    >
      {car.thumbnail && <image src={car.thumbnail} tint={unlocked ? "#ffffff" : "#060912"} style={{ width: 230, height: 230, imageRendering: "trilinear" }} />}
      <text style={{ ...text.h3, fontSize: 26, color: unlocked ? color.text : color.textFaint }}>{car.name.toUpperCase()}</text>
      <text style={{ ...text.caps, fontSize: 13 }}>{className[car.class].toUpperCase()}</text>
      <node style={{ positionType: "absolute", right: 14, top: 14, flexDirection: "row", gap: 6 }}>
        {isFresh(p, { kind: "car", id: car.id }) && <Badge label={word.new} />}
        {!unlocked && <LockIcon size={26} />}
        {picked && unlocked && <CheckIcon size={28} color={color.volt} />}
      </node>
    </button>
  );
}

function Details({ car }: { car: UiCar }) {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const unlocked = carUnlocked(p, car.id);
  return (
    <Panel style={{ width: 520, gap: space.m, flexShrink: 0 }} padding={32}>
      <text style={{ ...text.h2, fontSize: 52 }}>{car.name.toUpperCase()}</text>
      <text style={{ ...text.caps, color: color.cyan }}>{`${className[car.class]} · ${driveName[car.drive]}`.toUpperCase()}</text>
      {!unlocked && (
        <node style={{ flexDirection: "row", gap: 10, alignItems: "center", padding: 14, borderRadius: radius.m, backgroundColor: "rgba(255, 176, 32, 0.12)" }}>
          <LockIcon size={26} color={color.warning} />
          <text style={{ ...text.small, color: color.warning, flexShrink: 1 }}>{lockText(car.unlock, cat)}</text>
        </node>
      )}
      <StatBar label={carSelect.stats.speed} value={car.stars.speed} />
      <StatBar label={carSelect.stats.accel} value={car.stars.accel} />
      <StatBar label={carSelect.stats.handling} value={car.stars.handling} />
      <StatBar label={carSelect.stats.weight} value={car.stars.weight} tint={color.volt} />
      <node style={{ height: 1, backgroundColor: color.line, margin: { vertical: 6 } }} />
      <Caption>{garage.paints}</Caption>
      <node style={{ flexDirection: "row", flexWrap: "wrap", gap: 12 }}>
        {car.paints.map((paint) => {
          const open = paintUnlocked(p, paint);
          return (
            <node key={paint.id} style={{ flexDirection: "column", alignItems: "center", gap: 4, width: 96 }}>
              <node
                style={{
                  width: 46,
                  height: 46,
                  borderRadius: radius.pill,
                  border: 2,
                  borderColor: "rgba(255, 255, 255, 0.2)",
                  backgroundGradient: { type: "linear", angle: 135, stops: [{ color: rgbHex(paint.base) }, { color: rgbHex(paint.base), position: "60%" }, { color: rgbHex(paint.accent), position: "61%" }] },
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                {!open && <LockIcon size={20} color={color.text} />}
              </node>
              <text style={{ ...text.small, fontSize: 14, textAlign: "center", color: open ? color.textDim : color.textFaint }}>
                {paint.set === "base" ? paint.name : paintSetName[paint.set]}
              </text>
            </node>
          );
        })}
      </node>
    </Panel>
  );
}
