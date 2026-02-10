// maVoice overlay shader — dual waveform: user (bottom line) + AI (top bubble)
// Renders on a transparent strip at the bottom of the screen

struct Uniforms {
    resolution: vec2<f32>,    // viewport width, height
    time: f32,                // seconds since start
    intensity: f32,           // user overall fade [0..1]
    levels: vec4<f32>,        // 4 user audio levels [0..1]
    color: vec3<f32>,         // user RGB [0..1]
    mode: f32,                // 0=waveform, 1=processing
    ai_levels: vec4<f32>,     // 4 AI audio levels [0..1]
    ai_color: vec3<f32>,      // AI RGB [0..1]
    ai_intensity: f32,        // AI overall fade [0..1]
}

@group(0) @binding(0) var<uniform> u: Uniforms;

// Fullscreen quad via vertex index
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vi & 1u)) * 2.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// ── User waveform (bottom, full-width line) ──────────────────────

fn wave_y(x: f32, t: f32) -> f32 {
    var sum = 0.0;
    sum += u.levels.x * sin(x * 0.005 + t * 2.8);
    sum += u.levels.y * sin(x * 0.0075 + t * 3.7 + 1.257);
    sum += u.levels.z * sin(x * 0.01 + t * 4.6 + 2.514);
    sum += u.levels.w * sin(x * 0.0125 + t * 5.5 + 3.771);
    return sum * 0.25;
}

fn user_waveform(uv: vec2<f32>, w: f32, h: f32) -> f32 {
    let cy = h * 0.65; // user waveform sits in the lower portion
    let amp = h * 0.3;
    let wave_val = cy + wave_y(uv.x, u.time) * amp;
    let dist = abs(uv.y - wave_val);

    var alpha = 0.0;
    // Multi-layer glow
    alpha += 0.08 * exp(-dist * 0.06);
    alpha += 0.15 * exp(-dist * 0.15);
    alpha += 0.25 * exp(-dist * 0.5);
    alpha += 0.7 * exp(-dist * 2.0);

    return alpha * u.intensity;
}

fn user_processing(uv: vec2<f32>, w: f32, h: f32) -> f32 {
    let cy = h * 0.65;
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

// ── AI bubble (top center, half-oval orb) ────────────────────────

fn ai_wave_distort(angle: f32, t: f32) -> f32 {
    // Organic pulsing — the bubble edge ripples with AI audio levels
    var distort = 0.0;
    distort += u.ai_levels.x * sin(angle * 3.0 + t * 2.2) * 0.15;
    distort += u.ai_levels.y * sin(angle * 5.0 + t * 3.1 + 0.7) * 0.12;
    distort += u.ai_levels.z * sin(angle * 7.0 + t * 4.0 + 1.4) * 0.08;
    distort += u.ai_levels.w * sin(angle * 2.0 + t * 1.8 + 2.1) * 0.1;
    return distort;
}

fn ai_bubble(uv: vec2<f32>, w: f32, h: f32) -> f32 {
    // Bubble center: top center of the strip
    let cx = w * 0.5;
    let cy = h * 0.08; // near the very top

    // Oval dimensions — wider than tall, emanating downward
    let base_rx = h * 0.55; // horizontal radius
    let base_ry = h * 0.45; // vertical radius (stretches downward)

    // Compute angle and normalized distance from center
    let dx = uv.x - cx;
    let dy = uv.y - cy;
    let angle = atan2(dy, dx);

    // Audio-reactive radius distortion
    let avg_level = (u.ai_levels.x + u.ai_levels.y + u.ai_levels.z + u.ai_levels.w) * 0.25;
    let pulse = 1.0 + avg_level * 0.3 + ai_wave_distort(angle, u.time);

    let rx = base_rx * pulse;
    let ry = base_ry * pulse;

    // Elliptical distance (normalized so 1.0 = on the edge)
    let ex = dx / rx;
    let ey = dy / ry;
    let ellipse_dist = length(vec2<f32>(ex, ey));

    // Only render the lower half of the oval (emanating downward from top)
    if dy < -h * 0.05 {
        return 0.0; // clip above the top edge
    }

    var alpha = 0.0;

    // Core glow — bright center fading to edges
    alpha += 0.6 * exp(-ellipse_dist * 2.5);

    // Mid glow
    alpha += 0.3 * exp(-ellipse_dist * 1.5);

    // Outer atmospheric glow
    alpha += 0.12 * exp(-ellipse_dist * 0.8);

    // Edge highlight — subtle bright ring at the bubble boundary
    let edge_dist = abs(ellipse_dist - 0.85);
    alpha += 0.15 * exp(-edge_dist * 12.0) * avg_level;

    // Breathing when connected but AI is quiet — very subtle pulse
    let breath = 0.02 * (sin(u.time * 1.5) * 0.5 + 0.5);
    alpha += breath * exp(-ellipse_dist * 1.2);

    return alpha * u.ai_intensity;
}

// ── Composite ────────────────────────────────────────────────────

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy;
    let w = u.resolution.x;
    let h = u.resolution.y;

    // ── User visualization (bottom) ──
    var user_alpha = 0.0;
    if u.mode < 0.5 {
        user_alpha = user_waveform(uv, w, h);
    } else {
        user_alpha = user_processing(uv, w, h);
    }

    // ── AI bubble (top) ──
    let ai_alpha = ai_bubble(uv, w, h);

    // ── Edge fade (dissolve at strip borders) ──
    let edge_margin = h * 0.2;
    let top_fade = smoothstep(0.0, edge_margin, uv.y);
    let bot_fade = smoothstep(0.0, edge_margin, h - uv.y);
    let edge = top_fade * bot_fade;

    // Composite: blend user and AI colors by their respective alphas
    let user_final = clamp(user_alpha, 0.0, 1.0) * edge;
    let ai_final = clamp(ai_alpha, 0.0, 1.0) * edge;

    // Additive blend of the two layers
    let combined_color = u.color * user_final + u.ai_color * ai_final;
    let combined_alpha = clamp(user_final + ai_final, 0.0, 1.0);

    // Premultiplied alpha output
    return vec4<f32>(combined_color, combined_alpha);
}
