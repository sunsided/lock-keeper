//! The written page: word wrap onto ruled rows, and how far the pen has got.

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Text(String),
    Choice(usize),
    /// A verb in the writing: starts a command and waits for its noun.
    Verb(String),
    /// A thing or exit in the writing: completes the command, or is examined or walked to.
    Noun(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ink {
    Prose,
    Title,
    Speech,
    Said,
    Note,
    Pencil,
    Choice,
}

/// A stretch of text to write, before wrapping.
pub struct Piece {
    pub text: String,
    pub ink: Ink,
    pub action: Option<Action>,
}

/// Can a verb stand after `prev`? Prose uses "lock" and "open" as nouns and adjectives too, and
/// those are not commands. `prev` is empty at the start of a sentence.
fn verb_position(prev: &str) -> bool {
    matches!(
        prev,
        "" | "you" | "to" | "can" | "could" | "must" | "might" | "should"
    )
}

/// The words of a thing's name to mark in the writing: a person's proper names, or else the head
/// noun, so "pair of boots" marks only "boots". Lowercase.
pub fn main_words(name: &str) -> Vec<String> {
    let proper: Vec<_> = name
        .split(' ')
        .filter(|w| w.starts_with(char::is_uppercase))
        .map(str::to_lowercase)
        .collect();
    if proper.is_empty() {
        name.rsplit(' ').take(1).map(str::to_lowercase).collect()
    } else {
        proper
    }
}

/// Split `text` so the words `lookup` knows carry their action, without their punctuation.
/// `lookup` gets the lowercase word, and whether it stands where a verb can.
pub fn mark_words(
    text: &str,
    ink: Ink,
    lookup: &dyn Fn(&str, bool) -> Option<Action>,
) -> Vec<Piece> {
    let piece = |text: &str, action| Piece {
        text: text.to_owned(),
        ink,
        action,
    };
    let mut out = Vec::new();
    let mut prev = String::new();
    for word in text.split_inclusive(' ') {
        let core = word.trim_matches(|c: char| !c.is_alphanumeric());
        let lower = core.to_lowercase();
        match lookup(&lower, verb_position(&prev)) {
            Some(action) if !core.is_empty() => {
                let at = word.find(core).unwrap_or(0);
                if at > 0 {
                    out.push(piece(&word[..at], None));
                }
                out.push(piece(core, Some(action)));
                out.push(piece(&word[at + core.len()..], None));
            }
            _ => out.push(piece(word, None)),
        }
        let ends_sentence = word.trim_end().ends_with(['.', '!', '?', ':']);
        prev = if ends_sentence { String::new() } else { lower };
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub ink: Ink,
    pub action: Option<Action>,
    pub x: f32,
    pub w: f32,
}

#[derive(Debug, Default)]
pub struct Line {
    pub spans: Vec<Span>,
    /// How many characters the pen wrote before this line.
    pub start: usize,
    /// Seconds into the game when the line was laid out, for the ink drying.
    pub born: f32,
}

#[derive(Default)]
pub struct Book {
    pub lines: Vec<Line>,
    pub written: usize,
    pub shown: f32,
}

impl Book {
    pub fn blank(&mut self, now: f32) {
        if self.lines.last().is_some_and(|l| !l.spans.is_empty()) {
            self.lines.push(Line {
                start: self.written,
                born: now,
                ..Line::default()
            });
        }
    }

    /// Wrap `pieces` to `width` and append them as new lines.
    pub fn write(&mut self, pieces: &[Piece], width: f32, measure: &dyn Fn(&str) -> f32, now: f32) {
        let mut line = Line {
            start: self.written,
            born: now,
            ..Line::default()
        };
        let mut pen = 0.0;
        for piece in pieces {
            for word in piece.text.split_inclusive(' ') {
                if pen > 0.0 && pen + measure(word.trim_end()) > width {
                    self.lines.push(std::mem::take(&mut line));
                    line.start = self.written;
                    line.born = now;
                    pen = 0.0;
                }
                let advance = measure(word);
                match line.spans.last_mut() {
                    Some(s) if s.ink == piece.ink && s.action == piece.action => {
                        s.text.push_str(word);
                        s.w += advance;
                    }
                    _ => line.spans.push(Span {
                        text: word.to_owned(),
                        ink: piece.ink,
                        action: piece.action.clone(),
                        x: pen,
                        w: advance,
                    }),
                }
                pen += advance;
                self.written += word.chars().count();
            }
        }
        self.lines.push(line);
    }

    /// Old links and answers no longer apply after an input.
    pub fn expire(&mut self) {
        for span in self.lines.iter_mut().flat_map(|l| &mut l.spans) {
            span.action = None;
        }
    }

    pub fn advance(&mut self, chars: f32) {
        self.shown = (self.shown + chars).min(self.written as f32);
    }

    pub fn finish(&mut self) {
        self.shown = self.written as f32;
    }

    pub fn writing(&self) -> bool {
        (self.shown as usize) < self.written
    }

    /// The index after the last line that the pen has reached.
    pub fn end(&self) -> usize {
        self.lines.partition_point(|l| {
            l.start <= self.shown as usize && (l.start < self.written || l.spans.is_empty())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(text: &str, width: f32) -> Book {
        let mut b = Book::default();
        let piece = Piece {
            text: text.into(),
            ink: Ink::Prose,
            action: None,
        };
        b.write(&[piece], width, &|s| s.len() as f32, 0.0);
        b
    }

    #[test]
    fn wraps_at_words_and_counts_characters() {
        let b = book("the coroner comes at three", 12.0);
        let rows: Vec<_> = b.lines.iter().map(|l| l.spans[0].text.trim_end()).collect();
        assert_eq!(rows, ["the coroner", "comes at", "three"]);
        assert_eq!(b.written, 26);
        assert_eq!(b.lines[1].start, 12);
    }

    #[test]
    fn marks_known_words_without_their_punctuation() {
        let lookup =
            |w: &str, _: bool| (w == "stove").then(|| Action::Text("examine stove".into()));
        let pieces = mark_words("a black \"Stove,\" here", Ink::Prose, &lookup);
        let text: String = pieces.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(text, "a black \"Stove,\" here");
        let hit: Vec<_> = pieces.iter().filter(|p| p.action.is_some()).collect();
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].text, "Stove");
    }

    #[test]
    fn a_verb_word_used_as_a_noun_is_not_a_verb() {
        let lookup =
            |w: &str, verb_ok: bool| (w == "lock" && verb_ok).then(|| Action::Verb("lock".into()));
        let marked = |text: &str| {
            mark_words(text, Ink::Prose, &lookup)
                .iter()
                .any(|p| p.action.is_some())
        };
        assert!(!marked("the lock north"));
        assert!(!marked("drowned in his own lock."));
        assert!(marked("You lock the gate."));
        assert!(marked("It is late. Lock the gate."));
    }

    #[test]
    fn only_the_main_words_of_a_name_are_marked() {
        assert_eq!(main_words("pair of boots"), ["boots"]);
        assert_eq!(main_words("tool chest"), ["chest"]);
        assert_eq!(main_words("Hester Reed"), ["hester", "reed"]);
    }

    #[test]
    fn a_word_longer_than_the_page_stays_on_its_row() {
        let b = book("gangplank", 4.0);
        assert_eq!(b.lines.len(), 1);
    }

    #[test]
    fn lines_appear_as_the_pen_reaches_them() {
        let mut b = book("the coroner comes at three", 12.0);
        assert_eq!(b.end(), 1);
        b.advance(13.0);
        assert_eq!(b.end(), 2);
        b.finish();
        assert_eq!(b.end(), 3);
        assert!(!b.writing());
    }
}
