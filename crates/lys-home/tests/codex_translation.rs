//! The Codex translation run end to end (HOME-009 R8): synthetic Claude Code
//! files imported into a fresh home through `lys_home::cli::run` and
//! translated through the one entry point. Every text part, tool call and
//! tool result is carried whole, a tool result past the 4,000 characters
//! Codex's importer clips to included; foreign thinking is text and opaque
//! thinking is lost; every row names its kinds and reason, and the lost
//! count equals one the test makes itself by walking the home's parent ids
//! and the fixture's records, never the account; the same head translated
//! twice gives the same rollout bytes; and the Claude Code render of the
//! session is unchanged by the translation. No test name carries content.
//!
//! The importer leaves the head on the file's last chain record, so a
//! fixture whose sidechains would otherwise end the file closes with one
//! more main-chain record; and the record after a `summary` is a `system`
//! record with no `parentUuid`, which the importer hangs under the
//! compaction it gave a fresh id no record can name.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::Parser;
use serde_json::{Map, Value, json};

use lys_home::cli::{Cli, run};
use lys_home::harness::codex::account::Account;
use lys_home::harness::codex::rollout::{Translation, translate};
use lys_home::{Entry, EntryBase, EntryBody, Hash, Home, Session, SessionReader};

type Gate = Result<(), Box<dyn Error>>;
type Found<T> = Result<T, Box<dyn Error>>;

/// A uuid-shaped id for fixture record `n`.
fn uuid(n: u32) -> String {
    format!("{n:08x}-0000-4000-8000-{n:012x}")
}

/// A Claude Code record of the message's role.
fn record(id: &str, parent: Option<&str>, message: &Value) -> Value {
    json!({
        "parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "cc", "version": "2.1.281", "gitBranch": "", "uuid": id,
        "timestamp": "2026-01-01T00:00:00.000Z", "type": message["role"], "message": message
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

/// A harness record (`attachment` or `system`) with its own uuid.
fn harness(kind: &str, id: &str, parent: Option<&str>) -> Value {
    let mut value = json!({"type": kind, "uuid": id, "parentUuid": parent,
        "isSidechain": false, "timestamp": "2026-01-01T00:00:00.000Z"});
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

/// A user message of this content.
fn asked(content: &Value) -> Value {
    json!({"role": "user", "content": content})
}

/// An assistant message of these parts.
fn answer(parts: &Value) -> Value {
    json!({"id": "msg_fixture", "type": "message", "role": "assistant",
        "model": "claude-fixture", "content": parts, "stop_reason": "end_turn",
        "stop_sequence": null, "usage": {"input_tokens": 1, "output_tokens": 1}})
}

/// One text part, as a content list.
fn text(text: &str) -> Value {
    json!([{"type": "text", "text": text}])
}

fn os(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

/// Run one `lys-home` command in-process and return its report.
fn cli(words: &[OsString]) -> Found<Value> {
    let mut argv = vec![OsString::from("lys-home")];
    argv.extend(words.iter().cloned());
    Ok(run(Cli::try_parse_from(argv)?)?)
}

/// Import the records into session `s1` of a fresh home under `dir`.
fn import(dir: &Path, records: &[Value]) -> Found<Home> {
    let file = dir.join("claude-code.jsonl");
    let mut body = String::new();
    for value in records {
        body.push_str(&serde_json::to_string(value)?);
        body.push('\n');
    }
    std::fs::write(&file, body)?;
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

fn translated(home: &Home, out: &Path) -> Found<Translation> {
    let mut session = home.open_session("s1")?;
    Ok(translate(&mut session, out, "0.156.0", Some("UTC"))?)
}

/// The payload of every line of the rollout, in order.
fn items(translation: &Translation) -> Found<Vec<Value>> {
    let body = std::fs::read_to_string(&translation.rollout)?;
    let mut out = Vec::new();
    for line in body.lines() {
        let line: Value = serde_json::from_str(line)?;
        out.push(line["payload"].clone());
    }
    Ok(out)
}

fn account(translation: &Translation) -> Found<Account> {
    let bytes = std::fs::read(&translation.account)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Every text part of every message item, with its item's role.
fn texts(items: &[Value]) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    for item in items {
        let role = item["role"].as_str().unwrap_or("none");
        for part in item["content"].as_array().into_iter().flatten() {
            if let Some(text) = part["text"].as_str() {
                out.push((role, text));
            }
        }
    }
    out
}

/// The items of one type.
fn of_type<'a>(items: &'a [Value], kind: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    for item in items {
        if item["type"] == kind {
            out.push(item);
        }
    }
    out
}

#[test]
fn text_and_tools_are_carried_whole() -> Gate {
    let dir = tempfile::tempdir()?;
    let input = json!({"command": "fixture-command", "n": 3, "nested": {"list": [1, "two"]}});
    let (m1, m2, m3, m4) = (uuid(1), uuid(2), uuid(3), uuid(4));
    let saying = answer(&json!([{"type": "text", "text": "fixture-saying"},
        {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": input}]));
    let result = asked(&json!([{"type": "tool_result", "tool_use_id": "toolu_1",
        "content": "fixture-result", "is_error": false}]));
    let home = import(
        dir.path(),
        &[
            record(&m1, None, &asked(&json!("fixture-question"))),
            record(&m2, Some(&m1), &saying),
            record(&m3, Some(&m2), &result),
            record(&m4, Some(&m3), &answer(&text("fixture-answer"))),
        ],
    )?;
    let translation = translated(&home, &dir.path().join("o"))?;
    let items = items(&translation)?;
    let texts = texts(&items);
    let expected = [
        ("user", "fixture-question"),
        ("assistant", "fixture-saying"),
        ("assistant", "fixture-answer"),
    ];
    let mut found = 0;
    for want in expected {
        assert_eq!(texts.iter().filter(|t| **t == want).count(), 1);
        found += 1;
    }
    assert_eq!(found, 3);
    let calls = of_type(&items, "function_call");
    assert_eq!(calls.len(), 1);
    let arguments = calls[0]["arguments"].as_str().ok_or("no arguments")?;
    let arguments: Value = serde_json::from_str(arguments)?;
    assert_eq!(arguments, input);
    let outputs = of_type(&items, "function_call_output");
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["output"], "fixture-result");
    assert_eq!(outputs[0]["call_id"], calls[0]["call_id"]);
    Ok(())
}

#[test]
fn tool_result_past_importer_clip_is_whole() -> Gate {
    let dir = tempfile::tempdir()?;
    let mut long = String::new();
    for i in 0..10_000u32 {
        long.push(char::from_digit(i % 10, 10).ok_or("not a digit")?);
    }
    assert_eq!(long.chars().count(), 10_000);
    let (m1, m2, m3) = (uuid(1), uuid(2), uuid(3));
    let call = answer(&json!([{"type": "tool_use", "id": "toolu_1", "name": "Read",
        "input": {}}]));
    let result = asked(&json!([{"type": "tool_result", "tool_use_id": "toolu_1",
        "content": long, "is_error": false}]));
    let home = import(
        dir.path(),
        &[
            record(&m1, None, &asked(&text("fixture-question"))),
            record(&m2, Some(&m1), &call),
            record(&m3, Some(&m2), &result),
        ],
    )?;
    let translation = translated(&home, &dir.path().join("o"))?;
    let items = items(&translation)?;
    let outputs = of_type(&items, "function_call_output");
    assert_eq!(outputs.len(), 1);
    let output = outputs[0]["output"].as_str().ok_or("output is not a string")?;
    assert_eq!(output.chars().count(), 10_000);
    assert_eq!(output, long);
    Ok(())
}

#[test]
fn foreign_thinking_renders_as_text() -> Gate {
    let dir = tempfile::tempdir()?;
    let (m1, m2) = (uuid(1), uuid(2));
    let thought = answer(&json!([
        {"type": "thinking", "thinking": "fixture-thinking", "signature": "fixture-signature-zz"},
        {"type": "redacted_thinking", "data": "fixture-redacted-zz"},
        {"type": "text", "text": "fixture-saying"}]));
    let home = import(
        dir.path(),
        &[
            record(&m1, None, &asked(&text("fixture-question"))),
            record(&m2, Some(&m1), &thought),
        ],
    )?;
    let translation = translated(&home, &dir.path().join("o"))?;
    let items = items(&translation)?;
    let texts = texts(&items);
    assert!(texts.contains(&("assistant", "fixture-thinking")));
    let body = std::fs::read_to_string(&translation.rollout)?;
    assert!(!body.contains("fixture-signature-zz"));
    assert!(!body.contains("fixture-redacted-zz"));
    assert!(!body.contains("\"reasoning\""));
    let account = account(&translation)?;
    let mut as_text = 0;
    for row in &account.changed {
        if row.before == "thinking" && row.after == "output_text" {
            as_text += 1;
        }
    }
    assert_eq!(as_text, 1);
    let mut redacted = 0;
    for row in &account.lost {
        if row.reason == "redacted thinking dropped: another provider" {
            redacted += 1;
        }
    }
    assert_eq!(redacted, 1);
    Ok(())
}

/// Fixture record `id` under fixture record `parent`.
fn at(id: u32, parent: Option<u32>, message: &Value) -> Value {
    let parent = parent.map(uuid);
    record(&uuid(id), parent.as_deref(), message)
}

/// The fixture of `every_row_names_its_kinds_and_reason`, as Claude Code
/// records in file order.
fn everything() -> Found<Vec<Value>> {
    let u = uuid;
    let base64 = json!({"type": "image", "source": {"type": "base64",
        "media_type": "image/png", "data": "iVBORw0KGgo="}});
    let summary = json!({"type": "summary", "summary": "fixture-summary", "leafUuid": u(2),
        "timestamp": "2026-01-01T00:00:00.000Z"});
    let odd_parts = asked(&json!([{"type": "text", "text": "fixture-4"},
        {"type": "image", "source": {"type": "url", "url": "https://example.invalid/a.png"}},
        {"type": "document", "source": {"type": "text", "media_type": "text/plain",
            "data": "fixture-doc"}}]));
    let odd_answer = answer(&json!([{"type": "redacted_thinking", "data": "fixture-opaque"},
        {"type": "thinking", "thinking": "", "signature": "fixture-sig"},
        {"type": "text", "text": "fixture-5"},
        {"type": "tool_use", "name": "Bash", "input": {}}]));
    let no_id_result = asked(&json!([{"type": "tool_result", "content": "fixture-6",
        "is_error": false}]));
    let a1_ask = asked(&json!([{"type": "text", "text": "fixture-a1"}, base64]));
    let a1_answer = answer(&json!([{"type": "redacted_thinking", "data": "fixture-opaque-a"},
        {"type": "tool_use", "id": "toolu_a", "name": "Read", "input": {}}]));
    let a1_result = asked(&json!([{"type": "tool_result", "tool_use_id": "toolu_a",
        "content": "fixture-a3", "is_error": false}]));
    let a1 = Some("a1");
    let b1 = Some("b1");
    Ok(vec![
        at(1, None, &asked(&text("fixture-1"))),
        at(2, Some(1), &answer(&text("fixture-2"))),
        side(at(11, None, &asked(&text("fixture-b1"))), b1)?,
        side(at(12, Some(11), &answer(&text("fixture-b2"))), b1)?,
        summary,
        harness("system", &u(3), None),
        at(4, Some(3), &odd_parts),
        at(5, Some(4), &odd_answer),
        at(6, Some(5), &no_id_result),
        harness("attachment", &u(7), Some(&u(6))),
        at(8, Some(7), &answer(&text("fixture-8"))),
        side(at(21, None, &a1_ask), a1)?,
        side(at(22, Some(21), &a1_answer), a1)?,
        side(at(23, Some(22), &a1_result), a1)?,
        side(harness("attachment", &u(24), Some(&u(23))), a1)?,
        side(at(31, None, &asked(&text("fixture-n1"))), None)?,
        side(at(32, Some(31), &answer(&text("fixture-n2"))), None)?,
        at(9, Some(8), &asked(&text("fixture-9"))),
    ])
}

/// An entry under `parent`, stamped.
fn entry(id: &str, parent: &str, body: EntryBody) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: Some(parent.to_owned()),
            timestamp: "2026-01-01T00:00:01.000Z".to_owned(),
        },
        body,
    }
}

/// The appends of `every_row_names_its_kinds_and_reason`: four beside the
/// path, three at the head.
fn append_rest(session: &mut Session) -> Found<()> {
    session.append_beside(EntryBody::Label {
        target_id: uuid(9),
        label: Some("note".to_owned()),
    })?;
    session.append_beside(EntryBody::Compaction {
        summary: "fixture-off-summary".to_owned(),
        first_kept_entry_id: uuid(9),
        tokens_before: 0,
        rest: Map::new(),
    })?;
    session.append_beside(EntryBody::BranchSummary {
        from_id: uuid(1),
        summary: "fixture-off-branch".to_owned(),
        rest: Map::new(),
    })?;
    session.append_beside(EntryBody::ModelChange {
        provider: "fixture".to_owned(),
        model_id: "fixture-model".to_owned(),
    })?;
    let branch = EntryBody::BranchSummary {
        from_id: uuid(1),
        summary: "fixture-branch".to_owned(),
        rest: Map::new(),
    };
    session.append_entry(&entry("fixture-bs", &uuid(9), branch))?;
    let content = json!([{"type": "text", "text": "fixture-cm"}, {"type": "image",
        "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}}]);
    let custom = EntryBody::CustomMessage {
        custom_type: "ext".to_owned(),
        content,
        rest: Map::new(),
    };
    session.append_entry(&entry("fixture-cm", "fixture-bs", custom))?;
    let message = json!({"role": "bashExecution", "command": "fixture-cmd",
        "output": "fixture-out"});
    let bash = EntryBody::Message { message };
    session.append_entry(&entry("fixture-bx", "fixture-cm", bash))?;
    Ok(())
}

/// The home's entries walked by parent id, as the test reads them.
struct Walk {
    entries: Vec<Entry>,
    parent: BTreeMap<String, Option<String>>,
    path: Vec<String>,
    context: BTreeSet<String>,
    anchor: BTreeMap<String, String>,
}

impl Walk {
    fn read(file: &Path, head: &str) -> Found<Self> {
        let reader = SessionReader::open(file)?;
        let mut entries = Vec::new();
        let mut parent = BTreeMap::new();
        for id in reader.ids() {
            let entry = reader.entry(id)?;
            parent.insert(id.to_owned(), entry.parent_id().map(str::to_owned));
            entries.push(entry);
        }
        let mut path = vec![head.to_owned()];
        let mut at = head.to_owned();
        while let Some(Some(up)) = parent.get(&at) {
            path.push(up.clone());
            at = up.clone();
        }
        path.reverse();
        let mut walk = Self {
            entries,
            parent,
            context: path.iter().cloned().collect(),
            path,
            anchor: BTreeMap::new(),
        };
        walk.drop_compacted();
        walk.place_off_path();
        Ok(walk)
    }

    fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id() == id)
    }

    /// The context path: the whole path, less what the last compaction on
    /// it leaves before its first kept entry.
    fn drop_compacted(&mut self) {
        let mut last = None;
        for (i, id) in self.path.iter().enumerate() {
            if let Some(EntryBody::Compaction {
                first_kept_entry_id,
                ..
            }) = self.entry(id).map(|e| &e.body)
            {
                last = Some((i, first_kept_entry_id.clone()));
            }
        }
        let Some((at, kept)) = last else {
            return;
        };
        let mut keeping = false;
        for id in &self.path[..at] {
            keeping = keeping || *id == kept;
            if !keeping {
                self.context.remove(id);
            }
        }
    }

    /// Each entry off the path, by the nearest ancestor on it.
    fn place_off_path(&mut self) {
        for e in &self.entries {
            if self.path.iter().any(|id| id == e.id()) {
                continue;
            }
            let mut up = e.parent_id().map(str::to_owned);
            while let Some(id) = up {
                if self.path.contains(&id) {
                    self.anchor.insert(e.id().to_owned(), id);
                    break;
                }
                up = self.parent.get(&id).cloned().flatten();
            }
        }
    }

    fn on_context(&self, e: &Entry) -> bool {
        self.context.contains(e.id())
    }

    fn under_context(&self, id: &str) -> bool {
        let anchor = self.anchor.get(id);
        anchor.is_some_and(|a| self.context.contains(a))
    }

    /// Whether `id` descends from `root` by parent ids.
    fn descends(&self, id: &str, root: &str) -> bool {
        let mut up = self.parent.get(id).cloned().flatten();
        while let Some(at) = up {
            if at == root {
                return true;
            }
            up = self.parent.get(&at).cloned().flatten();
        }
        false
    }
}

fn is_message(e: &Entry) -> bool {
    matches!(e.body, EntryBody::Message { .. })
}

fn named(v: &Value, key: &str) -> bool {
    v[key].as_str().is_some_and(|s| !s.is_empty())
}

fn opaque_thinking(part: &Value) -> bool {
    let thinking = part["thinking"].as_str();
    let empty = thinking.is_none_or(|t| t.trim().is_empty());
    part["redacted"] == true || empty
}

fn parts(message: &Value) -> Vec<&Value> {
    let list = message["content"].as_array();
    list.into_iter().flatten().collect()
}

/// Term (1): the parts and messages on the context path Codex has no item
/// for, counted from the entries.
fn term_one(w: &Walk) -> u64 {
    let mut n = 0;
    for e in &w.entries {
        let EntryBody::Message { message } = &e.body else {
            continue;
        };
        if !w.on_context(e) {
            continue;
        }
        let role = message["role"].as_str();
        match role {
            Some("user" | "assistant") => {
                for part in parts(message) {
                    let lost = match part["type"].as_str() {
                        Some("text") => false,
                        Some("thinking") => opaque_thinking(part),
                        Some("toolCall") => !named(part, "id") || !named(part, "name"),
                        Some("image") => {
                            role == Some("assistant") || part["source"]["type"] != "base64"
                        }
                        _ => true,
                    };
                    n += u64::from(lost);
                }
            }
            Some("toolResult") => n += u64::from(!named(message, "toolCallId")),
            _ => n += 1,
        }
    }
    n
}

/// The carried sidechains' labels, and their message entries.
fn carried(w: &Walk) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut labels = BTreeSet::new();
    let mut members = BTreeSet::new();
    for e in &w.entries {
        let EntryBody::Label {
            target_id,
            label: Some(l),
        } = &e.body
        else {
            continue;
        };
        let parent_on_context = e.parent_id().is_some_and(|p| w.context.contains(p));
        if !l.starts_with("agent ") || !parent_on_context || !w.under_context(target_id) {
            continue;
        }
        labels.insert(e.id().to_owned());
        for m in &w.entries {
            let family = m.id() == target_id || w.descends(m.id(), target_id);
            if family && is_message(m) {
                members.insert(m.id().to_owned());
            }
        }
    }
    (labels, members)
}

/// The harness events the importer hangs under the `a1` sidechain, one per
/// `tool_result` part and one per `attachment` or `system` record; and the
/// unlabelled sidechain's messages, one per assistant record and, per user
/// record, one per `tool_result` part plus one if it holds any other part.
fn from_records(records: &[Value]) -> (u64, u64) {
    let (mut events, mut unlabelled) = (0, 0);
    let (mut in_a1, mut in_unlabelled) = (false, false);
    for r in records {
        if r["isSidechain"] != true {
            continue;
        }
        if r["parentUuid"].is_null() && r.get("message").is_some() {
            in_a1 = r["agentId"] == "a1";
            in_unlabelled = r.get("agentId").is_none();
        }
        let parts = parts(&r["message"]);
        let results = parts.iter().filter(|p| p["type"] == "tool_result").count() as u64;
        let others = parts.iter().any(|p| p["type"] != "tool_result");
        if in_a1 {
            let record_event = r["type"] == "attachment" || r["type"] == "system";
            events += results + u64::from(record_event);
        }
        if in_unlabelled {
            unlabelled += if r["type"] == "assistant" {
                1
            } else {
                results + u64::from(others)
            };
        }
    }
    (events, unlabelled)
}

#[test]
fn every_row_names_its_kinds_and_reason() -> Gate {
    let dir = tempfile::tempdir()?;
    let records = everything()?;
    let home = import(dir.path(), &records)?;
    {
        let mut session = home.open_session("s1")?;
        append_rest(&mut session)?;
    }
    cli(&[
        "lantern".into(),
        "light".into(),
        "--home".into(),
        os(home.root()),
        "--session".into(),
        "s1".into(),
        "--point".into(),
        "fixture-bx".into(),
        "--note".into(),
        "fixture-note".into(),
        "--by".into(),
        "fixture-lighter".into(),
    ])?;
    let mut session = home.open_session("s1")?;
    let head = session.head()?.ok_or("no head")?.to_owned();
    let w = Walk::read(session.file(), &head)?;
    let out = dir.path().join("o");
    let translation = translate(&mut session, &out, "0.156.0", Some("UTC"))?;
    let account = account(&translation)?;
    let (labels, members) = carried(&w);

    let one = term_one(&w);
    let (mut two, mut three) = (0, 0);
    for e in w.entries.iter().filter(|e| w.on_context(e)) {
        match &e.body {
            EntryBody::Message { .. }
            | EntryBody::Compaction { .. }
            | EntryBody::BranchSummary { .. } => {}
            EntryBody::CustomMessage { content, .. } => {
                for part in content.as_array().into_iter().flatten() {
                    three += u64::from(part["type"] != "text");
                }
            }
            _ => two += 1,
        }
    }
    let mut left = Vec::new();
    for id in &w.path {
        if !w.context.contains(id) {
            left.push(id);
        }
    }
    let mut four = left.len() as u64;
    for anchor in w.anchor.values() {
        four += u64::from(left.contains(&anchor));
    }
    let (mut five, mut six, mut seven) = (0, 0, 0);
    for e in &w.entries {
        if !w.under_context(e.id()) {
            continue;
        }
        if !is_message(e) {
            five += u64::from(!labels.contains(e.id()));
            continue;
        }
        if !members.contains(e.id()) {
            seven += 1;
            continue;
        }
        let EntryBody::Message { message } = &e.body else {
            continue;
        };
        for part in parts(message) {
            let lost = match part["type"].as_str() {
                Some("text" | "toolCall") => false,
                Some("thinking") => opaque_thinking(part),
                _ => true,
            };
            six += u64::from(lost);
        }
    }

    // Cross-checks against the fixture's own records.
    let (a1_events, unlabelled) = from_records(&records);
    let a1_target = uuid(21);
    let mut walked_a1_events = 0;
    for e in &w.entries {
        let event = e.is_custom("lys.harness_event");
        walked_a1_events += u64::from(event && w.descends(e.id(), &a1_target));
    }
    assert_eq!(walked_a1_events, a1_events);
    assert_eq!(a1_events, 2);
    assert_eq!(seven, unlabelled);
    assert_eq!(labels.len(), 1);

    let terms = [one, two, three, four, five, six, seven];
    assert_eq!(terms, [7, 3, 1, 5, 7, 2, 2]);
    assert_eq!(account.lost.len() as u64, terms.iter().sum::<u64>());
    assert!(!account.changed.is_empty());
    for row in &account.changed {
        assert!(!row.before.is_empty());
        assert!(!row.after.is_empty());
        assert!(!row.how.is_empty());
    }
    for row in &account.lost {
        assert!(!row.reason.is_empty() && !row.kind.is_empty());
    }
    let written = std::fs::read_to_string(&translation.account)?;
    let contents = ["fixture-1\"", "fixture-4", "fixture-a1", "fixture-n1", "fixture-summary"];
    for content in contents {
        assert!(!written.contains(content));
    }
    assert!(!written.contains("example.invalid"));
    Ok(())
}

/// A home holding a question and a signed answer.
fn simple(dir: &Path) -> Found<Home> {
    let (m1, m2) = (uuid(1), uuid(2));
    let reply = answer(&json!([
        {"type": "thinking", "thinking": "fixture-thinking", "signature": "fixture-sig"},
        {"type": "text", "text": "fixture-answer"}]));
    import(
        dir,
        &[
            record(&m1, None, &asked(&text("fixture-question"))),
            record(&m2, Some(&m1), &reply),
        ],
    )
}

#[test]
fn second_translation_gives_equal_bytes() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = simple(dir.path())?;
    let first = translated(&home, &dir.path().join("o1"))?;
    let second = translated(&home, &dir.path().join("o2"))?;
    let rollout_one = std::fs::read(&first.rollout)?;
    let rollout_two = std::fs::read(&second.rollout)?;
    assert_eq!(Hash::of(&rollout_one), Hash::of(&rollout_two));
    let one = account(&first)?;
    let mut two = account(&second)?;
    let before = two.lost.len();
    two.lost.retain(|r| r.kind != "lys.translation");
    assert_eq!(before - two.lost.len(), 1);
    assert_eq!(serde_json::to_vec(&one)?, serde_json::to_vec(&two)?);
    Ok(())
}

/// `lys-home render` of session `s1` to `out`.
fn render(home: &Home, out: &Path) -> Found<PathBuf> {
    cli(&[
        "render".into(),
        "--home".into(),
        os(home.root()),
        "--session".into(),
        "s1".into(),
        "--uuid".into(),
        "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".into(),
        "--cwd".into(),
        "/w".into(),
        "--model".into(),
        "claude-fixture".into(),
        "--out".into(),
        os(out),
    ])?;
    Ok(out.to_path_buf())
}

#[test]
fn claude_code_render_is_unchanged_by_hash() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = simple(dir.path())?;
    let a = render(&home, &dir.path().join("a.jsonl"))?;
    translated(&home, &dir.path().join("o"))?;
    let b = render(&home, &dir.path().join("b.jsonl"))?;
    assert_eq!(Hash::of(&std::fs::read(a)?), Hash::of(&std::fs::read(b)?));
    Ok(())
}
