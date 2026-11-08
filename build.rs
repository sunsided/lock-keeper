//! Compiles the story into one JSON bundle and generates the Treaty of Babel metadata, so the
//! binary needs no files next to it.

use loremaster_project::{Severity, check_files, read_dir};
use std::path::Path;

const STORY: &str = "story";

fn main() {
    println!("cargo:rerun-if-changed={STORY}");
    let files = read_dir(Path::new(STORY)).expect("read the story folder");
    let checked = check_files(files);
    for d in &checked.diagnostics {
        println!("cargo:warning={d}");
    }
    let project = match checked.project {
        Some(p)
            if !checked
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error) =>
        {
            p
        }
        _ => panic!("the story has errors"),
    };
    let json = serde_json::to_string(&(&project.story, &project.walkthroughs)).unwrap();
    let out = std::env::var("OUT_DIR").unwrap();
    std::fs::write(Path::new(&out).join("story.json"), json).unwrap();

    if let Err(e) = ifiction_build::generate() {
        panic!("{e}");
    }
}
