#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the launch template parser: the fixture parses to what the brief
//! states, each refusal names its cause, the parser and the schema file agree
//! on the five slots, and the hash is of the bytes as read.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::HomeError;
use crate::harness::claude_code::template::{SLOTS, parse_template};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);
const SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/design/home/launch-template.schema.json"
);

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(FIXTURE).unwrap()
}

fn fixture_value() -> Value {
    serde_json::from_slice(&fixture_bytes()).unwrap()
}

fn parse_value(
    value: &Value,
) -> Result<crate::harness::claude_code::template::Template, HomeError> {
    parse_template(&serde_json::to_vec(value).unwrap())
}

#[test]
fn the_fixture_parses_to_its_flags_fill_and_one_use_only_secret() {
    let template = parse_template(&fixture_bytes()).unwrap();
    assert_eq!(template.flags, ["--strict-mcp-config"]);
    assert_eq!(template.canon, None);
    assert_eq!(template.use_only.len(), 1);
    assert_eq!(template.use_only[0].env, "LYS_FIXTURE_TOKEN");
    assert_eq!(template.use_only[0].handle, "handle-fixture-0001");
    assert_eq!(template.env.len(), 1);
    assert_eq!(template.env["LYS_FIXTURE_MODE"], "fixture");
    assert!(template.mcp.contains_key("mcpServers"));
}

#[test]
fn an_unknown_slot_a_missing_slot_and_a_wrong_harness_or_fill_are_refused_by_name() {
    let mut voice = fixture_value();
    voice["slots"]["voice"] = json!({});
    let err = parse_value(&voice).unwrap_err();
    assert!(matches!(err, HomeError::UnknownSlot { .. }), "{err}");
    assert!(err.to_string().contains("voice"), "{err}");

    let mut missing = fixture_value();
    missing["slots"]
        .as_object_mut()
        .unwrap()
        .remove("instructions");
    let err = parse_value(&missing).unwrap_err();
    assert!(matches!(err, HomeError::MissingSlot { .. }), "{err}");
    assert!(err.to_string().contains("instructions"), "{err}");

    let mut codex = fixture_value();
    codex["harness"] = json!("codex");
    let err = parse_value(&codex).unwrap_err().to_string();
    assert!(err.contains("harness") && err.contains("codex"), "{err}");

    let mut fill = fixture_value();
    fill["slots"]["transcript"]["fill"] = json!("copy");
    let err = parse_value(&fill).unwrap_err().to_string();
    assert!(
        err.contains("transcript.fill") && err.contains("copy"),
        "{err}"
    );

    let mut extra = fixture_value();
    extra["voice"] = json!({});
    let err = parse_value(&extra).unwrap_err();
    assert!(matches!(err, HomeError::TemplateShape { .. }), "{err}");

    let mut bad_env = fixture_value();
    bad_env["slots"]["env"]["LYS_FIXTURE_N"] = json!(1);
    let err = parse_value(&bad_env).unwrap_err();
    assert!(matches!(err, HomeError::TemplateShape { .. }), "{err}");
}

#[test]
fn a_readable_secret_is_refused_naming_the_unbuilt_reader_and_its_variable() {
    let mut readable = fixture_value();
    readable["slots"]["secrets"]["readable"] =
        json!([{"env": "LYS_FIXTURE_READ", "handle": "handle-fixture-0002"}]);
    readable["slots"]["secrets"]["reader"] = json!("reader-command-text-never-printed");
    let err = parse_value(&readable).unwrap_err();
    assert!(
        matches!(err, HomeError::SecretReaderUnbuilt { .. }),
        "{err}"
    );
    let text = err.to_string();
    for needle in ["secret_reader_unbuilt", "SECRETS-002", "LYS_FIXTURE_READ"] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
    assert!(!text.contains("reader-command-text"), "{text}");
}

#[test]
fn a_variable_named_in_env_and_in_a_secret_is_refused_by_name() {
    let mut twice = fixture_value();
    twice["slots"]["env"]["LYS_FIXTURE_TOKEN"] = json!("x");
    let err = parse_value(&twice).unwrap_err();
    assert!(matches!(err, HomeError::DuplicateVariable { .. }), "{err}");
    assert!(err.to_string().contains("LYS_FIXTURE_TOKEN"), "{err}");
    let mut in_secrets = fixture_value();
    in_secrets["slots"]["secrets"]["use_only"] = json!([
        {"env": "LYS_FIXTURE_TOKEN", "handle": "handle-fixture-0001"},
        {"env": "LYS_FIXTURE_TOKEN", "handle": "handle-fixture-0003"}
    ]);
    let err = parse_value(&in_secrets).unwrap_err();
    assert!(matches!(err, HomeError::DuplicateVariable { .. }), "{err}");
}

#[test]
fn the_schema_file_names_exactly_the_slots_the_parser_accepts() {
    let schema: Value = serde_json::from_slice(&std::fs::read(SCHEMA).unwrap()).unwrap();
    let slots = &schema["properties"]["slots"];
    let named: BTreeSet<&str> = slots["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let accepted: BTreeSet<&str> = SLOTS.iter().copied().collect();
    assert_eq!(named, accepted);
    assert_eq!(slots["additionalProperties"], json!(false));
    let required: BTreeSet<&str> = slots["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(required, accepted);
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(schema["properties"]["harness"]["const"], "claude-code");
    assert_eq!(
        slots["properties"]["transcript"]["properties"]["fill"]["const"],
        "resume-by-path"
    );
}

#[test]
fn the_template_hash_is_the_sha256_of_the_file_bytes() {
    let bytes = fixture_bytes();
    let template = parse_template(&bytes).unwrap();
    let digest = Sha256::digest(&bytes);
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    });
    assert_eq!(template.hash.as_str(), hex);
    assert_eq!(hex.len(), 64);
}
