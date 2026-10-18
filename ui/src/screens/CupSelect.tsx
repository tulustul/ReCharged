// UI-5 Cup select: cup cards (tracks, difficulty, best trophy, lock reason). Confirm on an unlocked
// cup → car select.
import type { UiCup, UiDifficulty } from "../bevy";
import { Badge } from "../components/Badge";
import { LockOverlay } from "../components/bits";
import { TrophyIcon } from "../components/icons";
import { Screen } from "../components/Screen";
import { useSeen } from "../hooks";
import { back, go } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { draft } from "../state/draft";
import { catalog, profile } from "../state/stores";
import { cupUnlocked, isFresh, lockText, trophyOf } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { difficultyName, hint, timeName, trophyName, weatherName, word } from "../strings/common";
import { cupSelect } from "../strings/menu";
import { focusTransition } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

const ACCENT: Record<UiDifficulty, string> = {
  rookie: color.success,
  pro: color.cyan,
  elite: "#ff4fd8",
  elitePlus: color.volt,
};

export function CupSelect() {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const cups = cat?.cups ?? [];
  const pick = (cup: UiCup | undefined) => {
    if (!cup) return;
    if (!cupUnlocked(p, cup.id)) return sfx("error");
    sfx("select");
    const first = cat?.tracks[cup.races[0]?.track ?? 0];
    draft.update((d) => ({ ...d, mode: "championship", cup: cup.id, track: first?.id ?? d.track, reverse: false, mirror: false }));
    go("carSelect");
  };
  const nav = useFocusNav({ cups: { count: cups.length, cols: cups.length, wrap: true } }, { region: "cups", index: 0 }, {
    confirm: (f) => pick(cups[f.index]),
    back,
  });
  const focused = cups[nav.focus.index];
  useSeen(focused ? { kind: "cup", id: focused.id } : null, !!focused && isFresh(p, { kind: "cup", id: focused.id }));
  return (
    <Screen
      title={cupSelect.title}
      subtitle={cupSelect.subtitle}
      hints={[
        { action: "move", label: hint.select },
        { action: "confirm", label: hint.confirm, onPress: () => pick(focused) },
        { action: "back", label: hint.back, onPress: back },
      ]}
    >
      <node style={{ flexDirection: "row", gap: space.l, flexGrow: 1, alignItems: "center" }}>
        {cups.map((cup, i) => (
          <CupCard key={cup.id} cup={cup} index={i} focused={nav.is("cups", i)} onHover={() => nav.hover("cups", i)} onPress={() => pick(cup)} />
        ))}
      </node>
    </Screen>
  );
}

function CupCard({ cup, focused, onHover, onPress }: { cup: UiCup; index: number; focused: boolean; onHover: () => void; onPress: () => void }) {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const unlocked = cupUnlocked(p, cup.id);
  const trophy = trophyOf(p, cup.id);
  const accent = ACCENT[cup.difficulty];
  return (
    <button
      onPointerEnter={onHover}
      onClick={onPress}
      style={{
        flexGrow: 1,
        flexBasis: 0,
        height: 640,
        flexDirection: "column",
        gap: space.m,
        padding: 28,
        borderRadius: radius.l,
        border: 2,
        borderColor: focused ? color.cyan : color.glassBorder,
        backgroundColor: focused ? "rgba(18, 34, 72, 0.88)" : "rgba(10, 18, 40, 0.78)",
        boxShadow: focused ? [{ color: color.cyanGlow, blurRadius: 24 }] : undefined, // `[]` would paint a default black shadow
        transform: { scale: focused ? 1.03 : 1, translateY: focused ? -8 : 0 },
        transition: focusTransition,
      }}
      pressStyle={{ transform: { scale: 0.98, translateY: -8 } }}
    >
      <node style={{ positionType: "absolute", left: 0, right: 0, top: 0, height: 6, borderRadius: { top: radius.l, right: radius.l }, backgroundColor: accent }} />
      <node style={{ flexDirection: "row", justifyContent: "spaceBetween", alignItems: "center" }}>
        <node style={{ padding: { horizontal: 12, vertical: 4 }, borderRadius: radius.pill, backgroundColor: accent }}>
          <text style={{ ...text.caps, color: color.ink, letterSpacing: 1.5 }}>{difficultyName[cup.difficulty].toUpperCase()}</text>
        </node>
        {isFresh(p, { kind: "cup", id: cup.id }) && <Badge label={word.new} />}
      </node>
      <text style={{ ...text.h2, fontSize: 40, lineBreak: "wordBoundary" }}>{cup.name.toUpperCase()}</text>
      <text style={text.caps}>{cupSelect.races(cup.races.length).toUpperCase()}</text>
      <node style={{ flexDirection: "column", gap: 12, flexGrow: 1 }}>
        {cup.races.map((race, i) => {
          const track = cat?.tracks[race.track];
          const weather = race.weather.kind === "fixed" ? weatherName[race.weather.weather] : race.weather.kind === "dynamic" ? word.dynamic : word.random;
          return (
            <node key={i} style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
              <text style={{ ...text.label, color: accent, width: 22 }}>{`${i + 1}`}</text>
              <node style={{ flexDirection: "column", flexShrink: 1 }}>
                <node style={{ flexDirection: "row", gap: 8, alignItems: "center" }}>
                  <text style={{ ...text.label, fontSize: 21 }}>{track?.name ?? "?"}</text>
                  {race.reverse && <Tag label={word.reverse} />}
                  {race.mirror && <Tag label={word.mirror} />}
                </node>
                <text style={{ ...text.small, fontSize: 16 }}>{`${weather} · ${timeName[race.time]}`}</text>
              </node>
            </node>
          );
        })}
      </node>
      <node style={{ height: 1, backgroundColor: color.line }} />
      <node style={{ flexDirection: "row", alignItems: "center", gap: 12, height: 40 }}>
        <TrophyIcon size={34} color={trophy ? color[trophy] : color.textFaint} />
        <text style={{ ...text.label, color: trophy ? color[trophy] : color.textFaint }}>
          {trophy ? `${cupSelect.bestTrophy}: ${trophyName[trophy]}` : cupSelect.noTrophy}
        </text>
      </node>
      {!unlocked && <LockOverlay reason={lockText(cup.unlock, cat)} />}
    </button>
  );
}

function Tag({ label }: { label: string }) {
  return (
    <node style={{ padding: { horizontal: 7, vertical: 1 }, borderRadius: 4, border: 1, borderColor: color.lineStrong }}>
      <text style={{ ...text.caps, fontSize: 12, letterSpacing: 1, color: color.textDim }}>{label.toUpperCase()}</text>
    </node>
  );
}
