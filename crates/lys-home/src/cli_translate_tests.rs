#![cfg(test)]
//! Gates on `lys-home translate-codex` (HOME-009 R7): the report's exact
//! keys and counts with no content in it or in the account, and each refusal
//! leaving `--out` and the session as they were.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cli::{Cli, Command, run};
use crate::cli_translate::TranslateArgs;
use crate::record::blocks::Hash;

type Gate = Result<(), Box<dyn Error>>;

const CONTENT: [&str; 6] = [
    "fixture-question",
    "fixture-thinking",
    "fixture-argument",
    "fixture-output",
    "fixture-answer",
    "fixture-signature",
];

fn record(uuid: &str, parent: Option<&str>, message: &Value) -> Value {
    json!({"parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "s", "version": "2.1.281", "uuid": uuid, "timestamp": "2026-01-01T00:00:00.000Z",
        "type": message["role"], "message": message})
}

/// A home holding session `s1` imported from a synthetic Claude Code file.
fn imported() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let ids = [
        "10000000-0000-4000-8000-000000000001",
        "10000000-0000-4000-8000-000000000002",
        "10000000-0000-4000-8000-000000000003",
        "10000000-0000-4000-8000-000000000004",
    ];
    let records = [
        record(
            ids[0],
            None,
            &json!({"role": "user", "content": "fixture-question"}),
        ),
        record(
            ids[1],
            Some(ids[0]),
            &json!({"role": "assistant", "model": "m", "content": [
            {"type": "thinking", "thinking": "fixture-thinking", "signature": "fixture-signature"},
            {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {"command": "fixture-argument"}}]}),
        ),
        record(
            ids[2],
            Some(ids[1]),
            &json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": "fixture-output"}]}),
        ),
        record(
            ids[3],
            Some(ids[2]),
            &json!({"role": "assistant", "model": "m",
            "content": [{"type": "text", "text": "fixture-answer"}]}),
        ),
    ];
    let mut text = String::new();
    for r in &records {
        text.push_str(&serde_json::to_string(r)?);
        text.push('\n');
    }
    let source = dir.path().join("source.jsonl");
    std::fs::write(&source, text)?;
    let home = dir.path().join("home");
    run(Cli {
        command: Command::Import {
            home: home.clone(),
            claude_code: source,
            session: "s1".to_owned(),
        },
    })?;
    Ok((dir, home))
}

fn translate(
    home: &Path,
    out: &Path,
    version: &str,
    zone: &str,
) -> Result<Value, crate::HomeError> {
    run(Cli {
        command: Command::TranslateCodex(TranslateArgs {
            home: home.to_path_buf(),
            session: "s1".to_owned(),
            out: out.to_path_buf(),
            codex_version: version.to_owned(),
            zone: Some(zone.to_owned()),
        }),
    })
}

/// Every file under `dir` by path, with the SHA-256 of its bytes.
fn files(dir: &Path) -> Result<BTreeMap<PathBuf, String>, Box<dyn Error>> {
    let mut found = BTreeMap::new();
    if !dir.exists() {
        return Ok(found);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.insert(path.clone(), Hash::of(&std::fs::read(&path)?).to_string());
            }
        }
    }
    Ok(found)
}

#[test]
fn report_carries_paths_and_counts() -> Gate {
    let (dir, home) = imported()?;
    let printed = translate(&home, &dir.path().join("o"), "0.156.0", "UTC")?;
    assert_eq!(printed["command"], "translate-codex");
    let report = &printed["report"];
    let mut keys: Vec<&String> = report
        .as_object()
        .into_iter()
        .flat_map(|o| o.keys())
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "account", "blocks", "changed", "entries", "kept", "lost", "rollout"
        ]
    );
    let account_path = report["account"].as_str().ok_or("no account")?;
    assert!(Path::new(report["rollout"].as_str().ok_or("no rollout")?).is_file());
    let account_bytes = std::fs::read(account_path)?;
    let account: Value = serde_json::from_slice(&account_bytes)?;
    let rows: u64 = ["kept", "changed", "lost"]
        .iter()
        .map(|list| account[list].as_array().map_or(0, |a| a.len() as u64))
        .sum();
    let counted: u64 = ["kept", "changed", "lost"]
        .iter()
        .filter_map(|key| report[key].as_u64())
        .sum();
    assert_eq!(counted, rows);
    assert_eq!(report["entries"], 4);
    let printed = serde_json::to_string(&printed)?;
    let account_text = String::from_utf8(account_bytes)?;
    for content in CONTENT {
        assert!(!printed.contains(content), "{content}");
        assert!(!account_text.contains(content), "{content}");
    }
    Ok(())
}

#[test]
fn existing_rollout_is_refused_and_nothing_written() -> Gate {
    let (dir, home) = imported()?;
    let out = dir.path().join("o");
    let first = translate(&home, &out, "0.156.0", "UTC")?;
    let rollout = first["report"]["rollout"]
        .as_str()
        .ok_or("no rollout")?
        .to_owned();
    let session = home.join("sessions").join("s1.jsonl");
    let before = (files(&out)?, std::fs::read(&session)?);
    let refused = translate(&home, &out, "0.156.0", "UTC");
    assert_eq!(
        refused.err().map(|e| e.to_string()),
        Some(format!(
            "translate-codex target already exists: {rollout}; choose another --out"
        ))
    );
    assert_eq!((files(&out)?, std::fs::read(&session)?), before);
    Ok(())
}

#[test]
fn unmeasured_codex_version_is_refused() -> Gate {
    let (dir, home) = imported()?;
    let out = dir.path().join("o");
    let refused = translate(&home, &out, "0.157.0", "UTC");
    assert_eq!(
        refused.err().map(|e| e.to_string()).as_deref(),
        Some(
            "Codex 0.157.0 has no measured rollout shape: render for 0.156.0 or card a measurement of the new version"
        )
    );
    assert!(files(&out)?.is_empty());
    Ok(())
}

#[test]
fn unknown_zone_is_refused_and_nothing_written() -> Gate {
    let (dir, home) = imported()?;
    let out = dir.path().join("o");
    let refused = translate(&home, &out, "0.156.0", "Mars/Olympus");
    let message = refused.err().map(|e| e.to_string()).ok_or("not refused")?;
    assert!(message.ends_with("set TZ to an IANA name"), "{message}");
    assert!(files(&out)?.is_empty());
    Ok(())
}
