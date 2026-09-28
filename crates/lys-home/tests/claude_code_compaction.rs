#![cfg(test)]
//! The compacted fixture end to end through the binary and the crate's
//! public surface (HOME-030 R7): two homes give the same loss entries but
//! for ids and stamps; every entry of each span is read back by id and the
//! listing finds nothing missing; the context path is the last compaction,
//! its kept entries and what came after; the render holds the boundary, the
//! summary and the kept and later records and no loss line; and the
//! fixture file is untouched by all of it. The fixture is hand-built and no
//! test name carries any of its content.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use lys_home::Home;

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const SESSION: &str = "compacted";

type Gate = Result<(), Box<dyn Error>>;

/// The fixture's uuid numbered `n`.
fn u(n: u8) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude_code_compacted.jsonl")
}

fn sha(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(Sha256::digest(std::fs::read(path)?).to_vec())
}

fn run(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(BIN).args(args).output()?)
}

/// Import the fixture into a new home at `home` with the binary.
fn import(home: &Path) -> Gate {
    let home = home.to_str().ok_or("home is not unicode")?;
    let source = fixture();
    let source = source.to_str().ok_or("fixture is not unicode")?;
    let out = run(&[
        "import",
        "--home",
        home,
        "--claude-code",
        source,
        "--session",
        SESSION,
    ])?;
    assert_eq!(out.status.code(), Some(0));
    Ok(())
}

/// The listing's report and exit status.
fn listing(home: &Path) -> Result<(Value, Option<i32>), Box<dyn Error>> {
    let home = home.to_str().ok_or("home is not unicode")?;
    let out = run(&["compactions", "--home", home, "--session", SESSION])?;
    Ok((serde_json::from_slice(&out.stdout)?, out.status.code()))
}

/// The session file's `lys.loss` lines without `id`, `parentId` and
/// `timestamp`, re-serialised.
fn loss_lines(home: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let text = std::fs::read_to_string(home.join("sessions").join(format!("{SESSION}.jsonl")))?;
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let mut entry: Map<String, Value> = serde_json::from_str(line)?;
        if entry.get("customType").and_then(Value::as_str) != Some("lys.loss") {
            continue;
        }
        for key in ["id", "parentId", "timestamp"] {
            entry.remove(key);
        }
        out.push(serde_json::to_string(&entry)?);
    }
    Ok(out)
}

#[test]
fn two_homes_write_the_same_loss_entries_but_for_ids_and_stamps() -> Gate {
    let dir = tempfile::tempdir()?;
    let (one, two) = (dir.path().join("one"), dir.path().join("two"));
    import(&one)?;
    import(&two)?;
    let (first, second) = (loss_lines(&one)?, loss_lines(&two)?);
    assert_eq!(first.len(), 2);
    let mut compared = 0;
    for (a, b) in first.iter().zip(&second) {
        assert_eq!(a.as_bytes(), b.as_bytes());
        compared += 1;
    }
    assert_eq!(compared, 2);
    assert_eq!(second.len(), 2);
    Ok(())
}

#[test]
fn every_span_entry_reads_by_id_and_the_context_is_the_last_compaction_on() -> Gate {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    import(&root)?;
    {
        let session = Home::open(&root)?.open_session(SESSION)?;
        let spans = [
            vec![u(1), u(2), u(3), u(4)],
            vec![u(5), u(6), u(8), u(9), u(0x0a)],
        ];
        let mut read = 0;
        for id in spans.iter().flatten() {
            assert_eq!(session.entry(id)?.id(), id);
            read += 1;
        }
        assert_eq!(read, 9);
        let context: Vec<String> = session
            .context_path()?
            .iter()
            .map(|e| e.id().to_owned())
            .collect();
        assert_eq!(context, [u(0x0e), u(0x0b), u(0x0c), u(0x0f), u(0x10)]);
    }
    let (report, status) = listing(&root)?;
    let all = report["compactions"].as_array().ok_or("no compactions")?;
    assert_eq!(all.len(), 2);
    for compaction in all {
        assert_eq!(compaction["entries_missing"], Value::Array(Vec::new()));
        assert_eq!(compaction["blocks_missing"], Value::Array(Vec::new()));
    }
    assert_eq!(status, Some(0));
    Ok(())
}

#[test]
fn the_render_holds_the_boundary_the_summary_and_the_kept_records_only() -> Gate {
    let before = sha(&fixture())?;
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    import(&root)?;
    let (_, status) = listing(&root)?;
    assert_eq!(status, Some(0));
    let out = dir.path().join("out").join("rendered.jsonl");
    let rendered = run(&[
        "render",
        "--home",
        root.to_str().ok_or("home is not unicode")?,
        "--session",
        SESSION,
        "--uuid",
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "--cwd",
        "/elsewhere",
        "--model",
        "claude-fixture",
        "--out",
        out.to_str().ok_or("out is not unicode")?,
    ])?;
    assert_eq!(rendered.status.code(), Some(0));
    let text = std::fs::read_to_string(&out)?;
    let records = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    let boundaries = records
        .iter()
        .filter(|r| r["subtype"] == "compact_boundary")
        .count();
    let summaries: Vec<&Value> = records
        .iter()
        .filter(|r| r["isCompactSummary"] == true)
        .collect();
    assert_eq!((boundaries, summaries.len()), (1, 1));
    let summary = summaries[0]["message"]["content"]
        .as_str()
        .ok_or("no summary")?;
    assert!(summary.contains("heliotrope"));
    let kept: Vec<&str> = records
        .iter()
        .skip(2)
        .filter_map(|r| r["uuid"].as_str())
        .collect();
    assert_eq!(kept, [u(0x0b), u(0x0c), u(0x0f), u(0x10)]);
    assert_eq!(records.len(), 6);
    assert_eq!(text.lines().filter(|l| l.contains("lys.loss")).count(), 0);
    assert_eq!(sha(&fixture())?, before);
    Ok(())
}

#[test]
fn a_listing_without_its_session_exits_2_naming_it() -> Gate {
    let out = run(&["compactions", "--home", "h"])?;
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr)?.contains("--session"));
    assert!(out.stdout.is_empty());
    Ok(())
}
