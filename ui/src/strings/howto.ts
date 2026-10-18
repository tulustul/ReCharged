// How to Play strings (GDD §5.3–§6.2, §12).

export const howTo = {
  title: "How to Play",
  tabs: { controls: "Controls", charge: "Charge & Boost", items: "Items", recovery: "Recovery" },
  controlsIntro: "Your controls (they follow the device you use last).",
  charge: [
    ["Charge your battery", "The meter fills while you drift through corners above 40 % speed, while you fly through the air, and while you draft behind another car."],
    ["Clean landings", "Land on all four wheels after a big jump for an extra +10 %."],
    ["Boost", "Hold Boost to spend charge: +30 % top speed and +50 % power. You need at least 15 % to start."],
    ["Perfect start", "Press throttle just before GO for a free boost. Too early and your wheels spin."],
  ],
  items: [
    ["Volt Cubes", "Drive through a glowing cube to get a random item. You hold one item at a time."],
    ["Turbo Cell", "An instant 2.5 s boost."],
    ["Spark Rocket", "A homing rocket. Hold Look back to fire it behind you."],
    ["Leech Coil", "Tethers you to the car ahead, stealing charge and towing you."],
    ["Shock Orb", "A slow plasma ball that follows the track and knocks cars aside."],
    ["EMP Pulse", "Cuts the power of nearby cars and drains their charge."],
    ["Goo Bomb", "Sticks to you with a 6 s fuse — bump another car to pass it on."],
    ["Oil Slick & Spring Pad", "Dropped behind you: one slides, the other launches."],
    ["Shield Bubble", "Blocks one hit for 10 s. Raise it when a rocket locks on."],
    ["Storm Strike", "Rare, for the back of the pack: lightning hits every car ahead."],
  ],
  recovery: [
    ["Flip", "Tap Reset to flip your car back onto its wheels."],
    ["Respawn", "Hold Reset to return to the track at the last safe point."],
    ["Automatic help", "Cars right themselves after a second on the roof, and respawn when they fall off the world or leave the track for too long."],
    ["Wrong way?", "A warning shows when you drive against the track direction."],
  ],
} as const;
