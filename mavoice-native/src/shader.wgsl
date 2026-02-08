// maVoice overlay shader — WGSL port of the GLSL fragment shader
// Renders waveform (recording) and orbs (processing) on a transparent strip

struct Uniforms {
    resolution: vec2<f32>,  // viewport width, height in device pixels
    time: f32,              // seconds since start
    intensity: f32,         // overall fade [0..1]
    levels: vec4<f32>,      // 4 audio levels [0..1]
    color: vec3<f32>,       // RGB [0..1]
    mode: f32,              // 0=waveform, 1=processing
}

@group(0) @binding(0) var<uniform> u: Uniforms;

// Fullscreen quad via vertex index — no vertex buffer needed
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // Triangle strip: 0,1,2 + 2,1,3 covers the full clip space
    let x = f32(i32(vi & 1u)) * 2.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Compute waveform Y displacement at position x
fn wave_y(x: f32, t: f32) -> f32 {
    var sum = 0.0;
    // 4 overlapping sine waves — exact frequencies/speeds/phases from GLSL
    sum += u.levels.x * sin(x * 0.005 + t * 2.8);
    sum += u.levels.y * sin(x * 0.0075 + t * 3.7 + 1.257);
    sum += u.levels.z * sin(x * 0.01 + t * 4.6 + 2.514);
    sum += u.levels.w * sin(x * 0.0125 + t * 5.5 + 3.771);
    return sum * 0.25;
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy;
    let w = u.resolution.x;
    let h = u.resolution.y;
    let cy = h * 0.5;
    let amp = h * 0.35;

    var alpha = 0.0;

    if u.mode < 0.5 {
        // === WAVEFORM MODE ===
        let wave_val = cy + wave_y(uv.x, u.time) * amp;
        let dist = abs(uv.y - wave_val);

        // Multi-layer glow using exponential falloff
        // Layer 1: Wide atmospheric glow
        alpha += 0.08 * exp(-dist * 0.06);
        // Layer 2: Medium glow
        alpha += 0.15 * exp(-dist * 0.15);
        // Layer 3: Tight glow
        alpha += 0.25 * exp(-dist * 0.5);
        // Layer 4: Core line (very sharp falloff)
        alpha += 0.7 * exp(-dist * 2.0);

        alpha *= u.intensity;
    } else {
        // === PROCESSING MODE ===
        // Two traveling orbs
        let ox1 = w * 0.5 + sin(u.time * 2.5) * w * 0.35;
        let ox2 = w * 0.5 + sin(u.time * 2.5 + 2.2) * w * 0.25;

        let d1 = length(uv - vec2<f32>(ox1, cy));
        let d2 = length(uv - vec2<f32>(ox2, cy));

        alpha += 0.5 * exp(-d1 * 0.04);
        alpha += 0.3 * exp(-d2 * 0.06);

        // Thin baseline
        let line_dist = abs(uv.y - cy);
        alpha += 0.05 * exp(-line_dist * 0.3);
    }

    alpha = clamp(alpha, 0.0, 1.0);

    // Premultiplied alpha output (required for transparent compositing)
    return vec4<f32>(u.color * alpha, alpha);
}
