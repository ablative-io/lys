//! A lantern lives in the home (HOME-004 R7): the rendered Claude Code file
//! of a session holding lanterns and epilogues carries no lantern line, no
//! note and no epilogue, and has exactly as many lines as the render of the
//! same session before anything was lit. The renderer is not changed; this
//! pins that custom entries stay out of it.

use std::error::Error;
use std::path::{Path, PathBuf};

use lys_home::harness::claude_code::render::{RenderTarget, render_claude_code};
use lys_home::{Entry, EntryBase, EntryBody, Home, add_epilogue, light};

const FIXTURE: &str = "fixture-lantern";
const NOTE: &str = "The Fold Held Under Replay";

type Gate = Result<(), Box<dyn Error>>;

/// A home holding `fixture-lantern` with five message entries e1 to e5.
fn fixture_home(dir: &Path) -> Result<Home, Box<dyn Error>> {
    let home = Home::open(dir.join("home"))?;
    five_messages(&home, FIXTURE)?;
    Ok(home)
}

/// A session of the home with five message entries e1 to e5, its id in its
/// header and its file name alike.
fn five_messages(home: &Home, id: &str) -> Result<(), Box<dyn Error>> {
    let mut session = home.create_session(id, "/fixture", None)?;
    let mut prev: Option<String> = None;
    for id in ["e1", "e2", "e3", "e4", "e5"] {
        session.append_entry(&Entry {
            base: EntryBase {
                id: id.to_owned(),
                parent_id: prev.take(),
                timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            },
            body: EntryBody::Message {
                message: serde_json::json!({"role": "user", "content": [{"type": "text", "text": "fixture"}], "timestamp": 0}),
            },
        })?;
        prev = Some(id.to_owned());
    }
    Ok(())
}

fn render(home: &Home, session: &str, out: PathBuf) -> Result<String, Box<dyn Error>> {
    let session = home.open_session(session)?;
    let target = RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
        cwd: "/elsewhere".to_owned(),
        model: "claude-fixture".to_owned(),
        version: "2.1.281".to_owned(),
        out: Some(out),
        canon: None,
    };
    let report = render_claude_code(&session, &target, None)?;
    Ok(std::fs::read_to_string(report.path)?)
}

#[test]
fn a_rendered_file_carries_no_lantern_and_as_many_lines_as_before_lighting() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    // The same five entries as their own session, taken before anything is lit.
    let unlit = "fixture-unlit";
    five_messages(&home, unlit)?;
    let l1 = light(&home, FIXTURE, "e2", NOTE, "fixture-lighter")?.id;
    light(&home, FIXTURE, "e2", "second look", "fixture-lighter")?;
    add_epilogue(
        &home,
        FIXTURE,
        &l1,
        "and the replay fold rings true",
        "fixture-annotator",
    )?;
    add_epilogue(
        &home,
        FIXTURE,
        &l1,
        "a later word: cobalt",
        "fixture-annotator",
    )?;
    assert_eq!(home.read_session(FIXTURE)?.len(), 9);

    let lit = render(&home, FIXTURE, dir.path().join("lit.jsonl"))?;
    let before = render(&home, unlit, dir.path().join("unlit.jsonl"))?;
    assert_eq!(lit.lines().filter(|l| l.contains("lys.lantern")).count(), 0);
    assert_eq!(lit.lines().filter(|l| l.contains(NOTE)).count(), 0);
    assert_eq!(lit.lines().filter(|l| l.contains("cobalt")).count(), 0);
    assert_eq!(lit.lines().count(), before.lines().count());
    assert_eq!(lit.lines().count(), 5);
    Ok(())
}
