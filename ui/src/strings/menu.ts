// Front-end menu strings: main menu, cup/track/car select, lobby, garage, records.

export const mainMenu = {
  entries: {
    championship: { label: "Championship", text: "Four cups against seven rivals. Podium finishes unlock cars, tracks and cups." },
    quickRace: { label: "Quick Race", text: "Any track, any car, your rules: laps, rivals, weather and items." },
    timeTrial: { label: "Time Trial", text: "Three laps, no items, no mercy. Chase your ghost and the medal times." },
    versus: { label: "Versus", text: "Two to four players on one screen. Grab a controller and join in." },
    garage: { label: "Garage", text: "Every car in the collection, with its stats and paints." },
    records: { label: "Records", text: "Best laps, medals, trophies, Golden Batteries and career stats." },
    howToPlay: { label: "How to Play", text: "Controls, charge and boost, items, and getting back on track." },
    settings: { label: "Settings", text: "Gameplay, graphics, audio, controls and accessibility." },
    credits: { label: "Credits", text: "The people and tools behind ReCharged." },
    quit: { label: "Quit", text: "Leave the game." },
  },
  progress: "Progress",
  trophies: "Trophies",
  cars: "Cars",
  batteries: "Golden Batteries",
  medals: "Gold medals",
} as const;

export const cupSelect = {
  title: "Championship",
  subtitle: "Choose a cup",
  races: (n: number) => `${n} races`,
  bestTrophy: "Best",
  noTrophy: "No trophy yet",
  difficulty: "Difficulty",
} as const;

export const trackSelect = {
  title: { quickRace: "Quick Race", timeTrial: "Time Trial", versus: "Versus" },
  subtitle: "Choose a track",
  layout: "Layout",
  mirror: "Mirror",
  laps: "Laps",
  ai: "Rivals",
  difficulty: "AI difficulty",
  items: "Items",
  weather: "Weather",
  time: "Time of day",
  next: "Choose car",
  start: "Start race",
  bestLap: "Best lap",
  bestRace: "Best race",
  medal: "Medal",
  noRecord: "No record yet",
  ttRules: "3 laps · no items · ghost",
  indoorWeather: "Weather is outside only",
  noAi: "None",
} as const;

export const carSelect = {
  title: "Choose your car",
  stats: { speed: "Speed", accel: "Acceleration", handling: "Handling", weight: "Weight" },
  paint: "Paint",
  drive: "Drive",
  class: "Class",
  start: "Start race",
  confirm: "Confirm car",
  paintLocked: (set: string) => `${set} paints are locked`,
} as const;

export const lobby = {
  title: "Versus",
  subtitle: "Join in: everyone picks a car",
  join: "Press to join",
  ready: "READY",
  notReady: "Choose a car",
  waiting: "Waiting for players…",
  needTwo: "At least two players are needed",
  allReady: "All set! P1 picks the track",
  keyboardWarning: "Two players on one keyboard? Some keyboards can't register many keys at once.",
  leave: "Leave",
  schemeA: "WASD · Space · L-Ctrl",
  schemeB: "Arrows · Enter · Backspace",
} as const;

export const garage = {
  title: "Garage",
  subtitle: (unlocked: number, total: number) => `${unlocked} of ${total} cars`,
  paints: "Paints",
} as const;

export const records = {
  title: "Records",
  tabs: { tracks: "Tracks", trophies: "Trophies", batteries: "Golden Batteries", stats: "Stats" },
  layout: "Layout",
  bestLap: "Best lap",
  bestRace: "Best race",
  medal: "Medal",
  empty: "No records yet — finish a race to set one.",
  found: "Found",
  missing: "Not found",
  batteriesHint: "One per track, hidden off the main line of the forward layout.",
  stats: {
    races: "Races",
    wins: "Wins",
    podiums: "Podiums",
    distance: "Distance driven",
    itemsUsed: "Items used",
    hitsLanded: "Hits landed",
    timePlayed: "Time played",
  },
} as const;

/** Lock reasons; `cup` is the cup's display name. */
export const lockReason = {
  start: "Available",
  cupPodium: (cup: string) => `Finish on the podium in the ${cup}`,
  cupGold: (cup: string) => `Win the ${cup}`,
  goldMedals: (n: number) => `Earn ${n} Time Trial gold medals`,
  goldenBatteries: (n: number) => `Find all ${n} Golden Batteries`,
} as const;
