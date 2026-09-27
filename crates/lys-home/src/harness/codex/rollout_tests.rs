//! Gates on the rollout (HOME-009 R4): Codex's own layout and file name,
//! `session_meta` of five keys, the marker opening the thread, the thread id
//! derived from the head, ordinals and stamps as the entries record them,
//! compactions, branch summaries and custom messages as marked developer
//! text, entries a compaction left behind listed lost, and an unparsed stamp
//! refused with nothing written.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use tempfile::TempDir;

use crate::error::HomeError;
use crate::harness::claude_code::render::record_uuid;
use crate::harness::codex::account::part_hash;
use crate::harness::codex::rollout::{Translation, translate};
use crate::record::Home;
use crate::record::entries::{Entry, EntryBase, EntryBody};

type Gate = Result<(), Box<dyn Error>>;

const HEAD: &str = "11111111-1111-4111-8111-111111111111";
const STAMP: &str = "2000-01-02T03:04:05.678Z";

pub(crate) fn entry(id: &str, parent: Option<&str>, stamp: &str, body: EntryBody) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            timestamp: stamp.to_owned(),
        },
        body,
    }
}

pub(crate) fn said(role: &str, text: &str) -> EntryBody {
    EntryBody::Message {
        message: json!({"role": role, "content": [{"type": "text", "text": text}]}),
    }
}

fn compaction(first_kept: &str) -> EntryBody {
    EntryBody::Compaction {
        summary: "fixture summary".to_owned(),
        first_kept_entry_id: first_kept.to_owned(),
        tokens_before: 0,
        rest: Map::new(),
    }
}

/// A home in a temporary directory holding one session of these entries.
pub(crate) fn home_with(
    session: &str,
    entries: &[Entry],
) -> Result<(TempDir, Home), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut s = home.create_session(session, "/w", None)?;
    for e in entries {
        s.append_entry(e)?;
    }
    Ok((dir, home))
}

pub(crate) fn run(home: &Home, session: &str, out: &Path) -> Result<Translation, HomeError> {
    let mut s = home.open_session(session)?;
    translate(&mut s, out, "0.156.0", Some("Australia/Sydney"))
}

pub(crate) fn lines(path: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        out.push(serde_json::from_str(line)?);
    }
    Ok(out)
}

pub(crate) fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .iter()
        .flat_map(|object| object.keys().cloned())
        .collect();
    keys.sort();
    keys
}

/// The text of a message line's one part.
pub(crate) fn text_of(line: &Value) -> Option<&str> {
    line["payload"]["content"][0]["text"].as_str()
}

fn one_head() -> Result<(TempDir, Home), Box<dyn Error>> {
    home_with("s1", &[entry(HEAD, None, STAMP, said("user", "hello"))])
}

#[test]
fn path_follows_codex_layout() -> Gate {
    let (dir, home) = one_head()?;
    let out = dir.path().join("o");
    let done = run(&home, "s1", &out)?;
    let day = out.join("sessions").join("2000").join("01").join("02");
    let rollout: PathBuf =
        day.join("rollout-2000-01-02T14-04-05-11111111-1111-4111-8111-111111111111.jsonl");
    let account =
        day.join("rollout-2000-01-02T14-04-05-11111111-1111-4111-8111-111111111111.loss.json");
    assert_eq!(done.rollout, rollout);
    assert_eq!(done.account, account);
    assert!(rollout.is_file());
    assert!(account.is_file());
    Ok(())
}

#[test]
fn session_meta_holds_five_keys() -> Gate {
    let (dir, home) = one_head()?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    assert_eq!(lines[0]["type"], "session_meta");
    assert_eq!(
        sorted_keys(&lines[0]["payload"]),
        ["cli_version", "cwd", "id", "session_id", "timestamp"]
    );
    assert_eq!(lines[0]["payload"]["cwd"], "/w");
    assert_eq!(lines[0]["payload"]["cli_version"], "0.156.0");
    let text = std::fs::read_to_string(&done.rollout)?;
    for key in [
        "model_provider",
        "thread_source",
        "history_mode",
        "originator",
    ] {
        assert!(!text.contains(key), "{key}");
    }
    Ok(())
}

#[test]
fn marker_opens_the_thread() -> Gate {
    let (dir, home) = one_head()?;
    let head_hash = home.open_session("s1")?.head_hash()?.to_string();
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    assert_eq!(lines[1]["type"], "response_item");
    assert_eq!(lines[1]["payload"]["type"], "message");
    assert_eq!(lines[1]["payload"]["role"], "developer");
    assert_eq!(
        lines[1]["payload"]["content"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(lines[1]["payload"]["content"][0]["type"], "input_text");
    let marker = format!(
        "<TRANSLATED CONTEXT: THIS CODEX THREAD IS A FORK OF HOME SESSION s1 AT HEAD HASH {head_hash}, RENDERED FOR CODEX 0.156.0, NOT THAT SESSION>"
    );
    assert_eq!(text_of(&lines[1]), Some(marker.as_str()));
    let account = read_json(&done.account)?;
    assert_eq!(account["session"], "s1");
    assert_eq!(account["head_hash"], json!(head_hash));
    assert_eq!(account["codex_version"], "0.156.0");
    Ok(())
}

#[test]
fn thread_id_is_record_uuid_of_head() -> Gate {
    let (dir, home) = home_with("s1", &[entry("h-r0", None, STAMP, said("user", "hello"))])?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let thread = record_uuid("s1", "h-r0");
    assert_eq!(done.thread, thread);
    assert_eq!(thread.as_bytes()[14], b'5');
    let name = done
        .rollout
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned);
    assert_eq!(
        name,
        Some(format!("rollout-2000-01-02T14-04-05-{thread}.jsonl"))
    );
    let lines = lines(&done.rollout)?;
    assert_eq!(lines[0]["payload"]["id"], json!(thread));
    assert_eq!(lines[0]["payload"]["session_id"], json!(thread));
    assert_eq!(read_json(&done.account)?["thread"], json!(thread));
    Ok(())
}

#[test]
fn ordinal_is_the_line_number() -> Gate {
    let (dir, home) = home_with(
        "s1",
        &[
            entry("u1", None, STAMP, said("user", "one")),
            entry("a1", Some("u1"), STAMP, said("assistant", "two")),
            entry("u2", Some("a1"), STAMP, said("user", "three")),
        ],
    )?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    assert_eq!(lines.len(), 5);
    for (n, line) in lines.iter().enumerate() {
        assert_eq!(line["ordinal"], json!(n));
        assert!(line["type"] == "session_meta" || line["type"] == "response_item");
        assert_eq!(
            sorted_keys(line),
            ["ordinal", "payload", "timestamp", "type"]
        );
    }
    Ok(())
}

#[test]
fn line_stamps_are_the_entries_own() -> Gate {
    let (dir, home) = home_with(
        "s1",
        &[
            entry("u1", None, "2000-01-01T00:00:01Z", said("user", "one")),
            entry(
                "c1",
                Some("u1"),
                "2000-01-01T00:00:02.5+00:00",
                compaction("u1"),
            ),
            entry(
                "a1",
                Some("c1"),
                "2000-01-01T10:00:03+10:00",
                said("assistant", "two"),
            ),
        ],
    )?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    let head = "2000-01-01T10:00:03+10:00";
    assert_eq!(lines[0]["timestamp"], head);
    assert_eq!(lines[1]["timestamp"], head);
    assert_eq!(lines[0]["payload"]["timestamp"], head);
    let stamp_of = |text: &str| {
        lines
            .iter()
            .find(|line| text_of(line).is_some_and(|t| t.contains(text)))
            .map(|line| line["timestamp"].clone())
    };
    assert_eq!(stamp_of("one"), Some(json!("2000-01-01T00:00:01Z")));
    assert_eq!(
        stamp_of("<COMPACTION SUMMARY c1>"),
        Some(json!("2000-01-01T00:00:02.5+00:00"))
    );
    assert_eq!(stamp_of("two"), Some(json!(head)));
    Ok(())
}

#[test]
fn compaction_is_marked_text() -> Gate {
    let (dir, home) = home_with(
        "s1",
        &[
            entry("c1", None, STAMP, compaction("c1")),
            entry("u1", Some("c1"), STAMP, said("user", "after")),
        ],
    )?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    let marked: Vec<&Value> = lines
        .iter()
        .filter(|line| text_of(line).is_some_and(|t| t.starts_with("<COMPACTION SUMMARY c1>\n")))
        .collect();
    assert_eq!(marked.len(), 1);
    assert_eq!(marked[0]["payload"]["role"], "developer");
    assert_eq!(
        text_of(marked[0]),
        Some("<COMPACTION SUMMARY c1>\nfixture summary")
    );
    let account = read_json(&done.account)?;
    let rows: Vec<&Value> = account["changed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["entry"] == "c1")
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["hash"], Value::Null);
    assert_eq!(rows[0]["before"], "compaction");
    assert_eq!(rows[0]["after"], "marked text");
    assert_eq!(
        rows[0]["how"],
        "carried as developer text under the COMPACTION SUMMARY marker; firstKeptEntryId not carried; tokensBefore not carried"
    );
    Ok(())
}

#[test]
fn entries_left_by_a_compaction_are_lost() -> Gate {
    let label = EntryBody::Label {
        target_id: "u1".to_owned(),
        label: Some("note".to_owned()),
    };
    let (dir, home) = home_with(
        "s1",
        &[
            entry("u1", None, STAMP, said("user", "gone-one")),
            entry("a1", Some("u1"), STAMP, said("assistant", "gone-two")),
            entry("l1", Some("a1"), STAMP, label),
            entry("c1", Some("l1"), STAMP, compaction("c1")),
            entry("u2", Some("c1"), STAMP, said("user", "kept")),
        ],
    )?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let text = std::fs::read_to_string(&done.rollout)?;
    assert!(!text.contains("gone-one") && !text.contains("gone-two"));
    assert!(text.contains("kept"));
    let account = read_json(&done.account)?;
    let lost: Vec<&Value> = account["lost"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| ["u1", "a1", "l1"].contains(&row["entry"].as_str().unwrap_or("-")))
        .collect();
    assert_eq!(lost.len(), 3);
    let kinds: Vec<&Value> = lost.iter().map(|row| &row["kind"]).collect();
    assert_eq!(
        kinds,
        [&json!("message"), &json!("message"), &json!("label")]
    );
    for row in lost {
        assert_eq!(row["hash"], Value::Null);
        assert_eq!(row["reason"], "left off the context path by compaction c1");
    }
    Ok(())
}

#[test]
fn branch_summary_and_custom_message_are_marked_text() -> Gate {
    let image = json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}});
    let branch = EntryBody::BranchSummary {
        from_id: "u1".to_owned(),
        summary: "branch words".to_owned(),
        rest: Map::new(),
    };
    let custom = EntryBody::CustomMessage {
        custom_type: "ext".to_owned(),
        content: json!([{"type": "text", "text": "hi"}, image]),
        rest: Map::new(),
    };
    let (dir, home) = home_with(
        "s1",
        &[
            entry("u1", None, STAMP, said("user", "one")),
            entry("b1", Some("u1"), STAMP, branch),
            entry("m1", Some("b1"), STAMP, custom),
        ],
    )?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let lines = lines(&done.rollout)?;
    let developer = |prefix: &str| {
        lines
            .iter()
            .filter(|line| line["payload"]["role"] == "developer")
            .filter_map(text_of)
            .find(|text| text.starts_with(prefix))
            .map(str::to_owned)
    };
    assert!(developer("<BRANCH SUMMARY b1>\n").is_some());
    assert_eq!(
        developer("<CUSTOM MESSAGE m1>").as_deref(),
        Some("<CUSTOM MESSAGE m1>\nhi")
    );
    let account = read_json(&done.account)?;
    let changed = |id: &str| {
        account["changed"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| row["entry"] == id)
            .cloned()
            .collect::<Vec<Value>>()
    };
    let b1 = changed("b1");
    assert_eq!(b1.len(), 1);
    assert_eq!(b1[0]["before"], "branch_summary");
    assert_eq!(b1[0]["after"], "marked text");
    assert_eq!(
        b1[0]["how"],
        "carried as developer text under the BRANCH SUMMARY marker; fromId not carried"
    );
    let m1 = changed("m1");
    assert_eq!(m1.len(), 1);
    assert_eq!(m1[0]["before"], "custom_message");
    assert_eq!(m1[0]["after"], "marked text");
    assert_eq!(
        m1[0]["how"],
        "carried as developer text under the CUSTOM MESSAGE marker; customType not carried"
    );
    let lost: Vec<&Value> = account["lost"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["hash"] == json!(part_hash(&image)))
        .collect();
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0]["kind"], "image");
    assert_eq!(
        lost[0]["reason"],
        "image part 1 not carried: marked text holds text only"
    );
    let text = serde_json::to_string(&account)?;
    assert!(!text.contains("not conversation"));
    Ok(())
}

#[test]
fn unparsed_stamp_is_refused_and_nothing_written() -> Gate {
    let (dir, home) = home_with(
        "s1",
        &[
            entry("e1", None, STAMP, said("user", "one")),
            entry("e2", Some("e1"), "yesterday", said("assistant", "two")),
        ],
    )?;
    let out = dir.path().join("o");
    let refused = run(&home, "s1", &out);
    assert_eq!(
        refused.err().map(|e| e.to_string()).as_deref(),
        Some("entry e2 has a stamp that is not RFC 3339: re-import the source file")
    );
    assert!(!out.exists() || std::fs::read_dir(&out)?.next().is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn failed_write_leaves_nothing_and_a_retry_translates() -> Gate {
    let (dir, home) = one_head()?;
    let out = dir.path().join("o");
    let day = out.join("sessions").join("2000").join("01").join("02");
    let account =
        day.join("rollout-2000-01-02T14-04-05-11111111-1111-4111-8111-111111111111.loss.json");
    std::fs::create_dir_all(&day)?;
    // A dangling link passes the check before writing, then refuses the
    // account's create, after the rollout is already written.
    std::os::unix::fs::symlink(dir.path().join("nowhere"), &account)?;
    let refused = run(&home, "s1", &out);
    assert!(
        matches!(&refused, Err(HomeError::TranslationTargetExists { path }) if *path == account),
        "{refused:?}"
    );
    let mut left = Vec::new();
    for found in std::fs::read_dir(&day)? {
        left.push(found?.path());
    }
    assert_eq!(left, [account.as_path()]);
    let leaves = home
        .open_session("s1")?
        .customs_everywhere(crate::record::entries::CUSTOM_TRANSLATION)?;
    assert!(leaves.is_empty());
    std::fs::remove_file(&account)?;
    let done = run(&home, "s1", &out)?;
    assert!(done.rollout.is_file());
    assert_eq!(done.account, account);
    assert!(account.is_file());
    Ok(())
}
