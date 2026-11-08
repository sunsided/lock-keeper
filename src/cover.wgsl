// The title screen: an old desk with tea rings, and the worn notebook lying slightly askew on it.
// The canvas holds the upright book; this pass tilts it and paints everything around it.

const BOOK = vec4<f32>(390.0, 60.0, 890.0, 660.0);
const CENTRE = vec2<f32>(640.0, 360.0);
const TILT = -0.028;

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

// Distance inside the book rectangle; negative outside.
fn inside(q: vec2<f32>) -> f32 {
    return min(min(q.x - BOOK.x, BOOK.z - q.x), min(q.y - BOOK.y, BOOK.w - q.y));
}

fn turned(p: vec2<f32>) -> vec2<f32> {
    let s = sin(TILT);
    let c = cos(TILT);
    let v = p - CENTRE;
    return CENTRE + vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y);
}

// How much a cup left behind at `centre`: a dark broken rim and a faint disc of tea.
fn tea_ring(p: vec2<f32>, centre: vec2<f32>, r: f32) -> f32 {
    let wander = (fbm(p / 24.0 + centre) - 0.5) * 12.0;
    let d = length(p - centre) + wander;
    let rim = exp(-pow((d - r) / 2.6, 2.0));
    let broken = smoothstep(0.32, 0.6, fbm(p / 30.0 + centre * 0.3));
    let disc = 1.0 - smoothstep(r - 3.0, r, d);
    return rim * broken * 0.75 + disc * 0.13 * fbm(p / 16.0 + 5.0);
}

// A splash: blotches thinning out away from `centre`.
fn tea_splash(p: vec2<f32>, centre: vec2<f32>, r: f32) -> f32 {
    let falloff = 1.0 - smoothstep(0.0, r, length(p - centre));
    return smoothstep(0.62 - falloff * 0.3, 0.68 - falloff * 0.3, fbm(p / 13.0 + centre)) * falloff;
}

fn desk(p: vec2<f32>) -> vec3<f32> {
    let row = floor(p.y / 103.0);
    let in_plank = fract(p.y / 103.0) * 103.0;
    let shift = hash(vec2<f32>(row, 3.0));
    let streak = fbm(vec2<f32>(p.x / 150.0 + shift * 40.0, p.y / 5.0 + row * 9.0));
    let blotch = fbm(p / 70.0 + row);
    var c = vec3<f32>(0.13, 0.08, 0.044) * (0.62 + 0.3 * shift + 0.55 * streak) * (0.72 + 0.5 * blotch);

    // Fine pores along the grain, and dents of a long working life.
    let pores = smoothstep(0.66, 0.74, value(vec2<f32>(p.x / 34.0 + shift * 90.0, p.y / 1.5)));
    c *= 1.0 - pores * 0.42;
    c *= 1.0 - smoothstep(0.7, 0.78, fbm(p / 9.0 + 13.0)) * 0.3;
    c += smoothstep(0.74, 0.8, fbm(p * vec2<f32>(0.05, 0.9) + 3.0)) * vec3<f32>(0.09, 0.07, 0.05);

    // Gaps between the planks, chipped along their length.
    let gap = 1.0 + (value(vec2<f32>(p.x / 20.0, row)) - 0.5) * 1.8;
    c *= smoothstep(gap, gap + 1.6, min(in_plank, 103.0 - in_plank)) * 0.9 + 0.1;

    let tea = max(
        max(tea_ring(p, vec2<f32>(190.0, 165.0), 62.0), tea_ring(p, vec2<f32>(262.0, 214.0), 60.0)),
        max(tea_ring(p, vec2<f32>(1085.0, 510.0), 66.0), tea_ring(p, vec2<f32>(335.0, 585.0), 58.0)),
    );
    let splash = max(tea_splash(p, vec2<f32>(1045.0, 180.0), 120.0), tea_splash(p, vec2<f32>(150.0, 470.0), 90.0));
    c = mix(c, c * vec3<f32>(0.42, 0.28, 0.16), clamp(tea + splash * 0.7, 0.0, 0.92));
    return c;
}

fn cover(q: vec2<f32>, d: f32) -> vec3<f32> {
    var c = textureSample(input_tex, linear_sampler, q / post.size).rgb;
    let gold = smoothstep(0.2, 0.42, c.r - c.b);
    let leather = vec3<f32>(0.25, 0.1, 0.08);

    // The gilt has flaked; it thins but never leaves, so the title stays readable.
    let flake = smoothstep(0.42, 0.56, fbm(q / 2.8 + 21.0)) * 0.55 + 0.45 * smoothstep(0.3, 0.7, fbm(q / 22.0));
    c = mix(c, mix(leather, c, 0.42 + 0.58 * flake), gold);

    // Leather grain and creases.
    let grain = 0.74 + 0.5 * fbm(q / 2.1);
    c *= mix(grain, 1.0, gold * 0.6);
    let crease = 1.0 - smoothstep(0.0, 0.03, abs(fbm(q / vec2<f32>(95.0, 34.0)) - 0.5));
    c *= 1.0 - crease * 0.3 * (1.0 - gold);
    c = mix(c, c * vec3<f32>(0.62, 0.5, 0.42), tea_ring(q, vec2<f32>(752.0, 470.0), 72.0));

    // Rubbed pale along the edges and the corners, where a hand or a pocket wore it.
    let corner = length(max(vec2<f32>(0.0), abs(q - CENTRE) - vec2<f32>(225.0, 275.0))) / 28.0;
    let reach = 7.0 + 20.0 * fbm(q / 34.0);
    let rub = max(1.0 - smoothstep(0.0, reach, d), clamp(corner, 0.0, 1.0)) * smoothstep(0.32, 0.72, fbm(q / 9.0));
    c = mix(c, vec3<f32>(0.4, 0.26, 0.18) * (0.8 + 0.4 * fbm(q / 3.0)), rub * 0.62 * (1.0 - gold * 0.7));

    // Scratches that show the paler hide.
    for (var i = 0; i < 4; i++) {
        let f = f32(i);
        let a = hash(vec2<f32>(f, 2.0)) * 3.14;
        let o = BOOK.xy + vec2<f32>(hash(vec2<f32>(f, 7.0)) * 500.0, hash(vec2<f32>(f, 9.0)) * 600.0);
        let v = q - o;
        let along = dot(v, vec2<f32>(cos(a), sin(a)));
        let across = abs(dot(v, vec2<f32>(-sin(a), cos(a))) + (fbm(q / 20.0) - 0.5) * 6.0);
        let line = (1.0 - smoothstep(0.1, 0.8, across)) * (1.0 - smoothstep(12.0, 62.0, abs(along)));
        c = mix(c, vec3<f32>(0.42, 0.29, 0.22), line * 0.34 * (1.0 - gold));
    }
    return c;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * post.size;
    let q = turned(p);
    let d = inside(q);
    // The edge of the board is nicked and uneven.
    let edge = d - 3.5 * fbm(q / 15.0);
    let book = smoothstep(-0.8, 0.8, edge);

    var c = desk(p);
    if book > 0.0 {
        c = mix(c, cover(q, d), book);
    }
    return vec4<f32>(c, 1.0);
}
