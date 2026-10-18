//! Emission maths ([`Emitter`]) and the fixed-slot particle [`Pool`]: a free list, a budget,
//! round-robin replacement when full.

use bevy::prelude::*;

use crate::particles::{Look, Particle};

/// Fractional emission carried between frames, so a rate of 7.3/s really emits 7.3 per second
/// at any frame rate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Emitter {
    carry: f32,
}

impl Emitter {
    /// Whole particles to emit this frame at `rate` per second.
    pub fn count(&mut self, rate: f32, dt: f32) -> u32 {
        if rate <= 0.0 || dt <= 0.0 {
            self.carry = 0.0;
            return 0;
        }
        self.carry += rate * dt;
        let n = self.carry.floor();
        self.carry -= n;
        n as u32
    }
}

/// Fixed slots with a free list; at the budget the oldest-ish particle (round robin) is replaced,
/// so new events always show.
#[derive(Debug, Clone)]
pub struct Pool {
    parts: Vec<Particle>,
    alive: Vec<bool>,
    free: Vec<u32>,
    limit: usize,
    count: usize,
    cursor: usize,
}

impl Pool {
    /// `slots` slots, all of them usable.
    pub fn new(slots: usize) -> Self {
        let dead = Particle::new(Look::DEFAULT, Vec3::ZERO, Vec3::ZERO, 1.0);
        Self {
            parts: vec![dead; slots],
            alive: vec![false; slots],
            free: (0..slots as u32).rev().collect(),
            limit: slots,
            count: 0,
            cursor: 0,
        }
    }

    /// Number of slots.
    pub fn slots(&self) -> usize {
        self.parts.len()
    }

    /// Live particles.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Budget: only slots `0..limit` are used; shrinking kills the particles above it.
    pub fn set_limit(&mut self, limit: usize) {
        let limit = limit.min(self.parts.len());
        if limit == self.limit {
            return;
        }
        for i in limit..self.limit {
            if self.alive[i] {
                self.alive[i] = false;
                self.count -= 1;
            }
        }
        self.free.retain(|&i| (i as usize) < limit);
        for i in self.limit..limit {
            self.free.push(i as u32);
        }
        self.limit = limit;
        self.cursor %= limit.max(1);
    }

    /// Adds a particle; when every usable slot is busy it replaces one (round robin). Returns
    /// the slot, or `None` with a zero budget.
    pub fn emit(&mut self, p: Particle) -> Option<usize> {
        if self.limit == 0 {
            return None;
        }
        let slot = match self.free.pop() {
            Some(i) => {
                self.count += 1;
                i as usize
            }
            None => {
                let i = self.cursor;
                self.cursor = (self.cursor + 1) % self.limit;
                i
            }
        };
        self.parts[slot] = p;
        self.alive[slot] = true;
        Some(slot)
    }

    /// Integrates every live particle and retires the expired ones.
    pub fn step(&mut self, dt: f32) {
        for i in 0..self.limit {
            if !self.alive[i] {
                continue;
            }
            let p = &mut self.parts[i];
            p.step(dt);
            if p.age >= p.life {
                self.alive[i] = false;
                self.count -= 1;
                self.free.push(i as u32);
            }
        }
    }

    /// Kills everything (restart, end of session).
    pub fn clear(&mut self) {
        self.alive.iter_mut().for_each(|a| *a = false);
        self.free = (0..self.limit as u32).rev().collect();
        self.count = 0;
        self.cursor = 0;
    }

    /// The live particle in `slot`.
    pub fn get(&self, slot: usize) -> Option<&Particle> {
        self.alive
            .get(slot)
            .copied()
            .unwrap_or(false)
            .then(|| &self.parts[slot])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(life: f32) -> Particle {
        Particle::new(Look::DEFAULT, Vec3::ZERO, Vec3::ZERO, life)
    }

    #[test]
    fn emitter_carries_fractions_across_frames() {
        let mut e = Emitter::default();
        let total: u32 = (0..60).map(|_| e.count(7.5, 1.0 / 60.0)).sum();
        assert_eq!(total, 7, "7.5/s for 1 s emits 7 and carries the half");
        let more: u32 = (0..60).map(|_| e.count(7.5, 1.0 / 60.0)).sum();
        assert_eq!(more, 8, "the carried half completes the next one");
        // Rates far above the frame rate emit several per frame.
        assert_eq!(e.count(600.0, 0.1), 60);
        // Stopping drops the carry, so a restart does not burst.
        assert_eq!(e.count(0.0, 0.016), 0);
        assert_eq!(e.count(10.0, 0.05), 0);
    }

    #[test]
    fn emitter_rate_is_frame_rate_independent() {
        for fps in [30.0, 60.0, 144.0] {
            let mut e = Emitter::default();
            let frames = (fps * 2.0) as usize;
            let total: u32 = (0..frames).map(|_| e.count(55.0, 1.0 / fps)).sum();
            assert!((109..=110).contains(&total), "{fps} fps: {total}");
        }
    }

    #[test]
    fn pool_reuses_dead_slots() {
        let mut pool = Pool::new(4);
        let a = pool.emit(dot(0.1)).expect("slot");
        pool.emit(dot(10.0)).expect("slot");
        assert_eq!(pool.count(), 2);
        pool.step(0.2);
        assert_eq!(pool.count(), 1, "the short-lived one retired");
        assert!(pool.get(a).is_none());
        let b = pool.emit(dot(10.0)).expect("slot");
        assert_eq!(a, b, "its slot is reused first");
        assert_eq!(pool.count(), 2);
    }

    #[test]
    fn full_pool_replaces_round_robin_and_never_grows() {
        let mut pool = Pool::new(3);
        for _ in 0..3 {
            pool.emit(dot(10.0));
        }
        let stolen: Vec<usize> = (0..4).filter_map(|_| pool.emit(dot(10.0))).collect();
        assert_eq!(stolen, vec![0, 1, 2, 0]);
        assert_eq!(pool.count(), 3);
    }

    #[test]
    fn shrinking_the_budget_kills_particles_above_it() {
        let mut pool = Pool::new(10);
        for _ in 0..10 {
            pool.emit(dot(10.0));
        }
        pool.set_limit(4);
        assert_eq!(pool.count(), 4);
        for _ in 0..20 {
            let slot = pool.emit(dot(10.0)).expect("slot");
            assert!(slot < 4, "slot {slot} beyond the budget");
        }
        pool.set_limit(6);
        assert_eq!(pool.emit(dot(10.0)), Some(5), "grown slots are free first");
        pool.set_limit(0);
        assert_eq!((pool.count(), pool.emit(dot(1.0))), (0, None));
        pool.set_limit(2);
        pool.emit(dot(1.0));
        pool.clear();
        assert_eq!(pool.count(), 0);
    }
}
