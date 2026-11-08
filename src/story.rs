//! The story bundle compiled by `build.rs`.

use loremaster_model::{Story, Walkthrough};
use loremaster_runtime::{HintBook, Session, World};
use std::sync::Arc;

const BUNDLE: &str = include_str!(concat!(env!("OUT_DIR"), "/story.json"));

/// A new game of the embedded story, with its hint ladder.
pub fn new_session() -> Session {
    let (story, walks): (Story, Vec<(String, Walkthrough)>) =
        serde_json::from_str(BUNDLE).expect("the bundle was written by build.rs");
    let world = Arc::new(World::compile(story).expect("build.rs checked the story"));
    let mut session = Session::new(world.clone());
    session.set_hints(HintBook::new(world, &walks));
    session
}

/// The state of a machine or story variable, as text (for example the lock: `full`).
pub fn fact(session: &Session, id: &str) -> String {
    let snapshot = session.snapshot();
    let value = snapshot.state().facts.get(id);
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use loremaster_runtime::{Effect, Input};

    /// A click on a thing's name examines it by that name, so the parser has to know each one.
    #[test]
    fn every_thing_can_be_examined_by_its_name() {
        let mut s = new_session();
        s.start();
        for go in [
            "", "w", "w", "e", "up", "w", "e", "down", "e", "s", "n", "n", "e",
        ] {
            if !go.is_empty() {
                s.submit(Input::Text(go.into()));
            }
            let scene = s.scene();
            for thing in scene.things.iter().chain(&scene.inventory) {
                let name = thing.name.to_lowercase();
                let out = s
                    .submit(Input::Text(format!("examine {name}")))
                    .plain_text();
                assert!(
                    !out.contains("You see no") && !out.contains("understand"),
                    "examine {name} in {}: {out}",
                    scene.room
                );
            }
        }
    }

    #[test]
    fn the_bundle_plays() {
        let mut s = new_session();
        let step = s.start();
        assert!(step.plain_text().contains("Amos Reed"));
        let links = step.effects.iter().filter(
            |e| matches!(e, Effect::Text { block } if block.runs.iter().any(|r| r.link.is_some())),
        );
        println!("links: {}, time: {:?}", links.count(), s.scene().time);
        assert_eq!(fact(&s, "lock"), "full");
        let step = s.submit(Input::Text("w".into()));
        assert!(step.plain_text().contains("Kitchen"));
    }
}
