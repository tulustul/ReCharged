// UI-21 Cup end: podium of the top 3 cars standing on their 1-2-3 blocks (names on the blocks; the
// showroom's own car is hidden), your trophy, unlocks; the credits roll after a Grand Volt win.
// Opening it plays the podium stinger (`ui.stinger`).
import { useEffect } from "react";

import { bevy, type UiStandingRow } from "../bevy";
import { Button } from "../components/Button";
import { TrophyIcon } from "../components/icons";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { ordinal, rgbHex } from "../format";
import { backTo, resetTo } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { catalog, profile, session } from "../state/stores";
import { unlockLabel } from "../state/unlocks";
import { useRoomBackdrop } from "../hooks";
import { useStore } from "../state/useStore";
import { appStrings } from "../strings/app";
import { hint, trophyName } from "../strings/common";
import { cupEnd } from "../strings/race";
import { text } from "../theme/styles";
import { color, radius, slotColor, space } from "../theme/tokens";

/** The cup whose gold rolls the credits (GDD §9). */
const FINAL_CUP = "grand_volt_cup";

export function CupEnd() {
  const cup = useStore(session).cup;
  const cat = useStore(catalog);
  const p = useStore(profile);
  const trophy = cup?.trophy ?? null;
  useEffect(() => {
    if (trophy) bevy.ui.stinger({ stinger: trophy === "gold" ? "cupVictory" : "podiumFanfare" });
  }, [trophy]);
  const finish = () => {
    sfx("select");
    if (cup?.cup === FINAL_CUP && trophy === "gold") resetTo("main", "credits");
    else backTo("cupSelect");
  };
  useFocusNav({ go: { count: 1, cols: 1 } }, { region: "go", index: 0 }, { confirm: finish, back: finish });
  useRoomBackdrop();
  const rows = cup?.rows ?? [];
  const podium: [UiStandingRow | undefined, number][] = [
    [rows[1], 2],
    [rows[0], 1],
    [rows[2], 3],
  ];
  const name = cat?.cups.find((c) => c.id === cup?.cup)?.name ?? "";
  return (
    <Screen title={cupEnd.title} kicker={name} hints={[{ action: "confirm", label: hint.continue, onPress: finish }]}>
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <node style={{ flexGrow: 1, flexDirection: "row", alignItems: "flexEnd", justifyContent: "center", gap: space.l }}>
          {podium.map(([row, place]) => (row ? <Step key={place} row={row} place={place} /> : null))}
        </node>
        <Panel style={{ width: 500, gap: space.m, alignItems: "center", flexShrink: 0, justifyContent: "center" }} padding={36}>
          <TrophyIcon size={170} color={trophy ? color[trophy] : "rgba(255, 255, 255, 0.15)"} />
          <text style={{ ...text.h2, color: trophy ? color[trophy] : color.textDim, textAlign: "center", lineBreak: "wordBoundary" }}>
            {trophy ? cupEnd.trophy(trophyName[trophy]).toUpperCase() : ""}
          </text>
          {!trophy && <text style={{ ...text.body, textAlign: "center" }}>{cupEnd.noTrophy}</text>}
          {cup?.finalPosition && <text style={{ ...text.label, color: color.textDim }}>{ordinal(cup.finalPosition)}</text>}
          {(p?.fresh ?? []).length > 0 && (
            <node style={{ flexDirection: "column", gap: 6, alignItems: "center", margin: { top: space.s } }}>
              <text style={{ ...text.caps, color: color.volt }}>{appStrings.unlockedTitle.toUpperCase()}</text>
              {(p?.fresh ?? []).map((item) => (
                <text key={JSON.stringify(item)} style={text.label}>{unlockLabel(item, cat)}</text>
              ))}
            </node>
          )}
          <Button label={cupEnd.continue} variant="primary" focused onPress={finish} style={{ justifyContent: "center", alignSelf: "stretch", margin: { top: space.m } }} />
        </Panel>
      </node>
    </Screen>
  );
}

const HEIGHT: Record<number, number> = { 1: 310, 2: 250, 3: 205 };
const TONE: Record<number, string> = { 1: color.gold, 2: color.silver, 3: color.bronze };

function Step({ row, place }: { row: UiStandingRow; place: number }) {
  const car = useStore(catalog)?.cars.find((c) => c.id === row.car);
  const tint = row.slot !== null ? slotColor[row.slot] : rgbHex(row.color);
  const size = place === 1 ? 320 : 260;
  return (
    <node style={{ width: 330, flexDirection: "column", alignItems: "center" }}>
      {car?.thumbnail && (
        // Above the block (zIndex), overlapping it by the transparent rows under this car's
        // wheels (measured per thumbnail by rc_ui), so the car stands on the top edge.
        <image src={car.thumbnail} style={{ width: size, height: size, imageRendering: "trilinear", zIndex: 1, margin: { bottom: -Math.round(size * car.thumbnailFoot) } }} />
      )}
      <node
        style={{
          alignSelf: "stretch",
          height: HEIGHT[place],
          borderRadius: { top: radius.l, right: radius.l },
          backgroundGradient: { type: "linear", angle: 180, stops: [{ color: "rgba(30, 52, 104, 0.95)" }, { color: "rgba(10, 18, 40, 0.9)" }] },
          border: { top: 4 },
          borderColor: TONE[place],
          flexDirection: "column",
          alignItems: "center",
          padding: { top: 10 },
        }}
      >
        <text style={{ ...text.hero, fontSize: 84, color: TONE[place] }}>{`${place}`}</text>
        <text style={{ ...text.h3, color: tint }}>{row.name.toUpperCase()}</text>
        <text style={{ ...text.label, color: color.textDim }}>{cupEnd.points(row.points)}</text>
      </node>
    </node>
  );
}
