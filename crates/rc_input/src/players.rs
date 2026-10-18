//! Device assignment (GDD §12.3): the [`Roster`] behind `LocalPlayers`.
//!
//! - **Single** (default; menus and 1P races): slot 0 owns every device.
//! - **Versus**: one device per slot, from lobby joins (`LobbyState.open`: Confirm on an unassigned
//!   device joins the next free slot, Back on a joined one leaves) or, for a multi-player session
//!   started without a lobby (`--quick-race --players n`), auto-assigned pads first, then keyboard
//!   A and B.
//! - A slot whose pad disconnects is orphaned (`DeviceEvent::Disconnected` in a race, the slot is
//!   freed in the lobby); Confirm/Start on an unassigned device takes it over (`Joined`).

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use rc_core::prelude::*;

/// Assignment mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    /// Slot 0 owns every device.
    #[default]
    Single,
    /// One device per joined slot.
    Versus,
}

/// Who owns which device; written to `LocalPlayers` whenever it changes.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct Roster {
    mode: Mode,
    /// Connected pads in connection order.
    pads: Vec<Entity>,
    /// Versus slots, sorted by slot.
    slots: Vec<SlotBinding>,
    /// Versus slots that lost their pad.
    orphaned: Vec<PlayerSlot>,
    /// Last used device family in single mode.
    single_last_used: DeviceKind,
}

impl Roster {
    /// The `LocalPlayers` view.
    pub fn bindings(&self) -> Vec<SlotBinding> {
        match self.mode {
            Mode::Single => vec![SlotBinding {
                slot: PlayerSlot(0),
                devices: self.all_devices(),
                last_used: self.single_last_used,
            }],
            Mode::Versus => self.slots.clone(),
        }
    }

    /// Keyboard A, keyboard B, then every connected pad.
    pub fn devices(&self) -> impl Iterator<Item = InputDevice> + '_ {
        [InputDevice::KeyboardA, InputDevice::KeyboardB]
            .into_iter()
            .chain(self.pads.iter().map(|&p| InputDevice::Gamepad(p)))
    }

    fn all_devices(&self) -> Vec<InputDevice> {
        self.devices().collect()
    }

    /// The slot owning `device` (`None` = unassigned).
    pub fn slot_of(&self, device: InputDevice) -> Option<PlayerSlot> {
        match self.mode {
            Mode::Single => Some(PlayerSlot(0)),
            Mode::Versus => self
                .slots
                .iter()
                .find(|s| s.devices.contains(&device))
                .map(|s| s.slot),
        }
    }

    /// Whether the lobby/versus assignment is active.
    pub fn is_versus(&self) -> bool {
        self.mode == Mode::Versus
    }

    /// A pad connected.
    pub fn connect(&mut self, pad: Entity) {
        if !self.pads.contains(&pad) {
            self.pads.push(pad);
        }
    }

    /// A pad disconnected: the versus slot it belonged to (now orphaned), or slot 0 in single mode
    /// when the player was using a pad.
    pub fn disconnect(&mut self, pad: Entity) -> Option<PlayerSlot> {
        self.pads.retain(|&p| p != pad);
        let device = InputDevice::Gamepad(pad);
        match self.mode {
            Mode::Single => (self.single_last_used == DeviceKind::Gamepad).then_some(PlayerSlot(0)),
            Mode::Versus => {
                let slot = self
                    .slots
                    .iter_mut()
                    .find(|s| s.devices.contains(&device))?;
                slot.devices.retain(|&d| d != device);
                let id = slot.slot;
                if slot.devices.is_empty() && !self.orphaned.contains(&id) {
                    self.orphaned.push(id);
                }
                Some(id)
            }
        }
    }

    /// Lobby opened: a single-player roster starts empty (everyone joins); a versus roster (back
    /// from a Versus race) keeps its players.
    pub fn open_lobby(&mut self) {
        if self.mode == Mode::Single {
            self.mode = Mode::Versus;
            self.slots.clear();
            self.orphaned.clear();
        }
    }

    /// Lobby closed: nobody joined → back to single player.
    pub fn close_lobby(&mut self) {
        if self.slots.is_empty() {
            self.set_single();
        }
    }

    fn set_single(&mut self) {
        self.mode = Mode::Single;
        self.slots.clear();
        self.orphaned.clear();
    }

    /// `device` joins the lowest free slot below `max_slots`.
    pub fn join(&mut self, device: InputDevice, max_slots: u8) -> Option<PlayerSlot> {
        if self.mode != Mode::Versus || self.slot_of(device).is_some() {
            return None;
        }
        let max = max_slots.min(MAX_LOCAL_PLAYERS as u8);
        let free = (0..max)
            .map(PlayerSlot)
            .find(|s| self.slots.iter().all(|b| b.slot != *s))?;
        self.slots.push(SlotBinding {
            slot: free,
            devices: vec![device],
            last_used: kind_of(device),
        });
        self.slots.sort_by_key(|s| s.slot.0);
        Some(free)
    }

    /// `slot` leaves the lobby.
    pub fn leave(&mut self, slot: PlayerSlot) {
        self.slots.retain(|s| s.slot != slot);
        self.orphaned.retain(|&s| s != slot);
    }

    /// An unassigned `device` takes over the lowest orphaned slot.
    pub fn take_over(&mut self, device: InputDevice) -> Option<PlayerSlot> {
        if self.mode != Mode::Versus || self.slot_of(device).is_some() {
            return None;
        }
        self.orphaned.sort_by_key(|s| s.0);
        let slot = *self.orphaned.first()?;
        self.orphaned.remove(0);
        let binding = self.slots.iter_mut().find(|s| s.slot == slot)?;
        binding.devices.push(device);
        binding.last_used = kind_of(device);
        Some(slot)
    }

    /// Whether some slot waits for a takeover.
    pub fn has_orphans(&self) -> bool {
        !self.orphaned.is_empty()
    }

    /// Session start: make the roster match the session's human slots. One human (or none) →
    /// single; the same slots as now → kept (lobby players); otherwise pads first, then keyboard
    /// A and B, in slot order.
    pub fn reconcile(&mut self, humans: &[PlayerSlot]) {
        if humans.len() <= 1 {
            self.set_single();
            return;
        }
        let same = self.mode == Mode::Versus
            && self.slots.len() == humans.len()
            && humans
                .iter()
                .all(|h| self.slots.iter().any(|s| s.slot == *h));
        if same {
            return;
        }
        let mut free = self
            .pads
            .iter()
            .map(|&p| InputDevice::Gamepad(p))
            .chain([InputDevice::KeyboardA, InputDevice::KeyboardB]);
        let mut sorted = humans.to_vec();
        sorted.sort_by_key(|s| s.0);
        self.mode = Mode::Versus;
        self.orphaned.clear();
        self.slots = sorted
            .into_iter()
            .map(|slot| {
                let devices: Vec<InputDevice> = free.next().into_iter().collect();
                SlotBinding {
                    slot,
                    last_used: devices
                        .first()
                        .map_or(DeviceKind::KeyboardA, |&d| kind_of(d)),
                    devices,
                }
            })
            .collect();
    }

    /// The device family `slot` used last.
    pub fn last_used(&self, slot: PlayerSlot) -> Option<DeviceKind> {
        match self.mode {
            Mode::Single => Some(self.single_last_used),
            Mode::Versus => self
                .slots
                .iter()
                .find(|s| s.slot == slot)
                .map(|s| s.last_used),
        }
    }

    /// Records the device family `slot` used; `true` when it changed.
    pub fn set_last_used(&mut self, slot: PlayerSlot, kind: DeviceKind) -> bool {
        let current = match self.mode {
            Mode::Single => &mut self.single_last_used,
            Mode::Versus => match self.slots.iter_mut().find(|s| s.slot == slot) {
                Some(binding) => &mut binding.last_used,
                None => return false,
            },
        };
        let changed = *current != kind;
        *current = kind;
        changed
    }
}

/// Device family of a device.
pub fn kind_of(device: InputDevice) -> DeviceKind {
    match device {
        InputDevice::KeyboardA => DeviceKind::KeyboardA,
        InputDevice::KeyboardB => DeviceKind::KeyboardB,
        InputDevice::Gamepad(_) => DeviceKind::Gamepad,
    }
}

/// PreUpdate: follows pad connections (`Gamepad` components) and the lobby's open flag. Mutates
/// the roster only on a real change, so `LocalPlayers` readers see real changes.
pub(crate) fn track_devices(
    pads: Query<Entity, With<Gamepad>>,
    lobby: Option<Res<LobbyState>>,
    app_state: Option<Res<State<AppState>>>,
    mut roster: ResMut<Roster>,
    mut lobby_was_open: Local<bool>,
    mut gone: Local<Vec<Entity>>,
    mut events: MessageWriter<DeviceEvent>,
) {
    for pad in &pads {
        if !roster.pads.contains(&pad) {
            info!("gamepad {pad} connected");
            roster.connect(pad);
        }
    }
    gone.clear();
    gone.extend(roster.pads.iter().copied().filter(|&p| !pads.contains(p)));
    let lobby_open = lobby.is_some_and(|l| l.open);
    let in_race = app_state.is_some_and(|s| *s.get() == AppState::InRace);
    for &pad in gone.iter() {
        info!("gamepad {pad} disconnected");
        let Some(slot) = roster.disconnect(pad) else {
            continue;
        };
        if lobby_open {
            roster.leave(slot);
            events.write(DeviceEvent::Left { slot });
        } else if in_race {
            events.write(DeviceEvent::Disconnected { slot });
        }
    }
    if lobby_open != *lobby_was_open {
        *lobby_was_open = lobby_open;
        if lobby_open {
            roster.open_lobby();
        } else {
            roster.close_lobby();
        }
    }
}

/// `OnEnter(AppState::Loading)`: match the roster to the session's human slots.
pub(crate) fn reconcile_session(config: Option<Res<RaceConfig>>, mut roster: ResMut<Roster>) {
    let humans: Vec<PlayerSlot> = config
        .iter()
        .flat_map(|c| &c.racers)
        .filter_map(|r| match r.controller {
            Controller::Human { slot, .. } => Some(slot),
            Controller::Ai { .. } => None,
        })
        .collect();
    roster.reconcile(&humans);
}

/// Mirrors the roster into `LocalPlayers` (only on change, so readers see real changes).
pub(crate) fn publish(roster: Res<Roster>, mut players: ResMut<LocalPlayers>) {
    if roster.is_changed() {
        players.set_if_neq(LocalPlayers {
            slots: roster.bindings(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pads(n: usize) -> (World, Vec<Entity>) {
        let mut world = World::new();
        let pads = (0..n).map(|_| world.spawn_empty().id()).collect();
        (world, pads)
    }

    #[test]
    fn single_player_owns_every_device() {
        let (_w, p) = pads(2);
        let mut r = Roster::default();
        r.connect(p[0]);
        r.connect(p[1]);
        let b = r.bindings();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].slot, PlayerSlot(0));
        assert_eq!(b[0].devices.len(), 4);
        assert_eq!(r.slot_of(InputDevice::Gamepad(p[1])), Some(PlayerSlot(0)));
    }

    #[test]
    fn lobby_join_and_leave_fill_the_lowest_free_slot() {
        let (_w, p) = pads(2);
        let mut r = Roster::default();
        r.open_lobby();
        assert!(r.bindings().is_empty(), "everyone joins");
        assert_eq!(r.join(InputDevice::KeyboardB, 4), Some(PlayerSlot(0)));
        assert_eq!(r.join(InputDevice::KeyboardB, 4), None, "already joined");
        assert_eq!(r.join(InputDevice::Gamepad(p[0]), 4), Some(PlayerSlot(1)));
        assert_eq!(r.join(InputDevice::KeyboardA, 4), Some(PlayerSlot(2)));
        r.leave(PlayerSlot(1));
        assert_eq!(r.slot_of(InputDevice::Gamepad(p[0])), None);
        assert_eq!(
            r.join(InputDevice::Gamepad(p[1]), 4),
            Some(PlayerSlot(1)),
            "reuses the gap"
        );
        assert_eq!(r.join(InputDevice::Gamepad(p[0]), 3), None, "max_slots");
        let slots: Vec<u8> = r.bindings().iter().map(|s| s.slot.0).collect();
        assert_eq!(slots, [0, 1, 2]);
    }

    #[test]
    fn closing_an_empty_lobby_returns_to_single_and_reopening_keeps_versus_players() {
        let mut r = Roster::default();
        r.open_lobby();
        r.close_lobby();
        assert!(!r.is_versus());
        r.open_lobby();
        r.join(InputDevice::KeyboardA, 4);
        r.join(InputDevice::KeyboardB, 4);
        r.close_lobby();
        assert!(r.is_versus(), "players stay for the race");
        r.open_lobby();
        assert_eq!(
            r.bindings().len(),
            2,
            "back in the lobby after a race: still joined"
        );
    }

    #[test]
    fn disconnect_orphans_the_slot_until_a_button_press_takes_over() {
        let (_w, p) = pads(3);
        let mut r = Roster::default();
        for &pad in &p {
            r.connect(pad);
        }
        r.open_lobby();
        r.join(InputDevice::Gamepad(p[0]), 4);
        r.join(InputDevice::Gamepad(p[1]), 4);
        assert_eq!(r.disconnect(p[1]), Some(PlayerSlot(1)));
        assert!(r.has_orphans());
        assert_eq!(
            r.take_over(InputDevice::Gamepad(p[0])),
            None,
            "already a player"
        );
        assert_eq!(r.take_over(InputDevice::KeyboardB), Some(PlayerSlot(1)));
        assert!(!r.has_orphans());
        assert_eq!(r.slot_of(InputDevice::KeyboardB), Some(PlayerSlot(1)));
        assert_eq!(r.disconnect(p[2]), None, "unassigned pad");
    }

    #[test]
    fn single_player_pad_loss_matters_only_when_playing_on_a_pad() {
        let (_w, p) = pads(1);
        let mut r = Roster::default();
        r.connect(p[0]);
        assert_eq!(r.disconnect(p[0]), None, "playing on the keyboard");
        r.connect(p[0]);
        r.set_last_used(PlayerSlot(0), DeviceKind::Gamepad);
        assert_eq!(r.disconnect(p[0]), Some(PlayerSlot(0)));
    }

    #[test]
    fn sessions_without_a_lobby_auto_assign_pads_then_keyboards() {
        let (_w, p) = pads(1);
        let mut r = Roster::default();
        r.connect(p[0]);
        r.reconcile(&[PlayerSlot(0), PlayerSlot(1), PlayerSlot(2)]);
        let b = r.bindings();
        assert_eq!(b[0].devices, [InputDevice::Gamepad(p[0])]);
        assert_eq!(b[1].devices, [InputDevice::KeyboardA]);
        assert_eq!(b[2].devices, [InputDevice::KeyboardB]);
        assert_eq!(b[1].last_used, DeviceKind::KeyboardA);
        r.reconcile(&[PlayerSlot(0)]);
        assert!(!r.is_versus(), "a 1P session is single player");
    }

    #[test]
    fn lobby_players_are_kept_for_their_session() {
        let mut r = Roster::default();
        r.open_lobby();
        r.join(InputDevice::KeyboardB, 4);
        r.join(InputDevice::KeyboardA, 4);
        r.close_lobby();
        let before = r.bindings();
        r.reconcile(&[PlayerSlot(1), PlayerSlot(0)]);
        assert_eq!(r.bindings(), before);
    }

    #[test]
    fn last_used_reports_only_changes() {
        let mut r = Roster::default();
        assert!(!r.set_last_used(PlayerSlot(0), DeviceKind::KeyboardA));
        assert!(r.set_last_used(PlayerSlot(0), DeviceKind::Gamepad));
        assert_eq!(r.bindings()[0].last_used, DeviceKind::Gamepad);
        r.open_lobby();
        assert!(
            !r.set_last_used(PlayerSlot(3), DeviceKind::Gamepad),
            "no such slot"
        );
    }
}
