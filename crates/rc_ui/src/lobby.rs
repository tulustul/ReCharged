//! `lobby.*` channels: the Versus lobby (open/close, joined slots).

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::GameCommand;

use crate::input::UiDeviceKind;

/// A joined slot.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiLobbySlot {
    /// Slot 0–3 (P1–P4).
    pub slot: u8,
    /// Device family it uses (join glyph).
    pub device: UiDeviceKind,
}

/// Bevy → React event `lobby.slots` (sent on change) and the `lobby.snapshot` response.
#[react_event(name = "lobby.slots")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLobby {
    /// Devices may join and leave.
    pub open: bool,
    /// Joinable slots.
    pub max_slots: u8,
    /// Joined slots in slot order.
    pub slots: Vec<UiLobbySlot>,
}

impl UiLobby {
    fn of(lobby: Option<&LobbyState>, players: Option<&LocalPlayers>) -> Self {
        let lobby = lobby.copied().unwrap_or_default();
        Self {
            open: lobby.open,
            max_slots: lobby.max_slots,
            slots: players
                .map(|p| {
                    p.slots
                        .iter()
                        .map(|s| UiLobbySlot {
                            slot: s.slot.0,
                            device: s.last_used.into(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// React → Bevy request `lobby.snapshot`: the current [`UiLobby`].
#[react_request(name = "lobby.snapshot", response = UiLobby)]
pub struct LobbySnapshot;

/// React → Bevy message `lobby.open`: the Versus lobby screen opened (devices may join).
#[react_message(name = "lobby.open")]
pub struct OpenLobby;

/// React → Bevy message `lobby.close`: the Versus lobby screen closed.
#[react_message(name = "lobby.close")]
pub struct CloseLobby;

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<UiLobby>()
        .add_react_request_handler(
            |request: On<Request<LobbySnapshot>>,
             lobby: Option<Res<LobbyState>>,
             players: Option<Res<LocalPlayers>>| {
                request.respond(UiLobby::of(lobby.as_deref(), players.as_deref()));
            },
        )
        .add_react_handler(|_: On<OpenLobby>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::OpenLobby);
        })
        .add_react_handler(|_: On<CloseLobby>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::CloseLobby);
        });
}

/// Sends `lobby.slots` when the lobby or the joined players change.
pub(crate) fn push_slots(
    lobby: Option<Res<LobbyState>>,
    players: Option<Res<LocalPlayers>>,
    events: ReactEvents,
) {
    let changed = lobby.as_ref().is_some_and(|l| l.is_changed())
        || players.as_ref().is_some_and(|p| p.is_changed());
    if changed {
        events.send(&UiLobby::of(lobby.as_deref(), players.as_deref()));
    }
}
