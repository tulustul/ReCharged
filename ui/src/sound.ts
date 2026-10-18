import { bevy, type UiCueName } from "./bevy";

/** Plays a UI sound cue (rc_audio, UI bus): hover on focus moves, select, back, toggle, sliderTick,
 *  error, unlock. */
export function sfx(cue: UiCueName) {
  bevy.ui.sound({ cue });
}
