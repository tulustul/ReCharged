//! Stateless keyed RNG (ARCHITECTURE §3.4).
//!
//! Every random number is a pure function of `(seed, tick, stream, key)`, so simulation results
//! depend only on `RaceConfig::seed` and the input stream — never on system order or on how many
//! numbers another system drew. `key` is usually the racer index; when one system needs several
//! numbers in the same tick, it varies the key (e.g. `racer_index * 16 + n`).
//!
//! Changing the algorithm or a stream's discriminant changes every seeded race (ghosts, tests):
//! `draw_is_pinned` guards against doing so by accident.

/// One stream per purpose, so adding a draw for one feature never shifts another feature's numbers.
/// Add a variant for a new purpose; never renumber existing ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u64)]
pub enum RngStream {
    /// Item roulette result (rc_items).
    ItemRoll = 1,
    /// AI driving mistakes: late braking, wide lines (rc_ai).
    AiMistakes = 2,
    /// AI item reaction delays (rc_ai).
    AiReaction = 3,
    /// AI perfect-start chance (rc_ai).
    AiPerfectStart = 4,
    /// AI personal racing-line offset (rc_ai).
    AiLineOffset = 5,
    /// Random / Dynamic weather plan (rc_weather).
    WeatherPlan = 6,
    /// Lightning timing and intensity (rc_weather).
    Lightning = 7,
    /// Wind gusts (rc_weather).
    WindGust = 8,
    /// Small spawn/respawn jitter (rc_race).
    SpawnJitter = 9,
    /// Spring Pad launch spin (rc_items).
    SpringPadSpin = 10,
    /// Race setup choices: rival picks, grid order, quick-race defaults (rc_game).
    RaceSetup = 11,
}

/// SplitMix64 finaliser: a bijective 64-bit mixer with strong avalanche.
const fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The keyed draw: 64 uniformly distributed bits for `(seed, tick, stream, key)`.
pub fn draw(seed: u64, tick: u64, stream: RngStream, key: u64) -> u64 {
    let h = splitmix64(seed);
    let h = splitmix64(h ^ tick);
    let h = splitmix64(h ^ stream as u64);
    splitmix64(h ^ key)
}

/// Uniform `f32` in `[0, 1)`.
pub fn unit(seed: u64, tick: u64, stream: RngStream, key: u64) -> f32 {
    // 24 high bits fill an f32 mantissa exactly.
    (draw(seed, tick, stream, key) >> 40) as f32 / (1u64 << 24) as f32
}

/// Uniform `f32` in `[min, max)`.
pub fn range(seed: u64, tick: u64, stream: RngStream, key: u64, min: f32, max: f32) -> f32 {
    min + (max - min) * unit(seed, tick, stream, key)
}

/// Uniform index in `[0, n)`; returns 0 when `n == 0`.
pub fn index(seed: u64, tick: u64, stream: RngStream, key: u64, n: usize) -> usize {
    // Multiply-high maps 64 random bits onto [0, n) with negligible bias for small n.
    ((draw(seed, tick, stream, key) as u128 * n as u128) >> 64) as usize
}

/// `true` with probability `p` (clamped to `[0, 1]`).
pub fn chance(seed: u64, tick: u64, stream: RngStream, key: u64, p: f32) -> bool {
    unit(seed, tick, stream, key) < p.clamp(0.0, 1.0)
}

/// Index picked with probability proportional to `weights`; `None` if they are empty or all zero.
pub fn weighted(
    seed: u64,
    tick: u64,
    stream: RngStream,
    key: u64,
    weights: &[u32],
) -> Option<usize> {
    let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
    if total == 0 {
        return None;
    }
    let mut roll = ((draw(seed, tick, stream, key) as u128 * total as u128) >> 64) as u64;
    for (i, &w) in weights.iter().enumerate() {
        if roll < u64::from(w) {
            return Some(i);
        }
        roll -= u64::from(w);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: u64 = 20_000;

    fn mean(xs: &[f32]) -> f32 {
        xs.iter().sum::<f32>() / xs.len() as f32
    }

    fn correlation(a: &[f32], b: &[f32]) -> f32 {
        let (ma, mb) = (mean(a), mean(b));
        let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
        let va: f32 = a.iter().map(|x| (x - ma).powi(2)).sum();
        let vb: f32 = b.iter().map(|y| (y - mb).powi(2)).sum();
        cov / (va * vb).sqrt()
    }

    #[test]
    fn draw_is_deterministic() {
        for key in 0..100 {
            assert_eq!(
                draw(42, 7, RngStream::ItemRoll, key),
                draw(42, 7, RngStream::ItemRoll, key)
            );
        }
    }

    #[test]
    fn draw_is_pinned() {
        // Changing this value changes every seeded race, ghost and determinism test.
        assert_eq!(draw(1, 2, RngStream::ItemRoll, 3), 0xA7C7_973B_CBCF_0275);
    }

    #[test]
    fn streams_ticks_keys_and_seeds_are_independent() {
        let base: Vec<f32> = (0..N)
            .map(|k| unit(9, 100, RngStream::ItemRoll, k))
            .collect();
        let variants: [Vec<f32>; 4] = [
            (0..N)
                .map(|k| unit(9, 100, RngStream::AiMistakes, k))
                .collect(),
            (0..N)
                .map(|k| unit(9, 101, RngStream::ItemRoll, k))
                .collect(),
            (0..N)
                .map(|k| unit(9, 100, RngStream::ItemRoll, k + 1))
                .collect(),
            (0..N)
                .map(|k| unit(10, 100, RngStream::ItemRoll, k))
                .collect(),
        ];
        assert!((mean(&base) - 0.5).abs() < 0.01, "mean {}", mean(&base));
        for v in &variants {
            assert!((mean(v) - 0.5).abs() < 0.01);
            let r = correlation(&base, v);
            assert!(r.abs() < 0.03, "correlated streams: r = {r}");
            let same = base.iter().zip(v).filter(|(a, b)| a == b).count();
            assert!(same < 5, "{same} identical draws");
        }
    }

    #[test]
    fn helpers_stay_in_range() {
        for k in 0..N {
            let u = unit(3, 4, RngStream::SpawnJitter, k);
            assert!((0.0..1.0).contains(&u));
            let r = range(3, 4, RngStream::SpawnJitter, k, -2.0, 5.0);
            assert!((-2.0..5.0).contains(&r));
            assert!(index(3, 4, RngStream::SpawnJitter, k, 7) < 7);
        }
        assert_eq!(index(3, 4, RngStream::SpawnJitter, 0, 0), 0);
        assert!(!chance(3, 4, RngStream::SpawnJitter, 0, 0.0));
        assert!(chance(3, 4, RngStream::SpawnJitter, 0, 1.0));
    }

    #[test]
    fn weighted_pick_follows_weights() {
        let weights = [10, 0, 30, 60];
        let mut counts = [0u32; 4];
        for k in 0..N {
            let i = weighted(5, 0, RngStream::ItemRoll, k, &weights).expect("non-zero weights");
            counts[i] += 1;
        }
        assert_eq!(counts[1], 0, "zero weight must never be picked");
        for (i, &w) in weights.iter().enumerate() {
            let share = counts[i] as f32 / N as f32;
            assert!(
                (share - w as f32 / 100.0).abs() < 0.015,
                "index {i}: {share}"
            );
        }
        assert_eq!(weighted(5, 0, RngStream::ItemRoll, 0, &[]), None);
        assert_eq!(weighted(5, 0, RngStream::ItemRoll, 0, &[0, 0]), None);
    }
}
