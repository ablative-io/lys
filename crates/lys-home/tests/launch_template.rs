#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `render-launch` end to end through the built binary (HOME-002 R7): the
//! five files with their recorded hashes, twice identical, the launch line,
//! the handle and never the value, the event beside the head, the kept
//! template, and each refusal with nothing written.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use lys_home::harness::claude_code::launch_env::env_settings;
use lys_home::harness::claude_code::template::parse_template;
use lys_home::{Home, Session};

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);
const SESSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);
const UUID: &str = "00000000-0000-4000-8000-000000000001";
const SECRET_VALUE: &str = "fixture-secret-value-0001";
const HANDLE: &str = "handle-fixture-0001";

/// The SHA-256 of each written file, recorded from the fixture session and
/// template; a change to the render, the environment file or the fixtures
/// changes these on purpose and nowhere else.
const RECORDED: [(&str, &str); 5] = [
    (
        "00000000-0000-4000-8000-000000000001.jsonl",
        "510c6704870200f2a946ee56b27b1a790adc8d6f957627cead4776628f1ae238",
    ),
    (
        "00000000-0000-4000-8000-000000000001.loss.json",
        "460f96f14dad62da55296f49be6dee44a82c5bf4a810ee3859190d83dd923a26",
    ),
    (
        "mcp.json",
        "d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02",
    ),
    (
        "env.json",
        "8dfb898a5cc2c3442f6f79c631ba9d5c504ad98ee85ece40bf19566f943ddc71",
    ),
    (
        "instructions.md",
        "b5bafaf8781a5e055b1215a2413f6c9f12ce85abd3afb55cdff731c14b11ffdc",
    ),
];

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
}

/// A fresh home holding the fixture session under the id `fixture`.
fn fresh_home(dir: &Path) -> PathBuf {
    let root = dir.join("home");
    let home = Home::open(&root).unwrap();
    std::fs::copy(SESSION, home.session_path("fixture").unwrap()).unwrap();
    root
}

fn launch(home: &Path, template: &Path, out: &Path) -> Output {
    Command::new(BIN)
        .args([
            "render-launch",
            "--home",
            home.to_str().unwrap(),
            "--session",
            "fixture",
            "--template",
            template.to_str().unwrap(),
            "--uuid",
            UUID,
            "--cwd",
            "/fixture",
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            out.to_str().unwrap(),
        ])
        .env("LYS_FIXTURE_TOKEN", SECRET_VALUE)
        .output()
        .unwrap()
}

fn names_in(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn write_template(dir: &Path, name: &str, edit: impl FnOnce(&mut Value)) -> PathBuf {
    let mut value: Value = serde_json::from_slice(&std::fs::read(TEMPLATE).unwrap()).unwrap();
    edit(&mut value);
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    path
}

fn keys_at_any_depth(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                keys_at_any_depth(v, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                keys_at_any_depth(item, out);
            }
        }
        _ => {}
    }
}

#[test]
fn a_missing_template_argument_exits_2_naming_it() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(BIN)
        .args([
            "render-launch",
            "--home",
            dir.path().to_str().unwrap(),
            "--session",
            "fixture",
            "--uuid",
            UUID,
            "--cwd",
            "/fixture",
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--template"), "{stderr}");
    assert!(output.stdout.is_empty());
    dir.close().unwrap();
}

#[test]
fn two_launches_write_the_recorded_files_the_launch_line_and_two_events_beside_the_head() {
    let dir = tempfile::tempdir().unwrap();
    let home = fresh_home(dir.path());
    let template_bytes = std::fs::read(TEMPLATE).unwrap();
    let template_hash = sha256_hex(&template_bytes);
    let mut reports = Vec::new();
    let mut hashes: Vec<Vec<(String, String)>> = Vec::new();
    for run in ["one", "two"] {
        let out = dir.path().join(run);
        std::fs::create_dir(&out).unwrap();
        let output = launch(&home, Path::new(TEMPLATE), &out);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "one report line");
        assert!(!stdout.contains(SECRET_VALUE));
        let report: Value = serde_json::from_str(&stdout).unwrap();
        let names = names_in(&out);
        let expected: BTreeSet<String> = RECORDED.iter().map(|(n, _)| (*n).to_owned()).collect();
        assert_eq!(names, expected);
        assert_eq!(names.len(), 5);
        let mut run_hashes = Vec::new();
        for (name, _) in RECORDED {
            let bytes = std::fs::read(out.join(name)).unwrap();
            assert_eq!(
                bytes
                    .windows(SECRET_VALUE.len())
                    .filter(|w| *w == SECRET_VALUE.as_bytes())
                    .count(),
                0,
                "{name}"
            );
            run_hashes.push((name.to_owned(), sha256_hex(&bytes)));
        }
        let files = report["files"].as_array().unwrap();
        assert_eq!(files.len(), 5);
        for (f, (name, hash)) in files.iter().zip(&run_hashes) {
            assert_eq!(f["path"], json!(out.join(name)));
            assert_eq!(f["sha256"], json!(hash));
        }
        let launch_line = format!(
            "claude --resume {o}/{UUID}.jsonl --fork-session --mcp-config {o}/mcp.json --settings {o}/env.json --append-system-prompt-file {o}/instructions.md --strict-mcp-config",
            o = out.display()
        );
        assert_eq!(report["launch"], json!(launch_line));
        assert_eq!(report["command"], "render-launch");
        assert_eq!(report["template"], json!(template_hash));
        assert_eq!(report["uuid"], UUID);
        assert_eq!(report["render"]["records"], 4);
        assert_eq!(report["manifest"].as_str().unwrap().len(), 64);
        let mut keys = Vec::new();
        keys_at_any_depth(&report, &mut keys);
        assert!(keys.len() > 10);
        for banned in ["text", "content", "body"] {
            assert_eq!(keys.iter().filter(|k| k.as_str() == banned).count(), 0);
        }
        let env: Value =
            serde_json::from_slice(&std::fs::read(out.join("env.json")).unwrap()).unwrap();
        assert_eq!(env["env"]["LYS_FIXTURE_TOKEN"], HANDLE);
        assert!(
            home.join("templates")
                .join(&template_hash[..2])
                .join(&template_hash)
                .is_file()
        );
        hashes.push(run_hashes);
        reports.push(report);
    }
    assert_eq!(hashes[0], hashes[1], "twice identical");
    assert_eq!(reports[0]["session_head"], reports[1]["session_head"]);
    for (name, recorded) in RECORDED {
        let got = &hashes[0].iter().find(|(n, _)| n == name).unwrap().1;
        assert_eq!(got, recorded, "{name}");
    }
    let session = Session::open(home.join("sessions").join("fixture.jsonl")).unwrap();
    assert_eq!(session.head().unwrap(), Some("e4"));
    let events = session.customs_everywhere("lys.harness_event").unwrap();
    assert_eq!(events.len(), 2);
    for (event, report) in events.iter().zip(&reports) {
        assert_eq!(event.parent_id(), Some("e4"));
        let lys_home::EntryBody::Custom {
            data: Some(data), ..
        } = &event.body
        else {
            panic!("a custom entry with data");
        };
        assert_eq!(data["kind"], "template_render");
        assert_eq!(data["detail"]["template"], json!(template_hash));
        assert_eq!(data["detail"]["session_head"], report["session_head"]);
        assert_eq!(data["detail"]["files"], 5);
        assert_eq!(data["record"], report["manifest"]);
        assert_eq!(event.id(), report["event"].as_str().unwrap());
    }
    assert_eq!(session.context_path().unwrap().len(), 4);
    drop(session);
    dir.close().unwrap();
}

#[test]
fn a_template_with_an_unknown_slot_is_refused_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let home = fresh_home(dir.path());
    let session_file = home.join("sessions").join("fixture.jsonl");
    let before = std::fs::read(&session_file).unwrap();
    let template = write_template(dir.path(), "voice.json", |v| {
        v["slots"]["voice"] = json!({});
    });
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let output = launch(&home, &template, &out);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("voice"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert_eq!(names_in(&out).len(), 0);
    assert!(!home.join("templates").exists());
    assert_eq!(std::fs::read(&session_file).unwrap(), before);
    dir.close().unwrap();
}

#[test]
fn a_template_with_a_readable_secret_is_refused_naming_the_unbuilt_reader() {
    let dir = tempfile::tempdir().unwrap();
    let home = fresh_home(dir.path());
    let template = write_template(dir.path(), "readable.json", |v| {
        v["slots"]["secrets"]["readable"] =
            json!([{"env": "LYS_FIXTURE_READ", "handle": "handle-fixture-0002"}]);
    });
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let output = launch(&home, &template, &out);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("secret_reader_unbuilt"), "{stderr}");
    assert!(stderr.contains("SECRETS-002"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert_eq!(names_in(&out).len(), 0);
    assert!(!home.join("templates").exists());
    dir.close().unwrap();
}

#[test]
fn an_out_directory_already_holding_env_json_is_refused_by_that_path() {
    let dir = tempfile::tempdir().unwrap();
    let home = fresh_home(dir.path());
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let existing = out.join("env.json");
    std::fs::write(&existing, b"{\"env\":{}}\n").unwrap();
    let output = launch(&home, Path::new(TEMPLATE), &out);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(existing.to_str().unwrap()), "{stderr}");
    assert!(output.stdout.is_empty());
    let names = names_in(&out);
    assert_eq!(names.len(), 1);
    assert!(names.contains("env.json"));
    assert_eq!(std::fs::read(&existing).unwrap(), b"{\"env\":{}}\n");
    let session = Session::open(home.join("sessions").join("fixture.jsonl")).unwrap();
    assert_eq!(
        session
            .customs_everywhere("lys.harness_event")
            .unwrap()
            .len(),
        0
    );
    drop(session);
    dir.close().unwrap();
}

#[test]
fn the_environment_file_holds_the_variables_and_the_handle_and_nothing_else_twice() {
    let template = parse_template(&std::fs::read(TEMPLATE).unwrap()).unwrap();
    let first = env_settings(&template).unwrap();
    let second = env_settings(&template).unwrap();
    assert_eq!(first, second);
    let value: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(
        value,
        json!({"env": {"LYS_FIXTURE_MODE": "fixture", "LYS_FIXTURE_TOKEN": HANDLE}})
    );
    assert_eq!(value.as_object().unwrap().len(), 1);
    assert_eq!(value["env"].as_object().unwrap().len(), 2);
    assert!(!String::from_utf8_lossy(&first).contains(SECRET_VALUE));
}
