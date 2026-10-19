// Settings screen strings (GDD §15).
import type {
  UiAntiAliasing,
  UiCameraMode,
  UiDisplayMode,
  UiDriveAction,
  UiParticles,
  UiQuality,
  UiScheme,
  UiShadows,
  UiSpeedUnit,
  UiSplit,
} from "../bevy";

export const settingsText = {
  title: "Settings",
  tabs: {
    gameplay: "Gameplay",
    graphics: "Graphics",
    audio: "Audio",
    controls: "Controls",
    accessibility: "Accessibility",
  },
  rows: {
    speedUnit: ["Speed units", "How the HUD shows your speed."],
    cameraMode: ["Camera", "The camera a race starts with (cycle it while driving)."],
    cameraShake: ["Camera shake", "Shake on impacts and landings."],
    hudScale: ["HUD scale", "Size of the race HUD."],
    minimap: ["Minimap", "Show the track map in the HUD."],
    split: ["2-player split", "Top/bottom or side-by-side screens for two players."],
    aiCatchUp: ["AI catch-up", "Rivals behind speed up a little; rivals far ahead slow down."],
    aiDifficulty: ["AI difficulty", "Default difficulty for Quick Race and Versus."],
    hints: ["Hints", "Show onboarding hints during races."],
    resetProgress: ["Reset progress", "Erase unlocks, trophies, medals, records and ghosts."],
    preset: ["Quality", "Sets all graphics options at once."],
    displayMode: ["Display mode", "Windowed or borderless full screen."],
    windowSize: ["Window size", "Size of the window in windowed mode."],
    vsync: ["V-sync", "Sync frames to the display to avoid tearing."],
    fov: ["Field of view", "Horizontal field of view at 16:9."],
    shadows: ["Shadows", "Shadow quality."],
    antiAliasing: ["Anti-aliasing", "Smooths jagged edges (TAA becomes SMAA in split-screen)."],
    ambientOcclusion: ["Ambient occlusion", "Soft contact shadows."],
    bloom: ["Bloom", "Glow around bright lights."],
    motionBlur: ["Motion blur", "Blur at speed."],
    particles: ["Particles", "Amount of sparks, dust and spray."],
    volumetricFog: ["Volumetric fog", "Ultra quality only; never in split-screen."],
    master: ["Master", "Overall volume."],
    music: ["Music", ""],
    sfx: ["Sound effects", ""],
    engines: ["Engines", ""],
    ui: ["Interface", "Menu sounds."],
    ambience: ["Ambience", "Rooms, gardens, weather."],
    muteUnfocused: ["Mute when unfocused", "Silence the game when its window is in the background."],
    scheme: ["Scheme", "Bindings of this device."],
    resetBindings: ["Reset bindings", "Restore the default bindings of this scheme."],
    deadZone: ["Steering dead zone", "Stick movement ignored around the centre."],
    sensitivity: ["Steering sensitivity", "How quickly steering reaches full lock."],
    vibration: ["Vibration", "Rumble on impacts, hits, landings and boost."],
    vibrationIntensity: ["Vibration intensity", ""],
    reduceFlashing: ["Reduce flashing", "Soft glows instead of lightning, explosions and flashes."],
    colorblindMarkers: ["Colour-blind markers", "Racer markers with shapes as well as colours."],
    steeringAssist: ["Steering assist", "Gently steers toward the racing line and back on track."],
    autoAccelerate: ["Auto-accelerate", "The car accelerates by itself."],
  },
  reset: "Reset",
  resetTitle: "Reset all progress?",
  resetText: "Unlocks, trophies, medals, records and ghosts will be erased.",
  resetAgainTitle: "Are you absolutely sure?",
  resetAgainText: "This cannot be undone.",
  resetConfirm: "Erase everything",
  capture: "Press a key or button…",
  captureHint: "Esc or Start cancels",
  swapped: (action: string) => `Swapped with ${action}`,
  nowUnbound: (action: string) => `${action} is now unbound`,
  removedFrom: (action: string) => `Removed from ${action}`,
  cancelled: "Rebind cancelled",
  unbound: "—",
} as const;

export const speedUnitName: Record<UiSpeedUnit, string> = { kmh: "km/h", mph: "mph" };
export const cameraModeName: Record<UiCameraMode, string> = { chase: "Chase", farChase: "Far chase", hood: "Hood" };
export const splitName: Record<UiSplit, string> = { horizontal: "Top / bottom", vertical: "Side by side" };
export const qualityName: Record<UiQuality, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  ultra: "Ultra",
  custom: "Custom",
};
export const displayModeName: Record<UiDisplayMode, string> = { windowed: "Windowed", borderless: "Borderless" };
export const shadowsName: Record<UiShadows, string> = { off: "Off", low: "Low", high: "High" };
export const aaName: Record<UiAntiAliasing, string> = { off: "Off", fxaa: "FXAA", smaa: "SMAA", taa: "TAA" };
export const particlesName: Record<UiParticles, string> = { low: "Low", medium: "Medium", high: "High" };
export const schemeName: Record<UiScheme, string> = {
  keyboardA: "Keyboard A",
  keyboardB: "Keyboard B",
  gamepad: "Gamepad",
};
export const actionName: Record<UiDriveAction, string> = {
  throttle: "Throttle",
  brake: "Brake / reverse",
  steerLeft: "Steer left",
  steerRight: "Steer right",
  drift: "Drift",
  useItem: "Use item",
  boost: "Boost",
  lookBack: "Look back",
  reset: "Reset car",
  camera: "Camera",
  pause: "Pause",
};
