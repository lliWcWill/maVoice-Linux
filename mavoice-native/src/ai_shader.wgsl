// maVoice AI shader — Fibonacci spiral dot sphere
// Phyllotaxis pattern: dots arranged by golden angle, forming a 3D sphere illusion
// Audio-reactive: dots pulse in size/brightness with voice levels

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    intensity: f32,
    levels: vec4<f32>,
    color: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

const GOLDEN_ANGLE: f32 = 2.39996323;  // 137.508 degrees
const NUM_DOTS: i32 = 280;
const PI: f32 = 3.14159265;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vi & 1u)) * 2.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// ── Dot color based on position in spiral ──────────────────────
fn dot_color(idx: f32, norm_r: f32, angle: f32) -> vec3<f32> {
    // Palette: white center → cyan/teal → blue → purple at edges
    let white = vec3<f32>(0.95, 0.97, 1.0);
    let cyan  = vec3<f32>(0.3, 0.85, 0.9);
    let blue  = vec3<f32>(0.35, 0.45, 0.95);
    let purple = vec3<f32>(0.55, 0.25, 0.85);

    // Color varies with radial position + slight angle variation
    let phase = norm_r + sin(angle * 3.0 + idx * 0.02) * 0.15;
    let t = clamp(phase, 0.0, 1.0);

    // 3-stop gradient: white → cyan → blue/purple
    var col: vec3<f32>;
    if t < 0.35 {
        col = mix(white, cyan, t / 0.35);
    } else if t < 0.7 {
        col = mix(cyan, blue, (t - 0.35) / 0.35);
    } else {
        col = mix(blue, purple, (t - 0.7) / 0.3);
    }

    // Subtle time-based shimmer per dot
    let shimmer = sin(idx * 1.618 + u.time * 2.0) * 0.08 + 0.92;
    return col * shimmer;
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let px = frag_coord.xy;
    let w = u.resolution.x;
    let h = u.resolution.y;
    let center = vec2<f32>(w * 0.5, h * 0.5);

    // Sphere radius — fits within window with margin
    let base_radius = min(w, h) * 0.38;

    // Audio energy
    let avg_level = (u.levels.x + u.levels.y + u.levels.z + u.levels.w) * 0.25;

    // Breathing: subtle scale pulse
    let breath = 1.0 + sin(u.time * 1.5) * 0.02 + avg_level * 0.08;
    let radius = base_radius * breath;

    // Slow rotation
    let rot_speed = 0.15 + avg_level * 0.3;
    let rotation = u.time * rot_speed;

    // Accumulate dot contributions
    var total_color = vec3<f32>(0.0);
    var total_alpha: f32 = 0.0;

    for (var i = 0; i < NUM_DOTS; i++) {
        let fi = f32(i) + 1.0; // start from 1 to avoid center singularity
        let frac = fi / f32(NUM_DOTS);

        // ── Fibonacci spiral position ──
        // r grows with sqrt for even area distribution (sunflower pattern)
        let r = sqrt(fi / f32(NUM_DOTS)) * radius;
        let angle = fi * GOLDEN_ANGLE + rotation;

        let dot_pos = center + vec2<f32>(cos(angle), sin(angle)) * r;

        // Distance from this fragment to dot center
        let dist = length(px - dot_pos);

        // ── 3D sphere illusion ──
        // Dots near edge are "on the equator" — foreshortened, dimmer
        let norm_r = r / radius; // 0=center, 1=edge
        let sphere_z = sqrt(max(1.0 - norm_r * norm_r, 0.0)); // z on unit sphere
        let depth_brightness = 0.3 + 0.7 * sphere_z; // brighter at front

        // ── Audio-reactive dot size ──
        // Different dots respond to different frequency bands
        let band = i % 4;
        var band_level: f32;
        if band == 0 { band_level = u.levels.x; }
        else if band == 1 { band_level = u.levels.y; }
        else if band == 2 { band_level = u.levels.z; }
        else { band_level = u.levels.w; }

        // Base dot size: larger at center, smaller at edges (perspective)
        let base_size = mix(2.8, 1.2, norm_r);
        let audio_size = base_size * (1.0 + band_level * 1.5);

        // ── Gaussian dot shape ──
        let falloff = dist * dist / (audio_size * audio_size);
        let dot_alpha = exp(-falloff * 2.0) * depth_brightness;

        // Skip negligible contributions
        if dot_alpha < 0.005 { continue; }

        // ── Dot color ──
        let col = dot_color(fi, norm_r, angle);

        // Audio brightening: dots flash brighter when their band is loud
        let audio_bright = 1.0 + band_level * 0.8;

        // Additive color accumulation (light emission)
        total_color += col * dot_alpha * audio_bright;
        total_alpha = max(total_alpha, dot_alpha);
    }

    // Scale everything by intensity (0 when idle = fully transparent)
    total_color *= u.intensity;
    total_alpha *= u.intensity;
    total_alpha = clamp(total_alpha, 0.0, 1.0);

    // Subtle ambient glow behind the sphere
    let to_center = length(px - center) / radius;
    let ambient = 0.015 * exp(-to_center * to_center * 1.5) * u.intensity;
    let ambient_col = u.color * 0.5;
    total_color += ambient_col * ambient;
    total_alpha = clamp(total_alpha + ambient, 0.0, 1.0);

    // Apply sRGB gamma then premultiply (X11 ARGB compositing)
    let srgb = pow(clamp(total_color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    return vec4<f32>(srgb * total_alpha, total_alpha);
}
