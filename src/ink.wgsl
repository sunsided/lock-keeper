// Ink on damp paper. A port of the ink-bleed recipe of web-visual-fx to one pass: wander along the
// paper fibers, blur (wet), soft-step the coverage, pool pigment at the rim, granulate. Works on
// the transmittance against the paper, so the paper itself needs no separate layer.

struct Params {
    // Rows still wet: top y, bottom y, wetness 0..1; w is 1 when a verdict is stamped on the sketch.
    wet: vec4<f32>,
    // x: how wet the sketch still is, 0..1.
    sketch: vec4<f32>,
    // Stains: x, y, radius in canvas pixels, ink (0 blue-black, 1 oxblood). Radius 0 is unused.
    stains: array<vec4<f32>, 8>,
};
@group(1) @binding(0) var<uniform> params: Params;

const PAPER = vec3<f32>(0.91, 0.863, 0.737);
// The open notebook; the desk and the watch stay dry. Same numbers as BOOK_* in main.rs.
const BOOK = vec4<f32>(44.0, 26.0, 1168.0, 694.0);
const SKETCH = vec4<f32>(70.0, 100.0, 575.0, 395.0);

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn value(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    return value(p) * 0.5 + value(p * 2.03 + 17.0) * 0.25 + value(p * 4.1 + 41.0) * 0.14
        + value(p * 8.2 + 7.0) * 0.11;
}

fn box_mask(p: vec2<f32>, b: vec4<f32>, fade: f32) -> f32 {
    let d = min(min(p.x - b.x, b.z - p.x), min(p.y - b.y, b.w - p.y));
    return smoothstep(0.0, fade, d);
}

// Blurred sample in eight taps around `p`, plus the centre.
fn wet_sample(p: vec2<f32>, r: f32) -> vec3<f32> {
    var sum = textureSample(input_tex, linear_sampler, p / post.size).rgb * 2.0;
    for (var i = 0; i < 8; i++) {
        let a = f32(i) * 0.7854;
        let q = p + vec2<f32>(cos(a), sin(a)) * r;
        sum += textureSampleLevel(input_tex, linear_sampler, q / post.size, 0.0).rgb;
    }
    return sum / 10.0;
}

fn coverage(c: vec3<f32>) -> f32 {
    return clamp(1.0 - dot(c / PAPER, vec3<f32>(0.3, 0.59, 0.11)), 0.0, 0.98);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * post.size;
    let dry = textureSample(input_tex, nearest_sampler, in.uv);
    let page = box_mask(p, BOOK, 6.0);
    if page <= 0.0 {
        return dry;
    }

    let sketch = box_mask(p, SKETCH, 4.0) * (1.0 - params.wet.w);
    let row_wet = params.wet.z * smoothstep(params.wet.x - 6.0, params.wet.x, p.y)
        * (1.0 - smoothstep(params.wet.y, params.wet.y + 6.0, p.y)) * step(606.0, p.x);

    // Fountain pen for the writing; a watercolour wash in the sketch; wet rows run further.
    let sketch_wet = params.sketch.x;
    let spread = page * mix(mix(1.5, 4.5, row_wet), 7.0 + 5.0 * sketch_wet, sketch);
    let blur = mix(mix(0.55, 1.5, row_wet), 1.8 + 1.4 * sketch_wet, sketch);
    let pool = mix(mix(0.25, 0.75, row_wet), 0.8, sketch);
    let grain = mix(0.14, 0.45, sketch);

    let fibre = p / mix(9.0, 26.0, sketch);
    let warp = vec2<f32>(fbm(fibre), fbm(fibre + vec2<f32>(37.0, 11.0))) - 0.47;
    let q = p + warp * spread * 2.0;

    var c = wet_sample(q, blur);
    let a = coverage(c);
    // Reform an edge above the soft step; below it leave faint things (rules, foxing) alone.
    var a2 = max(a, smoothstep(0.16, 0.62, a) * min(1.0, a * 1.5));
    // Pigment collects where the wide blur says the wash ends.
    let wide = coverage(wet_sample(q, mix(3.0, 8.0, sketch)));
    let rim = clamp(2.0 * (1.0 - wide / max(a2, 0.05)), 0.0, 1.0);
    a2 = a2 * (1.0 - 0.35 * pool * (1.0 - rim)) * (1.0 + 0.55 * pool * rim);
    a2 *= 1.0 - grain + grain * smoothstep(0.25, 0.75, fbm(p / 2.6));
    a2 = clamp(a2, 0.0, 0.985);
    // Ordered dither for the washes only: letters and pen lines (dense coverage) stay whole.
    let cell = bayer4(vec2<i32>(p / 2.0));
    let soft = sketch * (1.0 - smoothstep(0.42, 0.6, a2));
    a2 = mix(a2, floor(a2 * 5.0 + cell) / 5.0, soft);
    var out = c * (1.0 - a2) / max(1.0 - a, 0.02);
    // Keep the hue of the ink where the step made it denser than it was.
    out = min(out, max(c, PAPER * (1.0 - a2)));

    for (var i = 0; i < 8; i++) {
        let s = params.stains[i];
        if s.z <= 0.0 {
            continue;
        }
        let d = length(p - s.xy);
        if d > s.z * 1.7 {
            continue;
        }
        let n = fbm(p / (s.z * 0.33) + s.xy * 0.13);
        let fine = fbm(p / 7.0 + s.xy);
        let edge = s.z * (0.62 + 0.72 * n) + (fine - 0.5) * 7.0;
        let body = 1.0 - smoothstep(edge - 3.0, edge + 1.5, d);
        let ring = smoothstep(edge - s.z * 0.2, edge - 1.0, d) * body;
        // Blooms: water pushed the pigment away from the middle.
        let bloom = 0.42 + 0.5 * smoothstep(0.3, 0.75, fbm(p / (s.z * 0.5) + 91.0));
        let amount = body * (0.3 * bloom + 0.55 * ring)
            * (0.75 + 0.25 * fbm(p / 2.4));
        let tint = mix(vec3<f32>(0.10, 0.14, 0.34), vec3<f32>(0.46, 0.08, 0.07), s.w);
        let dithered = floor(clamp(amount, 0.0, 0.9) * 6.0 + cell) / 6.0;
        out = mix(out, out * tint, dithered * page);
    }
    return vec4<f32>(mix(dry.rgb, out, page), dry.a);
}
