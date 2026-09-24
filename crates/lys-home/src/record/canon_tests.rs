#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the canon: examples copied whole with their signatures behind a
//! `lys.inherited` entry, no example twice, no authored thinking, and a render
//! that places the canon first and applies the thinking rule to it.

use serde_json::{Value, json};

use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::Home;
use crate::record::canon::{AddReport, Inherited, add_authored, add_from, create, load};
use crate::record::entries::{CUSTOM_INHERITED, EntryBody};

fn thinking_exchange(home: &Home) -> (String, Vec<String>) {
    let mut s = home.create_session("source", "/w", None).unwrap();
    let u = s
        .append(EntryBody::Message {
            message: json!({"role": "user", "content": [{"type": "text", "text": "is it verified?"}], "timestamp": 1}),
        })
        .unwrap();
    let a = s
        .append(EntryBody::Message {
            message: json!({
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "check the gate first", "thinkingSignature": "sig-real"},
                    {"type": "text", "text": "not yet; running it now"}
                ],
                "api": "anthropic-messages", "provider": "anthropic", "model": "claude-opus-5-5",
                "usage": {}, "stopReason": "stop", "timestamp": 2
            }),
        })
        .unwrap();
    drop(s);
    ("source".into(), vec![u, a])
}

fn inherited_of(entry: &crate::record::entries::Entry) -> Inherited {
    let EntryBody::Custom {
        data: Some(data), ..
    } = &entry.body
    else {
        panic!("not an inherited entry")
    };
    serde_json::from_value(data.clone()).unwrap()
}

#[test]
fn an_example_is_copied_whole_behind_an_inherited_entry_and_never_twice() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let canon = dir.path().join("canon").join("canon.jsonl");
    create(&canon).unwrap();
    assert!(matches!(
        create(&canon),
        Err(crate::error::HomeError::Exists { .. })
    ));
    let (session, ids) = thinking_exchange(&home);
    let s = home.open_session(&session).unwrap();
    let report = add_from(&canon, &s, &ids, "verify before claiming", "tom").unwrap();
    assert_eq!(
        report,
        AddReport {
            canon: canon.clone(),
            inherited_id: report.inherited_id.clone(),
            messages: 2,
            thinking_copied: 1,
            examples: 1,
        }
    );
    let loaded = load(&canon).unwrap();
    assert_eq!(loaded.header.id, "canon");
    assert_eq!(loaded.entries.len(), 3);
    assert!(loaded.entries[0].is_custom(CUSTOM_INHERITED));
    let inherited = inherited_of(&loaded.entries[0]);
    assert_eq!(
        (
            inherited.authored,
            inherited.from_session.as_deref(),
            &inherited.from_entries,
            inherited.model.as_str(),
            inherited.curated_by.as_str(),
            inherited.rule.as_str()
        ),
        (
            false,
            Some("source"),
            &ids,
            "claude-opus-5-5",
            "tom",
            "verify before claiming"
        )
    );
    // The copies keep their ids and bodies; only the parent links chain onto the canon.
    assert_eq!(loaded.entries[1].id(), ids[0]);
    assert_eq!(
        loaded.entries[1].parent_id(),
        Some(report.inherited_id.as_str())
    );
    assert_eq!(loaded.entries[2].parent_id(), Some(ids[0].as_str()));
    let EntryBody::Message { message } = &loaded.entries[2].body else {
        panic!()
    };
    assert_eq!(message["content"][0]["thinkingSignature"], "sig-real");
    assert_eq!(message["content"][0]["thinking"], "check the gate first");
    let source = s.entry(&ids[1]).unwrap();
    let EntryBody::Message { message: original } = &source.body else {
        panic!()
    };
    assert_eq!(message, original, "copied whole");
    // The same example again is refused by the entry id it would repeat.
    let err = add_from(&canon, &s, &ids, "again", "tom").unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::DuplicateEntry { .. }),
        "{err}"
    );
    assert_eq!(
        load(&canon).unwrap().entries.len(),
        3,
        "nothing was written"
    );
    drop(s);
    dir.close().unwrap();
}

#[test]
fn an_authored_example_carries_authored_everywhere_and_a_thinking_block_is_refused_by_line() {
    let dir = tempfile::tempdir().unwrap();
    let canon = dir.path().join("canon.jsonl");
    create(&canon).unwrap();
    let bad = dir.path().join("bad.txt");
    std::fs::write(
        &bad,
        "user: hello\nassistant: {\"type\":\"thinking\",\"thinking\":\"made up\"}\n",
    )
    .unwrap();
    let err = add_authored(&canon, &bad, "no", "tom")
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("line 2") && err.contains("never authored"),
        "{err}"
    );
    assert_eq!(
        load(&canon).unwrap().entries.len(),
        0,
        "nothing was written"
    );
    let good = dir.path().join("good.txt");
    std::fs::write(
        &good,
        "user: state the gate\nassistant: fmt, clippy, test\n",
    )
    .unwrap();
    let report = add_authored(&canon, &good, "name the gate", "tom").unwrap();
    assert_eq!(
        (report.messages, report.thinking_copied, report.examples),
        (2, 0, 1)
    );
    let loaded = load(&canon).unwrap();
    let inherited = inherited_of(&loaded.entries[0]);
    assert!(inherited.authored && inherited.from_session.is_none());
    assert_eq!(
        (
            inherited.provider.as_str(),
            inherited.api.as_str(),
            inherited.model.as_str()
        ),
        ("authored", "authored", "authored")
    );
    let EntryBody::Message { message } = &loaded.entries[2].body else {
        panic!()
    };
    assert_eq!(
        (
            message["provider"].as_str(),
            message["api"].as_str(),
            message["model"].as_str()
        ),
        (Some("authored"), Some("authored"), Some("authored"))
    );
    dir.close().unwrap();
}

#[test]
fn a_render_with_the_canon_places_it_first_and_applies_the_thinking_rule_to_it() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let canon = dir.path().join("canon.jsonl");
    create(&canon).unwrap();
    let (session, ids) = thinking_exchange(&home);
    let s = home.open_session(&session).unwrap();
    add_from(&canon, &s, &ids, "verify before claiming", "tom").unwrap();
    drop(s);
    let mut own = home.create_session("own", "/w", None).unwrap();
    own.append(EntryBody::Message {
        message: json!({"role": "user", "content": [{"type": "text", "text": "new work"}], "timestamp": 3}),
    })
    .unwrap();
    let target = |model: &str, name: &str| RenderTarget {
        session_id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".into(),
        cwd: "/elsewhere".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(dir.path().join(name)),
        canon: Some(canon.clone()),
    };
    let same =
        render_claude_code(&own, &target("claude-opus-5-5", "same.jsonl"), dir.path()).unwrap();
    assert_eq!(
        (
            same.records,
            same.inherited,
            same.thinking_kept,
            same.dropped
        ),
        (3, 1, 1, 0)
    );
    let lines: Vec<Value> = std::fs::read_to_string(&same.path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        lines[0]["message"]["content"], "is it verified?",
        "one text part renders as Claude Code's string form"
    );
    assert_eq!(lines[1]["message"]["content"][0]["signature"], "sig-real");
    assert_eq!(
        lines[2]["message"]["content"], "new work",
        "the session's own entry comes last"
    );
    let mut prev: Option<&str> = None;
    for l in &lines {
        assert_eq!(
            l["parentUuid"].as_str(),
            prev,
            "one chain through the canon"
        );
        prev = l["uuid"].as_str();
    }
    let other =
        render_claude_code(&own, &target("claude-sonnet-5", "other.jsonl"), dir.path()).unwrap();
    assert_eq!(
        (other.records, other.thinking_kept, other.thinking_as_text),
        (3, 0, 1)
    );
    let account: Value = serde_json::from_slice(&std::fs::read(&other.loss_path).unwrap()).unwrap();
    assert_eq!(account["model"], "claude-sonnet-5");
    assert_eq!(
        account["dropped"].as_array().map(Vec::len),
        Some(1),
        "the readable thinking became text; its signature is opaque to another model and is named in the loss account"
    );
    assert_eq!(
        account["dropped"][0]["hash"].as_str().map(str::len),
        Some(64)
    );
    let lines: Vec<Value> = std::fs::read_to_string(&other.path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        lines[1]["message"]["content"][0]["type"], "text",
        "the inherited thinking rendered as text for another model"
    );
    assert!(lines[1]["message"]["content"][0].get("signature").is_none());
    drop(own);
    dir.close().unwrap();
}
