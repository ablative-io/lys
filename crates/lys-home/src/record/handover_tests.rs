#![cfg(test)]
//! Gates on the handover (HOME-015): the refusals by name, the canon's
//! rule, the letter taken from the outgoing session and refused before
//! anything is written, the successor home written with the letter copied
//! whole, the `handover` subcommand, and the successor rendered for its own
//! model and for another. The fixture home holds one session `outgoing`
//! whose root-to-head path is `u1`, `a1`, `a2`, `u2`, `a3`, `x1`, with `b1`
//! a side branch off `a1`. No test name carries a content sentinel.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::error::HomeError;
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::canon::{Inherited, add_authored, load};
use crate::record::entries::{CUSTOM_INHERITED, Entry, EntryBase, EntryBody, INHERITED_FROM};
use crate::record::reader::SessionReader;

type Gate = Result<(), Box<dyn Error>>;

/// The fixture's outgoing session.
const OUTGOING: &str = "outgoing";
fn text(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

fn thinking(id: &str) -> Value {
    json!({"type": "thinking", "thinking": format!("fixture-thinking-{id}"),
        "thinkingSignature": format!("fixture-signature-{id}")})
}

fn entry(id: &str, parent: Option<&str>, message: Value) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            timestamp: format!("fixture-time-{id}"),
        },
        body: EntryBody::Message { message },
    }
}

fn user(id: &str, parent: Option<&str>) -> Entry {
    let message = json!({"role": "user", "content": [text(&format!("fixture-text-{id}"))]});
    entry(id, parent, message)
}

fn assistant(id: &str, parent: &str, part: Value, who: [&str; 3]) -> Entry {
    let [provider, api, model] = who;
    let message = json!({"role": "assistant", "content": [part], "provider": provider,
        "api": api, "model": model});
    entry(id, Some(parent), message)
}

const ANTHROPIC: [&str; 3] = ["anthropic", "anthropic-messages", "claude-fixture-model"];

/// A fresh directory holding the fixture home at `home/`; returns the
/// directory and the home's root.
fn fixture() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    let mut s = home.create_session(OUTGOING, "/work/outgoing", None)?;
    let body = |id: &str| text(&format!("fixture-text-{id}"));
    s.append_entry(&user("u1", None))?;
    s.append_entry(&assistant("a1", "u1", thinking("a1"), ANTHROPIC))?;
    s.append_entry(&assistant("a2", "a1", body("a2"), ANTHROPIC))?;
    s.append_entry(&user("u2", Some("a2")))?;
    s.append_entry(&assistant("a3", "u2", body("a3"), ANTHROPIC))?;
    s.append_entry(&assistant("b1", "a1", thinking("b1"), ANTHROPIC))?;
    let authored = ["authored"; 3];
    s.append_entry(&assistant("x1", "a3", body("x1"), authored))?;
    Ok((dir, root))
}

/// The rule-less data R2 names.
fn ruleless(rule: Option<&str>) -> Inherited {
    Inherited {
        authored: false,
        from_session: Some("s".into()),
        from_entries: vec!["e".into()],
        provider: "p".into(),
        api: "a".into(),
        model: "m".into(),
        curated_at: "t".into(),
        curated_by: "b".into(),
        rule: rule.map(str::to_owned),
    }
}

fn inherited_entry(id: &str) -> Result<Entry, Box<dyn Error>> {
    Ok(Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: None,
            timestamp: "t".to_owned(),
        },
        body: EntryBody::Custom {
            custom_type: CUSTOM_INHERITED.to_owned(),
            data: Some(serde_json::to_value(ruleless(None))?),
        },
    })
}

/// A canon file of a header line and one rule-less `lys.inherited` entry `k1`.
fn ruleless_canon(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("canon.jsonl");
    let header =
        json!({"type": "session", "version": 2, "id": "canon", "timestamp": "t", "cwd": "/canon"});
    let entry = serde_json::to_string(&inherited_entry("k1")?)?;
    std::fs::write(&path, format!("{header}\n{entry}\n"))?;
    Ok(path)
}

fn sha(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(Hash::of(&std::fs::read(path)?).to_string())
}

#[test]
fn each_refusal_begins_with_its_name_and_names_what_it_refuses() {
    let named = |e: &HomeError, name: &str, parts: &[&str]| {
        let shown = e.to_string();
        assert!(shown.starts_with(name), "{shown}");
        for part in parts {
            assert!(shown.contains(part), "{shown} lacks {part}");
        }
    };
    let session = || OUTGOING.to_owned();
    named(
        &HomeError::LetterNotAssistant {
            session: session(),
            id: "u1".into(),
        },
        "letter_not_assistant",
        &["u1"],
    );
    named(
        &HomeError::LetterNotContiguous {
            session: session(),
            id: "a3".into(),
        },
        "letter_not_contiguous",
        &["a3"],
    );
    named(
        &HomeError::LetterAuthored {
            session: session(),
            id: "x1".into(),
        },
        "letter_authored",
        &["x1"],
    );
    named(
        &HomeError::LetterWithoutThinking {
            session: session(),
            ids: vec!["a3".into(), "a4".into()],
        },
        "letter_without_thinking",
        &["a3", "a4"],
    );
    named(
        &HomeError::SuccessorNotEmpty {
            path: PathBuf::from("successor-dir"),
        },
        "successor_not_empty",
        &["successor-dir"],
    );
    named(
        &HomeError::CanonExampleWithoutRule { id: "k1".into() },
        "canon_example_without_rule",
        &["k1", "`rule`"],
    );
    assert_eq!(INHERITED_FROM, "inherited from ");
}

#[test]
fn an_inherited_rule_is_written_when_present_and_skipped_when_absent() -> Gate {
    assert_eq!(
        serde_json::to_string(&ruleless(Some("r")))?,
        r#"{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b","rule":"r"}"#
    );
    assert_eq!(
        serde_json::to_string(&ruleless(None))?,
        r#"{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b"}"#
    );
    Ok(())
}

#[test]
fn the_canon_refuses_an_example_without_a_rule_on_load_add_and_render() -> Gate {
    let (dir, root) = fixture()?;
    let canon = ruleless_canon(dir.path())?;
    let refused =
        |r: &HomeError| matches!(r, HomeError::CanonExampleWithoutRule { id } if id == "k1");
    let loaded = load(&canon);
    assert!(matches!(&loaded, Err(e) if refused(e)), "{loaded:?}");

    let turns = dir.path().join("turns.txt");
    std::fs::write(&turns, "user: q\nassistant: a\n")?;
    let before = sha(&canon)?;
    let added = add_authored(&canon, &turns, "r", "b");
    assert!(matches!(&added, Err(e) if refused(e)), "{added:?}");
    assert_eq!(sha(&canon)?, before);

    let home = Home::open(&root)?;
    let session = home.open_session(OUTGOING)?;
    let out = dir.path().join("rendered.jsonl");
    let target = RenderTarget {
        session_id: "render-target".into(),
        cwd: "/work/outgoing".into(),
        model: "claude-fixture-model".into(),
        version: "2.1.281".into(),
        out: Some(out.clone()),
        canon: Some(canon),
    };
    let rendered = render_claude_code(&session, &target, None);
    assert!(matches!(&rendered, Err(e) if refused(e)), "{rendered:?}");
    assert!(!out.exists());
    Ok(())
}

#[test]
fn a_ruleless_inherited_entry_in_a_session_of_a_home_reads_cleanly() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("inheritor", "/work", None)?;
    session.append_entry(&inherited_entry("h1")?)?;
    drop(session);
    let reader = SessionReader::open(home.session_path("inheritor")?)?;
    let EntryBody::Custom {
        data: Some(data), ..
    } = reader.entry("h1")?.body
    else {
        return Err("h1 is not a custom entry with data".into());
    };
    let inherited: Inherited = serde_json::from_value(data)?;
    assert_eq!(inherited.rule, None);
    Ok(())
}
