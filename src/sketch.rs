//! Ink sketches of the rooms, scribbled in once, stroke by stroke. After that only a fire or a lamp still moves.

use game_toolkit::prelude::*;

const INK: Color = Color::rgb8(38, 44, 66);
const WET: Color = Color::rgb8(4, 6, 22);

pub struct Pen<'p, 'a> {
    pub p: &'p mut Painter<'a>,
    pub origin: [f32; 2],
    pub scale: f32,
    /// How far the drawing has got: 1 when the last stroke is down, more while it dries.
    pub progress: f32,
    /// The number of strokes the drawing has, known from the frame before.
    pub known: usize,
    /// Strokes met so far in this frame.
    pub drawn: usize,
    pub seed: u32,
}

fn noise(n: u32) -> f32 {
    let n = n.wrapping_mul(2_654_435_761) ^ (n >> 7);
    (n.wrapping_mul(1_597_334_677) >> 16) as f32 / 65_535.0 - 0.5
}

impl Pen<'_, '_> {
    fn to_px(&self, (x, y): (f32, f32)) -> [f32; 2] {
        [
            self.origin[0] + x * self.scale,
            self.origin[1] + y * self.scale,
        ]
    }

    fn stroke(&mut self, a: (f32, f32), b: (f32, f32), weight: f32, color: Color) {
        let (a, b) = (self.to_px(a), self.to_px(b));
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let parts = ((len / 28.0) as u32).clamp(1, 12);
        let mut prev = a;
        for i in 1..=parts {
            self.seed = self.seed.wrapping_add(1);
            let t = i as f32 / parts as f32;
            let wob = if i == parts { 0.0 } else { 2.2 };
            let n = self.seed;
            let q = [
                a[0] + (b[0] - a[0]) * t + noise(n) * wob,
                a[1] + (b[1] - a[1]) * t + noise(n ^ 0x5555) * wob,
            ];
            self.drawn += 1;
            let limit = self.progress * self.known as f32;
            if (self.drawn as f32) < limit {
                // The strokes nearest the nib are still wet and black.
                let age = (limit - self.drawn as f32) / (self.known as f32 * 0.3);
                self.p.line(prev, q, weight, WET.lerp(color, age.min(1.0)));
            }
            prev = q;
        }
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32)) {
        self.stroke(a, b, 1.6, INK);
    }

    pub fn poly(&mut self, pts: &[(f32, f32)]) {
        for w in pts.windows(2) {
            self.line(w[0], w[1]);
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)]);
    }

    /// Diagonal shading, the darkness of a sketch.
    pub fn hatch(&mut self, x: f32, y: f32, w: f32, h: f32, gap: f32) {
        let mut d = gap;
        while d < w + h {
            let a = (x + d.min(w), y + (d - w).max(0.0));
            let b = (x + (d - h).max(0.0), y + d.min(h));
            self.stroke(a, b, 1.0, INK.with_alpha(0.55));
            d += gap;
        }
    }

    pub fn ring(&mut self, c: (f32, f32), r: f32) {
        let pts: Vec<_> = (0..=14)
            .map(|i| {
                let a = i as f32 / 14.0 * std::f32::consts::TAU;
                (c.0 + a.cos() * r, c.1 + a.sin() * r)
            })
            .collect();
        self.poly(&pts);
    }

    /// Crosshatching over the part of a box where `inside` holds: the dark of a sky or a corner.
    /// Two passes of diagonals, each broken up by hand, so the second one crosses the first.
    pub fn crosshatch(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        gap: f32,
        inside: &dyn Fn(f32, f32) -> bool,
    ) {
        for (pass, step) in [gap, gap * 1.35].into_iter().enumerate() {
            let mut d = step * 0.6;
            let mut row = 0u32;
            while d < w + h {
                let (mut ax, ay) = (d.min(w), (d - w).max(0.0));
                let (mut bx, by) = ((d - h).max(0.0), d.min(h));
                if pass == 1 {
                    (ax, bx) = (w - ax, w - bx);
                }
                self.hatch_line(
                    (x + ax, y + ay),
                    (x + bx, y + by),
                    row + pass as u32 * 1000,
                    inside,
                );
                d += step;
                row += 1;
            }
        }
    }

    /// One diagonal, as the strokes that lie inside the shape; the pen lifts now and then.
    fn hatch_line(
        &mut self,
        a: (f32, f32),
        b: (f32, f32),
        row: u32,
        inside: &dyn Fn(f32, f32) -> bool,
    ) {
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let steps = (len / 1.2) as u32;
        let mut run: Option<(f32, f32)> = None;
        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let at = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let lifted = noise(row.wrapping_mul(131).wrapping_add(i / 5)) > 0.36;
            if inside(at.0, at.1) && !lifted {
                run.get_or_insert(at);
            } else if let Some(start) = run.take() {
                self.stroke(start, at, 1.0, INK.with_alpha(0.62));
            }
        }
        if let Some(start) = run {
            self.stroke(start, b, 1.0, INK.with_alpha(0.62));
        }
    }

    /// Crosshatch the top corners of a room, `w` wide and `h` deep.
    fn dark_corners(&mut self, w: f32, h: f32) {
        let corner = move |x: f32, y: f32| {
            let across = x.min(100.0 - x);
            across / w + y / h < 1.0
        };
        self.crosshatch((0.0, 0.0, 100.0, h), 2.6, &corner);
    }

    /// A wash of dilute ink: the dark of the night or the water.
    fn wash(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let (pos, size) = (self.to_px((x, y)), [w * self.scale, h * self.scale]);
        let alpha = 0.34 * (self.progress * 2.0).min(1.0);
        self.p.rect(pos, size, INK.with_alpha(alpha));
    }

    fn water(&mut self, x: f32, y: f32, w: f32, rows: u32) {
        for r in 0..rows {
            let ry = y + r as f32 * 3.0;
            let mut sx = x + 3.0 + (r as f32 * 1.9).sin().abs() * 6.0;
            while sx < x + w - 10.0 {
                let len = 7.0 + 6.0 * (noise(r * 31 + sx as u32) + 0.5);
                self.stroke(
                    (sx, ry),
                    ((sx + len).min(x + w), ry),
                    1.1,
                    INK.with_alpha(0.7),
                );
                sx += len + 7.0;
            }
        }
    }
}

/// Draw `room` in a box of 100 by 58 units. `water` is the lock's fill, 0 to 1.
pub fn draw(pen: &mut Pen<'_, '_>, room: &str, water: f32, t: f32) {
    match room {
        "towpath" => {
            pen.wash(0.0, 0.0, 100.0, 37.0);
            pen.wash(0.0, 40.0, 100.0, 18.0);
            pen.crosshatch((0.0, 0.0, 100.0, 38.0), 3.0, &|x, y| {
                let cottage = (4.0..30.0).contains(&x)
                    && if x < 16.0 {
                        y > 18.0 - (x - 4.0) * 0.83
                    } else {
                        y > 8.0 + (x - 16.0) * 0.71
                    };
                let hut = (76.0..96.0).contains(&x) && y > 13.0 + (x - 86.0).abs() * 0.9;
                let moon = (x - 46.0).hypot(y - 10.0) < 7.5;
                let lamp_post = (x - 60.0).abs() < 2.0 && y > 10.0;
                y < 36.5 && !cottage && !hut && !moon && !lamp_post
            });
            pen.ring((46.0, 10.0), 4.0);
            pen.line((0.0, 40.0), (100.0, 36.0));
            pen.water(0.0, 44.0, 100.0, 4);
            pen.poly(&[
                (4.0, 38.0),
                (4.0, 18.0),
                (16.0, 8.0),
                (30.0, 18.0),
                (30.0, 39.0),
            ]);
            pen.rect(12.0, 26.0, 7.0, 13.0);
            pen.hatch(21.0, 22.0, 6.0, 7.0, 2.0);
            pen.poly(&[(60.0, 37.0), (60.0, 16.0)]);
            pen.ring((60.0, 14.0), 2.2);
            pen.poly(&[(78.0, 36.0), (78.0, 22.0), (94.0, 22.0), (94.0, 35.0)]);
            pen.poly(&[(76.0, 22.0), (86.0, 13.0), (96.0, 22.0)]);
        }
        "lock_side" => {
            pen.wash(0.0, 0.0, 100.0, 14.0);
            pen.crosshatch((0.0, 0.0, 100.0, 14.0), 2.6, &|x, y| {
                let moon = (x - 84.0).hypot(y - 5.0) < 4.5;
                let gate = (43.0..57.0).contains(&x) && y > 7.0;
                !moon && !gate
            });
            pen.line((0.0, 14.0), (22.0, 14.0));
            pen.line((78.0, 14.0), (100.0, 14.0));
            // The chamber, seen end on, between its stone walls.
            pen.poly(&[(22.0, 14.0), (22.0, 56.0), (78.0, 56.0), (78.0, 14.0)]);
            pen.hatch(2.0, 16.0, 18.0, 40.0, 4.0);
            pen.hatch(80.0, 16.0, 18.0, 40.0, 4.0);
            let top = 18.0 + (1.0 - water) * 34.0;
            pen.wash(23.0, top, 54.0, 56.0 - top);
            pen.water(23.0, top + 1.0, 54.0, ((56.0 - top) / 4.0) as u32);
            if water < 0.05 {
                // The drained chamber goes black towards the bottom.
                pen.crosshatch((23.0, 26.0, 46.0, 30.0), 2.4, &|_, y| y > 30.0);
                for r in 0..9 {
                    let y = 18.0 + r as f32 * 4.0;
                    pen.line((70.0, y), (75.0, y));
                }
                pen.line((70.0, 14.0), (70.0, 52.0));
                pen.line((75.0, 14.0), (75.0, 52.0));
            }
            // The gate and its balance beam, black against the sky.
            pen.rect(44.0, 8.0, 12.0, 16.0);
            pen.hatch(45.0, 9.0, 10.0, 14.0, 1.8);
            pen.line((12.0, 7.0), (88.0, 11.0));
            pen.line((12.0, 9.0), (88.0, 13.0));
            pen.ring((84.0, 5.0), 2.4);
        }
        "lock_bottom" => {
            pen.wash(0.0, 0.0, 100.0, 50.0);
            pen.line((12.0, 0.0), (20.0, 50.0));
            pen.line((88.0, 0.0), (80.0, 50.0));
            pen.crosshatch((0.0, 0.0, 22.0, 50.0), 2.4, &|x, y| x < 12.0 + y * 0.16);
            pen.crosshatch((78.0, 0.0, 22.0, 50.0), 2.4, &|x, y| x > 88.0 - y * 0.16);
            pen.line((20.0, 50.0), (80.0, 50.0));
            pen.rect(38.0, 38.0, 24.0, 12.0);
            pen.line((42.0, 38.0), (42.0, 50.0));
            pen.line((58.0, 38.0), (58.0, 50.0));
            pen.ring((50.0, 44.0), 3.5);
            for x in [24.0, 30.0, 66.0, 74.0] {
                pen.poly(&[(x, 50.0), (x + 1.0, 45.0), (x + 3.0, 41.0)]);
            }
        }
        "kitchen" => {
            pen.dark_corners(36.0, 22.0);
            pen.crosshatch((14.0, 38.0, 26.0, 8.0), 2.0, &|_, _| true);
            pen.line((0.0, 46.0), (100.0, 46.0));
            pen.rect(58.0, 22.0, 26.0, 24.0);
            pen.hatch(60.0, 25.0, 22.0, 9.0, 2.2);
            pen.rect(66.0, 36.0, 10.0, 8.0);
            pen.poly(&[(71.0, 22.0), (71.0, 2.0)]);
            // The fire is the one thing in the room still moving: a few hand-drawn poses.
            let beat = (t * 7.0).floor();
            let (lick, lean) = ((beat * 2.4).sin() * 1.3, (beat * 1.7).sin() * 0.9);
            pen.poly(&[(68.0, 44.0), (71.0 + lean, 39.0 + lick), (74.0, 44.0)]);
            pen.poly(&[(70.0, 44.0), (71.0 - lean, 41.5 - lick * 0.5), (72.0, 44.0)]);
            pen.rect(12.0, 34.0, 30.0, 3.0);
            pen.line((16.0, 37.0), (16.0, 46.0));
            pen.line((38.0, 37.0), (38.0, 46.0));
            pen.poly(&[(52.0, 44.0), (50.0, 46.0), (56.0, 46.0), (56.0, 41.0)]);
        }
        "parlour" => {
            pen.dark_corners(30.0, 24.0);
            pen.crosshatch((34.0, 36.0, 32.0, 10.0), 2.2, &|_, _| true);
            pen.line((0.0, 46.0), (100.0, 46.0));
            pen.rect(26.0, 32.0, 48.0, 3.0);
            pen.line((32.0, 35.0), (32.0, 46.0));
            pen.line((68.0, 35.0), (68.0, 46.0));
            pen.poly(&[(38.0, 32.0), (40.0, 28.0), (52.0, 29.5), (52.0, 32.0)]);
            pen.poly(&[(52.0, 32.0), (52.0, 29.5), (64.0, 28.0), (66.0, 32.0)]);
            pen.line((58.0, 32.0), (58.0, 24.0));
            pen.poly(&[
                (52.0, 24.0),
                (55.0, 16.0),
                (61.0, 16.0),
                (64.0, 24.0),
                (52.0, 24.0),
            ]);
            pen.rect(6.0, 8.0, 12.0, 16.0);
        }
        "lock_hut" => {
            pen.line((0.0, 46.0), (100.0, 46.0));
            pen.dark_corners(34.0, 26.0);
            pen.crosshatch((0.0, 0.0, 100.0, 6.0), 2.4, &|_, _| true);
            pen.crosshatch((41.0, 13.0, 18.0, 13.0), 2.6, &|_, _| true);
            pen.rect(40.0, 12.0, 20.0, 15.0);
            pen.line((50.0, 12.0), (50.0, 27.0));
            pen.rect(34.0, 34.0, 32.0, 12.0);
            pen.line((34.0, 38.0), (66.0, 38.0));
            pen.poly(&[
                (82.0, 12.0),
                (78.0, 16.0),
                (77.0, 38.0),
                (88.0, 38.0),
                (87.0, 16.0),
                (82.0, 12.0),
            ]);
            pen.hatch(79.0, 18.0, 7.0, 18.0, 2.5);
        }
        "barge_cabin" => {
            pen.crosshatch((10.0, 10.0, 80.0, 18.0), 2.4, &|x, y| {
                let across = (x - 10.0).min(90.0 - x);
                across / 22.0 + (y - 10.0) / 14.0 < 1.0
            });
            pen.poly(&[
                (4.0, 46.0),
                (10.0, 10.0),
                (90.0, 10.0),
                (96.0, 46.0),
                (4.0, 46.0),
            ]);
            pen.rect(14.0, 30.0, 34.0, 5.0);
            pen.line((14.0, 26.0), (14.0, 35.0));
            pen.rect(60.0, 34.0, 8.0, 12.0);
            pen.line((64.0, 34.0), (64.0, 10.0));
            pen.poly(&[(74.0, 46.0), (76.0, 36.0), (85.0, 33.0), (92.0, 46.0)]);
            pen.crosshatch((77.0, 37.0, 11.0, 8.0), 1.8, &|_, _| true);
            let sway = (t * 0.8).sin() * 1.5;
            pen.line((40.0, 10.0), (40.0 + sway, 18.0));
            pen.ring((40.0 + sway, 20.0), 2.0);
        }
        "amos_room" => {
            pen.dark_corners(28.0, 20.0);
            pen.crosshatch((32.0, 41.0, 55.0, 5.0), 1.9, &|_, _| true);
            pen.line((0.0, 46.0), (100.0, 46.0));
            pen.rect(30.0, 36.0, 58.0, 5.0);
            pen.line((30.0, 26.0), (30.0, 46.0));
            pen.line((88.0, 32.0), (88.0, 46.0));
            pen.poly(&[
                (34.0, 36.0),
                (38.0, 30.0),
                (46.0, 30.0),
                (50.0, 33.0),
                (70.0, 33.0),
                (84.0, 34.0),
                (86.0, 36.0),
            ]);
            pen.rect(4.0, 6.0, 18.0, 40.0);
            pen.poly(&[
                (13.0, 12.0),
                (9.0, 15.0),
                (8.0, 32.0),
                (18.0, 32.0),
                (17.0, 15.0),
                (13.0, 12.0),
            ]);
            pen.hatch(10.0, 16.0, 6.0, 15.0, 2.2);
        }
        "hester_room" => {
            pen.wash(49.0, 13.0, 22.0, 18.0);
            // The night outside the panes, and the dark under the slope of the roof.
            pen.crosshatch((49.0, 13.0, 22.0, 18.0), 2.3, &|x, y| {
                (x - 66.0).hypot(y - 17.0) > 3.6
            });
            pen.crosshatch((0.0, 0.0, 40.0, 22.0), 2.6, &|x, y| y < 22.0 - x * 0.5);
            pen.poly(&[(0.0, 22.0), (40.0, 2.0), (100.0, 2.0)]);
            pen.line((0.0, 46.0), (100.0, 46.0));
            pen.rect(48.0, 12.0, 24.0, 20.0);
            pen.line((60.0, 12.0), (60.0, 32.0));
            pen.line((48.0, 22.0), (72.0, 22.0));
            pen.ring((66.0, 17.0), 2.4);
            pen.rect(6.0, 36.0, 30.0, 5.0);
            pen.line((6.0, 41.0), (6.0, 46.0));
            pen.line((36.0, 41.0), (36.0, 46.0));
        }
        _ => {}
    }
}
