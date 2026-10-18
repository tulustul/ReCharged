import type { ReactNode } from "react";
import type { BevyStyle } from "bevy-react/jsx";

import { colors, space } from "../theme/tokens";

export function Panel({ children }: { children: ReactNode }) {
  return <node style={panel}>{children}</node>;
}

const panel: BevyStyle = {
  flexDirection: "column",
  alignItems: "center",
  gap: space.m,
  padding: space.l,
  borderRadius: 16,
  border: 2,
  borderColor: colors.panelBorder,
  backgroundColor: colors.panel,
};
