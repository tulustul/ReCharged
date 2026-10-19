// The centre of a view, one moment at a time: the 3-2-1-GO countdown, the finish banner ("1st!",
// time, "Waiting for other players…"), WRONG WAY, the lost banner (an arrow back to the track and,
// off the track, the respawn countdown), or the FINAL LAP banner.
import { useEffect } from "react";
import { interpolate, useSharedValue, withRepeat, withSequence, withSpring, withTiming, type SharedValue } from "bevy-react";

import { formatTime } from "../format";
import { useEnter } from "../hooks";
import { hud as words } from "../strings/hud";
import { spring } from "../theme/motion";
import { color } from "../theme/tokens";
import { fs, placeParts } from "./layout";
import { caps, chip, label, num, tone } from "./look";
import type { Banner } from "./store";

/** The lost banner's state: the arrow's angle and the respawn countdown (off the track). */
export type LostState = { guide: SharedValue; respawnIn: number | null };

type Props = { count: number | null; countId: number; banner: Banner | null; wrongWay: boolean; lost: LostState | null; waiting: boolean; k: number; calm: boolean; maxH: number };

export function Center({ count, countId, banner, wrongWay, lost, waiting, k, calm, maxH }: Props) {
  // Numbers stay above the car: at most a quarter of the view's height.
  if (count !== null) return <Count key={countId} n={count} size={Math.round(Math.min(fs(230, k, 90), maxH * 0.26))} />;
  if (banner?.kind === "finished") return <Finished rank={banner.rank} ms={banner.ms} waiting={waiting} k={k} />;
  if (wrongWay) return <WrongWay k={k} calm={calm} />;
  if (lost) return <Lost lost={lost} k={k} />;
  if (banner?.kind === "finalLap") return <FinalLap key={banner.id} k={k} />;
  return null;
}

/** One countdown number (0 = GO!) popping in. */
function Count({ n, size }: { n: number; size: number }) {
  const s = useSharedValue(1.8);
  useEffect(() => {
    s.value = withSpring(1, spring.bouncy);
  }, [s]);
  return (
    <node style={{ transform: { scale: { animated: s } } }}>
      <text style={num(n === 0 ? Math.round(size * 0.87) : size, n === 0 ? color.volt : color.text, "900")}>{n === 0 ? words.go : String(n)}</text>
    </node>
  );
}

const placeTint = (rank: number) => (rank === 1 ? color.gold : rank === 2 ? color.silver : rank === 3 ? color.bronze : color.text);

function Finished({ rank, ms, waiting, k }: { rank: number; ms: number; waiting: boolean; k: number }) {
  const s = useSharedValue(0.6);
  useEffect(() => {
    s.value = withSpring(1, spring.bouncy);
  }, [s]);
  const [place, suffix] = placeParts(rank);
  const tint = placeTint(rank);
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: Math.round(10 * k) }}>
      <node style={{ flexDirection: "row", alignItems: "flexEnd", padding: { horizontal: Math.round(30 * k), top: Math.round(4 * k), bottom: Math.round(10 * k) }, ...chip(k, true), borderColor: tint, transform: { scale: { animated: s } } }}>
        <text style={num(fs(190, k, 80), tint, "900")}>{place}</text>
        <text style={{ ...num(fs(80, k, 36), tint, "900"), margin: { bottom: Math.round(18 * k) } }}>{`${suffix}!`}</text>
      </node>
      <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(12 * k), padding: { horizontal: Math.round(18 * k), vertical: Math.round(7 * k) }, ...chip(k, true) }}>
        <text style={caps(fs(18, k), color.cyan)}>{words.finished}</text>
        <text style={num(fs(30, k, 20))}>{formatTime(ms)}</text>
      </node>
      {waiting && (
        <node style={{ padding: { horizontal: Math.round(16 * k), vertical: Math.round(6 * k) }, ...chip(k, true) }}>
          <text style={label(fs(24, k, 18), color.textDim)}>{words.waiting}</text>
        </node>
      )}
    </node>
  );
}

function WrongWay({ k, calm }: { k: number; calm: boolean }) {
  const s = useSharedValue(1);
  useEffect(() => {
    s.value = calm ? 1 : withRepeat(withSequence(withTiming(1.08, { duration: 300 }), withTiming(1, { duration: 300 })));
  }, [s, calm]);
  const icon = Math.round(Math.max(70 * k, 40));
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(16 * k), padding: { horizontal: Math.round(28 * k), vertical: Math.round(12 * k) }, borderRadius: Math.round(16 * k), backgroundColor: "rgba(120, 10, 24, 0.72)", border: 2, borderColor: tone.bad, transform: { scale: { animated: s } } }}>
      {/* A U-turn arrow: turn around. */}
      <svg viewBox="0 0 48 48" style={{ width: icon, height: icon }}>
        <path d="M14 40 L14 20 A10 10 0 0 1 34 20 L34 30" fill="none" stroke="#ffffff" strokeWidth={5} strokeLinecap="round" />
        <polygon points={[25, 27, 34, 40, 43, 27]} fill="#ffffff" />
      </svg>
      <text style={num(fs(76, k, 36), "#ffffff", "900")}>{words.wrongWay}</text>
    </node>
  );
}

/** An arrow turning toward the track (0° = straight ahead = up), "OFF TRACK" or "THIS WAY". */
function Lost({ lost, k }: { lost: LostState; k: number }) {
  const icon = Math.round(Math.max(72 * k, 42));
  const off = lost.respawnIn !== null;
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: Math.round(8 * k) }}>
      <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(16 * k), padding: { horizontal: Math.round(26 * k), vertical: Math.round(10 * k) }, borderRadius: Math.round(16 * k), backgroundColor: "rgba(70, 40, 0, 0.74)", border: 2, borderColor: tone.warn }}>
        <node style={{ width: icon, height: icon, transform: { rotate: { animated: lost.guide } } }}>
          <svg viewBox="0 0 48 48" style={{ width: icon, height: icon }}>
            <polygon points={[24, 3, 43, 24, 31, 24, 31, 45, 17, 45, 17, 24, 5, 24]} fill="#ffffff" stroke="rgba(3, 7, 18, 0.85)" strokeWidth={3} strokeLinejoin="round" />
          </svg>
        </node>
        <text style={num(fs(64, k, 32), "#ffffff", "900")}>{off ? words.offTrack : words.thisWay}</text>
      </node>
      {off && (
        <node style={{ padding: { horizontal: Math.round(16 * k), vertical: Math.round(5 * k) }, ...chip(k, true) }}>
          <text style={label(fs(24, k, 18), tone.warn)}>{words.respawnIn(lost.respawnIn ?? 0)}</text>
        </node>
      )}
    </node>
  );
}

function FinalLap({ k }: { k: number }) {
  const enter = useEnter(-120, 0);
  const t = useSharedValue(0);
  useEffect(() => {
    t.value = withTiming(1, { duration: 400 });
  }, [t]);
  return (
    <node style={{ ...enter, padding: { horizontal: Math.round(34 * k), vertical: Math.round(8 * k) }, borderRadius: Math.round(10 * k), backgroundColor: "rgba(40, 34, 0, 0.6)", border: 2, borderColor: color.volt }}>
      <text style={{ ...num(fs(84, k, 40), color.volt, "900"), letterSpacing: Math.round(4 * k) }}>{words.finalLap}</text>
      <node style={{ positionType: "absolute", left: 0, bottom: -Math.round(8 * k), height: Math.max(3, Math.round(5 * k)), width: "100%", backgroundColor: color.volt, transform: { scaleX: { animated: interpolate(t, [0, 1], [0.001, 1]) } } }} />
    </node>
  );
}

