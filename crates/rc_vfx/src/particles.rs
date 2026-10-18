//! Particles as plain data (ARCHITECTURE §5.10, §9): what a particle looks like ([`Look`]), its
//! motion ([`Particle`]) and how its look is packed for the GPU ([`pack_tag`]). Emission maths
//! and the fixed-slot pool are in [`crate::pool`].
//!
//! Every particle is shown by one *slot* entity sharing a single quad mesh and a single material
//! ([`ParticleMaterial`](crate::assets::ParticleMaterial)), so Bevy batches them into instanced
//! draws. The look of a slot comes from its `Transform` (centre, size, orientation) and its
//! `MeshTag`; the shader (`assets/shaders/vfx/particle.wesl`) billboards the quad per camera, so
//! split-screen views each see particles facing them.

use bevy::prelude::*;

/// Particles alive at once at the High density setting (ARCHITECTURE §9: ≤ 4000 total).
pub const MAX_PARTICLES: usize = 4000;

/// Largest emissive intensity a tag can carry (×, linear; art bible §1.2 tops out near 20).
pub const MAX_INTENSITY: f32 = 40.0;

/// The procedural shapes of `particle.wesl` (`shape` field of the tag).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Shape {
    /// Soft round glow.
    #[default]
    Dot = 0,
    /// Hot white core inside a soft halo.
    Core = 1,
    /// Thin line with soft ends (sparks, arcs, bolts, streaks).
    Streak = 2,
    /// Noisy soft cloud (dust, smoke, mist).
    Puff = 3,
    /// Thin ring (shock waves, scan rings, splash rings).
    Ring = 4,
    /// Four-point twinkle.
    Star = 5,
    /// Hard-edged irregular flake (pebbles, mud, grass bits).
    Chip = 6,
    /// Solid square (confetti).
    Square = 7,
    /// Teardrop (water droplets).
    Drop = 8,
    /// Lightning-bolt icon (Volt Cubes).
    Bolt = 9,
    /// Lock-on reticle: ring with notches and a centre dot.
    Reticle = 10,
}

/// How a particle quad is oriented (`mode` field of the tag).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Facing {
    /// Billboard facing the camera, spinning in the view plane.
    #[default]
    Camera,
    /// Stretched along its velocity (sparks, droplets).
    Velocity,
    /// Stretched along the fixed direction of `Particle::rot`'s Y axis (bolt segments, beams).
    Axis,
    /// A world-oriented quad in the XY plane of `Particle::rot` (ground rings, confetti).
    Flat,
}

impl Facing {
    /// Shader mode: 0 billboard, 1 axial, 2 flat.
    fn mode(self) -> u32 {
        match self {
            Self::Camera => 0,
            Self::Velocity | Self::Axis => 1,
            Self::Flat => 2,
        }
    }
}

/// What a particle looks like and how it moves; emitters keep these as `const` presets and copy
/// them with tweaks (`Look { size: [..], ..SPARK }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Procedural shape.
    pub shape: Shape,
    /// Orientation.
    pub facing: Facing,
    /// Unlit and additive (colour is HDR intensity); otherwise lit by the scene and alpha-blended.
    pub emissive: bool,
    /// Colour at birth and at death (linear; emissive: HDR rgb, alpha unused; lit: rgb + alpha).
    pub color: [LinearRgba; 2],
    /// Size (u) at birth and at death: diameter, or width for stretched facings.
    pub size: [f32; 2],
    /// Fraction of the life spent fading in.
    pub fade_in: f32,
    /// Fraction of the life spent fading out.
    pub fade_out: f32,
    /// Velocity damping, 1/s.
    pub drag: f32,
    /// Downward acceleration, u/s² (negative rises).
    pub gravity: f32,
    /// `Facing::Velocity`: extra length per u/s of speed, s.
    pub stretch: f32,
    /// Length / width of stretched facings.
    pub aspect: f32,
    /// Fraction of the vertical speed kept when bouncing on the floor (0 = stick).
    pub bounce: f32,
}

impl Look {
    /// A neutral look (soft white dot, 0.2 u, half a second of fade-out).
    pub const DEFAULT: Look = Look {
        shape: Shape::Dot,
        facing: Facing::Camera,
        emissive: false,
        color: [LinearRgba::WHITE, LinearRgba::WHITE],
        size: [0.2, 0.2],
        fade_in: 0.0,
        fade_out: 0.5,
        drag: 0.0,
        gravity: 0.0,
        stretch: 0.0,
        aspect: 1.0,
        bounce: 0.0,
    };

    /// Same look with other start/end colours.
    pub fn colored(self, start: LinearRgba, end: LinearRgba) -> Self {
        Self {
            color: [start, end],
            ..self
        }
    }
}

/// One live particle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// Appearance and motion parameters.
    pub look: Look,
    /// Position, u.
    pub pos: Vec3,
    /// Velocity, u/s.
    pub vel: Vec3,
    /// Orientation of `Axis` and `Flat` particles.
    pub rot: Quat,
    /// Angular velocity of `Flat` particles, rad/s (world axes).
    pub tumble: Vec3,
    /// Billboard spin angle, rad.
    pub spin: f32,
    /// Billboard spin speed, rad/s.
    pub spin_rate: f32,
    /// Seconds lived.
    pub age: f32,
    /// Seconds to live.
    pub life: f32,
    /// Size multiplier of this particle.
    pub scale: f32,
    /// Height of the ground it bounces on (−∞ = none).
    pub floor: f32,
}

impl Particle {
    /// A particle of `look` at `pos` moving at `vel`, living `life` s.
    pub fn new(look: Look, pos: Vec3, vel: Vec3, life: f32) -> Self {
        Self {
            look,
            pos,
            vel,
            rot: Quat::IDENTITY,
            tumble: Vec3::ZERO,
            spin: 0.0,
            spin_rate: 0.0,
            age: 0.0,
            life: life.max(1e-3),
            scale: 1.0,
            floor: f32::NEG_INFINITY,
        }
    }

    /// Multiplies the size.
    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Orientation (`Axis`: its Y axis is the stretch direction; `Flat`: the quad's plane).
    pub fn rotated(mut self, rot: Quat) -> Self {
        self.rot = rot;
        self
    }

    /// Billboard spin: start angle and speed.
    pub fn spinning(mut self, angle: f32, rate: f32) -> Self {
        self.spin = angle;
        self.spin_rate = rate;
        self
    }

    /// Angular velocity of a `Flat` particle (confetti, chips).
    pub fn tumbling(mut self, tumble: Vec3) -> Self {
        self.tumble = tumble;
        self
    }

    /// Bounces on (or sticks to) the ground at height `y`.
    pub fn floored(mut self, y: f32) -> Self {
        self.floor = y;
        self
    }

    /// Life fraction 0–1.
    pub fn t(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }

    /// One integration step.
    pub(crate) fn step(&mut self, dt: f32) {
        self.age += dt;
        let look = &self.look;
        self.vel.y -= look.gravity * dt;
        self.vel *= (-look.drag * dt).exp();
        self.pos += self.vel * dt;
        if self.pos.y < self.floor {
            self.pos.y = self.floor;
            if self.vel.y < 0.0 {
                self.vel.y = -self.vel.y * look.bounce;
                // Ground friction: what bounces also skids to a stop.
                self.vel.x *= 0.6;
                self.vel.z *= 0.6;
            }
        }
        self.spin += self.spin_rate * dt;
        if self.tumble != Vec3::ZERO {
            self.rot = Quat::from_scaled_axis(self.tumble * dt) * self.rot;
        }
    }

    /// Colour now: the look's gradient with the fade applied to opacity (lit) or intensity
    /// (emissive).
    pub fn color(&self) -> LinearRgba {
        let t = self.t();
        let [a, b] = self.look.color;
        let c = a.mix(&b, t);
        let f = fade(t, self.look.fade_in, self.look.fade_out);
        if self.look.emissive {
            LinearRgba::new(c.red * f, c.green * f, c.blue * f, 1.0)
        } else {
            LinearRgba {
                alpha: c.alpha * f,
                ..c
            }
        }
    }

    /// Size now (u): eases out from the start size to the end size.
    pub fn size(&self) -> f32 {
        let t = self.t();
        let e = 1.0 - (1.0 - t) * (1.0 - t);
        let [a, b] = self.look.size;
        (a + (b - a) * e) * self.scale
    }

    /// The slot's transform: centre, size and orientation as `particle.wesl` reads them.
    pub fn transform(&self) -> Transform {
        let size = self.size().max(1e-4);
        let look = &self.look;
        let (rotation, scale) = match look.facing {
            Facing::Camera => (Quat::from_rotation_z(self.spin), Vec3::splat(size)),
            Facing::Velocity => {
                let speed = self.vel.length();
                let dir = if speed > 1e-4 {
                    self.vel / speed
                } else {
                    self.rot * Vec3::Y
                };
                let len = size * look.aspect + speed * look.stretch * self.scale;
                (
                    Quat::from_rotation_arc(Vec3::Y, dir),
                    Vec3::new(size, len, size),
                )
            }
            Facing::Axis => (self.rot, Vec3::new(size, size * look.aspect, size)),
            Facing::Flat => (self.rot, Vec3::splat(size)),
        };
        Transform {
            translation: self.pos,
            rotation,
            scale,
        }
    }

    /// The slot's `MeshTag` value.
    pub fn tag(&self) -> u32 {
        pack_tag(
            self.color(),
            self.look.emissive,
            self.look.shape,
            self.look.facing,
        )
    }
}

/// Opacity envelope: smooth fade-in over the first `fade_in` of the life and fade-out over the
/// last `fade_out`.
pub fn fade(t: f32, fade_in: f32, fade_out: f32) -> f32 {
    let rise = if fade_in > 0.0 {
        (t / fade_in).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let fall = if fade_out > 0.0 {
        ((1.0 - t) / fade_out).clamp(0.0, 1.0)
    } else {
        1.0
    };
    smooth(rise) * smooth(fall)
}

fn smooth(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

/// Packs a particle's look into a `MeshTag` (decoded by `particle.wesl`):
/// bits 0–15 colour (RGB565, gamma 2), 16–23 opacity (lit) or intensity / [`MAX_INTENSITY`]
/// (emissive), 24–27 shape, 28–29 facing mode, 30 emissive.
pub fn pack_tag(color: LinearRgba, emissive: bool, shape: Shape, facing: Facing) -> u32 {
    let (rgb, level) = if emissive {
        let peak = color.red.max(color.green).max(color.blue).max(0.0);
        let norm = if peak > 0.0 { 1.0 / peak } else { 0.0 };
        (
            [color.red * norm, color.green * norm, color.blue * norm],
            peak / MAX_INTENSITY,
        )
    } else {
        ([color.red, color.green, color.blue], color.alpha)
    };
    let q = |v: f32, max: f32| (v.clamp(0.0, 1.0).sqrt() * max).round() as u32;
    let r = q(rgb[0], 31.0);
    let g = q(rgb[1], 63.0);
    let b = q(rgb[2], 31.0);
    let level = (level.clamp(0.0, 1.0) * 255.0).round() as u32;
    r | g << 5
        | b << 11
        | level << 16
        | (shape as u32) << 24
        | facing.mode() << 28
        | u32::from(emissive) << 30
}

/// Decodes [`pack_tag`] back to (colour, emissive) — the shader's maths, for tests.
#[cfg(test)]
pub fn unpack_color(tag: u32) -> (LinearRgba, bool) {
    let emissive = tag >> 30 & 1 == 1;
    let ch = |bits: u32, max: f32| {
        let v = bits as f32 / max;
        v * v
    };
    let rgb = [
        ch(tag & 31, 31.0),
        ch(tag >> 5 & 63, 63.0),
        ch(tag >> 11 & 31, 31.0),
    ];
    let level = (tag >> 16 & 255) as f32 / 255.0;
    if emissive {
        let i = level * MAX_INTENSITY;
        (
            LinearRgba::new(rgb[0] * i, rgb[1] * i, rgb[2] * i, 1.0),
            true,
        )
    } else {
        (LinearRgba::new(rgb[0], rgb[1], rgb[2], level), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(life: f32) -> Particle {
        Particle::new(Look::DEFAULT, Vec3::ZERO, Vec3::ZERO, life)
    }

    #[test]
    fn particles_fall_drag_and_bounce() {
        let look = Look {
            gravity: 10.0,
            drag: 1.0,
            bounce: 0.5,
            ..Look::DEFAULT
        };
        let mut p = Particle::new(look, Vec3::new(0.0, 1.0, 0.0), Vec3::X * 4.0, 5.0).floored(0.0);
        for _ in 0..120 {
            p.step(1.0 / 60.0);
        }
        assert!(p.pos.y >= 0.0, "never below the floor");
        assert!(p.pos.x > 0.5 && p.vel.x < 4.0, "moves and slows down");
    }

    #[test]
    fn tags_round_trip_colour_and_intensity() {
        let lit = LinearRgba::new(0.5, 0.25, 0.1, 0.6);
        let tag = pack_tag(lit, false, Shape::Puff, Facing::Camera);
        let (c, emissive) = unpack_color(tag);
        assert!(!emissive);
        for (got, want) in [(c.red, 0.5), (c.green, 0.25), (c.blue, 0.1), (c.alpha, 0.6)] {
            assert!((got - want).abs() < 0.04, "{got} vs {want}");
        }
        let hot = LinearRgba::rgb(12.0, 6.0, 1.5);
        let tag = pack_tag(hot, true, Shape::Streak, Facing::Velocity);
        let (c, emissive) = unpack_color(tag);
        assert!(emissive);
        assert!(
            (c.red - 12.0).abs() < 0.3 && (c.green - 6.0).abs() < 0.5,
            "{c:?}"
        );
        assert_eq!(tag >> 24 & 15, Shape::Streak as u32);
        assert_eq!(tag >> 28 & 3, 1, "velocity facing = axial mode");
        // Out-of-range values clamp instead of spilling into other fields.
        let tag = pack_tag(
            LinearRgba::rgb(500.0, 0.0, 0.0),
            true,
            Shape::Dot,
            Facing::Flat,
        );
        assert_eq!(tag >> 24 & 15, 0);
        assert_eq!(tag >> 28 & 3, 2);
    }

    #[test]
    fn fade_envelope_and_size_easing() {
        assert_eq!(fade(0.0, 0.0, 0.5), 1.0);
        assert_eq!(fade(1.0, 0.0, 0.5), 0.0);
        assert_eq!(fade(0.0, 0.2, 0.0), 0.0);
        assert!(fade(0.5, 0.2, 0.2) > 0.99);
        let mut p = dot(1.0);
        p.look.size = [1.0, 3.0];
        let sizes: Vec<f32> = (0..=4)
            .map(|i| {
                p.age = i as f32 * 0.25;
                p.size()
            })
            .collect();
        assert_eq!((sizes[0], sizes[4]), (1.0, 3.0));
        assert!(
            sizes.windows(2).all(|w| w[1] >= w[0]),
            "monotonic {sizes:?}"
        );
        assert!(sizes[2] > 2.0, "eases out: most growth early");
    }

    #[test]
    fn velocity_facing_stretches_along_motion() {
        let look = Look {
            facing: Facing::Velocity,
            stretch: 0.05,
            ..Look::DEFAULT
        };
        let p = Particle::new(look, Vec3::ZERO, Vec3::new(0.0, 0.0, -20.0), 1.0);
        let t = p.transform();
        assert!((t.rotation * Vec3::Y).dot(Vec3::NEG_Z) > 0.999);
        assert!((t.scale.y - (0.2 + 1.0)).abs() < 1e-4, "{:?}", t.scale);
    }
}
