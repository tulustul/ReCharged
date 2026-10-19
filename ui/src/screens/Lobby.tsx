// UI-9 Versus lobby: 4 join slots (device glyph, P1–P4, slot colour, car, ready). Each joined
// player steers their own focus (ui.nav carries the slot); devices join/leave in rc_input. When
// everyone (2+) is ready, P1 picks the track.
import { useEffect, type ReactNode } from "react";

import { bevy, type UiCar, type UiDeviceKind } from "../bevy";
import { Glyph } from "../components/Glyph";
import { ChevronIcon, ControllerIcon, KeyboardIcon } from "../components/icons";
import { Screen } from "../components/Screen";
import { rgbHex } from "../format";
import { useRoomBackdrop } from "../hooks";
import { useNav } from "../nav/input";
import { go, back } from "../nav/routes";
import { sfx } from "../sound";
import { draft } from "../state/draft";
import { allReady, lobbyChoices, slotStep, syncSlots, type SlotChoice } from "../state/lobby";
import { catalog, lobby, profile } from "../state/stores";
import { paintUnlocked } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { className, hint, word } from "../strings/common";
import { lobby as strings } from "../strings/menu";
import { focusTransition, spring } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, slotColor, space } from "../theme/tokens";

/** Pause between "everyone ready" and the track select, ms (time to un-ready). */
const START_DELAY_MS = 1400;

export function Lobby() {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const state = useStore(lobby);
  const choices = useStore(lobbyChoices);
  const cars = (cat?.cars ?? []).filter((c) => p?.unlocked.cars.includes(c.id));
  const joined = (state?.slots ?? []).map((s) => s.slot);
  const paintsOf = (car: number) => (cars[car]?.paints ?? []).filter((paint) => paintUnlocked(p, paint)).length || 1;

  useEffect(() => {
    bevy.lobby.open(null);
    // Back from the track select: everyone confirms again.
    lobbyChoices.update((all) => Object.fromEntries(Object.entries(all).map(([slot, c]) => [slot, { ...c, ready: false }])));
  }, []);
  useEffect(() => {
    lobbyChoices.set(syncSlots(lobbyChoices.get(), joined, cars.length));
  }, [joined.join(","), cars.length]);
  useRoomBackdrop();
  const ready = allReady(joined, choices);
  useEffect(() => {
    if (!ready) return;
    const timer = setTimeout(() => {
      draft.update((d) => ({ ...d, mode: "versus", cup: null }));
      go("trackSelect");
    }, START_DELAY_MS);
    return () => clearTimeout(timer);
  }, [ready]);

  const leave = () => {
    sfx("back");
    bevy.lobby.close(null);
    back();
  };
  useNav((e) => {
    // Devices without a slot only leave the lobby (joining is rc_input's).
    if (e.slot === null || !joined.includes(e.slot)) {
      if (e.action === "back") leave();
      return;
    }
    const current = choices[e.slot];
    if (!current || e.action === "back") return; // Back on a joined slot leaves it (rc_input)
    const { choice, cue } = slotStep(current, e.action, cars.length, paintsOf);
    if (cue) sfx(cue);
    if (choice !== current) lobbyChoices.update((all) => ({ ...all, [e.slot as number]: choice }));
  });

  const devices = (state?.slots ?? []).map((s) => s.device);
  const twoKeyboards = devices.includes("keyboardA") && devices.includes("keyboardB");
  const status = ready ? strings.allReady : joined.length < 2 ? (joined.length === 0 ? strings.waiting : strings.needTwo) : strings.notReady;
  return (
    <Screen
      title={strings.title}
      subtitle={strings.subtitle}
      hints={[{ action: "back", label: strings.leave, onPress: leave }]}
      footerLeft={
        <>
          <text style={{ ...text.label, color: ready ? color.volt : color.textDim }}>{status}</text>
          {twoKeyboards && <text style={{ ...text.small, color: color.warning }}>{strings.keyboardWarning}</text>}
        </>
      }
    >
      <node style={{ flexDirection: "row", gap: space.l, flexGrow: 1 }}>
        {[0, 1, 2, 3].map((slot) => {
          const s = state?.slots.find((x) => x.slot === slot);
          return s && choices[slot] ? (
            <JoinedSlot key={slot} slot={slot} device={s.device} choice={choices[slot]} cars={cars} paintsOf={paintsOf} />
          ) : (
            <EmptySlot key={slot} slot={slot} />
          );
        })}
      </node>
    </Screen>
  );
}

function SlotFrame({ slot, active, children }: { slot: number; active: boolean; children: ReactNode }) {
  return (
    <node
      style={{
        flexGrow: 1,
        flexBasis: 0,
        flexDirection: "column",
        alignItems: "center",
        gap: 12,
        padding: { horizontal: 24, top: 30, bottom: 20 },
        borderRadius: radius.l,
        border: 2,
        borderColor: active ? slotColor[slot] : color.glassBorder,
        backgroundColor: active ? "rgba(14, 26, 56, 0.85)" : "rgba(8, 14, 30, 0.55)",
      }}
    >
      <node style={{ positionType: "absolute", left: 0, right: 0, top: 0, height: 8, borderRadius: { top: radius.l, right: radius.l }, backgroundColor: slotColor[slot], opacity: active ? 1 : 0.35 }} />
      {children}
    </node>
  );
}

function EmptySlot({ slot }: { slot: number }) {
  return (
    <SlotFrame slot={slot} active={false}>
      <text style={{ ...text.title, color: slotColor[slot], opacity: 0.55 }}>{word.player(slot)}</text>
      <node style={{ flexGrow: 1, alignItems: "center", justifyContent: "center", flexDirection: "column", gap: space.m }}>
        <node style={{ width: 120, height: 120, borderRadius: radius.pill, border: 3, borderColor: "rgba(255, 255, 255, 0.18)", alignItems: "center", justifyContent: "center" }}>
          <text style={{ ...text.hero, fontSize: 80, color: "rgba(255, 255, 255, 0.35)" }}>+</text>
        </node>
        <text style={{ ...text.h3, color: color.textDim }}>{strings.join.toUpperCase()}</text>
        <node style={{ flexDirection: "row", gap: 10, alignItems: "center" }}>
          <Glyph action="confirm" device="gamepad" size={34} tint={color.textDim} />
          <Glyph action="confirm" device="keyboardA" lobby size={34} tint={color.textDim} />
          <Glyph action="confirm" device="keyboardB" lobby size={34} tint={color.textDim} />
        </node>
      </node>
    </SlotFrame>
  );
}

function JoinedSlot({ slot, device, choice, cars, paintsOf }: { slot: number; device: UiDeviceKind; choice: SlotChoice; cars: UiCar[]; paintsOf: (car: number) => number }) {
  const p = useStore(profile);
  const car = cars[choice.car];
  const paints = (car?.paints ?? []).filter((paint) => paintUnlocked(p, paint));
  const tint = slotColor[slot];
  const rowStyle = (on: boolean) => ({
    flexDirection: "row" as const,
    alignItems: "center" as const,
    gap: 10,
    padding: { horizontal: 10, vertical: 6 },
    borderRadius: radius.m,
    backgroundColor: on && !choice.ready ? "rgba(255, 255, 255, 0.08)" : color.clear,
    transition: focusTransition,
  });
  return (
    <SlotFrame slot={slot} active>
      <node style={{ flexDirection: "row", alignItems: "center", gap: 14 }}>
        <text style={{ ...text.title, color: tint }}>{word.player(slot)}</text>
        {device === "gamepad" ? <ControllerIcon size={40} color={tint} /> : <KeyboardIcon size={40} color={tint} />}
      </node>
      <text style={{ ...text.small, fontSize: 15 }}>{device === "keyboardA" ? strings.schemeA : device === "keyboardB" ? strings.schemeB : ""}</text>
      {car?.thumbnail && <image src={car.thumbnail} style={{ width: 250, height: 250, imageRendering: "trilinear", transform: { scale: choice.ready ? 1.05 : 1 }, transition: { transform: spring.bouncy } }} />}
      <node style={rowStyle(choice.row === "car")}>
        <ChevronIcon dir="left" size={22} color={choice.row === "car" && !choice.ready ? tint : color.textFaint} />
        <node style={{ flexDirection: "column", alignItems: "center", width: 220 }}>
          <text style={{ ...text.h3 }}>{(car?.name ?? "").toUpperCase()}</text>
          <text style={{ ...text.caps, fontSize: 13 }}>{car ? className[car.class].toUpperCase() : ""}</text>
        </node>
        <ChevronIcon dir="right" size={22} color={choice.row === "car" && !choice.ready ? tint : color.textFaint} />
      </node>
      <node style={{ ...rowStyle(choice.row === "paint"), gap: 8 }}>
        {paints.map((paint, i) => (
          <node
            key={paint.id}
            style={{
              width: 30,
              height: 30,
              borderRadius: radius.pill,
              border: 2,
              borderColor: i === Math.min(choice.paint, paintsOf(choice.car) - 1) ? color.text : "rgba(255, 255, 255, 0.15)",
              backgroundColor: rgbHex(paint.base),
            }}
          />
        ))}
      </node>
      <node style={{ flexGrow: 1 }} />
      <node
        style={{
          alignSelf: "stretch",
          height: 56,
          borderRadius: radius.m,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: choice.ready ? color.volt : "rgba(255, 255, 255, 0.06)",
          transition: { backgroundColor: { duration: 150 } },
        }}
      >
        <text style={{ ...text.h3, color: choice.ready ? color.ink : color.textDim }}>{choice.ready ? strings.ready : strings.notReady.toUpperCase()}</text>
      </node>
      <node style={{ flexDirection: "row", gap: space.m, alignItems: "center" }}>
        <Glyph action="confirm" device={device} lobby size={28} />
        <text style={text.small}>{hint.ready}</text>
        <Glyph action="back" device={device} lobby size={28} />
        <text style={text.small}>{strings.leave}</text>
      </node>
    </SlotFrame>
  );
}
