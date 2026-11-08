//! The Lock-Keeper's Night: a host of the loremaster session, drawn as a constable's notebook on
//! a desk, lit by a candle that follows the mouse.

mod ink;
mod page;
mod sketch;
mod story;

use game_toolkit::prelude::*;
use loremaster_runtime::{
    BlockKind, Effect, EndingInfo, Input, JournalEntry, LinkKind, Scene, Session, Status, Step,
};
use page::{Action, Book, Ink, Piece};
use std::collections::BTreeSet;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key as WKey, NamedKey};

mod ifiction_meta {
    ifiction::include_generated!();
}

const W: f32 = 1280.0;
const H: f32 = 720.0;
const ROW: f32 = 31.0;
const PAGE_L: f32 = 44.0;
const SPINE: f32 = 606.0;
const BOOK_R: f32 = 1168.0;
const BOOK_T: f32 = 26.0;
const BOOK_B: f32 = 694.0;
const TEXT_X: f32 = 646.0;
const TEXT_W: f32 = 494.0;
const FIRST_ROW: f32 = 58.0;
/// Rows of writing above the input row.
const ROWS: usize = 19;
const PEN_SPEED: f32 = 110.0;
const TILT: f32 = 0.0055;
const INK_DRIES: f32 = 0.6;
/// Seconds the pen takes over a sketch, and how much of that again it stays wet.
const SCRIBBLE: f32 = 1.6;
const SKETCH_DRIES: f32 = 0.8;

const CASE_Y: f32 = 492.0;
const CASE_ROW: f32 = 28.0;
const HANDS: Layer = Layer(2);
const PAPER: Color = Color::rgb8(232, 220, 188);
const WET: Color = Color::rgb8(6, 12, 48);
const LANTERN: &str = r"
struct Params {
    lamp: vec2<f32>,
    flicker: f32,
    dread: f32,
    // What lies on the desk: the book's rectangle, its turn, the watch's radius (0: no watch).
    book: vec4<f32>,
    tilt: f32,
    watch: f32,
    _pad: vec2<f32>,
};
@group(1) @binding(0) var<uniform> params: Params;

const WATCH = vec2<f32>(1222.0, 92.0);
const SPINE = 606.0;
// In canvas pixels: the flame above the desk, and the heights of what lies on it.
const FLAME = 300.0;
const BOOK_THICK = 17.0;
const WATCH_THICK = 12.0;

fn grain(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
}

// Distance inside the book; negative outside.
fn book_d(p: vec2<f32>) -> f32 {
    let b = params.book;
    let centre = (b.xy + b.zw) * 0.5;
    let half_size = (b.zw - b.xy) * 0.5;
    let s = sin(params.tilt);
    let c = cos(params.tilt);
    let v = p - centre;
    let q = abs(vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y));
    return min(half_size.x - q.x, half_size.y - q.y);
}

// Is there something between `p` and the flame? Follow the ray up to the thing's height.
fn shadow_at(p: vec2<f32>, flame: vec2<f32>) -> f32 {
    let dist = length(flame - p);
    let soft = 1.5 + dist * 0.014;
    var dark = 0.0;
    let on_watch = params.watch > 0.0 && length(p - WATCH) <= params.watch + 5.0;
    if book_d(p) < 0.0 && !on_watch {
        let up = p + (flame - p) * BOOK_THICK / FLAME;
        dark = smoothstep(-soft, soft, book_d(up));
    }
    if params.watch > 0.0 && length(p - WATCH) > params.watch {
        let up = p + (flame - p) * WATCH_THICK / FLAME;
        dark = max(dark, smoothstep(soft, -soft, length(up - WATCH) - params.watch));
    }
    return dark;
}

// The pages curve down into the spine, so each side of it turns towards or away from the flame.
fn page_curve(p: vec2<f32>, flame: vec2<f32>) -> f32 {
    if params.watch <= 0.0 || book_d(p) < 0.0 {
        return 1.0;
    }
    let from_spine = p.x - SPINE;
    let slope = -sign(from_spine) * pow(max(0.0, 1.0 - abs(from_spine) / 80.0), 2.0) * 0.8;
    let to_flame = (flame.x - p.x) / length(vec2<f32>(flame.x - p.x, FLAME));
    return 1.0 + min(0.0, slope * to_flame) * 0.6 + max(0.0, slope * to_flame) * 0.1;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(input_tex, linear_sampler, in.uv);
    let aspect = post.window_size.x / post.window_size.y;
    let d = length((in.uv - params.lamp) * vec2<f32>(aspect, 1.0));
    // The pool shrinks as three o'clock comes, but never so far that the page is unreadable.
    let reach = mix(1.35, 0.85, params.dread) * (1.0 + params.flicker * 0.05);
    let smooth_pool = (1.0 - smoothstep(0.12, reach, d)) * (1.0 + params.flicker * 0.12);
    // The light falls off in ordered-dither steps of two canvas pixels. The steps are small
    // against the contrast of ink on paper, so the text stays readable anywhere on the page.
    let p = (in.pos.xy - post.canvas_origin) / post.canvas_extent * post.canvas_size;
    let pool = floor(smooth_pool * 5.0 + bayer4(vec2<i32>(p / 2.0))) / 5.0;
    let flame = (params.lamp * post.window_size - post.canvas_origin) / post.canvas_extent
        * post.canvas_size;
    let tint = mix(vec3<f32>(0.62, 0.62, 0.74), vec3<f32>(1.1, 1.0, 0.82), pool);
    let lit = mix(0.74, 1.06, pool) * page_curve(p, flame);
    // Shadow takes the flame's light away, down to the dim light of the room.
    let shade = 1.0 - shadow_at(p, flame) * mix(0.38, 0.72, pool);
    let light = c.rgb * tint * lit * shade;
    let speck = grain(floor(in.uv * post.window_size)) * 0.03 * light;
    return vec4<f32>(light + speck, c.a);
}
";

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LampParams {
    lamp: [f32; 2],
    flicker: f32,
    dread: f32,
    book: [f32; 4],
    tilt: f32,
    watch: f32,
    _pad: [f32; 2],
}

struct LockKeeper {
    session: Session,
    book: Book,
    hand: BitmapFont,
    heading: BitmapFont,
    print: BitmapFont,
    lantern: PostEffectId,
    ink: PostEffectId,
    cover: PostEffectId,
    tilt: PostEffectId,
    stains: ink::Stains,
    /// Top, bottom and wetness of the rows that are still drying.
    wet: [f32; 3],
    /// Proofs that already have their blot, and the last hour that struck.
    stained: Vec<String>,
    hour: u32,
    /// The words of the story's verbs that a click can start a command with.
    verbs: BTreeSet<String>,
    lamp: [f32; 2],
    typed: String,
    now: f32,
    /// Back from the newest row, with the mouse wheel.
    scroll: usize,
    opened: bool,
    /// What can be clicked, from the last render.
    hot: Vec<([f32; 4], Action)>,
    scene: Scene,
    journal: Vec<JournalEntry>,
    ending: Option<EndingInfo>,
    water: f32,
    /// When the sketch was begun, what it shows, and how many strokes it has.
    sketch_from: f32,
    sketch_of: String,
    sketch_strokes: usize,
    /// `--shot FILE CMD;CMD`: play the commands and save a screenshot, for checking the look.
    shot: Option<(String, Vec<String>)>,
}

fn load_font(gfx: &mut Graphics, fnt: &str, png: &[u8]) -> Result<BitmapFont> {
    BitmapFont::from_bmfont(gfx, fnt, png)
}

/// Minutes of the night gone, from 22:00, and the hour hand's source.
fn clock(time: &str) -> (u32, u32) {
    let mut parts = time.split(':').filter_map(|p| p.trim().parse::<u32>().ok());
    (parts.next().unwrap_or(22), parts.next().unwrap_or(0))
}

fn ink_color(ink: Ink) -> Color {
    match ink {
        Ink::Prose => Color::rgb8(26, 38, 92),
        Ink::Title => Color::rgb8(20, 20, 30),
        Ink::Speech => Color::rgb8(122, 30, 26),
        Ink::Said => Color::rgb8(34, 86, 62),
        Ink::Note | Ink::Pencil => Color::rgb8(92, 86, 78),
        Ink::Choice => Color::rgb8(150, 60, 18),
    }
}

impl LockKeeper {
    fn begin(&mut self) {
        self.opened = true;
        let step = self.session.start();
        self.apply(step);
    }

    fn write(&mut self, pieces: &[Piece], now: f32) {
        let hand = &self.hand;
        self.book
            .write(pieces, TEXT_W, &|s| hand.measure(s, 1)[0] as f32, now);
    }

    fn apply(&mut self, step: Step) {
        let now = self.now;
        self.book.expire();
        self.scene = self.session.scene();
        for effect in step.effects {
            match effect {
                Effect::Text { block } => {
                    let ink = match block.kind {
                        BlockKind::Command | BlockKind::Exits | BlockKind::Author => continue,
                        BlockKind::Title | BlockKind::End => Ink::Title,
                        BlockKind::Speech => Ink::Speech,
                        BlockKind::Said => Ink::Said,
                        BlockKind::Note => Ink::Note,
                        _ => Ink::Prose,
                    };
                    let mut pieces = Vec::new();
                    for run in &block.runs {
                        let text = run.text.replace('*', "");
                        let ink = if run.speaker.is_some() {
                            Ink::Speech
                        } else {
                            ink
                        };
                        if let Some(l) = run.link.as_ref().filter(|l| l.kind == LinkKind::Exit) {
                            let action = Action::Noun(l.id.clone());
                            pieces.push(Piece {
                                text,
                                ink,
                                action: Some(action),
                            });
                        } else if ink == Ink::Prose {
                            pieces.extend(page::mark_words(&text, ink, &|w, verb_ok| {
                                self.word_action(w, verb_ok)
                            }));
                        } else {
                            pieces.push(Piece {
                                text,
                                ink,
                                action: None,
                            });
                        }
                    }
                    self.book.blank(now);
                    self.write(&pieces, now);
                }
                Effect::Choices { choices, .. } => {
                    self.book.blank(now);
                    for c in choices {
                        let piece = Piece {
                            text: format!("{}. {}", c.index + 1, c.text),
                            ink: Ink::Choice,
                            action: Some(Action::Choice(c.index)),
                        };
                        self.write(&[piece], now);
                    }
                }
                _ => {}
            }
        }
        // shortcut: Status::Await is unhandled; this story has no await rules. Handle it when one does.
        if let Status::Ended { ending } = step.status {
            self.ending = ending;
        }
        self.scene = self.session.scene();
        self.journal = self.session.journal();
        self.drop_stains();
        let water = match story::fact(&self.session, "lock").as_str() {
            "full" => 1.0,
            "emptying" => 0.45,
            _ => 0.0,
        };
        // A new room or a changed lock is a new drawing; otherwise the sketch stays as it is.
        if self.sketch_of != self.scene.room || self.water != water {
            self.sketch_of.clone_from(&self.scene.room);
            self.water = water;
            self.sketch_from = self.now;
            self.sketch_strokes = 0;
        }
        self.scroll = 0;
    }

    /// What a word of the writing can do: be a verb to start a command with, an exit, or a thing
    /// that is here. Things win, so "boots" is never a verb.
    fn word_action(&self, word: &str, verb_ok: bool) -> Option<Action> {
        const SKIP: [&str; 8] = ["the", "and", "with", "here", "from", "this", "that", "into"];
        if word.len() < 3 || SKIP.contains(&word) {
            return None;
        }
        let things = self.scene.things.iter().chain(&self.scene.inventory);
        for thing in things {
            if page::main_words(&thing.name).iter().any(|w| w == word) {
                return Some(Action::Noun(thing.name.to_lowercase()));
            }
        }
        if self.scene.exits.iter().any(|e| e.direction == word) {
            return Some(Action::Noun(word.to_owned()));
        }
        (verb_ok && self.verbs.contains(word)).then(|| Action::Verb(word.to_owned()))
    }

    /// A proof blots the case list, an hour striking blots a margin, a verdict blots the sketch.
    fn drop_stains(&mut self) {
        let now = self.now;
        for (i, entry) in self.journal.iter().enumerate() {
            if entry.done && !self.stained.contains(&entry.quest) {
                self.stained.push(entry.quest.clone());
                let pos = [505.0, CASE_Y + 12.0 + i as f32 * CASE_ROW];
                self.stains.add(pos, 26.0, ink::BLUE_BLACK, now);
            }
        }
        let (hour, _) = clock(&self.scene.time);
        if hour != self.hour {
            self.hour = hour;
            let pos = [
                BOOK_R - 34.0 - (hour * 37 % 60) as f32,
                660.0 - (hour * 53 % 500) as f32,
            ];
            self.stains.add(pos, 34.0, ink::BLUE_BLACK, now);
        }
        if self.ending.is_some() {
            self.stains.add([450.0, 300.0], 110.0, ink::OXBLOOD, now);
        }
    }

    fn perform(&mut self, action: Action) {
        if self.ending.is_some() {
            return;
        }
        let action = match action {
            // A verb waits on the input row for its noun, as in the old point-and-click games.
            Action::Verb(v) => {
                self.typed = format!("{v} ");
                return;
            }
            Action::Noun(n) if self.scene.exits.iter().any(|e| e.direction == n) => Action::Text(n),
            Action::Noun(n) if self.typed.ends_with(' ') => {
                Action::Text(format!("{}{n}", std::mem::take(&mut self.typed)))
            }
            Action::Noun(n) => Action::Text(format!("examine {n}")),
            other => other,
        };
        let input = match action {
            Action::Choice(i) => Input::Choice(i),
            Action::Verb(_) | Action::Noun(_) => return,
            Action::Text(t) => {
                let now = self.now;
                self.book.blank(now);
                let echo = Piece {
                    text: format!("> {t}"),
                    ink: Ink::Pencil,
                    action: None,
                };
                self.write(&[echo], now);
                match t.parse::<usize>() {
                    Ok(n) if n > 0 && !self.session.choices().is_empty() => Input::Choice(n - 1),
                    _ => Input::Text(t),
                }
            }
        };
        let step = self.session.submit(input);
        self.apply(step);
    }

    fn restart(&mut self) {
        self.session = story::new_session();
        self.book = Book::default();
        self.ending = None;
        self.stains = ink::Stains::default();
        self.stained.clear();
        self.hour = 22;
        self.typed.clear();
        self.begin();
    }

    fn enter(&mut self) {
        if self.book.writing() {
            self.book.finish();
        } else if self.ending.is_some() {
            self.restart();
        } else if !self.typed.trim().is_empty() {
            let t = std::mem::take(&mut self.typed);
            self.perform(Action::Text(t.trim().to_owned()));
        }
    }

    fn draw_desk(p: &mut Painter<'_>) {
        p.clear(Color::rgb8(30, 19, 12));
        for i in 0..7 {
            let y = i as f32 * H / 7.0;
            let shade = 0.13 + 0.025 * ((i * 5 % 3) as f32);
            p.rect(
                [0.0, y],
                [W, H / 7.0],
                [shade * 1.25, shade * 0.8, shade * 0.5, 1.0],
            );
            p.rect([0.0, y], [W, 2.0], [0.04, 0.02, 0.01, 1.0]);
            for g in 0..9 {
                let n = (i * 37 + g * 91) % 100;
                let gy = y + 8.0 + (n % 80) as f32;
                let gx = (g * 173 + i * 59) as f32 % W;
                p.line(
                    [gx, gy],
                    [gx + 200.0 + n as f32 * 2.0, gy + 2.0],
                    1.0,
                    [0.05, 0.03, 0.015, 0.7],
                );
            }
        }
    }

    fn draw_paper(p: &mut Painter<'_>) {
        p.rect(
            [PAGE_L - 8.0, BOOK_T - 6.0],
            [BOOK_R - PAGE_L + 16.0, BOOK_B - BOOK_T + 12.0],
            Color::rgb8(62, 30, 22),
        );
        p.rect([PAGE_L, BOOK_T], [BOOK_R - PAGE_L, BOOK_B - BOOK_T], PAPER);
        // Gutter shadow.
        for i in 0..14 {
            let a = 0.2 * (1.0 - i as f32 / 14.0);
            p.rect(
                [SPINE - 2.0 * i as f32, BOOK_T],
                [2.0, BOOK_B - BOOK_T],
                [0.2, 0.13, 0.05, a],
            );
            p.rect(
                [SPINE + 2.0 * i as f32, BOOK_T],
                [2.0, BOOK_B - BOOK_T],
                [0.2, 0.13, 0.05, a],
            );
        }
        // Foxing: the age of the paper.
        for i in 0u32..26 {
            let x = PAGE_L + 12.0 + ((i * 197 + 31) % 1090) as f32;
            let y = BOOK_T + 10.0 + ((i * 113 + 7) % 640) as f32;
            p.circle([x, y], 2.0 + (i % 5) as f32, [0.55, 0.38, 0.16, 0.13]);
        }
        // The ruled lines and the margin of the right page.
        let rule = [0.28, 0.40, 0.62, 0.28];
        for r in 0..=ROWS + 1 {
            let y = FIRST_ROW + ROW * r as f32 + 25.0;
            p.line([SPINE + 16.0, y], [BOOK_R - 10.0, y], 1.0, rule);
        }
        p.line(
            [TEXT_X - 10.0, BOOK_T],
            [TEXT_X - 10.0, BOOK_B],
            1.0,
            [0.7, 0.2, 0.15, 0.4],
        );
    }

    fn draw_writing(&mut self, p: &mut Painter<'_>, now: f32, mouse: [f32; 2]) {
        let end = self.book.end();
        let end = end.saturating_sub(self.scroll).max(ROWS.min(end));
        let first = end.saturating_sub(ROWS);
        let shown = self.book.shown as usize;
        let mut wet = [f32::MAX, 0.0, 0.0f32];
        for (row, line) in self.book.lines[first..end].iter().enumerate() {
            let y = FIRST_ROW + ROW * row as f32;
            let dry = ((now - line.born) / INK_DRIES).clamp(0.0, 1.0);
            if dry < 1.0 && !line.spans.is_empty() && shown > line.start {
                wet = [wet[0].min(y), wet[1].max(y + ROW), wet[2].max(1.0 - dry)];
            }
            let mut pen = line.start;
            for span in &line.spans {
                let count = span.text.chars().count();
                let visible = shown.saturating_sub(pen).min(count);
                pen += count;
                if visible == 0 {
                    continue;
                }
                let text: String = span.text.chars().take(visible).collect();
                let x = TEXT_X + span.x;
                let color = WET.lerp(ink_color(span.ink), dry);
                p.bitmap_text(&self.hand, [x, y - 4.0], &text, 1, color);
                if let Some(action) = &span.action {
                    let rect = [x, y, span.w, ROW];
                    let over = in_rect(rect, mouse);
                    let alpha = if over { 1.0 } else { 0.6 };
                    p.line(
                        [x, y + 25.0],
                        [x + span.w * visible as f32 / count as f32 - 1.0, y + 25.0],
                        1.4,
                        ink_color(Ink::Choice).with_alpha(alpha),
                    );
                    self.hot.push((rect, action.clone()));
                }
            }
        }
        self.wet = wet;
        // The input row.
        let y = FIRST_ROW + ROW * ROWS as f32;
        let caret = if now.fract() < 0.55 && !self.book.writing() {
            "_"
        } else {
            ""
        };
        let text = format!("> {}{caret}", self.typed);
        p.bitmap_text(
            &self.hand,
            [TEXT_X, y - 4.0],
            &text,
            1,
            ink_color(Ink::Pencil),
        );
    }

    fn label(&self, p: &mut Painter<'_>, x: f32, y: f32, s: &str) {
        p.bitmap_text(&self.print, [x, y], s, 1, Color::rgb8(110, 76, 44));
    }

    fn draw_left(&mut self, p: &mut Painter<'_>, now: f32, mouse: [f32; 2]) {
        let x = PAGE_L + 26.0;
        p.bitmap_text(
            &self.heading,
            [x, 34.0],
            &self.scene.room_name,
            1,
            Color::rgb8(20, 20, 30),
        );
        let progress = (now - self.sketch_from) / SCRIBBLE;
        let scale = 5.0;
        let mut pen = sketch::Pen {
            p: &mut *p,
            origin: [x + 3.0, 102.0],
            scale,
            progress,
            known: self.sketch_strokes,
            drawn: 0,
            seed: 1,
        };
        sketch::draw(&mut pen, &self.scene.room, self.water, now);
        self.sketch_strokes = pen.drawn;

        let mut y = 412.0;
        self.label(p, x, y, "WAYS OUT");
        y += 20.0;
        let mut ex = x;
        for exit in &self.scene.exits {
            let text = format!("{} ", exit.direction);
            let w = self.hand.measure(&text, 1)[0] as f32;
            let rect = [ex, y, w, ROW];
            let over = in_rect(rect, mouse);
            let color = if over {
                ink_color(Ink::Choice)
            } else {
                ink_color(Ink::Prose)
            };
            p.bitmap_text(&self.hand, [ex, y - 4.0], &text, 1, color);
            self.hot.push((rect, Action::Text(exit.direction.clone())));
            ex += w + 14.0;
        }
        y += 40.0;
        self.label(p, x, y, "THE CASE");
        y += 20.0;
        for entry in &self.journal {
            let c = ink_color(Ink::Prose);
            p.rect_outline([x + 2.0, y + 6.0], [14.0, 14.0], 1.4, c);
            if entry.done {
                p.line(
                    [x + 4.0, y + 13.0],
                    [x + 9.0, y + 19.0],
                    2.2,
                    ink_color(Ink::Speech),
                );
                p.line(
                    [x + 9.0, y + 19.0],
                    [x + 20.0, y + 2.0],
                    2.2,
                    ink_color(Ink::Speech),
                );
            }
            p.bitmap_text(&self.hand, [x + 26.0, y - 4.0], &entry.title, 1, c);
            y += ROW - 3.0;
        }
        y = y.max(610.0);
        self.label(p, x, y, "IN MY POCKETS");
        y += 20.0;
        let names: Vec<_> = self
            .scene
            .inventory
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        p.bitmap_text(
            &self.hand,
            [x, y - 4.0],
            &names.join(", "),
            1,
            ink_color(Ink::Prose),
        );

        if let Some(ending) = &self.ending {
            let w = self.heading.measure(&ending.title, 1)[0] as f32 + 40.0;
            let (sx, sy) = (x + (500.0 - w) / 2.0, 210.0);
            let red = Color::rgb8(150, 24, 22);
            p.rect([sx, sy], [w, 74.0], PAPER.with_alpha(0.86));
            p.rect_outline([sx, sy], [w, 74.0], 4.0, red);
            p.rect_outline([sx + 6.0, sy + 6.0], [w - 12.0, 62.0], 1.5, red);
            p.bitmap_text(&self.heading, [sx + 20.0, sy + 6.0], &ending.title, 1, red);
        }
    }

    fn draw_watch(&self, p: &mut Painter<'_>) {
        let c = [1222.0, 92.0];
        let r = 52.0;
        // The chain, running off the desk.
        let mut prev = [c[0] - 6.0, c[1] + r];
        for i in 1..14 {
            let t = i as f32 / 13.0;
            let q = [
                c[0] - 6.0 + 30.0 * t,
                c[1] + r + 300.0 * t + (t * 9.0).sin() * 8.0,
            ];
            p.line(prev, q, 2.5, Color::rgb8(150, 120, 60));
            prev = q;
        }
        p.circle(c, r + 5.0, Color::rgb8(176, 142, 76));
        p.circle(c, r, Color::rgb8(238, 230, 208));
        p.circle_outline(c, r, 1.5, Color::rgb8(60, 44, 20));
        let at = |a: f32, l: f32| {
            let a = a * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            [c[0] + a.cos() * l, c[1] + a.sin() * l]
        };
        for h in 0..12 {
            let a = h as f32 / 12.0;
            p.line_ex(
                at(a, r - 7.0),
                at(a, r - 2.0),
                2.0,
                Color::rgb8(40, 34, 28),
                HANDS,
                BlendMode::Alpha,
            );
        }
        // Three o'clock is marked in red: the coroner.
        let three = at(0.25, r - 13.0);
        p.rect_ex(
            [three[0] - 4.0, three[1] - 4.0],
            [8.0, 8.0],
            Color::rgb8(170, 30, 26),
            HANDS,
            BlendMode::Alpha,
        );
        let (hour, minute) = clock(&self.scene.time);
        let m = minute as f32 / 60.0;
        let h = ((hour % 12) as f32 + m) / 12.0;
        p.line_ex(
            c,
            at(h, r * 0.5),
            3.5,
            Color::rgb8(24, 20, 16),
            HANDS,
            BlendMode::Alpha,
        );
        p.line_ex(
            c,
            at(m, r * 0.78),
            2.0,
            Color::rgb8(24, 20, 16),
            HANDS,
            BlendMode::Alpha,
        );
    }

    fn draw_cover(&self, p: &mut Painter<'_>, now: f32) {
        let (x, y, w, h) = (390.0, 60.0, 500.0, 600.0);
        p.rect([x, y], [w, h], Color::rgb8(70, 30, 24));
        p.rect([x, y], [36.0, h], Color::rgb8(48, 20, 16));
        let gold = Color::rgb8(201, 164, 92);
        p.rect_outline([x + 56.0, y + 26.0], [w - 82.0, h - 52.0], 2.0, gold);
        for (line, ty) in [("The", 150.0), ("Lock-Keeper's", 205.0), ("Night", 260.0)] {
            let tw = self.heading.measure(line, 1)[0] as f32;
            p.bitmap_text(
                &self.heading,
                [x + 18.0 + (w - tw) / 2.0, ty],
                line,
                1,
                gold,
            );
        }
        p.circle_outline([x + 18.0 + w / 2.0, 410.0], 46.0, 2.0, gold);
        p.line(
            [x + 18.0 + w / 2.0, 370.0],
            [x + 18.0 + w / 2.0, 450.0],
            2.0,
            gold,
        );
        p.line(
            [x + 18.0 + w / 2.0 - 40.0, 410.0],
            [x + 18.0 + w / 2.0 + 40.0, 410.0],
            2.0,
            gold,
        );
        let label = "County Constabulary - Autumn 1923";
        let tw = self.print.measure(label, 1)[0] as f32;
        p.bitmap_text(
            &self.print,
            [x + 18.0 + (w - tw) / 2.0, 535.0],
            label,
            1,
            gold,
        );
        let a = 0.55 + 0.45 * (now * 2.0).sin().abs();
        let open = "open the notebook";
        let tw = self.hand.measure(open, 1)[0] as f32;
        p.bitmap_text(
            &self.hand,
            [x + 18.0 + (w - tw) / 2.0, 572.0],
            open,
            1,
            gold.with_alpha(a),
        );
    }
}

/// Where a point on the screen falls on the page, which lies turned by `TILT` (see `tilt.wgsl`).
fn page_point((x, y): (f32, f32)) -> [f32; 2] {
    let (s, c) = TILT.sin_cos();
    let v = [x - W / 2.0, y - H / 2.0];
    [W / 2.0 + c * v[0] + s * v[1], H / 2.0 - s * v[0] + c * v[1]]
}

/// Smooth noise over time, in -1 to 1.
fn noise(t: f32) -> f32 {
    let hash = |i: f32| ((i * 12.9898).sin() * 43_758.547).rem_euclid(1.0);
    let f = t.rem_euclid(1.0);
    let s = f * f * (3.0 - 2.0 * f);
    (hash(t.floor()) + (hash(t.floor() + 1.0) - hash(t.floor())) * s) * 2.0 - 1.0
}

/// A candle: a restless flutter, and now and then a dip when a draught finds it.
fn candle_flicker(t: f32) -> f32 {
    let draught = (noise(t * 0.6) - 0.5).max(0.0) * 2.0;
    (noise(t * 3.7) * 0.6 + noise(t * 8.3) * 0.4 - draught).clamp(-1.0, 1.0)
}

/// Where a `--shot` holds the lamp: the middle, or `LK_SHOT_LAMP=x,y` in window fractions.
fn shot_lamp(shooting: bool) -> Option<[f32; 2]> {
    if !shooting {
        return None;
    }
    let at = std::env::var("LK_SHOT_LAMP").ok().and_then(|v| {
        let (x, y) = v.split_once(',')?;
        Some([x.parse().ok()?, y.parse().ok()?])
    });
    Some(at.unwrap_or([0.5, 0.45]))
}

fn in_rect(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

impl Game for LockKeeper {
    fn init(ctx: &mut Context) -> Result<Self> {
        let hand = load_font(
            &mut ctx.gfx,
            include_str!("../assets/fonts/hand.fnt"),
            include_bytes!("../assets/fonts/hand.png"),
        )?;
        let heading = load_font(
            &mut ctx.gfx,
            include_str!("../assets/fonts/heading.fnt"),
            include_bytes!("../assets/fonts/heading.png"),
        )?;
        let print = load_font(
            &mut ctx.gfx,
            include_str!("../assets/fonts/print.fnt"),
            include_bytes!("../assets/fonts/print.png"),
        )?;
        let params = LampParams {
            lamp: [0.5, 0.45],
            flicker: 0.0,
            dread: 0.0,
            book: [0.0; 4],
            tilt: 0.0,
            watch: 0.0,
            _pad: [0.0; 2],
        };
        let fx = ShaderEffect::with_params(&ctx.gfx, "lantern", LANTERN, &params);
        let lantern = ctx.gfx.add_post_effect(PostStage::Window, fx);
        let params = ink::InkParams {
            wet: [0.0; 4],
            sketch: [0.0; 4],
            stains: [[0.0; 4]; ink::MAX_STAINS],
        };
        let fx = ShaderEffect::with_params(&ctx.gfx, "ink", include_str!("ink.wgsl"), &params);
        let ink = ctx.gfx.add_post_effect(PostStage::Canvas, fx);
        let fx = ShaderEffect::new(&ctx.gfx, "cover", include_str!("cover.wgsl"));
        let cover = ctx.gfx.add_post_effect(PostStage::Canvas, fx);
        let fx = ShaderEffect::new(&ctx.gfx, "tilt", include_str!("tilt.wgsl"));
        let tilt = ctx.gfx.add_post_effect(PostStage::Canvas, fx);
        let session = story::new_session();
        let scene = session.scene();
        let verbs = session
            .verbs()
            .into_iter()
            .flat_map(|v| v.words)
            .filter(|w| !w.contains(' '))
            .collect();
        let mut args = std::env::args().skip(1);
        let shot = (args.next().as_deref() == Some("--shot")).then(|| {
            let file = args.next().unwrap_or_else(|| "shot.png".into());
            let cmds = args.next().unwrap_or_default();
            (
                file,
                cmds.split(';')
                    .filter(|c| !c.is_empty())
                    .map(str::to_owned)
                    .collect(),
            )
        });
        Ok(Self {
            session,
            book: Book::default(),
            hand,
            heading,
            print,
            lantern,
            ink,
            cover,
            tilt,
            stains: ink::Stains::default(),
            wet: [0.0; 3],
            stained: Vec::new(),
            hour: 22,
            verbs,
            lamp: [0.5, 0.45],
            typed: String::new(),
            now: 0.0,
            scroll: 0,
            opened: false,
            hot: Vec::new(),
            scene,
            journal: Vec::new(),
            ending: None,
            water: 1.0,
            sketch_from: 0.0,
            sketch_of: String::new(),
            sketch_strokes: 0,
            shot,
        })
    }

    fn raw_window_event(&mut self, _ctx: &mut Context, e: &WindowEvent) -> bool {
        let WindowEvent::KeyboardInput { event, .. } = e else {
            return false;
        };
        // A screenshot run plays its own commands.
        if self.shot.is_some() {
            return true;
        }
        if event.state != ElementState::Pressed {
            return false;
        }
        if !self.opened {
            if !event.repeat {
                self.begin();
            }
            return true;
        }
        match &event.logical_key {
            WKey::Named(NamedKey::Enter) => {
                if !event.repeat {
                    self.enter();
                }
            }
            WKey::Named(NamedKey::Backspace) => {
                self.typed.pop();
            }
            WKey::Named(NamedKey::Escape) => self.typed.clear(),
            WKey::Named(NamedKey::PageUp) => {
                self.scroll = (self.scroll + ROWS / 2).min(self.book.lines.len())
            }
            WKey::Named(NamedKey::PageDown) => self.scroll = self.scroll.saturating_sub(ROWS / 2),
            _ => {
                if let Some(text) = &event.text
                    && self.typed.len() < 48
                    && self.ending.is_none()
                {
                    self.typed
                        .extend(text.chars().filter(|c| c.is_ascii() && !c.is_control()));
                }
            }
        }
        true
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        self.now = ctx.time.elapsed.as_secs_f32();
        self.book.advance(dt * PEN_SPEED);

        if ctx.input.mouse_pressed(MouseButton::Left) {
            if !self.opened {
                self.begin();
            } else if self.book.writing() {
                self.book.finish();
            } else if self.ending.is_some() {
                self.restart();
            } else {
                let m = page_point(ctx.input.mouse_pos());
                let hit = self
                    .hot
                    .iter()
                    .find(|(r, _)| in_rect(*r, m))
                    .map(|(_, a)| a.clone());
                if let Some(action) = hit {
                    self.perform(action);
                }
            }
        }
        let wheel = ctx.input.scroll().1;
        if wheel > 0.0 {
            self.scroll = (self.scroll + 2).min(self.book.lines.len());
        } else if wheel < 0.0 {
            self.scroll = self.scroll.saturating_sub(2);
        }

        // The candle lags behind the hand.
        let window = ctx.gfx.window_size();
        let m = ctx.input.mouse_pos_window();
        let target = if let Some(at) = shot_lamp(self.shot.is_some()) {
            at
        } else if m == (0.0, 0.0) {
            [0.5, 0.45]
        } else {
            [m.0 / window.0 as f32, m.1 / window.1 as f32]
        };
        for (l, t) in self.lamp.iter_mut().zip(target) {
            *l += (t.clamp(0.0, 1.0) - *l) * (dt * 5.0).min(1.0);
        }
        let t = ctx.time.elapsed.as_secs_f32();
        let flicker = candle_flicker(t);
        // The flame wanders a little on its wick, so the shadows breathe with it.
        let flame = [
            self.lamp[0] + noise(t * 4.0 + 7.0) * 0.004,
            self.lamp[1] + noise(t * 4.0 + 19.0) * 0.006,
        ];
        let (book, tilt, watch) = if self.opened {
            ([36.0, 20.0, 1176.0, 700.0], TILT, 57.0)
        } else {
            ([390.0, 60.0, 890.0, 660.0], -0.028, 0.0)
        };
        let (hour, minute) = clock(&self.scene.time);
        let gone = ((hour + 24 - 22) % 24) as f32 * 60.0 + minute as f32;
        let dread = (gone / 300.0).clamp(0.0, 1.0);
        let params = LampParams {
            lamp: flame,
            flicker,
            dread,
            book,
            tilt,
            watch,
            _pad: [0.0; 2],
        };
        if let Some(fx) = ctx.gfx.post_effect_mut::<ShaderEffect>(self.lantern) {
            fx.set_params(&params);
        }

        // The cover is leather, not damp paper.
        ctx.gfx.set_post_effect_enabled(self.ink, self.opened);
        ctx.gfx.set_post_effect_enabled(self.cover, !self.opened);
        ctx.gfx.set_post_effect_enabled(self.tilt, self.opened);
        let scribbled = (self.now - self.sketch_from - SCRIBBLE) / SKETCH_DRIES;
        let sketch_wet = 1.0 - scribbled.clamp(0.0, 1.0);
        let params = ink::InkParams {
            sketch: [sketch_wet, 0.0, 0.0, 0.0],
            wet: [
                self.wet[0],
                self.wet[1],
                self.wet[2],
                f32::from(self.ending.is_some()),
            ],
            stains: self.stains.uniforms(self.now),
        };
        if let Some(fx) = ctx.gfx.post_effect_mut::<ShaderEffect>(self.ink) {
            fx.set_params(&params);
        }

        if self.shot.is_some() {
            self.drive_shot(ctx);
        }
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let now = ctx.time.elapsed.as_secs_f32();
        let mouse = page_point(ctx.input.mouse_pos());
        self.hot.clear();
        let mut p = frame.painter(&mut ctx.gfx);
        Self::draw_desk(&mut p);
        if !self.opened {
            self.draw_cover(&mut p, now);
            return;
        }
        Self::draw_paper(&mut p);
        self.draw_left(&mut p, now, mouse);
        self.draw_writing(&mut p, now, mouse);
        self.draw_watch(&mut p);
    }
}

impl LockKeeper {
    fn drive_shot(&mut self, ctx: &mut Context) {
        let frame = ctx.time.frame;
        // LK_SHOT_FRAME catches an animation part way; the default waits for it to settle.
        let shot_at = std::env::var("LK_SHOT_FRAME").map_or(260, |v| v.parse().unwrap_or(260));
        if frame == 20
            && self
                .shot
                .as_ref()
                .is_some_and(|s| !s.0.ends_with("cover.png"))
        {
            self.begin();
            for cmd in self.shot.as_ref().map(|s| s.1.clone()).unwrap_or_default() {
                self.perform(Action::Text(cmd));
            }
            self.book.finish();
        }
        if frame == shot_at
            && let Some((file, _)) = &self.shot
        {
            ctx.gfx.request_screenshot(file.as_str());
        }
        if frame == shot_at + 10 {
            ctx.quit();
        }
    }
}

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("--ifid") => {
            println!("{}", ifiction_meta::ifid());
            return Ok(());
        }
        Some("--ifiction") => {
            print!("{}", ifiction_meta::ifiction_xml());
            return Ok(());
        }
        _ => {}
    }
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    run::<LockKeeper>(AppConfig {
        title: ifiction_meta::TITLE.into(),
        width: 1280,
        height: 720,
        virtual_resolution: Some((W as u32, H as u32)),
        scale_mode: ScaleMode::Fit,
        scale_filter: ScaleFilter::Linear,
        border_color: Color::rgb8(8, 6, 4),
        canvas_id: Some("lock-keeper".into()),
        // Everything is compiled in; there is no asset folder to preload.
        web_preload: Some(Vec::new()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tilt_moves_clicks_by_a_few_pixels_at_most() {
        assert_eq!(page_point((W / 2.0, H / 2.0)), [W / 2.0, H / 2.0]);
        let corner = page_point((1160.0, 690.0));
        assert!((corner[0] - 1160.0).abs() < 4.0 && (corner[1] - 690.0).abs() < 4.0);
        assert_ne!(corner, [1160.0, 690.0]);
    }

    #[test]
    fn the_candle_stays_in_range_and_moves() {
        let values: Vec<f32> = (0..2000)
            .map(|i| candle_flicker(i as f32 * 0.016))
            .collect();
        assert!(values.iter().all(|v| (-1.0..=1.0).contains(v)));
        let spread = values.iter().cloned().fold(f32::MIN, f32::max)
            - values.iter().cloned().fold(f32::MAX, f32::min);
        assert!(spread > 0.5, "flat flame: {spread}");
        // Frame to frame it must not jump: this is light on a whole screen.
        assert!(values.windows(2).all(|w| (w[0] - w[1]).abs() < 0.3));
    }
}
