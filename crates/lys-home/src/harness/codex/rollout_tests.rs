//! Gates on the rollout (HOME-009 R4): Codex's own layout named from the
//! head's stamp in the given zone, `session_meta` with its five keys, the
//! marker first, the thread id derived from the head, ordinals that are the
//! line numbers, every line stamped with its entry's own stamp, a
//! compaction, a branch summary and a custom message carried as marked
//! text, the entries a compaction left off listed lost, and an unparsed
//! stamp refused with nothing written. No test name carries content.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::harness::claude_code::render::record_uuid;
use crate::harness::codex::account::{Account, Changed, part_hash};
use crate::harness::codex::rollout::{Translation, marker, translate};
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::{Home, Session};

pub(super) type Gate = Result<(), Box<dyn Error>>;
pub(super) type Found<T> = Result<T, Box<dyn Error>>;

pub(super) const STAMP: &str = "2026-01-01T00:00:00.000Z";

pub(super) fn entry(id: &str, parent: Option<&str>, stamp: &str, body: EntryBody) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            timestamp: stamp.to_owned(),
        },
        body,
    }
}

pub(super) fn message(value: Value) -> EntryBody {
    EntryBody::Message { message: value }
}

pub(super) fn user(id: &str, parent: Option<&str>, stamp: &str, text: &str) -> Entry {
    entry(
        id,
        parent,
        stamp,
        message(json!({"role": "user", "content": [{"type": "text", "text": text}]})),
    )
}

pub(super) fn assistant(id: &str, parent: Option<&str>, stamp: &str, text: &str) -> Entry {
    entry(
        id,
        parent,
        stamp,
        message(json!({"role": "assistant", "content": [{"type": "text", "text": text}]})),
    )
}

pub(super) fn compaction(id: &str, parent: Option<&str>, stamp: &str, kept: &str) -> Entry {
    entry(
        id,
        parent,
        stamp,
        EntryBody::Compaction {
            summary: "fixture-summary".to_owned(),
            first_kept_entry_id: kept.to_owned(),
            tokens_before: 0,
            rest: Map::new(),
        },
    )
}

/// A home under `dir` holding session `s1` with these entries appended in
/// order, the last one the head.
pub(super) fn session_of(dir: &Path, entries: &[Entry]) -> Found<(Home, Session)> {
    let home = Home::open(dir.join("home"))?;
    let mut session = home.create_session("s1", "/w", None)?;
    for e in entries {
        session.append_entry(e)?;
    }
    Ok((home, session))
}

pub(super) fn run(session: &mut Session, out: &Path) -> Found<Translation> {
    Ok(translate(session, out, "0.156.0", Some("UTC"))?)
}

pub(super) fn lines(path: &Path) -> Found<Vec<Value>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        out.push(serde_json::from_str(line)?);
    }
    Ok(out)
}

pub(super) fn account(translation: &Translation) -> Found<Account> {
    let bytes = std::fs::read(&translation.account)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// The changed rows of one entry.
pub(super) fn changed_of<'a>(account: &'a Account, entry: &str) -> Vec<&'a Changed> {
    let mut out = Vec::new();
    for row in &account.changed {
        if row.entry == entry {
            out.push(row);
        }
    }
    out
}

/// The one text of every line's payload that has one.
pub(super) fn texts_of(lines: &[Value]) -> Vec<&str> {
    let mut out = Vec::new();
    for line in lines {
        out.extend(text_of(&line["payload"]));
    }
    out
}

/// A home under `dir` holding session `s1` with one user entry `id`.
fn single(dir: &Path, id: &str) -> Found<(Home, Session)> {
    session_of(dir, &[user(id, None, STAMP, "fixture-text")])
}

/// The keys of a JSON object, in its order.
fn keys_of(value: &Value) -> Found<Vec<String>> {
    let object = value.as_object().ok_or("not an object")?;
    Ok(object.keys().cloned().collect())
}

/// The first part's text of a developer or user message payload.
pub(super) fn text_of(payload: &Value) -> Option<&str> {
    payload
        .get("content")
        .and_then(Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(|part| part.get("text"))
        .and_then(Value::as_str)
}

/// Every file under a directory, recursively.
pub(super) fn files_under(dir: &Path) -> Found<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut pending = vec![dir.to_path_buf()];
    while let Some(at) = pending.pop() {
        for item in std::fs::read_dir(&at)? {
            let path = item?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

const HEAD: &str = "11111111-1111-4111-8111-111111111111";

#[test]
fn path_follows_codex_layout() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = session_of(
        dir.path(),
        &[user(HEAD, None, "2000-01-02T03:04:05.678Z", "fixture-text")],
    )?;
    let out = dir.path().join("o");
    let translation = translate(&mut session, &out, "0.156.0", Some("Australia/Sydney"))?;
    let stem = format!("rollout-2000-01-02T14-04-05-{HEAD}");
    let day = out.join("sessions").join("2000").join("01").join("02");
    let rollout = day.join(format!("{stem}.jsonl"));
    let account = day.join(format!("{stem}.loss.json"));
    assert_eq!(translation.rollout, rollout);
    assert_eq!(translation.account, account);
    assert_eq!(files_under(&out)?, [rollout, account]);
    Ok(())
}

#[test]
fn session_meta_holds_five_keys() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = single(dir.path(), "u1")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines[0]["type"], "session_meta");
    let keys = keys_of(&lines[0]["payload"])?;
    assert_eq!(keys.join(","), "cli_version,cwd,id,session_id,timestamp");
    assert_eq!(lines[0]["payload"]["cwd"], "/w");
    assert_eq!(lines[0]["payload"]["cli_version"], "0.156.0");
    let text = std::fs::read_to_string(&translation.rollout)?;
    let mut absent = 0;
    for word in ["model_provider", "thread_source", "history_mode", "originator"] {
        assert!(!text.contains(word));
        absent += 1;
    }
    assert_eq!(absent, 4);
    Ok(())
}

#[test]
fn marker_opens_the_thread() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = single(dir.path(), "u1")?;
    let head_hash = session.head_hash()?.to_string();
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines[1]["type"], "response_item");
    assert_eq!(lines[1]["payload"]["role"], "developer");
    let parts = lines[1]["payload"]["content"].as_array();
    let parts = parts.ok_or("no content")?;
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "input_text");
    let expected = format!(
        "<TRANSLATED CONTEXT: THIS CODEX THREAD IS A FORK OF HOME SESSION s1 AT HEAD HASH {head_hash}, RENDERED FOR CODEX 0.156.0, NOT THAT SESSION>"
    );
    assert_eq!(parts[0]["text"], expected.as_str());
    assert_eq!(marker("s1", &head_hash, "0.156.0"), expected);
    let account = account(&translation)?;
    assert_eq!(account.session, "s1");
    assert_eq!(account.head_hash, head_hash);
    assert_eq!(account.codex_version, "0.156.0");
    Ok(())
}

#[test]
fn thread_id_is_record_uuid_of_head() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = single(dir.path(), "h-r0")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let thread = record_uuid("s1", "h-r0");
    assert_eq!(translation.thread, thread);
    assert_eq!(thread.chars().nth(14), Some('5'));
    let name = translation.rollout.to_string_lossy();
    assert!(name.ends_with(&format!("-{thread}.jsonl")));
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines[0]["payload"]["id"], thread.as_str());
    assert_eq!(lines[0]["payload"]["session_id"], thread.as_str());
    Ok(())
}

#[test]
fn ordinal_is_the_line_number() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = session_of(
        dir.path(),
        &[
            user("u1", None, STAMP, "fixture-text-1"),
            assistant("a1", Some("u1"), STAMP, "fixture-text-2"),
            user("u2", Some("a1"), STAMP, "fixture-text-3"),
        ],
    )?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines.len(), 5);
    for (n, line) in (0u64..).zip(&lines) {
        assert_eq!(line["ordinal"], n);
        let kind = line["type"].as_str();
        assert!(matches!(kind, Some("session_meta" | "response_item")));
    }
    Ok(())
}

#[test]
fn line_stamps_are_the_entries_own() -> Gate {
    let dir = tempfile::tempdir()?;
    let head = entry(
        "a1",
        Some("c1"),
        "2000-01-01T10:00:03+10:00",
        message(json!({"role": "assistant", "content": [
            {"type": "text", "text": "fixture-text-a"},
            {"type": "toolCall", "id": "toolu_1", "name": "Bash", "arguments": {}}]})),
    );
    let (_home, mut session) = session_of(
        dir.path(),
        &[
            user("u1", None, "2000-01-01T00:00:01Z", "fixture-text-u"),
            compaction("c1", Some("u1"), "2000-01-01T00:00:02.5+00:00", "u1"),
            head,
        ],
    )?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[0]["timestamp"], "2000-01-01T10:00:03+10:00");
    assert_eq!(lines[1]["timestamp"], "2000-01-01T10:00:03+10:00");
    assert_eq!(
        lines[0]["payload"]["timestamp"],
        "2000-01-01T10:00:03+10:00"
    );
    let compaction = text_of(&lines[2]["payload"]).ok_or("no text")?;
    assert!(compaction.starts_with("<COMPACTION SUMMARY c1>"));
    assert_eq!(lines[2]["timestamp"], "2000-01-01T00:00:02.5+00:00");
    assert_eq!(lines[3]["payload"]["role"], "user");
    assert_eq!(lines[3]["timestamp"], "2000-01-01T00:00:01Z");
    let mut of_head = 0;
    for line in &lines[4..] {
        assert_eq!(line["timestamp"], "2000-01-01T10:00:03+10:00");
        of_head += 1;
    }
    assert_eq!(of_head, 2);
    Ok(())
}

#[test]
fn compaction_is_marked_text() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = session_of(
        dir.path(),
        &[
            user("u0", None, STAMP, "fixture-text-0"),
            compaction("c1", Some("u0"), STAMP, "c1"),
            user("u2", Some("c1"), STAMP, "fixture-text-2"),
        ],
    )?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    assert_eq!(lines[2]["payload"]["role"], "developer");
    assert_eq!(
        text_of(&lines[2]["payload"]),
        Some("<COMPACTION SUMMARY c1>\nfixture-summary")
    );
    let account = account(&translation)?;
    let rows = changed_of(&account, "c1");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].hash, None);
    assert_eq!(rows[0].before, "compaction");
    assert_eq!(rows[0].after, "marked text");
    assert_eq!(
        rows[0].how,
        "carried as developer text under the COMPACTION SUMMARY marker; \
         firstKeptEntryId not carried; tokensBefore not carried"
    );
    Ok(())
}

#[test]
fn entries_left_by_a_compaction_are_lost() -> Gate {
    let dir = tempfile::tempdir()?;
    let label = entry(
        "l1",
        Some("a1"),
        STAMP,
        EntryBody::Label {
            target_id: "u1".to_owned(),
            label: Some("fixture-label".to_owned()),
        },
    );
    let (_home, mut session) = session_of(
        dir.path(),
        &[
            user("u1", None, STAMP, "fixture-text-1"),
            assistant("a1", Some("u1"), STAMP, "fixture-text-2"),
            label,
            compaction("c1", Some("l1"), STAMP, "c1"),
            user("u2", Some("c1"), STAMP, "fixture-text-3"),
        ],
    )?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let text = std::fs::read_to_string(&translation.rollout)?;
    assert!(!text.contains("fixture-text-1"));
    assert!(!text.contains("fixture-text-2"));
    assert!(!text.contains("fixture-label"));
    let account = account(&translation)?;
    assert_eq!(account.lost.len(), 3);
    let expected = [("u1", "message"), ("a1", "message"), ("l1", "label")];
    for (row, (entry, kind)) in account.lost.iter().zip(expected) {
        assert_eq!(row.entry, entry);
        assert_eq!(row.kind, kind);
        assert_eq!(row.hash, None);
        assert_eq!(row.reason, "left off the context path by compaction c1");
    }
    Ok(())
}

#[test]
fn branch_summary_and_custom_message_are_marked_text() -> Gate {
    let dir = tempfile::tempdir()?;
    let image = json!({"type": "image", "source": {"type": "base64",
        "media_type": "image/png", "data": "iVBORw0KGgo="}});
    let branch = entry(
        "b1",
        Some("u1"),
        STAMP,
        EntryBody::BranchSummary {
            from_id: "u1".to_owned(),
            summary: "fixture-branch".to_owned(),
            rest: Map::new(),
        },
    );
    let custom = entry(
        "m1",
        Some("b1"),
        STAMP,
        EntryBody::CustomMessage {
            custom_type: "ext".to_owned(),
            content: json!([{"type": "text", "text": "hi"}, image]),
            rest: Map::new(),
        },
    );
    let (_home, mut session) = session_of(
        dir.path(),
        &[user("u1", None, STAMP, "fixture-text-1"), branch, custom],
    )?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    let texts = texts_of(&lines);
    assert!(texts.iter().any(|t| t.starts_with("<BRANCH SUMMARY b1>")));
    assert!(texts.contains(&"<CUSTOM MESSAGE m1>\nhi"));
    let account = account(&translation)?;
    let b1 = changed_of(&account, "b1");
    assert_eq!(b1.len(), 1);
    assert_eq!(b1[0].before, "branch_summary");
    assert_eq!(b1[0].after, "marked text");
    assert_eq!(
        b1[0].how,
        "carried as developer text under the BRANCH SUMMARY marker; fromId not carried"
    );
    let m1 = changed_of(&account, "m1");
    assert_eq!(m1.len(), 1);
    assert_eq!(m1[0].before, "custom_message");
    assert_eq!(m1[0].after, "marked text");
    assert_eq!(
        m1[0].how,
        "carried as developer text under the CUSTOM MESSAGE marker; customType not carried"
    );
    let images: Vec<_> = account.lost.iter().filter(|r| r.kind == "image").collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].hash, Some(part_hash(&image)?));
    assert_eq!(
        images[0].reason,
        "image part 1 not carried: marked text holds text only"
    );
    for row in &account.lost {
        assert!(!row.reason.contains("not conversation"));
    }
    Ok(())
}

#[test]
fn unparsed_stamp_is_refused_and_nothing_written() -> Gate {
    let dir = tempfile::tempdir()?;
    let (_home, mut session) = session_of(
        dir.path(),
        &[
            user("e1", None, STAMP, "fixture-text-1"),
            assistant("e2", Some("e1"), "yesterday", "fixture-text-2"),
        ],
    )?;
    let out = dir.path().join("o");
    let error = translate(&mut session, &out, "0.156.0", Some("UTC"))
        .err()
        .ok_or("an unparsed stamp was accepted")?;
    assert_eq!(
        error.to_string(),
        "entry e2 has a stamp that is not RFC 3339: re-import the source file"
    );
    assert!(files_under(&out)?.is_empty());
    Ok(())
}
