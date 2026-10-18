//! Shared constants (ARCHITECTURE §4.7, GDD §4.2). Units: world units (1 u = 22.5 cm), seconds.

/// Gravity in u/s² (≈ 0.57 g at world scale; GDD §4.2 feel value, tune within 25–40).
pub const GRAVITY: f32 = 25.0;

/// Simulation (fixed update) rate in Hz.
pub const SIM_HZ: f64 = 60.0;

/// Duration of one simulation tick in seconds (`1 / SIM_HZ`).
pub const SIM_DT: f32 = 1.0 / SIM_HZ as f32;

/// Displayed toy-scale speed: km/h per u/s (deliberately not true scale).
pub const SPEED_DISPLAY_KMH: f32 = 1.8;

/// Miles per hour per km/h, for the `mph` speed unit setting.
pub const MPH_PER_KMH: f32 = 0.621;

/// Maximum racers on a grid.
pub const MAX_RACERS: usize = 8;

/// Maximum local (split-screen) players.
pub const MAX_LOCAL_PLAYERS: usize = 4;

/// Effective top speed never exceeds this multiple of a car's base top speed (GDD §5.1).
pub const TOP_SPEED_CAP: f32 = 1.5;

/// Player slot colours P1–P4 (sRGB 0–1): cyan, yellow, magenta, green (GDD §14).
pub const SLOT_COLORS: [[f32; 3]; MAX_LOCAL_PLAYERS] = [
    [0.0, 0.85, 1.0],
    [1.0, 0.86, 0.1],
    [1.0, 0.2, 0.8],
    [0.25, 0.9, 0.3],
];

/// Converts simulation ticks to seconds.
pub fn ticks_to_secs(ticks: u32) -> f32 {
    ticks as f32 * SIM_DT
}

/// Converts seconds to whole simulation ticks (rounded to nearest).
pub fn secs_to_ticks(secs: f32) -> u32 {
    (secs.max(0.0) * SIM_HZ as f32).round() as u32
}

/// Collision layers as `u32` bit masks (map them to rapier `Group`s via `Group::from_bits_truncate`).
pub mod layers {
    /// Car chassis.
    pub const CAR: u32 = 1 << 0;
    /// Track ribbons, walls, ground areas, room shell.
    pub const TRACK: u32 = 1 << 1;
    /// Static and dynamic props.
    pub const PROP: u32 = 1 << 2;
    /// Item projectiles.
    pub const PROJECTILE: u32 = 1 << 3;
    /// Pickup sensors.
    pub const PICKUP: u32 = 1 << 4;
    /// Ghost car (Time Trial) and ghosted cars after a respawn: no car–car contacts.
    pub const GHOST_CAR: u32 = 1 << 5;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_conversions_round_trip() {
        assert_eq!(secs_to_ticks(1.0), 60);
        assert_eq!(secs_to_ticks(-3.0), 0);
        assert!((ticks_to_secs(90) - 1.5).abs() < 1e-6);
        assert_eq!(secs_to_ticks(ticks_to_secs(12_345)), 12_345);
    }

    #[test]
    fn collision_layers_are_distinct_bits() {
        let all = [
            layers::CAR,
            layers::TRACK,
            layers::PROP,
            layers::PROJECTILE,
            layers::PICKUP,
            layers::GHOST_CAR,
        ];
        let mut seen = 0u32;
        for bit in all {
            assert_eq!(bit.count_ones(), 1);
            assert_eq!(seen & bit, 0);
            seen |= bit;
        }
    }
}
