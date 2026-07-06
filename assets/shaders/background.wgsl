// Pixelated three-colour marble backdrop.
// Chunky cells, a whirlpool twist around the centre, and a sine-folded
// domain warp that stretches the palette into flowing paint ribbons.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct SwirlParams {
    colour_1: vec4<f32>,
    colour_2: vec4<f32>,
    colour_3: vec4<f32>,
    resolution: vec2<f32>,
    time: f32,
    spin_time: f32,
    contrast: f32,
    spin_amount: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: SwirlParams;

// Chunky cell count along the screen diagonal.
const CELLS_ACROSS: f32 = 420.0;
const WARP_STEPS: i32 = 5;

fn rot(p: vec2<f32>, a: f32) -> vec2<f32> {
    let s = sin(a);
    let c = cos(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let res = params.resolution;
    let diag = length(res);

    // Snap to chunky cells, then recentre so the swirl sits mid-screen.
    let cell = diag / CELLS_ACROSS;
    let snapped = (floor(mesh.uv * res / cell) + 0.5) * cell;
    var p = (snapped - 0.5 * res) / diag * 2.0;

    // Whirlpool: twist decays away from the centre, whole field creeps round.
    let t = params.time;
    let r = length(p);
    var ang = atan2(p.y, p.x);
    ang += 0.07 * params.spin_time;
    ang -= params.spin_amount * (2.1 + 0.35 * sin(0.23 * params.spin_time)) * exp(-2.4 * r);
    p = r * vec2<f32>(cos(ang), sin(ang));

    // Marble: fold the plane through rotated travelling sine waves.
    var q = (p + vec2<f32>(0.23, -0.17)) * 26.0;
    let flow = 0.42 * t;
    for (var i = 0; i < WARP_STEPS; i++) {
        let fi = f32(i);
        q += 1.1 * vec2<f32>(
            sin(0.53 * q.y + flow + 2.3 * fi),
            cos(0.47 * q.x - 0.71 * flow + 1.6 * fi),
        );
        q = rot(q, 0.71 + 0.33 * fi);
    }
    let f = 0.5 + 0.5 * sin(0.37 * q.x + 0.29 * q.y);

    // Three bands: colour_2 low, colour_3 seam, colour_1 high.
    // Contrast narrows the blend so ribbons get crisp dark outlines.
    let e = 0.07 / (0.7 + 0.5 * params.contrast);
    let hi = smoothstep(0.58 - e, 0.58 + e, f);
    let lo = 1.0 - smoothstep(0.42 - e, 0.42 + e, f);
    let body = mix(params.colour_2.rgb, params.colour_1.rgb, smoothstep(0.42, 0.58, f));
    // Seam darkness fades with contrast so low-contrast presets stay calm.
    let seam = (1.0 - hi - lo) * clamp(0.5 + 0.16 * params.contrast, 0.0, 1.0);
    let rgb = mix(body, params.colour_3.rgb, seam);

    return vec4<f32>(rgb, 1.0);
}
