#![cfg(test)]
//! The memory view opens each session of the home once, and answers what it
//! answered when the memories and the context given last were each read
//! from every session on their own.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use lys_home::harness::claude_code::given::{
    ConfigDir, ConfigSource, DocumentKind, GivenDocument, Resolution,
};
use lys_home::{Entry, EntryBase, EntryBody, GivenRecord, Home, last_given, light, recall_all};
use serde_json::json;

use super::{MemoryView, VisibleTo, given, line, remembered, skipped};

type TestResult = Result<(), Box<dyn Error>>;

const SESSIONS: usize = 20;

fn record() -> GivenRecord {
    GivenRecord::claude_code(
        Resolution {
            config_dir: ConfigDir {
                path: PathBuf::from("/c"),
                source: ConfigSource::Template,
            },
            documents: vec![GivenDocument {
                kind: DocumentKind::AppendedInstructions,
                path: PathBuf::from("instructions.md"),
                length: 21,
                sha256: "ab".repeat(32),
            }],
        },
        vec!["HOME".to_owned(), "PATH".to_owned()],
    )
}

/// A home of `SESSIONS` sessions, each holding one message, one given entry
/// under that message and one lantern on it.
fn home(root: &Path) -> Result<Home, Box<dyn Error>> {
    let home = Home::open(root)?;
    for n in 0..SESSIONS {
        let id = format!("session-{n:02}");
        {
            let mut session = home.create_session(&id, "/w", None)?;
            session.append_entry(&Entry {
                base: EntryBase {
                    id: "m1".to_owned(),
                    parent_id: None,
                    timestamp: format!("2026-01-01T00:00:{n:02}.000Z"),
                },
                body: EntryBody::Message {
                    message: json!({
                        "role": "user",
                        "content": [{ "type": "text", "text": "said" }],
                        "timestamp": 0
                    }),
                },
            })?;
            record().append_under(&mut session, "m1")?;
        }
        light(&home, &id, "m1", &format!("note {n}"), "the-lighter")?;
    }
    Ok(home)
}

fn empty(agent: &str) -> MemoryView {
    MemoryView {
        agent: agent.to_owned(),
        home: false,
        memories: Vec::new(),
        skipped: Vec::new(),
        last_given: None,
        visible_to: VisibleTo {
            agent: agent.to_owned(),
            responsible: None,
            administrator: true,
        },
        notes_shown: false,
    }
}

#[test]
fn the_memory_view_opens_each_session_once() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let root = dir.path().join("agent-a");
    let home = home(&root)?;

    let mut opens: BTreeMap<String, u32> = BTreeMap::new();
    let view = remembered(empty("agent-a"), &root, |home, session| {
        *opens.entry(session.to_owned()).or_default() += 1;
        home.read_session(session)
    })?;
    assert_eq!(opens.len(), SESSIONS, "every session of the home is read");
    assert!(
        opens.values().all(|count| *count == 1),
        "each session is opened once: {opens:?}"
    );
    assert_eq!(view.memories.len(), SESSIONS);

    let recalled = recall_all(&home)?;
    let last = last_given(&home)?;
    let mut before = empty("agent-a");
    before.home = true;
    before.memories = recalled.lanterns.into_iter().map(line).collect();
    before.skipped = skipped(recalled.skipped, last.skipped, &root);
    before.last_given = last.last.map(given);
    assert_eq!(
        serde_json::to_value(&view)?,
        serde_json::to_value(&before)?,
        "the view answers what reading every session twice answered"
    );
    Ok(())
}
