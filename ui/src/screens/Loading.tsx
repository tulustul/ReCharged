import type { BevyStyle } from "bevy-react/jsx";

import { appStrings } from "../strings/app";
import { colors, fontSize } from "../theme/tokens";

export function Loading() {
  return <text style={label}>{appStrings.loading}</text>;
}

const label: BevyStyle = { color: colors.text, fontSize: fontSize.title / 2 };
