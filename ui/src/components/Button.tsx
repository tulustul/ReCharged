import type { BevyStyle } from "bevy-react/jsx";

import { colors, fontSize, space } from "../theme/tokens";

// ponytail: mouse-only placeholder; T05 builds the navigable Button (ui.nav focus, cues).
export function Button({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button style={base} hoverStyle={hover} pressStyle={press} onClick={onClick}>
      <text style={labelStyle}>{label}</text>
    </button>
  );
}

const base: BevyStyle = {
  minWidth: 280,
  padding: { horizontal: space.l, vertical: space.m },
  borderRadius: 10,
  border: 2,
  borderColor: colors.panelBorder,
  backgroundColor: "#1a2547",
  justifyContent: "center",
  alignItems: "center",
};

const hover: BevyStyle = { borderColor: colors.cyan, backgroundColor: "#22325f" };

const press: BevyStyle = { borderColor: colors.volt, backgroundColor: "#2b3f78" };

const labelStyle: BevyStyle = { color: colors.text, fontSize: fontSize.button, fontWeight: "bold" };
