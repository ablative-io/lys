//! Gates on what hangs beside the context path (HOME-009 R5): a labelled
//! sidechain carried as marked text after its parent's items, lys entries
//! and harness events listed lost, a sidechain's own events and an
//! unlabelled branch listed and never carried, every other off-path kind
//! lost with its reason, a forked child's carried prompt written last, an
//! image marked text cannot hold lost with its reason, and a sidechain under
//! a compacted entry lost naming the compaction. Each fixture is a synthetic
//! Claude Code file imported through `cli::run`.
//!
//! The importer leaves the head on the file's last chain record, so each
//! fixture whose sidechains would otherwise end the file closes with one
//! more main-chain record naming the main chain's last message; and since
//! the importer gives a compaction a fresh id no record can name, the record
//! after a `summary` is a `system` record with no `parentUuid`, which the
//! importer hangs under the compaction, as Claude Code's own compact
//! boundary follows its summary.

use std::ffi::OsString;
use std::path::Path;

use clap::Parser;
use serde_json::{Map, Value, json};

use crate::cli::{Cli, run as run_cli};
use crate::harness::codex::account::{Account, Changed, Lost, part_hash};
use crate::harness::codex::rollout::translate;
use crate::harness::codex::rollout_tests::{Found, Gate, account, lines, run, text_of};
use crate::record::entries::{CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::reader::SessionReader;
use crate::record::{Home, Session};

const M1: &str = "11111111-1111-4111-8111-111111111111";
const M2: &str = "22222222-2222-4222-8222-222222222222";
const M3: &str = "33333333-3333-4333-8333-333333333333";
const M4: &str = "44444444-4444-4444-8444-444444444444";
const S1: &str = "55555555-5555-4555-8555-555555555555";
const S2: &str = "66666666-6666-4666-8666-666666666666";
const S3: &str = "77777777-7777-4777-8777-777777777777";
const N1: &str = "88888888-8888-4888-8888-888888888888";
const N2: &str = "99999999-9999-4999-8999-999999999999";
const X1: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const NOT_CONVERSATION: &str = "Codex has no item for it and it is not conversation";

/// A Claude Code record of the message's role, at second `at`.
fn record(id: &str, parent: Option<&str>, at: u8, message: &Value) -> Value {
    json!({
        "parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "cc", "version": "2.1.281", "gitBranch": "", "uuid": id,
        "timestamp": format!("2026-01-01T00:00:{at:02}.000Z"),
        "type": message["role"], "message": message
    })
}

/// A record on a sidechain, with its agent id when it has one.
fn side(mut value: Value, agent: Option<&str>) -> Found<Value> {
    let object = value.as_object_mut().ok_or("not a record")?;
    object.insert("isSidechain".to_owned(), json!(true));
    if let Some(agent) = agent {
        object.insert("agentId".to_owned(), json!(agent));
    }
    Ok(value)
}

/// A record on a sidechain with no agent id.
fn plain(value: Value) -> Found<Value> {
    side(value, None)
}

/// A record on the sidechain of agent `a1`.
fn a1(value: Value) -> Found<Value> {
    side(value, Some("a1"))
}

/// A harness record (`attachment` or `system`) with its own uuid.
fn harness(kind: &str, id: &str, parent: Option<&str>, at: u8) -> Value {
    let mut value = json!({"type": kind, "uuid": id, "parentUuid": parent,
        "isSidechain": false, "timestamp": format!("2026-01-01T00:00:{at:02}.000Z")});
    if let Some(object) = value.as_object_mut() {
        if kind == "attachment" {
            let attachment = json!({"type": "skill_listing", "skillCount": 1});
            object.insert("attachment".to_owned(), attachment);
        } else {
            object.insert("subtype".to_owned(), json!("compact_boundary"));
        }
    }
    value
}

/// A user message of one text part.
fn asked(text: &str) -> Value {
    json!({"role": "user", "content": [{"type": "text", "text": text}]})
}

/// An assistant message of these parts.
fn answer(parts: &Value) -> Value {
    json!({"id": "msg_fixture", "type": "message", "role": "assistant",
        "model": "claude-fixture", "content": parts, "stop_reason": "end_turn",
        "stop_sequence": null, "usage": {"input_tokens": 1, "output_tokens": 1}})
}

/// An assistant message of one text part.
fn said(text: &str) -> Value {
    answer(&json!([{"type": "text", "text": text}]))
}

fn base64_image() -> Value {
    json!({"type": "image", "source": {"type": "base64", "media_type": "image/png",
        "data": "iVBORw0KGgo="}})
}

fn os(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

/// Run one `lys-home` command in-process and return its report.
fn cli(words: &[OsString]) -> Found<Value> {
    let mut argv = vec![OsString::from("lys-home")];
    argv.extend(words.iter().cloned());
    Ok(run_cli(Cli::try_parse_from(argv)?)?)
}

/// Import the records into session `s1` of a fresh home under `dir`.
fn import(dir: &Path, records: &[Value]) -> Found<Home> {
    let file = dir.join("claude-code.jsonl");
    let mut text = String::new();
    for value in records {
        text.push_str(&serde_json::to_string(value)?);
        text.push('\n');
    }
    std::fs::write(&file, text)?;
    let home = dir.join("home");
    cli(&[
        "import".into(),
        "--home".into(),
        os(&home),
        "--claude-code".into(),
        os(&file),
        "--session".into(),
        "s1".into(),
    ])?;
    Ok(Home::open(home)?)
}

/// Light a lantern at `point` of session `s1` and return its id.
fn light(home: &Home, point: &str) -> Found<String> {
    let lit = cli(&[
        "lantern".into(),
        "light".into(),
        "--home".into(),
        os(home.root()),
        "--session".into(),
        "s1".into(),
        "--point".into(),
        point.into(),
        "--note".into(),
        "fixture-note".into(),
        "--by".into(),
        "fixture-lighter".into(),
    ])?;
    let id = lit["id"].as_str().ok_or("no lantern id")?;
    Ok(id.to_owned())
}

/// Every entry of a session file, in file order.
fn entries_of(session: &Session) -> Found<Vec<Entry>> {
    let reader = SessionReader::open(session.file())?;
    let mut out = Vec::new();
    for id in reader.ids() {
        out.push(reader.entry(id)?);
    }
    Ok(out)
}

/// The id and stamp of the label naming an agent.
fn agent_label(entries: &[Entry], agent: &str) -> Found<(String, String)> {
    let wanted = format!("agent {agent}");
    for entry in entries {
        if let EntryBody::Label { label: Some(l), .. } = &entry.body
            && *l == wanted
        {
            return Ok((entry.id().to_owned(), entry.base.timestamp.clone()));
        }
    }
    Err("no agent label".into())
}

/// The lost rows that pass `keep`, in order.
fn lost_where(account: &Account, keep: impl Fn(&Lost) -> bool) -> Vec<&Lost> {
    let mut out = Vec::new();
    for row in &account.lost {
        if keep(row) {
            out.push(row);
        }
    }
    out
}

/// The changed rows that pass `keep`, in order.
fn changed_where(account: &Account, keep: impl Fn(&Changed) -> bool) -> Vec<&Changed> {
    let mut out = Vec::new();
    for row in &account.changed {
        if keep(row) {
            out.push(row);
        }
    }
    out
}

/// The first part's text of each line's payload, `None` where it has none.
fn texts(lines: &[Value]) -> Vec<Option<&str>> {
    let mut out = Vec::new();
    for line in lines {
        out.push(text_of(&line["payload"]));
    }
    out
}

/// Main user, main assistant, two sidechain records of agent `a1`, and the
/// closing main-chain record.
fn carried_fixture() -> Found<Vec<Value>> {
    Ok(vec![
        record(M1, None, 1, &asked("fixture-main-1")),
        record(M2, Some(M1), 2, &said("fixture-main-2")),
        a1(record(S1, None, 5, &asked("fixture-side-1")))?,
        a1(record(S2, Some(S1), 6, &said("fixture-side-2")))?,
        record(M3, Some(M2), 7, &asked("fixture-main-3")),
    ])
}

#[test]
fn sidechain_is_carried_as_marked_text() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = import(dir.path(), &carried_fixture()?)?;
    let mut session = home.open_session("s1")?;
    let (label, label_stamp) = agent_label(&entries_of(&session)?, "a1")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    let heading = format!("<SIDECHAIN {S1} AGENT a1 UNDER ENTRY {M2}>");
    let texts = texts(&lines);
    let mut found = Vec::new();
    for (at, text) in texts.iter().enumerate() {
        if text.is_some_and(|t| t.starts_with(&heading)) {
            found.push(at);
        }
    }
    assert_eq!(found.len(), 1);
    let at = found[0];
    let text = texts[at].ok_or("no text")?;
    assert_eq!(text.lines().next(), Some(heading.as_str()));
    assert!(text.contains("fixture-side-1"));
    assert!(text.contains("fixture-side-2"));
    assert_eq!(lines[at]["payload"]["role"], "developer");
    assert_eq!(lines[at - 1]["payload"]["role"], "assistant");
    assert_eq!(texts[at - 1], Some("fixture-main-2"));
    assert_eq!(lines[at]["timestamp"], label_stamp.as_str());
    let account = account(&translation)?;
    let sidechain = changed_where(&account, |r| r.before == "sidechain");
    assert_eq!(sidechain.len(), 2);
    assert!(sidechain.iter().all(|r| r.after == "marked text"));
    let labels = changed_where(&account, |r| r.before == "label");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].entry, label);
    assert_eq!(labels[0].hash, None);
    assert_eq!(labels[0].after, "marker line");
    assert_eq!(labels[0].how, "agent id carried in the marker line");
    assert!(lost_where(&account, |r| r.kind == "label").is_empty());
    Ok(())
}

#[test]
fn lys_entries_are_listed_lost() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = import(
        dir.path(),
        &[
            record(M1, None, 1, &asked("fixture-main-1")),
            harness("attachment", X1, Some(M1), 2),
            record(M2, Some(X1), 3, &said("fixture-main-2")),
        ],
    )?;
    let lantern = light(&home, M2)?;
    let mut session = home.open_session("s1")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let account = account(&translation)?;
    let lanterns = lost_where(&account, |r| r.kind == "lys.lantern");
    assert_eq!(lanterns.len(), 1);
    assert_eq!(lanterns[0].entry, lantern);
    assert_eq!(lanterns[0].reason, NOT_CONVERSATION);
    let events = lost_where(&account, |r| r.kind == "lys.harness_event");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entry, X1);
    assert_eq!(events[0].reason, NOT_CONVERSATION);
    Ok(())
}

#[test]
fn sidechain_descendants_and_unlabelled_branch_are_listed() -> Gate {
    let dir = tempfile::tempdir()?;
    let call = json!([{"type": "tool_use", "id": "toolu_s1", "name": "Read",
        "input": {"file_path": "/fixture"}}]);
    let result = json!({"role": "user", "content": [{"type": "tool_result",
        "tool_use_id": "toolu_s1", "content": "fixture-result", "is_error": false}]});
    let home = import(
        dir.path(),
        &[
            record(M1, None, 1, &asked("fixture-main-1")),
            record(M2, Some(M1), 2, &said("fixture-main-2")),
            a1(record(S1, None, 3, &answer(&call)))?,
            a1(record(S2, Some(S1), 4, &result))?,
            a1(harness("attachment", S3, Some(S2), 5))?,
            plain(record(N1, None, 6, &asked("fixture-unlabelled-1")))?,
            plain(record(N2, Some(N1), 7, &said("fixture-unlabelled-2")))?,
            record(M3, Some(M2), 8, &asked("fixture-main-3")),
        ],
    )?;
    let mut session = home.open_session("s1")?;
    let entries = entries_of(&session)?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let account = account(&translation)?;
    let events = lost_where(&account, |r| r.kind == "lys.harness_event");
    assert_eq!(events.len(), 2);
    let mut completed = None;
    for entry in &entries {
        if entry.parent_id() == Some(S2) && entry.is_custom("lys.harness_event") {
            completed = Some(entry.id());
        }
    }
    let completed = completed.ok_or("no tool_completed event under the result")?;
    assert_eq!(events[0].entry, completed);
    assert_eq!(events[1].entry, S3);
    let unlabelled = lost_where(&account, |r| r.kind == "unlabelled branch");
    assert_eq!(unlabelled.len(), 2);
    assert_eq!(unlabelled[0].entry, N1);
    assert_eq!(unlabelled[1].entry, N2);
    for row in &unlabelled {
        assert_eq!(
            row.reason,
            "off the context path with no agent label: not carried"
        );
    }
    let text = std::fs::read_to_string(&translation.rollout)?;
    assert!(!text.contains("fixture-unlabelled"));
    Ok(())
}

#[test]
fn off_path_label_compaction_branch_summary_and_model_change_are_lost() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = import(dir.path(), &carried_fixture()?)?;
    let mut session = home.open_session("s1")?;
    let (label, _) = agent_label(&entries_of(&session)?, "a1")?;
    let note = session.append_beside(EntryBody::Label {
        target_id: M3.to_owned(),
        label: Some("note".to_owned()),
    })?;
    let compaction = session.append_beside(EntryBody::Compaction {
        summary: "fixture-off-summary".to_owned(),
        first_kept_entry_id: M1.to_owned(),
        tokens_before: 0,
        rest: Map::new(),
    })?;
    let branch = session.append_beside(EntryBody::BranchSummary {
        from_id: M1.to_owned(),
        summary: "fixture-off-branch".to_owned(),
        rest: Map::new(),
    })?;
    let model = session.append_beside(EntryBody::ModelChange {
        provider: "fixture".to_owned(),
        model_id: "fixture-model".to_owned(),
    })?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let account = account(&translation)?;
    let off_path = "off the context path: not carried";
    let appended = [
        ("label", note, NOT_CONVERSATION),
        ("compaction", compaction, off_path),
        ("branch_summary", branch, off_path),
        ("model_change", model, NOT_CONVERSATION),
    ];
    let mut checked = 0;
    for (kind, id, reason) in &appended {
        let rows = lost_where(&account, |r| r.kind == *kind);
        assert_eq!(rows.len(), 1);
        assert_eq!(&rows[0].entry, id);
        assert_eq!(rows[0].hash, None);
        assert_eq!(rows[0].reason, *reason);
        checked += 1;
    }
    assert_eq!(checked, 4);
    let markers = changed_where(&account, |r| r.after == "marker line");
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].before, "label");
    assert_eq!(markers[0].entry, label);
    let text = std::fs::read_to_string(&translation.rollout)?;
    assert!(!text.contains("fixture-off"));
    Ok(())
}

#[test]
fn forked_child_opens_on_the_point() -> Gate {
    let dir = tempfile::tempdir()?;
    let image = base64_image();
    let go = json!({"type": "text", "text": "go"});
    let point = json!({"role": "user", "content": [go, image]});
    let home = import(
        dir.path(),
        &[
            record(M1, None, 1, &asked("fixture-main-1")),
            record(M2, Some(M1), 2, &said("fixture-main-2")),
            record(M3, Some(M2), 3, &point),
            record(M4, Some(M3), 4, &said("fixture-main-4")),
        ],
    )?;
    let lantern = light(&home, M3)?;
    let forked = cli(&[
        "fork".into(),
        "--home".into(),
        os(home.root()),
        "--lantern".into(),
        lantern.into(),
    ])?;
    let child = forked["report"]["child"].as_str().ok_or("no child")?;
    let mut session = home.open_session(child)?;
    let mut stamp = None;
    for entry in entries_of(&session)? {
        if entry.is_custom(CUSTOM_FORKED_FROM) {
            stamp = Some(entry.base.timestamp);
        }
    }
    let stamp = stamp.ok_or("no forked_from entry")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    let last = lines.last().ok_or("no lines")?;
    assert_eq!(last["payload"]["role"], "user");
    let prompt = json!([{"type": "input_text", "text": "go"}]);
    assert_eq!(last["payload"]["content"], prompt);
    assert_eq!(last["timestamp"], stamp.as_str());
    let texts = texts(&lines);
    let history = texts.iter().position(|t| *t == Some("fixture-main-2"));
    let history = history.ok_or("no walked history")?;
    assert_eq!(history, lines.len() - 2);
    let mut users_after = 0;
    for line in &lines[history + 1..] {
        if line["payload"]["role"] == "user" {
            users_after += 1;
        }
    }
    assert_eq!(users_after, 1);
    let account = account(&translation)?;
    let points = changed_where(&account, |r| r.entry == M3);
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].before, "point");
    assert_eq!(points[0].after, "first prompt");
    let how = "text parts joined by newlines into one input_text part, \
               read from the parent session's file";
    assert!(points[0].how.starts_with(how));
    let dropped = lost_where(&account, |r| r.entry == M3);
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].hash, Some(part_hash(&image)?));
    assert_eq!(dropped[0].kind, "image");
    assert_eq!(
        dropped[0].reason,
        "image part 1 not carried: a carried prompt holds text parts only"
    );
    Ok(())
}

#[test]
fn sidechain_image_is_lost_with_its_reason() -> Gate {
    let dir = tempfile::tempdir()?;
    let image = base64_image();
    let text = json!({"type": "text", "text": "look"});
    let look = json!({"role": "user", "content": [text, image]});
    let home = import(
        dir.path(),
        &[
            record(M1, None, 1, &asked("fixture-main-1")),
            record(M2, Some(M1), 2, &said("fixture-main-2")),
            a1(record(S1, None, 3, &look))?,
            record(M3, Some(M2), 4, &asked("fixture-main-3")),
        ],
    )?;
    let mut session = home.open_session("s1")?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let lines = lines(&translation.rollout)?;
    let mut sidechain = None;
    for text in texts(&lines).into_iter().flatten() {
        if text.starts_with("<SIDECHAIN ") {
            sidechain = Some(text);
        }
    }
    let sidechain = sidechain.ok_or("no sidechain item")?;
    assert!(sidechain.lines().any(|l| l == "look"));
    assert!(!sidechain.contains("base64"));
    assert!(!sidechain.contains("iVBORw0KGgo="));
    let account = account(&translation)?;
    assert_eq!(account.lost.len(), 1);
    assert_eq!(account.lost[0].hash, Some(part_hash(&image)?));
    assert_eq!(account.lost[0].kind, "image");
    assert_eq!(
        account.lost[0].reason,
        "image part 1 not carried: marked text holds text only"
    );
    Ok(())
}

#[test]
fn sidechain_under_a_compacted_entry_is_lost() -> Gate {
    let dir = tempfile::tempdir()?;
    let summary = json!({"type": "summary", "summary": "fixture-summary", "leafUuid": M2,
        "timestamp": "2026-01-01T00:00:05.000Z"});
    let home = import(
        dir.path(),
        &[
            record(M1, None, 1, &asked("fixture-main-1")),
            record(M2, Some(M1), 2, &said("fixture-main-2")),
            a1(record(S1, None, 3, &asked("fixture-side-1")))?,
            a1(record(S2, Some(S1), 4, &said("fixture-side-2")))?,
            summary,
            harness("system", X1, None, 6),
            record(M3, Some(X1), 7, &asked("fixture-main-3")),
        ],
    )?;
    let mut session = home.open_session("s1")?;
    let entries = entries_of(&session)?;
    let mut compaction = None;
    for entry in &entries {
        if matches!(entry.body, EntryBody::Compaction { .. }) {
            compaction = Some(entry.id());
        }
    }
    let compaction = compaction.ok_or("no compaction")?;
    let (label, _) = agent_label(&entries, "a1")?;
    let out = dir.path().join("o");
    let translation = translate(&mut session, &out, "0.156.0", Some("UTC"))?;
    let lines = lines(&translation.rollout)?;
    for text in texts(&lines).into_iter().flatten() {
        assert!(!text.starts_with("<SIDECHAIN"));
    }
    let account = account(&translation)?;
    let reason = format!("left off the context path by compaction {compaction}");
    let mut checked = 0;
    for id in [label.as_str(), S1, S2] {
        let rows = lost_where(&account, |r| r.entry == id);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].reason, reason);
        assert_eq!(rows[0].hash, None);
        checked += 1;
    }
    assert_eq!(checked, 3);
    Ok(())
}
