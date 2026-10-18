# bevy_kira_audio 0.27 digest (with Bevy 0.20)

Versions: **bevy_kira_audio 0.27.1** → kira 0.12.5 (cpal backend), on Bevy 0.20.0.

Tags:
- **[C]** compiled in the scratch crate. Not run: playing audio needs a sound device, so no runtime test.
- **UNVERIFIED** is marked where it applies.

Snippets assume `use bevy::prelude::*; use bevy_kira_audio::prelude::*;`. The prelude exports `Audio`, `AudioChannel`, `AudioControl`, `AudioInstance`, `AudioSource`, `AudioTween`, `AudioEasing`, `PlaybackState`, `Decibels`, `Frame`, the effect builders and handles, `StaticSoundData`, and more.

## 1. Cargo

```toml
# Bevy's `audio` feature (bevy_audio) must be OFF: it is in Bevy's `default`.
bevy = { version = "0.20", default-features = false, features = ["3d", "ui"] }
bevy_kira_audio = { version = "0.27", features = ["wav"] }   # defaults: "ogg" (vorbis) + "spatial"
```
[C]

- `bevy = { features = ["default"] }` or plain `bevy = "0.20"` turns `audio` on. Then `bevy::prelude::*` and `bevy_kira_audio::prelude::*` both export `AudioSource`, and `Handle<AudioSource>` fails with `error[E0659]: AudioSource is ambiguous` [C]. You also get two audio outputs and two `.ogg` loaders.
- Cargo can't remove one feature from `default`, so list Bevy's collections instead, e.g. `["3d", "ui"]` (add `"2d"` if you use sprites) plus extras like `"jpeg"`.
- bevy_kira_audio features:
  - `ogg` (default; kira `ogg` + `vorbis`)
  - `wav` (kira `wav` + `pcm`)
  - `mp3`
  - `flac`
  - `spatial` (default; `SpatialAudioPlugin`)
  - `settings_loader` (RON files that preconfigure loop points etc.)
- It depends on Bevy sub-crates directly (`bevy_app`, `bevy_asset`, `bevy_ecs`, `bevy_transform`, …), not on `bevy`, so it never re-enables Bevy features.

## 2. Plugin, buses (channels + tracks), volumes

**The model.**
- A *channel* is a typed resource `AudioChannel<T>` (`T` is your marker; `#[derive(Resource)]` works as usual in 0.20). `Audio` = `AudioChannel<MainTrack>` is the default channel.
- Channel commands (`play`, `stop`, `set_volume`, …) are queued and executed in `PostUpdate`.
- `add_audio_channel_with_track::<T>(AudioTrack)` routes the channel through its own kira **sub-track** (a mixer bus) with an effect chain. Put a `VolumeControl` effect on it and you have a **real bus gain** that multiplies with per-sound volumes.

```rust
#[derive(Resource)] pub struct MusicBus;
#[derive(Resource)] pub struct SfxBus;
#[derive(Resource)] pub struct EngineBus;
#[derive(Resource)] pub struct UiBus;      // + AmbienceBus the same way

#[derive(Resource)]
pub struct BusVolumes { pub music: VolumeControlHandle, pub sfx: VolumeControlHandle,
                        pub engine: VolumeControlHandle, pub ui: VolumeControlHandle }

impl Plugin for KiraAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AudioPlugin);              // kira's AudioPlugin; must come BEFORE ..._with_track
        let mut music = AudioTrack::new();
        let mut sfx = AudioTrack::new();
        let mut engine = AudioTrack::new();
        let mut ui = AudioTrack::new();
        let volumes = BusVolumes {
            music: music.add_effect(VolumeControlBuilder::new(Decibels(-6.0))),
            sfx: sfx.add_effect(VolumeControlBuilder::new(Decibels::IDENTITY)),    // 0 dB
            engine: engine.add_effect(VolumeControlBuilder::new(Decibels(-3.0))),
            ui: ui.add_effect(VolumeControlBuilder::new(Decibels::IDENTITY)),
        };
        app.add_audio_channel_with_track::<MusicBus>(music)
            .add_audio_channel_with_track::<SfxBus>(sfx)
            .add_audio_channel_with_track::<EngineBus>(engine)
            .add_audio_channel_with_track::<UiBus>(ui)
            .insert_resource(volumes);
    }
}

fn set_bus_volume(keys: Res<ButtonInput<KeyCode>>, mut buses: ResMut<BusVolumes>) {
    if keys.just_pressed(KeyCode::KeyM) {
        buses.music.set_volume(Decibels::SILENCE, AudioTween::linear(Duration::from_millis(300)));  // -60 dB = silent
    }
    if keys.just_pressed(KeyCode::Minus) {
        buses.sfx.set_volume(Decibels(-12.0), AudioTween::default());   // settings slider
    }
}
```
[C]

- **Do not use `AudioChannel::set_volume` as a bus slider** when sounds on that channel have their own volumes (engine by RPM, distance attenuation). It overwrites the volume of **every instance currently playing** in the channel, and it is applied to new sounds before their own `with_volume`. Same for `set_playback_rate` and `set_panning`. Use the track `VolumeControlHandle` above.
- Channel-wide controls that are still useful: `pause()` / `resume()` (whole bus: pause menu), `stop()`, `is_playing_sound()`, `state(&instance_handle)`.
  - Each returns a tween builder: `sfx.pause().fade_out(AudioTween::linear(Duration::from_millis(200)));`. [C]
- `AudioSettings { sound_capacity, sub_track_capacity }` (default 128/128) must be inserted **before** `AudioPlugin` and can't change at runtime. Every sound with its own effects uses one sub-track.
- Runtime string-keyed channels: `ResMut<DynamicAudioChannels>` with `create_channel("p1_engine")`, `channel("p1_engine").play(..)`, `is_channel`, `remove_channel`. [C]

## 3. Playing: one-shots and loops

```rust
fn load_sounds(mut commands: Commands, server: Res<AssetServer>, music: Res<AudioChannel<MusicBus>>) {
    let sounds = Sounds {
        engine_loop: server.load("audio/engine_loop.ogg"),   // Handle<bevy_kira_audio::AudioSource>
        boing: server.load("audio/boing.wav"),               // "wav" feature
        music: server.load("audio/music.ogg"),
    };
    music.play(sounds.music.clone()).looped().fade_in(AudioTween::linear(Duration::from_secs(2)));
    commands.insert_resource(sounds);
}

/// One-shot on the SFX bus.
fn honk(keys: Res<ButtonInput<KeyCode>>, sounds: Res<Sounds>, sfx: Res<AudioChannel<SfxBus>>) {
    if keys.just_pressed(KeyCode::KeyH) {
        sfx.play(sounds.boing.clone())
            .with_volume(Decibels(-3.0))                     // impl Into<Decibels>: -3.0 also works
            .with_playback_rate(1.0 + 0.1 * rand_ish() as f64);   // f64! small random pitch variation
    }
}
```
[C]

- `play(handle)` returns a `PlayAudioCommand` builder. Its methods:
  - looping: `.looped()`, `.loop_from(secs)`, `.loop_until(secs)`
  - level, pitch, position: `.with_volume(dB)`, `.with_playback_rate(f64)`, `.with_panning(-1.0..=1.0)` (0 = centre), `.start_from(secs)`, `.reverse()`
  - fades and state: `.fade_in(AudioTween)` / `.linear_fade_in(d)`, `.paused()`
  - effects and spatial: `.with_effect(FilterBuilder::new().cutoff(4000.0))`, `.add_effect(..) -> EffectHandle`, `.with_emitter(entity)` (spatial)
  - `.handle()` returns the `Handle<AudioInstance>` for later control.
- The command is queued when the builder **drops** (end of the statement) and runs in `PostUpdate`.
  - If the `AudioSource` hasn't loaded yet, the command is retried every frame until it has. No need to wait for the asset.
- `AudioTween::new(duration, AudioEasing::OutPowi(2))`, `AudioTween::linear(d)`, `.with_easing(..)`. The default is 10 ms linear (click-free).
  - `AudioEasing` = `kira::Easing`: `Linear`, `InPowi(i32)`, `OutPowi`, `InOutPowi`, `InPowf(f64)`, ….

## 4. Controlling an instance (engine RPM, fades, stop)

```rust
#[derive(Component)] pub struct EngineSound(pub Handle<AudioInstance>);
#[derive(Component, Default)] pub struct Rpm(pub f32);   // 0..1, written by the car sim

fn start_engine(mut commands: Commands, sounds: Res<Sounds>, engine_bus: Res<AudioChannel<EngineBus>>) {
    let handle = engine_bus.play(sounds.engine_loop.clone()).looped().with_volume(Decibels(-8.0)).handle();
    commands.spawn((EngineSound(handle), Rpm::default()));   // put these on the car entity
}

/// Engine pitch follows RPM every frame.
fn engine_pitch(q: Query<(&EngineSound, &Rpm)>, mut instances: ResMut<Assets<AudioInstance>>) {
    for (sound, rpm) in &q {
        // None until the play command has actually run (first frame / source still loading).
        let Some(mut instance) = instances.get_mut(&sound.0) else { continue };   // AssetMut: needs `mut`
        let rate = 0.6 + 1.4 * rpm.0 as f64;          // playback rate = speed AND pitch
        instance.set_playback_rate(rate, AudioTween::linear(Duration::from_millis(50)));
        instance.set_decibels(-12.0 + 9.0 * rpm.0, AudioTween::default());
        instance.set_panning(0.0, AudioTween::default());
    }
}
```
[C]

- `AudioInstance` methods:
  - `set_playback_rate(f64, tween)`, `set_decibels(impl Into<Decibels>, tween)` (the name is not `set_volume`), `set_panning(f32, tween)`
  - `pause(tween)`, `resume(tween)`, `stop(tween)` (fades out its effects too), `seek_to(secs)`, `seek_by(secs)`
  - `state() -> PlaybackState::{Queued, Playing{position}, Paused{..}, Pausing{..}, Stopping{..}, Stopped, WaitingToResume{..}, Resuming{..}}` (`.position()`)
  - Also `Assets<AudioInstance>::state(&handle)` (`AudioInstanceAssetsExt`).
- Instance calls reach the audio thread immediately. Channel calls are deferred to `PostUpdate`.
- Calling `set_playback_rate` every frame with a 30–50 ms tween is smooth.
  - Rates far from 1.0 sound "chipmunky". For realism, layer 2–3 loops recorded at different RPMs (idle/mid/high) on the engine bus. Crossfade their `set_decibels` by RPM and pitch each within ±30%.
- **Instances are never removed from `Assets<AudioInstance>` by the plugin**, even when stopped. Prune them periodically if you fire many one-shots [C]:
  ```rust
  fn prune_stopped(mut instances: ResMut<Assets<AudioInstance>>) {
      let stopped: Vec<_> = instances.iter()
          .filter(|(_, i)| matches!(i.state(), PlaybackState::Stopped)).map(|(id, _)| id).collect();
      for id in stopped { instances.remove(id); }
  }
  // .add_systems(Update, prune_stopped.run_if(bevy::time::common_conditions::on_timer(Duration::from_secs(5))))
  ```

## 5. Spatial and distance attenuation

**The built-in plugin** (feature `spatial`). It is not added by `AudioPlugin`: `app.add_plugins(SpatialAudioPlugin)`.
- Put `SpatialAudioReceiver` on the listener and `SpatialAudioEmitter { instances: vec![handle] }` on emitters (plus `SpatialRadius { radius }`, or the `DefaultSpatialRadius` resource, default 25 m). Or use `play(..).with_emitter(entity)`.
- Volume falls linearly from 0 dB to −60 dB over the radius. Panning is the cosine of the angle to the receiver's right vector.
- **Exactly one `SpatialAudioReceiver`** is supported (`receiver.single()`).
- It **overwrites** the volume and panning of the emitter's instances every frame, so it fights with an RPM-based `set_decibels`.

**For split screen:** skip `SpatialAudioPlugin` and attenuate by distance to the **nearest** player camera yourself. Keep panning centred, since one stereo output can't serve 2–4 viewpoints [C]:
```rust
#[derive(Component)] pub struct Listener;    // on each player camera
#[derive(Component)] pub struct Emitter { pub instance: Handle<AudioInstance>, pub base_db: f32, pub radius: f32 }

pub fn split_screen_attenuation(listeners: Query<&GlobalTransform, With<Listener>>,
                                emitters: Query<(&GlobalTransform, &Emitter)>,
                                mut instances: ResMut<Assets<AudioInstance>>) {
    for (t, e) in &emitters {
        let d = listeners.iter().map(|l| l.translation().distance(t.translation())).fold(f32::INFINITY, f32::min);
        let k = (1.0 - d / e.radius).clamp(0.0, 1.0);
        let db = if k <= 0.0 { -60.0 } else { e.base_db + 20.0 * k.log10() };
        if let Some(mut inst) = instances.get_mut(&e.instance) { inst.set_decibels(db, AudioTween::default()); }
    }
}
```
- Combine it with the RPM volume by adding the dB values in one system rather than writing twice.
- In single-player you can pan by `camera.right().dot(dir)`.
- kira 0.12 has its own listener/spatial tracks, but bevy_kira_audio doesn't expose them. UNVERIFIED beyond that.

## 6. Effects (per sound or per bus)

```rust
#[derive(Resource)] pub struct TunnelSfx;
#[derive(Resource)] pub struct Muffle(pub FilterHandle);

pub fn effects_plugin(app: &mut App) {          // after AudioPlugin
    let mut track = AudioTrack::new();
    let _reverb: ReverbHandle = track.add_effect(ReverbBuilder::new());     // a reverb bus for tunnels
    app.add_audio_channel_with_track::<TunnelSfx>(track);
}
fn per_sound_effects(mut commands: Commands, audio: Res<Audio>, sounds: Res<Sounds>) {
    audio.play(sounds.music.clone())
        .with_effect(FilterBuilder::new().mode(FilterMode::LowPass).cutoff(300.0))
        .looped();
    let mut cmd = audio.play(sounds.engine_loop.clone());
    let filter: FilterHandle = cmd.add_effect(FilterBuilder::new());   // keep the handle to automate it
    cmd.looped();
    drop(cmd);                                                         // queued here
    commands.insert_resource(Muffle(filter));
}
fn muffle_on_pause(mut muffle: ResMut<Muffle>) {
    muffle.0.set_cutoff(800.0, AudioTween::linear(Duration::from_millis(200)));
}
```
[C]

- Built-in effect builders and their handles:
  - `FilterBuilder` / `FilterHandle` (`set_cutoff`, `set_resonance`, `set_mix`, `set_mode`)
  - `ReverbBuilder` / `ReverbHandle` (`set_feedback`, `set_damping`, `set_stereo_width`, `set_mix`)
  - `DelayBuilder`, `DistortionBuilder` (damaged engine), `CompressorBuilder` (master glue), `EqFilterBuilder`
  - `VolumeControlBuilder`, `PanningControlBuilder`
- Per-sound effects run first, then the channel track's effects.
- A sound's reverb/delay tail rings out after the sound ends (`EffectTail`). Stopping the sound fades the tail too.
- Pause-menu muffle: a `FilterHandle` low-pass on the SFX and engine bus tracks, tweened down on pause.
- Custom DSP: implement `Effect { fn process(&mut self, input: &mut [Frame], dt: f64, info: &Info) }` plus `AudioEffect` for a builder.

## 7. Custom / procedural sources

```rust
/// Procedural one-shot: build StaticSoundData from samples, add as an asset.
fn synth_beep(mut sources: ResMut<Assets<AudioSource>>) -> Handle<AudioSource> {
    use bevy_kira_audio::prelude::{Frame, StaticSoundData, StaticSoundSettings};
    let sample_rate = 48_000u32;
    let frames: Vec<Frame> = (0..sample_rate / 4)
        .map(|i| {
            let s = (i as f32 / sample_rate as f32 * 880.0 * std::f32::consts::TAU).sin() * 0.3;
            Frame { left: s, right: s }
        })
        .collect();
    let sound = StaticSoundData { sample_rate, frames: frames.into(), settings: StaticSoundSettings::default(), slice: None };
    sources.add(AudioSource { sound })       // then channel.play(handle) like any file
}
```
[C]

- bevy_kira_audio plays only **`StaticSoundData`**: fully decoded, in memory. There is no streaming and no infinite generator API. Use pre-rendered buffers (loop them with `.looped()`), or synthesise inside a custom `Effect` on a looped silent source (UNVERIFIED).
- Long music loads fully into memory. A few minutes of ogg decodes to tens of MB of PCM; acceptable.

## 8. Formats, loading, headless

| Feature | Loader | Extension |
|---|---|---|
| `ogg` (default) | vorbis | `.ogg`, `.oga`, `.spx` |
| `wav` | pcm | `.wav` |
| `mp3` | | `.mp3` |
| `flac` | | `.flac` |
| `settings_loader` | RON settings files | see the crate's `settings_loader` example |

- Assets load through Bevy's `AssetServer` (`Handle<bevy_kira_audio::AudioSource>`). Hot reload follows Bevy's `file_watcher` feature.
- No audio device (CI or headless): kira logs `Failed to setup audio` and every play command just stays queued. Simplest is not to add `KiraAudioPlugin` in headless sim tests.
- `Decibels` is a newtype: `Decibels(-6.0)`, `Decibels::IDENTITY` (0 dB), `Decibels::SILENCE` (−60 dB). Playback rate is `f64`, panning `f32`.

## 9. Recommended setup for ReCharged

- **Buses:** `MusicBus`, `SfxBus` (impacts, pickups, weapons), `EngineBus` (all engines), `UiBus`, `AmbienceBus`. Each gets its own `AudioTrack` with a `VolumeControlBuilder`, and the handles live in a `BusVolumes` resource driven by the settings menu.
  - Master volume: an extra `VolumeControl` on each bus, or multiply in your settings code. The main track volume isn't exposed by bevy_kira_audio. UNVERIFIED.
- **Engines:** one looped instance (or 2–3 layered) per car on `EngineBus`. Each frame, set `playback_rate` from RPM and `set_decibels` = throttle dB + nearest-listener attenuation (§5).
- **One-shots:** `SfxBus.play(..)` with a small random `with_playback_rate` (0.95–1.05) and a voice limit (cap concurrent instances per sound yourself). Prune stopped instances.
- **Pause:** `engine_bus.pause()` + `sfx.pause()` with a 100 ms fade, a low-pass on the music bus, and `Time<Virtual>::pause()` for the game.
