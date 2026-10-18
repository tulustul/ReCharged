// Race-session strings: loading, pause, results, cup standings, cup end, Versus scoreboard.

export const loading = {
  title: "Loading",
  tip: "Tip",
  tips: [
    "Drifting charges your battery. Spend the charge on a straight for a boost.",
    "Hold Reset to put the car back on the track; tap it to flip over.",
    "Press throttle just before GO for a perfect start — too early and the wheels spin.",
    "Look back to fire rockets behind you.",
    "A shield blocks one hit. Raise it when a rocket locks on.",
    "Every forward layout hides one Golden Battery off the racing line.",
    "Landing flat after a jump keeps your speed.",
    "Heavy cars push light ones around, but lose more speed on landings.",
    "Pass the Goo Bomb on by touching another car before it blows.",
    "Wet and snowy outdoor tracks have less grip — brake earlier.",
  ],
} as const;

export const pause = {
  title: "Paused",
  resume: "Resume",
  restart: "Restart",
  howToPlay: "How to Play",
  settings: "Settings",
  quit: "Quit race",
  quitTitle: "Quit the race?",
  quitText: "This race will not count.",
  quitCupTitle: "Quit the cup?",
  quitCupText: "Progress in this cup will be lost.",
} as const;

export const race = {
  modeName: {
    quickRace: "Quick Race",
    timeTrial: "Time Trial",
    versus: "Versus",
    championship: "Championship",
  },
  raceOf: (n: number, total: number) => `Race ${n} of ${total}`,
} as const;

export const results = {
  title: "Results",
  col: { pos: "Pos", name: "Racer", car: "Car", time: "Time", best: "Best lap", points: "Pts" },
  projected: "est.",
  actions: {
    continue: "Continue",
    restart: "Restart",
    retry: "Retry",
    changeTrack: "Change track",
    menu: "Menu",
    rematch: "Rematch",
    nextTrack: "Next track",
    lobby: "Lobby",
  },
  waiting: "Waiting for results…",
  finished: "You finished",
  newBestLap: "BEST LAP!",
  newRecord: "NEW RECORD!",
  ghostSaved: "GHOST SAVED",
} as const;

export const standings = {
  title: "Cup Standings",
  races: "Races",
  table: "Standings",
  nextRace: "Next race",
  quitCup: "Quit cup",
  toCeremony: "Continue",
  quitTitle: "Quit the cup?",
  quitText: "Progress in this cup will be lost.",
  gained: (n: number) => `+${n}`,
  points: "pts",
} as const;

export const cupEnd = {
  title: "Cup Complete",
  trophy: (name: string) => `${name} Trophy`,
  noTrophy: "Off the podium — try again for a trophy!",
  continue: "Continue",
  points: (n: number) => `${n} pts`,
} as const;

export const versus = {
  title: "Scoreboard",
  subtitle: (n: number) => (n === 1 ? "After 1 race" : `After ${n} races`),
  points: "Points",
  wins: "Wins",
} as const;
