// The open notebook lies a hair off square on the desk. Catmull-Rom sampling keeps the
// handwriting as sharp at this tiny angle as it is upright; the desk and the watch stay put.

// The book with its shadow, and TILT, the same as in main.rs.
const REGION = vec4<f32>(34.0, 18.0, 1184.0, 710.0);
const CENTRE = vec2<f32>(640.0, 360.0);
const TILT = 0.0055;
const WATCH = vec3<f32>(1222.0, 92.0, 59.0);

fn weights(t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return vec4<f32>(
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    );
}

fn sharp(q: vec2<f32>) -> vec4<f32> {
    let at = q - 0.5;
    let base = vec2<i32>(floor(at));
    let f = at - floor(at);
    let wx = weights(f.x);
    let wy = weights(f.y);
    var sum = vec4<f32>(0.0);
    for (var y = 0; y < 4; y++) {
        for (var x = 0; x < 4; x++) {
            sum += textureLoad(input_tex, base + vec2<i32>(x - 1, y - 1), 0) * wx[x] * wy[y];
        }
    }
    return clamp(sum, vec4<f32>(0.0), vec4<f32>(1.0));
}

fn inside(p: vec2<f32>) -> f32 {
    return min(min(p.x - REGION.x, REGION.z - p.x), min(p.y - REGION.y, REGION.w - p.y));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * post.size;
    let here = textureSample(input_tex, nearest_sampler, in.uv);
    if inside(p) < -8.0 || distance(p, WATCH.xy) < WATCH.z {
        return here;
    }
    let s = sin(TILT);
    let c = cos(TILT);
    let v = p - CENTRE;
    let q = CENTRE + vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y);
    if inside(q) >= 0.0 {
        return sharp(q);
    }
    // A sliver the turned book no longer covers: bare desk, from just past the old edge.
    if inside(p) >= 0.0 {
        let left = p.x - REGION.x;
        let right = REGION.z - p.x;
        let top = p.y - REGION.y;
        let bottom = REGION.w - p.y;
        let nearest = min(min(left, right), min(top, bottom));
        var out_at = p;
        if nearest == left {
            out_at.x = REGION.x - 3.0;
        } else if nearest == right {
            out_at.x = REGION.z + 3.0;
        } else if nearest == top {
            out_at.y = REGION.y - 3.0;
        } else {
            out_at.y = REGION.w + 3.0;
        }
        return textureLoad(input_tex, vec2<i32>(out_at), 0);
    }
    return here;
}
