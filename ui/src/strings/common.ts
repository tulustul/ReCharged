// Shared words: hint labels, option values and content names used across screens.
import type {
  UiCarClass,
  UiDifficulty,
  UiDrive,
  UiMedal,
  UiPaintSet,
  UiTimeOfDay,
  UiTrophy,
  UiWeatherKind,
} from "../bevy";

export const hint = {
  select: "Select",
  confirm: "Confirm",
  back: "Back",
  change: "Change",
  tabs: "Switch tab",
  start: "Start race",
  next: "Next",
  ready: "Ready",
  scroll: "Scroll",
  rebind: "Rebind",
  close: "Close",
  continue: "Continue",
} as const;

export const word = {
  on: "On",
  off: "Off",
  locked: "Locked",
  new: "NEW",
  laps: "Laps",
  lap: "Lap",
  none: "—",
  forward: "Forward",
  reverse: "Reverse",
  mirror: "Mirror",
  random: "Random",
  dynamic: "Dynamic",
  races: "races",
  player: (slot: number) => `P${slot + 1}`,
} as const;

export const difficultyName: Record<UiDifficulty, string> = {
  rookie: "Rookie",
  pro: "Pro",
  elite: "Elite",
  elitePlus: "Elite+",
};

export const className: Record<UiCarClass, string> = {
  rookie: "Rookie",
  pro: "Pro",
  elite: "Elite",
  secret: "Secret",
};

export const driveName: Record<UiDrive, string> = { rwd: "RWD", awd: "4WD" };

export const weatherName: Record<UiWeatherKind, string> = {
  clear: "Clear",
  cloudy: "Cloudy",
  rain: "Rain",
  storm: "Storm",
  fog: "Fog",
  snow: "Snow",
};

export const timeName: Record<UiTimeOfDay, string> = {
  morning: "Morning",
  noon: "Noon",
  sunset: "Sunset",
  night: "Night",
};

export const paintSetName: Record<UiPaintSet, string> = {
  base: "Base",
  neon: "Neon",
  carbon: "Carbon",
  gold: "Gold",
  chrome: "Chrome",
};

export const medalName: Record<UiMedal, string> = {
  bronze: "Bronze",
  silver: "Silver",
  gold: "Gold",
  platinum: "Platinum",
};

export const trophyName: Record<UiTrophy, string> = { bronze: "Bronze", silver: "Silver", gold: "Gold" };
