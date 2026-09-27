//! Gates on `translate-codex` (HOME-009 R7): the report's exact keys and
//! counts with no content in the report or the account, an existing target
//! refused with nothing written, an unmeasured Codex version refused, and
//! an unknown zone refused with nothing written. Each fixture is a synthetic
//! Claude Code file imported through `cli::run`; no test name carries
//! content.

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::Parser;
use serde_json::{Value, json};

use crate::cli::{Cli, run};
use crate::record::blocks::Hash;

type Gate = Result<(), Box<dyn Error>>;
type Found<T> = Result<T, Box<dyn Error>>;

const SENTINELS: [&str; 6] = [
    "fixture-sentinel-question",
    "fixture-sentinel-thinking",
    "fixture-sentinel-saying",
    "fixture-sentinel-argument",
    "fixture-sentinel-output",
    "fixture-sentinel-answer",
];
const U1: &str = "11111111-1111-4111-8111-111111111111";
const U2: &str = "22222222-2222-4222-8222-222222222222";
const U3: &str = "33333333-3333-4333-8333-333333333333";
const U4: &str = "44444444-4444-4444-8444-444444444444";

/// A Claude Code record of the message's role.
fn record(id: &str, parent: Option<&str>, message: &Value) -> Value {
    json!({
        "parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "cc", "version": "2.1.281", "gitBranch": "", "uuid": id,
        "timestamp": "2026-01-01T00:00:00.000Z", "type": message["role"], "message": message
    })
}

/// An assistant message of these parts.
fn answer(parts: &Value) -> Value {
    json!({"id": "msg_fixture", "type": "message", "role": "assistant",
        "model": "claude-fixture", "content": parts, "stop_reason": "end_turn",
        "stop_sequence": null, "usage": {"input_tokens": 1, "output_tokens": 1}})
}

/// A path as one command-line word.
fn os(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

fn command(words: &[OsString]) -> Found<Cli> {
    let mut argv = vec![OsString::from("lys-home")];
    argv.extend(words.iter().cloned());
    Ok(Cli::try_parse_from(argv)?)
}

/// A home under `dir` holding session `s1`: a question, a signed thinking
/// part with text and a tool call, its result, and an answer.
fn fixture_home(dir: &Path) -> Found<PathBuf> {
    let question = json!({"role": "user", "content": SENTINELS[0]});
    let saying = answer(&json!([
        {"type": "thinking", "thinking": SENTINELS[1], "signature": "c2ln"},
        {"type": "text", "text": SENTINELS[2]},
        {"type": "tool_use", "id": "toolu_1", "name": "Bash",
            "input": {"command": SENTINELS[3]}}
    ]));
    let result = json!({"role": "user", "content": [{"type": "tool_result",
        "tool_use_id": "toolu_1", "content": SENTINELS[4], "is_error": false}]});
    let last = answer(&json!([{"type": "text", "text": SENTINELS[5]}]));
    let records = [
        record(U1, None, &question),
        record(U2, Some(U1), &saying),
        record(U3, Some(U2), &result),
        record(U4, Some(U3), &last),
    ];
    let file = dir.join("claude-code.jsonl");
    let mut text = String::new();
    for value in &records {
        text.push_str(&serde_json::to_string(value)?);
        text.push('\n');
    }
    std::fs::write(&file, text)?;
    let home = dir.join("home");
    let import = command(&[
        "import".into(),
        "--home".into(),
        os(&home),
        "--claude-code".into(),
        os(&file),
        "--session".into(),
        "s1".into(),
    ])?;
    run(import)?;
    Ok(home)
}

/// The command line of a translation of session `s1`.
fn translation(home: &Path, out: &Path, version: &str, zone: &str) -> Found<Cli> {
    command(&[
        "translate-codex".into(),
        "--home".into(),
        os(home),
        "--session".into(),
        "s1".into(),
        "--out".into(),
        os(out),
        "--codex-version".into(),
        version.into(),
        "--zone".into(),
        zone.into(),
    ])
}

/// The refusal a translation ends in, as its message.
fn refusal(home: &Path, out: &Path, version: &str, zone: &str) -> Found<String> {
    match run(translation(home, out, version, zone)?) {
        Ok(_) => Err("the translation was not refused".into()),
        Err(refused) => Ok(refused.to_string()),
    }
}

/// Every file under a directory with its SHA-256, in path order.
fn files_under(dir: &Path) -> Found<Vec<(PathBuf, String)>> {
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
                let hash = Hash::of(&std::fs::read(&path)?).to_string();
                out.push((path, hash));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The length of one list of the account.
fn rows_in(account: &Value, list: &str) -> u64 {
    account[list].as_array().map_or(0, Vec::len) as u64
}

#[test]
fn report_carries_paths_and_counts() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    let out = dir.path().join("o");
    let printed = run(translation(&home, &out, "0.156.0", "UTC")?)?;
    assert_eq!(printed["command"], "translate-codex");
    let report = &printed["report"];
    let object = report.as_object().ok_or("no report")?;
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(
        keys.join(","),
        "account,blocks,changed,entries,kept,lost,rollout"
    );
    let account_path = report["account"].as_str().ok_or("no account path")?;
    let account_bytes = std::fs::read(account_path)?;
    let account: Value = serde_json::from_slice(&account_bytes)?;
    let mut rows = 0;
    let mut reported = 0;
    for list in ["kept", "changed", "lost"] {
        let count = report[list].as_u64().ok_or("a count is not a number")?;
        assert_eq!(count, rows_in(&account, list));
        reported += count;
        rows += rows_in(&account, list);
    }
    assert_eq!(reported, rows);
    assert!(rows > 0);
    assert_eq!(report["entries"], 4);
    assert_eq!(report["blocks"], 6);
    let printed_text = serde_json::to_string(&printed)?;
    let account_text = String::from_utf8(account_bytes)?;
    let rollout_path = report["rollout"].as_str().ok_or("no rollout path")?;
    let rollout = std::fs::read_to_string(rollout_path)?;
    let mut checked = 0;
    for sentinel in SENTINELS {
        assert!(!printed_text.contains(sentinel));
        assert!(!account_text.contains(sentinel));
        assert!(rollout.contains(sentinel));
        checked += 1;
    }
    assert_eq!(checked, SENTINELS.len());
    Ok(())
}

#[test]
fn existing_rollout_is_refused_and_nothing_written() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    let out = dir.path().join("o");
    let first = run(translation(&home, &out, "0.156.0", "UTC")?)?;
    let rollout = first["report"]["rollout"].as_str();
    let rollout = rollout.ok_or("no rollout path")?;
    let files = files_under(&out)?;
    assert_eq!(files.len(), 2);
    let session_file = home.join("sessions").join("s1.jsonl");
    let session_bytes = std::fs::read(&session_file)?;
    let refused = refusal(&home, &out, "0.156.0", "UTC")?;
    let tail = "choose another --out";
    let expected = format!("translate-codex target already exists: {rollout}; {tail}");
    assert_eq!(refused, expected);
    assert_eq!(files_under(&out)?, files);
    assert_eq!(std::fs::read(&session_file)?, session_bytes);
    Ok(())
}

#[test]
fn unmeasured_codex_version_is_refused() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    let out = dir.path().join("o");
    assert_eq!(
        refusal(&home, &out, "0.157.0", "UTC")?,
        "Codex 0.157.0 has no measured rollout shape: render for 0.156.0 or card a measurement of the new version"
    );
    assert!(files_under(&out)?.is_empty());
    Ok(())
}

#[test]
fn unknown_zone_is_refused_and_nothing_written() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    let out = dir.path().join("o");
    let refused = refusal(&home, &out, "0.156.0", "Mars/Olympus")?;
    assert!(refused.ends_with("set TZ to an IANA name"));
    assert!(files_under(&out)?.is_empty());
    Ok(())
}
