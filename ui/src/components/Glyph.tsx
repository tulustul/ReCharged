// Button glyphs: keycaps, gamepad face buttons by position, shoulders, D-pad, sticks, Start/Select.
import type { UiBinding, UiDeviceKind } from "../bevy";
import { actionGlyph, bindingGlyph, type GlyphSpec, type MenuAction } from "../glyphs";
import { color, radius } from "../theme/tokens";

type Props = { size?: number; tint?: string } & (
  | { action: MenuAction; device: UiDeviceKind; lobby?: boolean }
  | { binding: UiBinding }
  | { spec: GlyphSpec }
);

export function Glyph(props: Props) {
  const spec =
    "spec" in props ? props.spec : "binding" in props ? bindingGlyph(props.binding) : actionGlyph(props.action, props.device, props.lobby);
  return <GlyphShape spec={spec} size={props.size ?? 30} tint={props.tint ?? color.text} />;
}

function GlyphShape({ spec, size, tint }: { spec: GlyphSpec; size: number; tint: string }) {
  switch (spec.kind) {
    case "key":
      return (
        <node
          style={{
            minWidth: size,
            height: size,
            padding: { horizontal: Math.round(size * 0.22) },
            border: 2,
            borderColor: tint,
            borderRadius: radius.s,
            backgroundColor: "rgba(255, 255, 255, 0.07)",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <text style={{ fontSize: Math.round(size * (spec.label.length > 2 ? 0.42 : 0.52)), fontWeight: "700", color: tint, lineBreak: "noWrap" }}>
            {spec.label}
          </text>
        </node>
      );
    case "shoulder":
      return (
        <node
          style={{
            width: Math.round(size * 1.45),
            height: Math.round(size * 0.86),
            border: 2,
            borderColor: tint,
            // Corners clockwise from the top-left: a shoulder button is round on top.
            borderRadius: { top: size * 0.4, right: size * 0.4, bottom: 5, left: 5 },
            backgroundColor: "rgba(255, 255, 255, 0.07)",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <text style={{ fontSize: Math.round(size * 0.42), fontWeight: "800", color: tint, lineBreak: "noWrap" }}>{spec.label}</text>
        </node>
      );
    case "face": {
      const dots: [number, number, string][] = [
        [12, 4.5, "north"],
        [19.5, 12, "east"],
        [12, 19.5, "south"],
        [4.5, 12, "west"],
      ];
      return (
        <svg viewBox="0 0 24 24" style={{ width: size, height: size, flexShrink: 0 }}>
          {dots.map(([cx, cy, at]) =>
            at === spec.at ? (
              <circle key={at} cx={cx} cy={cy} r={4.2} fill={tint} />
            ) : (
              <circle key={at} cx={cx} cy={cy} r={3.2} fill="none" stroke={tint} strokeWidth={1.4} opacity={0.55} />
            ),
          )}
        </svg>
      );
    }
    case "dpad": {
      const arm = (dir: string, d: string) => (
        <path key={dir} d={d} fill={spec.dir === null || spec.dir === dir ? tint : "none"} stroke={tint} strokeWidth={1.4} opacity={spec.dir === null || spec.dir === dir ? 1 : 0.55} />
      );
      return (
        <svg viewBox="0 0 24 24" style={{ width: size, height: size, flexShrink: 0 }}>
          {arm("up", "M9 2 H15 V8.5 L12 11 L9 8.5 Z")}
          {arm("down", "M9 22 H15 V15.5 L12 13 L9 15.5 Z")}
          {arm("left", "M2 9 V15 H8.5 L11 12 L8.5 9 Z")}
          {arm("right", "M22 9 V15 H15.5 L13 12 L15.5 9 Z")}
        </svg>
      );
    }
    case "stick": {
      const arrow: Record<string, number[]> = {
        up: [12, 1, 9, 4.5, 15, 4.5],
        down: [12, 23, 9, 19.5, 15, 19.5],
        left: [1, 12, 4.5, 9, 4.5, 15],
        right: [23, 12, 19.5, 9, 19.5, 15],
      };
      return (
        <node style={{ width: size, height: size, alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
          <svg viewBox="0 0 24 24" style={{ positionType: "absolute", left: 0, top: 0, width: size, height: size }}>
            <circle cx={12} cy={12} r={spec.press ? 8.5 : 7.5} fill={spec.press ? tint : "none"} stroke={tint} strokeWidth={1.8} />
            {spec.dir && <polygon points={arrow[spec.dir]} fill={tint} />}
          </svg>
          <text style={{ fontSize: Math.round(size * 0.36), fontWeight: "800", color: spec.press ? color.navy : tint, lineBreak: "noWrap" }}>
            {spec.side}
          </text>
        </node>
      );
    }
    case "start":
    case "select":
    case "home":
      return (
        <svg viewBox="0 0 24 24" style={{ width: size, height: size, flexShrink: 0 }}>
          <rect x={1.5} y={5} width={21} height={14} rx={7} fill="none" stroke={tint} strokeWidth={1.8} />
          {spec.kind === "start" && [9, 12, 15].map((y) => <line key={y} x1={8} y1={y} x2={16} y2={y} stroke={tint} strokeWidth={1.6} strokeLinecap="round" />)}
          {spec.kind === "select" && (
            <>
              <rect x={7.5} y={8} width={6} height={5} rx={1} fill="none" stroke={tint} strokeWidth={1.4} />
              <rect x={10.5} y={11} width={6} height={5} rx={1} fill={tint} />
            </>
          )}
          {spec.kind === "home" && <circle cx={12} cy={12} r={3} fill={tint} />}
        </svg>
      );
    case "mouse":
      return (
        <svg viewBox="0 0 24 24" style={{ width: size, height: size, flexShrink: 0 }}>
          <rect x={6} y={2.5} width={12} height={19} rx={6} fill="none" stroke={tint} strokeWidth={1.8} />
          <path d="M12 2.5 a6 6 0 0 0 -6 6 V10 H12 Z" fill={tint} />
          <line x1={12} y1={2.5} x2={12} y2={10} stroke={tint} strokeWidth={1.4} />
        </svg>
      );
  }
}
