// Bottom-right of a view: digital speed (unit setting) over a speed bar, and the battery charge
// meter — fill and charging glow follow shared values every frame; it glows when boost-ready (from
// 15 %) and flashes while charge is spent.
import { useEffect } from "react";
import { interpolate, interpolateColor, useSharedValue, withRepeat, withTiming, type SharedValue } from "bevy-react";

import type { UiSpeedUnit } from "../bevy";
import { hud as words, unitName } from "../strings/hud";
import { color } from "../theme/tokens";
import { fs, speedIn } from "./layout";
import { caps, chip, num, tone } from "./look";
import { GAUGE_MAX, type PlayerValues } from "./values";

/** rc_core `BOOST_MIN_CHARGE`: the meter's ready mark. */
const BOOST_MIN = 0.15;

type Props = { kmh: number; unit: UiSpeedUnit; ready: boolean; spending: boolean; values: PlayerValues; k: number; calm: boolean };

export function Gauges({ kmh, unit, ready, spending, values, k, calm }: Props) {
  const w = Math.round(300 * k);
  return (
    <node style={{ flexDirection: "column", alignItems: "flexEnd", gap: Math.round(10 * k), padding: { horizontal: Math.round(16 * k), top: Math.round(6 * k), bottom: Math.round(10 * k) }, ...chip(k) }}>
      <node style={{ flexDirection: "row", alignItems: "flexEnd", gap: Math.round(8 * k) }}>
        <text style={num(fs(76, k, 36))}>{String(speedIn(kmh, unit))}</text>
        <text style={{ ...caps(fs(20, k), color.textDim), margin: { bottom: Math.round(8 * k) } }}>{unitName[unit].toUpperCase()}</text>
      </node>
      <Bar value={values.speed} max={GAUGE_MAX} w={w} h={Math.max(4, Math.round(7 * k))} fill={color.cyanHi} />
      <Battery charge={values.charge} glow={values.glow} w={w} h={Math.round(40 * k)} ready={ready} spending={spending} calm={calm} k={k} />
      <text style={caps(fs(16, k), ready ? color.volt : color.textFaint)}>{ready ? words.boostReady : words.charge}</text>
    </node>
  );
}

/** A left-anchored fill (`scaleX` + compensating `translateX`: transforms only, no relayout). */
function fillTransform(v: SharedValue, max: number, w: number) {
  return {
    transform: {
      scaleX: { animated: interpolate(v, [0, max], [0.001, 1]) },
      translateX: { animated: interpolate(v, [0, max], [-w / 2, 0]) },
    },
  };
}

function Bar({ value, max, w, h, fill }: { value: SharedValue; max: number; w: number; h: number; fill: string }) {
  return (
    <node style={{ width: w, height: h, borderRadius: h, backgroundColor: "rgba(255, 255, 255, 0.14)", overflowX: "clip", overflowY: "clip" }}>
      <node style={{ width: w, height: h, borderRadius: h, backgroundGradient: { type: "linear", angle: 90, stops: [{ color: color.cyanMid }, { color: fill }] }, ...fillTransform(value, max, w) }} />
    </node>
  );
}

type BatteryProps = { charge: SharedValue; glow: SharedValue; w: number; h: number; ready: boolean; spending: boolean; calm: boolean; k: number };

function Battery({ charge, glow, w, h, ready, spending, calm, k }: BatteryProps) {
  const r = Math.round(8 * k);
  const inner = w - 8;
  const flash = useSharedValue(0);
  useEffect(() => {
    flash.value = spending && !calm ? withRepeat(withTiming(1, { duration: 140 }), { reverse: true }) : 0;
  }, [flash, spending, calm]);
  const steady = spending ? "#ffffff" : ready ? color.volt : color.cyan;
  return (
    <node style={{ flexDirection: "row", alignItems: "center" }}>
      <node
        style={{
          width: w,
          height: h,
          padding: 3,
          borderRadius: r,
          border: 2,
          borderColor: ready ? color.volt : "rgba(170, 200, 255, 0.5)",
          backgroundColor: tone.chipStrong,
          boxShadow: ready ? [{ color: color.voltGlow, blurRadius: Math.round(18 * k) }] : undefined,
          overflowX: "clip",
          overflowY: "clip",
        }}
      >
        <node
          style={{
            width: inner - 4,
            height: "100%",
            borderRadius: r - 3,
            backgroundColor: spending && !calm ? { animated: interpolateColor(flash, [0, 1], [color.volt, "#ffffff"]) } : steady,
            ...fillTransform(charge, 1, inner - 4),
          }}
        />
        {/* Charging glow over the filled part only (same transform as the fill). */}
        <node style={{ positionType: "absolute", left: 3, top: 3, bottom: 3, width: inner - 4, borderRadius: r - 3, backgroundColor: color.cyanHi, opacity: { animated: interpolate(glow, [0, 1], [0, 0.5]) }, ...fillTransform(charge, 1, inner - 4) }} />
        <node style={{ positionType: "absolute", left: 3 + (inner - 4) * BOOST_MIN, top: 2, bottom: 2, width: 2, backgroundColor: "rgba(255, 255, 255, 0.7)" }} />
      </node>
      <node style={{ width: Math.round(7 * k), height: Math.round(h * 0.42), borderRadius: { top: 0, right: 3, bottom: 3, left: 0 }, backgroundColor: ready ? color.volt : "rgba(170, 200, 255, 0.5)" }} />
    </node>
  );
}
