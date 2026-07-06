// Screen-space CRT post pass: gentle anisotropic
// tube warp, feathered rectangular mask, constant chroma offset, RGB-phased
// scanlines with phosphor beading, animated grain, bright-pass bloom, and a
// radial vignette.
//
// All strengths come from the CrtPost uniform; with every setting at zero the
// pass is an identity copy. The view texture samples as linear light, but the
// scanline/grain/bloom maths reads better on gamma-space values, so samples
// are encoded, processed, and decoded at the end.
//
// Tuned gentle on purpose: the HUD's small pixel text sits near the bottom
// corners, so warp and fringe stay small and constant across the frame.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct CrtPost {
    time: f32,
    crt: f32,
    grain: f32,
    vignette: f32,
    bloom: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(2) var<uniform> settings: CrtPost;

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

/// Gamma-space screen sample, clamped to the frame.
fn grab(uv: vec2<f32>) -> vec3<f32> {
    let c = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));
    return to_gamma(textureSampleLevel(screen_texture, texture_sampler, c, 0.0).rgb);
}

fn hash21(p: vec2<f32>) -> f32 {
    var h = fract(p * vec2<f32>(0.1031, 0.0973));
    h += dot(h, h.yx + 33.33);
    return fract((h.x + h.y) * h.x);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(screen_texture));
    let c = settings.crt;

    // Tube warp: cross-axis bulge, a touch stronger vertically, slight zoom.
    var p = in.uv * 2.0 - vec2<f32>(1.0);
    p *= 1.0 - 0.006 * c;
    p += p.yx * p.yx * p * vec2<f32>(0.038, 0.054) * c;
    let uv = p * 0.5 + vec2<f32>(0.5);

    // Feathered rectangular screen edge; only fades in with the warp.
    var mask = (1.0 - smoothstep(0.986, 1.004, abs(p.x)))
        * (1.0 - smoothstep(0.986, 1.004, abs(p.y)));
    mask = mix(1.0, mask, clamp(c * 6.0, 0.0, 1.0));

    // Constant sub-pixel chroma offset — uniform across the frame so edge
    // text never rainbows.
    let ca = vec2<f32>(0.7 / dims.x, 0.0) * min(c * 1.5, 1.0);
    var rgb = vec3<f32>(
        grab(uv + ca).r,
        grab(uv).g,
        grab(uv - ca).b,
    );

    // RGB-phased scanlines (~8px pitch) with soft horizontal phosphor
    // beading; mild amplitude so text stays legible.
    let row = uv.y * dims.y * 0.75;
    let bead = 0.82 + 0.18 * sin(uv.x * dims.x * 0.66);
    let scan = vec3<f32>(
        sin(row),
        sin(row + 2.094),
        sin(row + 4.189),
    );
    rgb *= 1.0 + 0.065 * c * bead * (scan - vec3<f32>(0.25));

    // Animated grain on chunky 2px cells.
    let seed = floor(uv * dims / 2.0) + vec2<f32>(fract(settings.time * 7.31) * 191.0);
    rgb += (hash21(seed) - 0.5) * 0.07 * settings.grain;

    // Bloom: cross of neighbours, bright pass, gentle add.
    if settings.bloom > 0.5 && c > 0.001 {
        let sx = vec2<f32>(2.5 / dims.x, 0.0);
        let sy = vec2<f32>(0.0, 2.5 / dims.y);
        let glow = grab(uv) * 0.34
            + (grab(uv + sx) + grab(uv - sx)) * 0.165
            + (grab(uv + sy) + grab(uv - sy)) * 0.165;
        rgb += max(glow - vec3<f32>(0.72), vec3<f32>(0.0)) * 0.5 * c;
    }

    // Radial vignette.
    let vig = smoothstep(0.35, 1.9, dot(p, p));
    rgb *= 1.0 - vig * settings.vignette * 0.5;

    return vec4<f32>(to_linear(rgb) * mask, 1.0);
}
