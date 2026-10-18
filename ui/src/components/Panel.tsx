import type { ReactNode } from "react";
import type { BevyStyle } from "bevy-react";

import { useSplit } from "../hooks";
import { glass, glassFlat } from "../theme/styles";
import { space } from "../theme/tokens";

/** A frosted-glass surface; translucent (no backdrop blur) in split-screen overlays. */
export function Panel({ children, style, padding = space.l }: { children?: ReactNode; style?: BevyStyle; padding?: number }) {
  const split = useSplit();
  return <node style={{ ...(split ? glassFlat : glass), padding, flexDirection: "column", ...style }}>{children}</node>;
}
