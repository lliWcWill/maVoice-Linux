// maVoice user waveform shader — glowing audio-reactive line
// Original style with boosted sensitivity and richer wave motion

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    intensity: f32,
    levels: vec4<f32>,
    color: vec3<f32>,
    mode: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vi & 1u)) * 2.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// ── Gain curve: boost quiet signals, compress loud ones ──────────
fn gain(v: f32) -> f32 {
    return pow(clamp(v, 0.0, 1.0), 0.55);
}

// ── Wave function — 6 harmonics for rich, reactive motion ────────
fn wave_y(x: f32, t: f32) -> f32 {
    let l0 = gain(u.levels.x);
    let l1 = gain(u.levels.y);
    let l2 = gain(u.levels.z);
    let l3 = gain(u.levels.w);

    var sum = 0.0;
    // Primary waves — driven by each frequency band
    sum += l0 * sin(x * 0.004 + t * 2.8);
    sum += l1 * sin(x * 0.007 + t * 3.7 + 1.257);
    sum += l2 * sin(x * 0.011 + t * 4.6 + 2.514);
    sum += l3 * sin(x * 0.016 + t * 5.5 + 3.771);
    // Secondary harmonics — add detail and texture
    sum += l0 * 0.5 * sin(x * 0.022 + t * 6.3 + 0.8);
    sum += l2 * 0.4 * sin(x * 0.009 + t * 7.1 + 4.2);

    // Average over ~5.4 effective waves (not 6) to keep peaks meaningful
    return sum * 0.18;
}

fn user_waveform(uv: vec2<f32>, w: f32, h: f32) -> f32 {
    let cy = h * 0.55; // center the wave vertically
    let amp = h * 0.42; // bigger amplitude — uses most of the 64px
    let wave_val = cy + wave_y(uv.x, u.time) * amp;
    let dist = abs(uv.y - wave_val);

    // Multi-layer glow — wider and brighter
    var alpha = 0.0;
    alpha += 0.06 * exp(-dist * 0.04);  // wide atmospheric haze
    alpha += 0.12 * exp(-dist * 0.10);  // outer glow
    alpha += 0.22 * exp(-dist * 0.30);  // mid glow
    alpha += 0.40 * exp(-dist * 0.8);   // inner glow
    alpha += 0.80 * exp(-dist * 2.5);   // hot core

    return alpha * u.intensity;
}

fn user_processing(uv: vec2<f32>, w: f32, h: f32) -> f32 {
    let cy = h * 0.55;
    let ox1 = w * 0.5 + sin(u.time * 2.5) * w * 0.35;
    let ox2 = w * 0.5 + sin(u.time * 2.5 + 2.2) * w * 0.25;

    let d1 = length(uv - vec2<f32>(ox1, cy));
    let d2 = length(uv - vec2<f32>(ox2, cy));

    var alpha = 0.0;
    alpha += 0.5 * exp(-d1 * 0.04);
    alpha += 0.3 * exp(-d2 * 0.06);

    let line_dist = abs(uv.y - cy);
    alpha += 0.05 * exp(-line_dist * 0.3);

    return alpha;
}

// ── Composite ────────────────────────────────────────────────────

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy;
    let w = u.resolution.x;
    let h = u.resolution.y;

    var user_alpha = 0.0;
    if u.mode < 0.5 {
        user_alpha = user_waveform(uv, w, h);
    } else {
        user_alpha = user_processing(uv, w, h);
    }

    // Edge fade at strip borders
    let edge_margin = h * 0.15;
    let top_fade = smoothstep(0.0, edge_margin, uv.y);
    let bot_fade = smoothstep(0.0, edge_margin, h - uv.y);
    let edge = top_fade * bot_fade;

    let alpha = clamp(user_alpha * edge, 0.0, 1.0);

    // Apply sRGB gamma then premultiply (X11 ARGB compositing)
    let srgb = pow(clamp(u.color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    return vec4<f32>(srgb * alpha, alpha);
}
