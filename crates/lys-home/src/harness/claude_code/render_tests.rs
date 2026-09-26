#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the Claude Code renderer: the chain, the same-model thinking rule,
//! the loss account, the refusal of an existing path, and the round trip.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::import_tests::fixture;
use crate::harness::claude_code::render::{
    RENDER_NAMESPACE, RenderTarget, record_uuid, render_claude_code, uuid_string, uuid_v5,
};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBase, EntryBody};

fn target(model: &str, out: std::path::PathBuf) -> RenderTarget {
    RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into(),
        cwd: "/elsewhere".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(out),
        canon: None,
    }
}

fn message_hashes(s: &crate::record::Session) -> Vec<String> {
    s.context_path()
        .unwrap()
        .iter()
        .filter_map(|e| match &e.body {
            EntryBody::Message { message } => {
                Some(Hash::of(&serde_json::to_vec(&message["content"]).unwrap()).to_string())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn same_model_keeps_signed_thinking_and_the_round_trip_keeps_every_block() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    std::fs::write(&src, fixture()).unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("one", "/w", None).unwrap();
    import_claude_code(&src, &mut s, &blocks).unwrap();
    let out = dir.path().join("out").join("same.jsonl");
    let report = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!(
        (
            report.records,
            report.thinking_kept,
            report.dropped,
            report.authored
        ),
        (4, 1, 0, false)
    );
    let lines: Vec<Value> = std::fs::read_to_string(&out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    for pair in lines.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
        assert_eq!(pair[1]["sessionId"], "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
    }
    assert_eq!(lines[1]["message"]["content"][0]["signature"], "sig-abc");
    assert_eq!(lines[2]["message"]["content"][0]["type"], "tool_result");
    // Re-import the rendered file: the same content hashes on the path.
    let mut again = home.create_session("two", "/w", None).unwrap();
    import_claude_code(&out, &mut again, &blocks).unwrap();
    assert_eq!(message_hashes(&s), message_hashes(&again));
    // Existing path refused, bytes untouched.
    let before = std::fs::read(&out).unwrap();
    let err = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("same.jsonl"), "{err}");
    assert_eq!(std::fs::read(&out).unwrap(), before);
    drop((s, again));
    dir.close().unwrap();
}

#[test]
fn a_different_model_gets_thinking_as_text_and_a_loss_account_entry() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    std::fs::write(&src, fixture()).unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("one", "/w", None).unwrap();
    import_claude_code(&src, &mut s, &blocks).unwrap();
    let out = dir.path().join("other.jsonl");
    let report = render_claude_code(
        &s,
        &target("claude-sonnet-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!(
        (
            report.thinking_kept,
            report.thinking_as_text,
            report.dropped
        ),
        (0, 1, 1)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(!text.contains("\"signature\""));
    let account: Value = serde_json::from_slice(&std::fs::read(report.loss_path).unwrap()).unwrap();
    assert_eq!(account["dropped"].as_array().unwrap().len(), 1);
    assert_eq!(account["dropped"][0]["hash"].as_str().unwrap().len(), 64);
    assert_eq!(account["authored"], false);
    drop(s);
    dir.close().unwrap();
}

/// A session of message entries with the given ids, each under the one before.
fn session_of(home: &Home, session: &str, ids: &[&str]) -> crate::record::Session {
    let mut s = home.create_session(session, "/w", None).unwrap();
    let mut prev: Option<String> = None;
    for (n, id) in ids.iter().enumerate() {
        let role = if n % 2 == 0 { "user" } else { "assistant" };
        let message = if role == "user" {
            serde_json::json!({"role": "user", "content": [{"type": "text", "text": format!("turn {n}")}], "timestamp": 0})
        } else {
            serde_json::json!({"role": "assistant", "content": [{"type": "text", "text": format!("turn {n}")}],
                "api": "anthropic-messages", "provider": "anthropic", "model": "claude-opus-5-5", "stopReason": "stop", "timestamp": 0})
        };
        s.append_entry(&Entry {
            base: EntryBase {
                id: (*id).to_owned(),
                parent_id: prev.clone(),
                timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            },
            body: EntryBody::Message { message },
        })
        .unwrap();
        prev = Some((*id).to_owned());
    }
    s
}

#[test]
fn a_session_renders_byte_identically_twice_with_distinct_chained_uuids() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "det", &["e1", "e2", "e3", "e4"]);
    let first = dir.path().join("first").join("r.jsonl");
    let second = dir.path().join("second").join("r.jsonl");
    let a = render_claude_code(
        &s,
        &target("claude-opus-5-5", first.clone()),
        Some(dir.path()),
    )
    .unwrap();
    let b = render_claude_code(
        &s,
        &target("claude-opus-5-5", second.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!((a.records, b.records), (4, 4));
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
    assert_eq!(
        std::fs::read(&a.loss_path).unwrap(),
        std::fs::read(&b.loss_path).unwrap()
    );
    let lines: Vec<Value> = std::fs::read_to_string(&first)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 4);
    let uuids: std::collections::BTreeSet<&str> =
        lines.iter().map(|l| l["uuid"].as_str().unwrap()).collect();
    assert_eq!(uuids.len(), 4);
    for u in &uuids {
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "5", "a derived uuid is version 5");
        assert!(matches!(&u[19..20], "8" | "9" | "a" | "b"), "{u}");
    }
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    for pair in lines.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
    }
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_uuid_shaped_entry_id_is_kept_and_any_other_maps_to_one_fixed_uuid() {
    const KEPT: &str = "5f0c0b8e-2a1d-4c3b-9e7f-0123456789ab";
    const ONES: &str = "11111111-1111-4111-8111-111111111111";
    assert_eq!(record_uuid("kept", KEPT), KEPT);
    assert_eq!(record_uuid("kept", ONES), ONES);
    assert_eq!(record_uuid("kept", "e1"), record_uuid("kept", "e1"));
    assert_ne!(record_uuid("kept", "e1"), record_uuid("kept", "e2"));
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "kept", &["e1", ONES, KEPT]);
    let out = dir.path().join("kept.jsonl");
    let report = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!(report.records, 3);
    let lines: Vec<Value> = std::fs::read_to_string(&out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[1]["uuid"], ONES);
    assert_eq!(lines[2]["uuid"], KEPT);
    assert_eq!(lines[1]["parentUuid"], lines[0]["uuid"]);
    assert_eq!(lines[2]["parentUuid"], ONES);
    assert_ne!(lines[0]["uuid"], "e1");
    drop(s);
    dir.close().unwrap();
}

/// The uuid of the first record of a one-entry session `session` whose entry
/// id is `id`, read back from the rendered file.
fn rendered_uuid(dir: &std::path::Path, home: &Home, session: &str, id: &str) -> String {
    let s = session_of(home, session, &[id]);
    let out = dir.join(format!("{session}.jsonl"));
    let report =
        render_claude_code(&s, &target("claude-opus-5-5", out.clone()), Some(dir)).unwrap();
    assert_eq!(report.records, 1);
    let line: Value = serde_json::from_str(std::fs::read_to_string(&out).unwrap().trim()).unwrap();
    drop(s);
    line["uuid"].as_str().unwrap().to_owned()
}

/// RFC 9562's UUID version 5, written out here with the sha1 crate directly so the
/// render's own `uuid_v5` is checked against a second spelling of the rule.
fn uuid5_by_hand(namespace: &[u8; 16], name: &str) -> String {
    use std::fmt::Write as _;
    let mut input = namespace.to_vec();
    input.extend_from_slice(name.as_bytes());
    let digest = sha1::Sha1::digest(&input);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

#[test]
fn the_render_namespace_is_the_uuid5_of_the_url_namespace_over_the_lys_name() {
    const URL_NAMESPACE: [u8; 16] = [
        0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ];
    const NAME: &str = "lys/home/claude-code/render-uuid/v1";
    const EXPECTED: &str = "32c05904-d1f1-550c-9eee-2f6c8f98b665";
    assert_eq!(uuid_string(&RENDER_NAMESPACE), EXPECTED);
    assert_eq!(uuid5_by_hand(&URL_NAMESPACE, NAME), EXPECTED);
    assert_eq!(
        uuid_string(&uuid_v5(&URL_NAMESPACE, NAME.as_bytes())),
        EXPECTED
    );
}

#[test]
fn a_non_uuid_entry_id_derives_the_uuid5_under_the_session_namespace_over_the_id_and_its_role() {
    // Python: uuid.uuid5(uuid.uuid5(UUID("32c05904-…"), "one"), "u1#record").
    const EXPECTED: &str = "8614322e-bd70-5b0c-baa8-571010e52a8c";
    assert_eq!(record_uuid("one", "u1"), EXPECTED);
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    assert_eq!(rendered_uuid(dir.path(), &home, "one", "u1"), EXPECTED);
    dir.close().unwrap();
}

#[test]
fn the_same_entry_id_in_two_sessions_derives_two_uuids_salted_by_the_session_id() {
    // Python: uuid.uuid5(uuid.uuid5(UUID("32c05904-…"), "two"), "u1#record").
    const ONE: &str = "8614322e-bd70-5b0c-baa8-571010e52a8c";
    const TWO: &str = "9c843336-82b9-5a39-b7cd-2889252f8e4e";
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let a = rendered_uuid(dir.path(), &home, "one", "u1");
    let b = rendered_uuid(dir.path(), &home, "two", "u1");
    assert_eq!((a.as_str(), b.as_str()), (ONE, TWO));
    assert_ne!(a, b, "the session id is the salt");
    assert_eq!(record_uuid("two", "u1"), TWO);
    dir.close().unwrap();
}

#[test]
fn the_render_reads_no_clock_and_no_random_source() {
    const SOURCE: &str = include_str!("render.rs");
    const FORBIDDEN: [&str; 6] = [
        "fresh_id",
        "rand::",
        "now(",
        "SystemTime",
        "OffsetDateTime",
        "Instant",
    ];
    let found: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|token| SOURCE.contains(token))
        .collect();
    assert_eq!(
        found,
        Vec::<&str>::new(),
        "render.rs names a clock or a random source"
    );
    assert_eq!(FORBIDDEN.len(), 6);
}

#[test]
fn a_render_with_no_out_and_no_home_directory_is_refused_by_name_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "noplace", &["e1", "e2"]);
    let mut target = target("claude-opus-5-5", dir.path().join("unused.jsonl"));
    target.out = None;
    let before: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let err = render_claude_code(&s, &target, None).unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::NoRenderPlace),
        "{err}"
    );
    assert!(err.to_string().contains("--out"), "{err}");
    let after: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(before, after);
    assert_eq!(before.len(), 1, "only the home");
    let report = render_claude_code(&s, &target, Some(dir.path())).unwrap();
    assert!(
        report
            .path
            .starts_with(dir.path().join(".claude").join("projects"))
    );
    assert_eq!(report.records, 2);
    drop(s);
    dir.close().unwrap();
}

/// The determinism fixture: a user record holding exactly two tool results,
/// and one holding a tool result beside the user's own text, so the importer
/// writes two `-r0` entries whose uuids the render derives.
const MULTI_RESULT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/multi_result.jsonl"
);

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
}

#[test]
fn the_multi_result_fixture_renders_twice_to_equal_bytes_with_derived_distinct_chained_uuids() {
    // The two derived values: Python's uuid.uuid5(uuid.uuid5(UUID("32c05904-…"), "multi"),
    // "<entry id>#record") over the importer's `-r0` ids.
    const EXPECTED: [&str; 8] = [
        "11111111-1111-4111-8111-111111111111",
        "22222222-2222-4222-8222-222222222222",
        "d0426444-d38e-5376-aef6-035f7c3634e1",
        "33333333-3333-4333-8333-333333333333",
        "44444444-4444-4444-8444-444444444444",
        "dcc867fa-10a3-5142-89a0-38367729ae4b",
        "55555555-5555-4555-8555-555555555555",
        "66666666-6666-4666-8666-666666666666",
    ];
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("multi", "/w", None).unwrap();
    import_claude_code(std::path::Path::new(MULTI_RESULT), &mut s, &blocks).unwrap();
    // The importer's ids on the path, unchanged: the split results carry `-r0`.
    let ids: Vec<String> = s
        .context_path()
        .unwrap()
        .iter()
        .filter(|e| matches!(e.body, EntryBody::Message { .. }))
        .map(|e| e.id().to_owned())
        .collect();
    assert_eq!(
        ids,
        [
            "11111111-1111-4111-8111-111111111111",
            "22222222-2222-4222-8222-222222222222",
            "33333333-3333-4333-8333-333333333333-r0",
            "33333333-3333-4333-8333-333333333333",
            "44444444-4444-4444-8444-444444444444",
            "55555555-5555-4555-8555-555555555555-r0",
            "55555555-5555-4555-8555-555555555555",
            "66666666-6666-4666-8666-666666666666",
        ]
    );
    let a_path = dir.path().join("a.jsonl");
    let b_path = dir.path().join("b.jsonl");
    let a = render_claude_code(
        &s,
        &target("claude-opus-5-5", a_path.clone()),
        Some(dir.path()),
    )
    .unwrap();
    let b = render_claude_code(
        &s,
        &target("claude-opus-5-5", b_path.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!((a.records, b.records), (8, 8));
    let a_bytes = std::fs::read(&a_path).unwrap();
    assert_eq!(
        sha256_hex(&a_bytes),
        sha256_hex(&std::fs::read(&b_path).unwrap()),
        "two renders, one SHA-256"
    );
    assert_eq!(
        std::fs::read(&a.loss_path).unwrap(),
        std::fs::read(&b.loss_path).unwrap()
    );
    let lines: Vec<Value> = std::str::from_utf8(&a_bytes)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let uuids: Vec<&str> = lines.iter().map(|l| l["uuid"].as_str().unwrap()).collect();
    assert_eq!(uuids, EXPECTED);
    let distinct: std::collections::BTreeSet<&str> = uuids.iter().copied().collect();
    assert_eq!(distinct.len(), 8);
    for u in &uuids {
        assert_eq!(u.len(), 36);
        for (i, b) in u.bytes().enumerate() {
            if matches!(i, 8 | 13 | 18 | 23) {
                assert_eq!(b, b'-', "{u}");
            } else {
                assert!(b.is_ascii_hexdigit(), "{u}");
            }
        }
    }
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    for pair in lines.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
    }
    assert_eq!(lines.windows(2).count(), 7);
    drop(s);
    dir.close().unwrap();
}
