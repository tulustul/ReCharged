# T09 — Audio system (`rc_audio`) & sound effects

**Goal:** a punchy, readable soundscape: procedurally synthesised SFX for the whole GDD list and a
kira-based audio system with buses, engines, positional one-shots, ambience and a music director.

**Read first:** PRACTICES (§7 audio); ARCHITECTURE §1–4, §4.5 (messages), §5.11; GDD §16; digests:
`kira-notes.md` (all — note: channel volume overwrites instance volume → use a volume effect per
bus; finished instances must be pruned; spatial audio has one listener → do our own attenuation),
bevy §0.

## Owned paths
`crates/rc_audio/**`, `tools/audio/sfx/**`, `assets/audio/sfx/**`, `assets/audio/ambience/**`,
`docs/design/rc_audio.md`. Music files come from T10 (use temporary silence/placeholders until then;
the music director must work with whatever cue files exist).

## Deliverables
1. **SFX generator** (`tools/audio/sfx/`, Python + numpy, ffmpeg for OGG): a small synthesis kit
   (oscillators, noise colours, filters, envelopes, FM, distortion, simple convolution/feedback
   reverb) and one recipe per sound of GDD §16.2, incl. 3 engine timbres as seamless loops (buggy
   whine, truck growl, sports electric scream — designed to be pitch-shifted by playback rate),
   skid/scrub loops, surface rumble loops, ambience loops (room tone, birds, crickets, wind, rain
   light/heavy + muffled indoor variants, supermarket hum, workshop hum), thunder ×3, UI set.
   Verify each with waveform/spectrogram PNGs, loudness (peak ≤ −1 dBFS), and loop-seam checks.
2. **rc_audio** (bevy_kira_audio): buses Music/Sfx/Engines/Ui/Ambience with volume effects driven
   by `Settings.audio` (master × bus), mute-when-unfocused; registers & completes the `"audio"`
   loading key (preload handles).
   - Engines: one loop per racer (timbre from `CarInfo.engine`), playback rate from `rpm` & load,
     volume from throttle; other racers attenuated + panned by distance to the **nearest**
     `GameCamera` (own engine −6 dB each in split-screen); boost/drift layers.
   - One-shots from messages (`CarImpact`, `CarLanded`, `ItemEvent`, `HitEvent`, `Explosion`,
     `RaceEvent`, `CarRespawned`, `WeatherEvent`) with distance attenuation; projectile/hazard loops
     (`Projectile`, `Hazard` entities); `UiSound` cues.
   - Music director: menu theme in menus, the track's race cue in races with a synchronized
     final-lap variant, stingers on results/podium/cup end, crossfades, pause ducking.
   - Ambience by environment, time of day and `WeatherState` (muffled outside weather indoors).
   - Pause pauses Engines/Sfx/Ambience; no audio device → muted without panics; prune finished
     instances.
3. `docs/design/rc_audio.md` (bus layout, mixing levels, cue table) and a sound list with recipes.

## Tests
Pure: attenuation/pan maths, bus volume composition, engine rate mapping, music state machine.
Headless where possible (kira without a device must not panic).

## Acceptance
All GDD §16.2 sounds exist as OGG with spectrogram previews inspected; tests/clippy/fmt clean; a
game run logs no audio errors; report the cue → file table.
