#![cfg(test)]
//! The fewshot writer as a library function (DIRECTORY-052 R3): turns written
//! through `lys_home::write_fewshot` read back record for record as the
//! `fewshot` command writes the same turns from a file, so a caller in another
//! crate writes exactly what the command writes. The second party is the
//! command itself, run through `run` from a turns file it parses on its own.

use std::error::Error;
use std::path::Path;

use serde_json::Value;

use lys_home::cli::{Cli, Command, run};
use lys_home::{HomeError, Role, write_fewshot};

type TestResult = Result<(), Box<dyn Error>>;

const TURNS: &str = concat!(
    "user: what is this team for?\n",
    "assistant: the identity screens\n",
    "user: who checks you?\n",
    "assistant: the reviewer\n",
);

fn read_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut records = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        records.push(serde_json::from_str(line)?);
    }
    Ok(records)
}

/// A record with the fields each write draws afresh (ids and instants) taken
/// out, so two writes of the same turns compare as one.
fn settled(record: &Value) -> Value {
    let mut record = record.clone();
    if let Some(fields) = record.as_object_mut() {
        for fresh in ["uuid", "parentUuid", "sessionId", "timestamp"] {
            fields.remove(fresh);
        }
    }
    record
}

fn turns() -> Vec<(Role, String)> {
    vec![
        (Role::User, "what is this team for?".to_owned()),
        (Role::Assistant, "the identity screens".to_owned()),
        (Role::User, "who checks you?".to_owned()),
        (Role::Assistant, "the reviewer".to_owned()),
    ]
}

#[test]
fn turns_written_through_the_library_read_back_as_the_command_writes_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let library = dir.path().join("library.jsonl");
    let written = write_fewshot(&library, &turns(), "/authored")?;
    assert_eq!(written, 4);

    let file = dir.path().join("turns.txt");
    std::fs::write(&file, TURNS)?;
    let command = dir.path().join("command.jsonl");
    let report = run(Cli {
        command: Command::Fewshot {
            out: command.clone(),
            turns: file,
            cwd: "/authored".into(),
        },
    })?;
    assert_eq!(report["records"], 4);

    let from_library = read_jsonl(&library)?;
    let from_command = read_jsonl(&command)?;
    assert_eq!(from_library.len(), 4);
    assert_eq!(from_library.len(), from_command.len());
    for (one, other) in from_library.iter().zip(&from_command) {
        assert_eq!(settled(one), settled(other));
    }
    assert_eq!(
        from_library[0]["message"]["content"],
        "what is this team for?"
    );
    assert_eq!(
        from_library[1]["message"]["content"][0]["text"],
        "the identity screens"
    );
    assert_eq!(from_library[0]["parentUuid"], Value::Null);
    for pair in from_library.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
        assert_eq!(pair[1]["sessionId"], pair[0]["sessionId"]);
    }
    Ok(())
}

#[test]
fn the_library_writer_never_writes_over_a_transcript() -> TestResult {
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("held.jsonl");
    std::fs::write(&out, "held\n")?;
    let refused = write_fewshot(&out, &turns(), "/authored");
    assert!(
        matches!(refused, Err(HomeError::Exists { .. })),
        "{refused:?}"
    );
    assert_eq!(std::fs::read_to_string(&out)?, "held\n");
    Ok(())
}
