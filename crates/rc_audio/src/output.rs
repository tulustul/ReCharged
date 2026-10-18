//! Is there an audio device? Decided once by a probe; everything else runs only when it is.
//!
//! Without a device bevy_kira_audio never drains channel commands (they would pile up forever),
//! so every system that queues a command runs `.run_if(audio_on)`.

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

use crate::Step;
use crate::buses::UiBus;

/// Frames to wait for the probe sound to appear before declaring "no device".
const PROBE_FRAMES: u32 = 5;
/// The silent probe sound: 10 ms at 48 kHz.
const PROBE_LEN_FRAMES: usize = 480;

/// Whether sound can play. Set by the startup probe; insert `Unavailable` before the plugin to
/// force muted operation (headless tests).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputState {
    /// The probe is still running.
    #[default]
    Probing,
    /// An audio device plays our sounds.
    Available,
    /// No audio device: the game runs muted.
    Unavailable,
}

#[derive(Resource)]
struct Probe {
    instance: Handle<AudioInstance>,
    frames: u32,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<OutputState>()
        .add_systems(Startup, start_probe)
        .add_systems(
            Update,
            finish_probe
                .in_set(Step::Probe)
                .run_if(resource_exists::<Probe>),
        );
}

/// Run condition: an audio device is available.
pub(crate) fn audio_on(state: Res<OutputState>) -> bool {
    *state == OutputState::Available
}

fn start_probe(
    mut commands: Commands,
    state: Res<OutputState>,
    mut sources: ResMut<Assets<AudioSource>>,
    ui: Res<AudioChannel<UiBus>>,
) {
    if *state != OutputState::Probing {
        return;
    }
    let sound = StaticSoundData {
        sample_rate: 48_000,
        frames: vec![Frame::ZERO; PROBE_LEN_FRAMES].into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    };
    let source = sources.add(AudioSource { sound });
    let instance = ui.play(source).with_volume(Decibels::SILENCE).handle();
    commands.insert_resource(Probe {
        instance,
        frames: 0,
    });
}

/// The play command runs in `PostUpdate`; with a device the instance exists one frame later.
fn finish_probe(
    mut commands: Commands,
    mut probe: ResMut<Probe>,
    instances: Res<Assets<AudioInstance>>,
    mut state: ResMut<OutputState>,
) {
    if instances.get(&probe.instance).is_some() {
        *state = OutputState::Available;
        info!("audio output ready");
    } else if probe.frames < PROBE_FRAMES {
        probe.frames += 1;
        return;
    } else {
        *state = OutputState::Unavailable;
        warn!("no audio device: the game runs muted");
    }
    commands.remove_resource::<Probe>();
}
