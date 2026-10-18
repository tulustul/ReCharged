// The menus' background: over the 3D showroom only a vignette (readability); without one, a navy
// gradient with electric glows and speed streaks so the menus never sit on a flat colour.
import { useEffect, useState } from "react";
import { interpolate, useSharedValue, withRepeat, withTiming } from "bevy-react";

import { appState } from "../state/stores";
import { useStore } from "../state/useStore";
import { fill } from "../theme/styles";

// The `shape` typing does not list the runtime's keyword form yet (bevy-react docs, Gradients).
const farthestCorner = { keyword: "farthestCorner" } as never;
const closestSide = { keyword: "closestSide" } as never;

// `ShowroomReady` arrives before the 3D view is drawn (pipelines recompile after a race: 0.2–1 s
// measured under load), so the navy layers stay, then fade out over the showroom instead of cutting
// to black. ponytail: fixed hold; a "first frame drawn" signal from rc_render would make it exact.
const HOLD_MS = 1000;
const FADE_MS = 400;

export function Backdrop() {
  const showroom = useStore(appState)?.showroom ?? false;
  const [navy, setNavy] = useState<"shown" | "fading" | "gone">(showroom ? "gone" : "shown");
  const fade = useSharedValue(1);
  const drift = useSharedValue(0);
  useEffect(() => {
    drift.value = withRepeat(withTiming(1, { duration: 14000, easing: "easeInOut" }), { reverse: true });
  }, [drift]);
  useEffect(() => {
    if (!showroom) {
      fade.value = 1;
      setNavy("shown");
      return;
    }
    const start = setTimeout(() => {
      setNavy("fading");
      fade.value = withTiming(0, { duration: FADE_MS });
    }, HOLD_MS);
    const end = setTimeout(() => setNavy("gone"), HOLD_MS + FADE_MS);
    return () => {
      clearTimeout(start);
      clearTimeout(end);
    };
  }, [showroom, fade]);
  return (
    <node style={fill}>
      {navy !== "gone" && (
        // Opacity only while fading: its presence promotes the subtree to a layer.
        <node style={{ ...fill, ...(navy === "fading" ? { opacity: { animated: fade } } : {}) }}>
          <node style={{ ...fill, backgroundGradient: { type: "linear", angle: 180, stops: [{ color: "#0b1734" }, { color: "#070d1f", position: "55%" }, { color: "#04070f" }] } }} />
          <node
            style={{
              positionType: "absolute",
              left: -420,
              top: -520,
              width: 1500,
              height: 1500,
              backgroundGradient: { type: "radial", shape: closestSide, stops: [{ color: "rgba(25, 217, 255, 0.16)" }, { color: "rgba(25, 217, 255, 0)" }] },
              transform: { translateX: { animated: interpolate(drift, [0, 1], [0, 120]) }, translateY: { animated: interpolate(drift, [0, 1], [0, 60]) } },
            }}
          />
          <node
            style={{
              positionType: "absolute",
              right: -520,
              bottom: -640,
              width: 1500,
              height: 1500,
              backgroundGradient: { type: "radial", shape: closestSide, stops: [{ color: "rgba(242, 225, 15, 0.055)" }, { color: "rgba(242, 225, 15, 0)" }] },
              transform: { translateX: { animated: interpolate(drift, [0, 1], [0, -90]) } },
            }}
          />
          <Streaks />
        </node>
      )}
      <node style={{ ...fill, backgroundGradient: { type: "radial", shape: farthestCorner, stops: [{ color: "rgba(0, 0, 0, 0)", position: "55%" }, { color: "rgba(0, 0, 0, 0.55)" }] } }} />
      <node style={{ positionType: "absolute", left: 0, right: 0, bottom: 0, height: 220, backgroundGradient: { type: "linear", angle: 180, stops: [{ color: "rgba(2, 4, 10, 0)" }, { color: "rgba(2, 4, 10, 0.55)" }] } }} />
    </node>
  );
}

/** Thin diagonal light streaks (static: the svg rasterizes once per resize). */
function Streaks() {
  const lines: [number, number, number, string][] = [
    // [x at the top, width, length factor, colour]
    [980, 2, 1, "rgba(25, 217, 255, 0.10)"],
    [1180, 1, 0.7, "rgba(255, 255, 255, 0.06)"],
    [1260, 3, 0.85, "rgba(25, 217, 255, 0.07)"],
    [1520, 1, 1, "rgba(242, 225, 15, 0.08)"],
    [1640, 2, 0.6, "rgba(25, 217, 255, 0.06)"],
    [1820, 1, 0.9, "rgba(255, 255, 255, 0.05)"],
  ];
  return (
    <svg viewBox="0 0 1920 1080" style={fill}>
      {lines.map(([x, w, len, c]) => (
        <polygon key={x} points={[x, 0, x + w * 2, 0, x + w * 2 - 620 * len, 1080 * len, x - 620 * len, 1080 * len]} fill={c} />
      ))}
      <line x1={0} y1={812} x2={1920} y2={812} stroke="rgba(25, 217, 255, 0.05)" strokeWidth={1} />
    </svg>
  );
}
