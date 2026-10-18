// A tab bar with the previous/next tab glyphs at its ends (LB/RB, Q/E); tabs are clickable.
import { Glyph } from "./Glyph";
import { useDevice } from "../nav/input";
import { spring } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius } from "../theme/tokens";

type Props = {
  tabs: { id: string; label: string }[];
  active: number;
  onSelect: (index: number) => void;
};

export function Tabs({ tabs, active, onSelect }: Props) {
  const device = useDevice();
  const step = (delta: number) => onSelect((active + delta + tabs.length) % tabs.length);
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <button onClick={() => step(-1)} style={edge}>
        <Glyph action="prevTab" device={device} size={30} />
      </button>
      {tabs.map((tab, i) => (
        <button
          key={tab.id}
          onClick={() => onSelect(i)}
          style={{
            height: 52,
            padding: { horizontal: 22 },
            borderRadius: radius.m,
            justifyContent: "center",
            alignItems: "center",
            backgroundColor: i === active ? "rgba(25, 217, 255, 0.14)" : color.clear,
            transition: { backgroundColor: { duration: 120 } },
          }}
          hoverStyle={{ backgroundColor: i === active ? "rgba(25, 217, 255, 0.2)" : "rgba(255, 255, 255, 0.06)" }}
        >
          <text style={{ ...text.h3, fontSize: 26, color: i === active ? color.cyanHi : color.textDim }}>{tab.label.toUpperCase()}</text>
          <node
            style={{
              positionType: "absolute",
              left: 18,
              right: 18,
              bottom: 4,
              height: 3,
              borderRadius: 2,
              backgroundColor: color.cyan,
              transform: { scaleX: i === active ? 1 : 0 },
              transition: { transform: spring.snappy },
            }}
          />
        </button>
      ))}
      <button onClick={() => step(1)} style={edge}>
        <Glyph action="nextTab" device={device} size={30} />
      </button>
    </node>
  );
}

const edge = {
  padding: 6,
  borderRadius: radius.s,
  backgroundColor: color.clear,
} as const;
