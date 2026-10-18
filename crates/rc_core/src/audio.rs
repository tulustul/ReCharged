//! Audio cues that only the flow/UI can time.

use bevy::prelude::*;

use crate::catalog::MusicCue;

/// Play a music stinger (e.g. `PodiumFanfare` / `CupVictory` when the Cup-end ceremony opens).
///
/// Producers: rc_ui / rc_game. Consumer: rc_audio. In-race cues (results jingle, final lap) are
/// timed by rc_audio itself from race state and do not use this message.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicStinger(pub MusicCue);
