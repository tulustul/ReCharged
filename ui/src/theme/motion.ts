// Motion presets: springs for focus/press feedback, timings for fades and screen transitions.
// Bevy runs every animation; React only renders end states.

export const spring = {
  /** Focus moves, presses. */
  snappy: { stiffness: 420, damping: 28 },
  /** Panels settling in. */
  soft: { stiffness: 190, damping: 22 },
  /** Playful overshoot (badges, trophies). */
  bouncy: { stiffness: 320, damping: 13 },
} as const;

export const ms = {
  fast: 110,
  base: 200,
  slow: 380,
  /** Plain screen fade. */
  fade: 300,
  /** Main menu → mode charge morph. */
  charge: 640,
} as const;

/** The transition every focusable control shares. */
export const focusTransition = {
  transform: spring.snappy,
  backgroundColor: { duration: ms.fast },
  backgroundGradient: { duration: ms.fast },
} as const;
