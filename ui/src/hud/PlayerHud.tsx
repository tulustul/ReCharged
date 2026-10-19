// One player's HUD inside their view (GDD §13): position, lap and times top-left, item top-right,
// speed and charge bottom-right, the heading-up minimap bottom-left (1P/2P), and a centre column for
// the turn indicator, notices, warnings, the countdown/banners (wrong way, lost) and hints. Sizes
// follow the view (`hudLayout`) and HUD scale.
import type { UiHudSetup, UiHudTick, UiRacePhase, UiViewport } from "../bevy";
import { useStore } from "../state/useStore";
import { Center } from "./Center";
import { Gauges } from "./Gauges";
import { Hints } from "./Hints";
import { ItemSlot } from "./ItemSlot";
import { hudLayout } from "./layout";
import { Minimap } from "./Minimap";
import { Notices } from "./Notices";
import { Standing } from "./Standing";
import { countdown, moments, notices, type Moments } from "./store";
import { TurnNote, turnNoteHeight } from "./TurnNote";
import type { Dot, PlayerValues } from "./values";
import { Warnings } from "./Warnings";

const NONE: Moments = { banner: null, split: null, lap: null, hint: null };

type Props = { view: UiViewport; setup: UiHudSetup; tick: UiHudTick; values: PlayerValues; dots: Dot[]; views: number; phase: UiRacePhase | null };

export function PlayerHud({ view, setup, tick, values, dots, views, phase }: Props) {
  const m = useStore(moments)[view.slot] ?? NONE;
  const count = useStore(countdown);
  const list = useStore(notices).filter((n) => n.slot === view.slot);
  const p = tick.players.find((x) => x.slot === view.slot);
  if (!p) return null;
  const { rect } = view;
  const { k, compact, pad } = hudLayout(rect, setup.hudScale);
  const calm = setup.reduceFlashing;
  const finished = p.finishMs !== null;
  const showMap = setup.minimap && views <= 2 && setup.outline.length >= 4;
  const items = setup.items && setup.mode.kind !== "timeTrial";
  const waiting = finished && setup.mode.kind === "versus" && tick.players.some((x) => x.finishMs === null);
  const shownCount = count ? count.n : phase === "countdown" && tick.countdown > 0 ? tick.countdown : null;
  const wrongWay = p.wrongWay && !finished;
  const lost = p.lost && !wrongWay ? { guide: values.guide, respawnIn: p.respawnIn } : null;
  const turn = phase === "racing" && !wrongWay && !lost ? p.turn : null;
  return (
    <node style={{ positionType: "absolute", left: rect.x, top: rect.y, width: rect.w, height: rect.h, overflowX: "clip", overflowY: "clip", border: views > 1 ? 1 : 0, borderColor: "rgba(0, 0, 0, 0.6)" }}>
      <node style={{ positionType: "absolute", left: pad, top: pad }}>
        <Standing setup={setup} p={p} raceMs={tick.raceMs} moments={m} k={k} compact={compact} showSlot={views > 1} />
      </node>
      {items && (
        <node style={{ positionType: "absolute", right: pad, top: pad }}>
          <ItemSlot item={p.item} roulette={p.roulette} k={k} />
        </node>
      )}
      <node style={{ positionType: "absolute", right: pad, bottom: pad }}>
        <Gauges kmh={p.kmh} unit={setup.speedUnit} ready={p.boostReady} spending={p.spending} values={values} k={k} calm={calm} />
      </node>
      {showMap && (
        <node style={{ positionType: "absolute", left: pad, bottom: pad }}>
          <Minimap
            size={Math.round(Math.min(270 * k, rect.h * 0.42))}
            lines={setup}
            racers={setup.racers}
            dots={dots}
            own={[p.racer]}
            k={k}
            markers={setup.colorblindMarkers}
            heading={values.heading}
            zoom={setup.mapZoom}
            gate={finished ? undefined : p.nextGate}
          />
        </node>
      )}
      <node style={{ positionType: "absolute", left: 0, right: 0, top: Math.round(rect.h * (compact ? 0.06 : 0.08)), flexDirection: "column", alignItems: "center", gap: Math.round(12 * k) }}>
        {/* Full views keep its place, so notices never jump when a corner comes up; compact ones
            cannot spare the height above the centre banners. */}
        {compact ? turn && <TurnNote turn={turn} k={k} compact /> : <node style={{ height: turnNoteHeight(k), alignItems: "center", justifyContent: "center" }}>{turn && <TurnNote turn={turn} k={k} compact={false} />}</node>}
        <Notices list={list} k={k} />
        <Warnings lockOn={p.lockOn} goo={p.goo} shield={p.shield} arrow={values.arrow} k={k} calm={calm} />
      </node>
      {/* Above the car (the chase camera keeps it in the middle of the view). */}
      <node style={{ positionType: "absolute", left: 0, right: 0, top: Math.round(rect.h * (compact ? 0.13 : 0.2)), flexDirection: "column", alignItems: "center" }}>
        <Center count={shownCount} countId={count?.id ?? -tick.countdown} banner={m.banner} wrongWay={wrongWay} lost={lost} waiting={waiting} k={k} calm={calm} maxH={rect.h} />
      </node>
      {/* Below the car, between the minimap and the gauges. */}
      <node style={{ positionType: "absolute", left: 0, right: 0, bottom: pad, flexDirection: "column", alignItems: "center" }}>
        <Hints slot={view.slot} k={k} hint={m.hint} finished={finished} intro={phase === "intro"} endsIn={tick.endsIn} />
      </node>
    </node>
  );
}
