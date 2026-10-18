// Morph `chargeSwap` (rc_ui `filters.rs`): Main menu → mode. A crackling charge front sweeps
// across the screen: the new screen behind it, the old one ahead, a jagged electric band of
// `tint` light with a hot `core` at the front, the image splitting into its channels near it.
//
// Packing (declaration order of `ChargeSwap`):
//   params[0] = (angle_radians, width_px, split_px, -)
//   params[1] = tint (straight linear RGBA; alpha = strength)
//   params[2] = core (straight linear RGBA; alpha = strength)
//   params[7].x = progress (prelude)
//
// PREMULTIPLY: both images are premultiplied captures, resampled directly; the glow is light
// added within a raised alpha. Progress 0 / 1 return the exact samples (identity contract).

import bevy_react::filter::{
    FullscreenVertexOutput,
    morph_progress,
    morph_sample_from,
    morph_sample_to,
    uniforms,
};

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash(i: i32, seed: u32) -> f32 {
    return f32(pcg(bitcast<u32>(i) ^ pcg(seed))) / 4294967296.0;
}

// 1D value noise in [-1, 1].
fn noise(x: f32, seed: u32) -> f32 {
    let i = i32(floor(x));
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash(i, seed), hash(i + 1, seed), u) * 2.0 - 1.0;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let progress = morph_progress();
    if progress <= 0.0 {
        return morph_sample_from(in.uv);
    }
    if progress >= 1.0 {
        return morph_sample_to(in.uv);
    }
    let res = uniforms.resolution;
    let angle = uniforms.params[0].x;
    let width = max(uniforms.params[0].y, 1.0);
    let split_px = uniforms.params[0].z;
    let tint = uniforms.params[1];
    let core = uniforms.params[2];

    let dir = vec2<f32>(cos(angle), sin(angle));
    let perp = vec2<f32>(-dir.y, dir.x);
    let px = in.uv * res;

    // Sweep coordinate over the rect's projection onto `dir` (as in the built-in linear wipe).
    let extent = res * dir;
    let sweep_min = min(0.0, extent.x) + min(0.0, extent.y);
    let sweep_max = max(0.0, extent.x) + max(0.0, extent.y);
    let s = dot(px, dir);

    // The front: jagged like a spark gap, re-rolled a few times a second of progress.
    let flicker = u32(progress * 14.0);
    let t = dot(px, perp);
    let jag = noise(t / (width * 1.6), flicker) * 0.6 + noise(t / (width * 0.45), flicker + 7u) * 0.4;
    let margin = width * 4.0;
    let edge = mix(sweep_min - margin, sweep_max + margin, progress) + jag * width * 1.4;
    let d = s - edge;

    // Glow falls off from the front; strongest mid-transition.
    let g = exp(-abs(d) / width) * sin(progress * 3.14159265);
    let ofs = dir * (split_px * g) / res;
    var r: vec4<f32>;
    var c: vec4<f32>;
    var b: vec4<f32>;
    if d < 0.0 {
        r = morph_sample_to(in.uv - ofs);
        c = morph_sample_to(in.uv);
        b = morph_sample_to(in.uv + ofs);
    } else {
        r = morph_sample_from(in.uv - ofs);
        c = morph_sample_from(in.uv);
        b = morph_sample_from(in.uv + ofs);
    }
    var out = vec4<f32>(r.r, c.g, b.b, max(r.a, max(c.a, b.a)));

    let hot = smoothstep(width * 0.35, 0.0, abs(d));
    let glow = tint.rgb * tint.a * g + core.rgb * core.a * hot;
    let a = clamp(max(out.a, max(glow.r, max(glow.g, glow.b))), 0.0, 1.0);
    out = vec4<f32>(min(out.rgb + glow, vec3<f32>(a)), a);
    return out;
}
