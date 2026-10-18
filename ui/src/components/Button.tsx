// The focusable button. Focus comes from the screen's focus model (ui.nav or pointer hover), never
// from Bevy's InputFocus: no tabIndex, so Enter/Space never double-activate.
import type { ReactNode } from "react";
import type { BevyStyle } from "bevy-react";

import { Badge } from "./Badge";
import { LockIcon } from "./icons";
import { focusTransition } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, size } from "../theme/tokens";

export type ButtonVariant = "menu" | "primary" | "secondary" | "danger";

type Props = {
  label: string;
  focused?: boolean;
  onPress?: () => void;
  onHover?: () => void;
  /** Shown dimmed with a lock; the screen decides what pressing does (error cue). */
  locked?: boolean;
  badge?: string;
  variant?: ButtonVariant;
  icon?: ReactNode;
  width?: number | string;
  /** Smaller (secondary main-menu entries). */
  compact?: boolean;
  style?: BevyStyle;
};

// Compositing is in linear light: navy-tinted translucency stays navy (white-based reads grey).
const idle = { type: "linear" as const, angle: 90, stops: [{ color: "rgba(28, 48, 98, 0.62)" }, { color: "rgba(16, 28, 60, 0.42)" }] };
const hot: Record<ButtonVariant, typeof idle> = {
  menu: { ...idle, stops: [{ color: color.cyan }, { color: color.cyanMid }] },
  primary: { ...idle, stops: [{ color: color.volt }, { color: "#ffc61a" }] },
  secondary: { ...idle, stops: [{ color: color.cyan }, { color: color.cyanMid }] },
  danger: { ...idle, stops: [{ color: color.danger }, { color: "#d93a52" }] },
};

export function Button({ label, focused = false, onPress, onHover, locked, badge, variant = "secondary", icon, width, compact, style }: Props) {
  const menu = variant === "menu";
  const ink = focused ? color.ink : locked ? color.textFaint : color.text;
  return (
    <button
      onClick={onPress}
      onPointerEnter={onHover}
      style={{
        width,
        height: compact ? 50 : menu ? 66 : 58,
        flexDirection: "row",
        alignItems: "center",
        gap: 14,
        padding: { left: menu ? 30 : 24, right: 24 },
        borderRadius: radius.m,
        border: 1,
        borderColor: focused ? "rgba(255, 255, 255, 0.5)" : variant === "primary" ? color.voltGlow : color.glassBorder,
        backgroundGradient: focused ? hot[variant] : idle,
        boxShadow: focused ? [{ color: variant === "primary" ? color.voltGlow : color.cyanGlow, blurRadius: 18 }] : [],
        transform: { scale: focused ? 1.035 : 1 },
        transition: focusTransition,
        ...style,
      }}
      pressStyle={{ transform: { scale: 0.97 } }}
    >
      {/* The volt charge bar that marks focus. */}
      <node
        style={{
          positionType: "absolute",
          left: 8,
          top: compact ? 12 : 14,
          bottom: compact ? 12 : 14,
          width: 5,
          borderRadius: 3,
          backgroundColor: focused && variant !== "primary" ? color.volt : color.clear,
        }}
      />
      {icon}
      <text style={{ ...(menu ? text.menu : text.button), fontSize: compact ? 28 : menu ? size.menu : size.button, color: ink }}>
        {menu ? label.toUpperCase() : label}
      </text>
      <node style={{ flexGrow: 1 }} />
      {locked && <LockIcon size={menu ? 26 : 22} color={focused ? color.ink : color.textFaint} />}
      {badge && <Badge label={badge} />}
    </button>
  );
}
