//! The lantern subcommands run as the binary (HOME-004 R6): each report's
//! exact keys, each refusal's exit code with nothing on stdout, clap's
//! refusals, and no transcript line anywhere in the output.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use lys_home::{Entry, EntryBase, EntryBody, Home};

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const FIXTURE: &str = "fixture-lantern";
const TRANSCRIPT_LINE: &str = "FIXTURE-TRANSCRIPT-LINE-q7";
const NOTE: &str = "The Fold Held Under Replay";
const WORDS: &str = "a later word: cobalt";

type Gate = Result<(), Box<dyn Error>>;

/// A fresh home holding `fixture-lantern` with five message entries e1 to
/// e5, the third carrying the transcript line.
fn fixture_home(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let root = dir.join("home");
    let home = Home::open(&root)?;
    let mut session = home.create_session(FIXTURE, "/fixture", None)?;
    let mut prev: Option<String> = None;
    for (n, id) in ["e1", "e2", "e3", "e4", "e5"].iter().enumerate() {
        let text = if n == 2 { TRANSCRIPT_LINE } else { "fixture" };
        session.append_entry(&Entry {
            base: EntryBase {
                id: (*id).to_owned(),
                parent_id: prev.take(),
                timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            },
            body: EntryBody::Message {
                message: serde_json::json!({"role": "user", "content": [{"type": "text", "text": text}], "timestamp": 0}),
            },
        })?;
        prev = Some((*id).to_owned());
    }
    Ok(root)
}

fn lantern(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(BIN).arg("lantern").args(args).output()?)
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort_unstable();
    keys
}

fn one_object(output: &Output) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout.clone())?;
    assert_eq!(text.lines().count(), 1);
    Ok(serde_json::from_str(text.trim())?)
}

fn refused(output: &Output, code: i32) -> Result<String, Box<dyn Error>> {
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty());
    Ok(String::from_utf8(output.stderr.clone())?)
}

fn head_of(home: &Path) -> Result<(String, Entry), Box<dyn Error>> {
    let session = Home::open(home)?.open_session(FIXTURE)?;
    let head = session.head()?.ok_or("no head")?.to_owned();
    let entry = session.entry(&head)?;
    Ok((head, entry))
}

#[test]
fn light_epilogue_and_recall_print_their_reports_and_refuse_by_name() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path())?;
    let home_arg = home.to_str().ok_or("home path is not text")?;
    let mut outputs: Vec<Output> = Vec::new();

    let lit = lantern(&[
        "light",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--point",
        "e2",
        "--note",
        NOTE,
        "--by",
        "fixture-lighter",
    ])?;
    let report = one_object(&lit)?;
    assert_eq!(sorted_keys(&report), ["id", "lit_at", "point", "session"]);
    assert_eq!(report["session"], FIXTURE);
    assert_eq!(report["point"], "e2");
    let l1 = report["id"].as_str().ok_or("no id")?.to_owned();
    assert_eq!(head_of(&home)?.0, l1);
    assert!(!String::from_utf8_lossy(&lit.stdout).contains(NOTE));
    outputs.push(lit);

    let unknown = lantern(&[
        "light",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--point",
        "no-such-entry",
        "--note",
        NOTE,
        "--by",
        "fixture-lighter",
    ])?;
    assert!(refused(&unknown, 1)?.contains("no-such-entry"));
    outputs.push(unknown);

    let blank = lantern(&[
        "light",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--point",
        "e2",
        "--note",
        "",
        "--by",
        "fixture-lighter",
    ])?;
    refused(&blank, 1)?;
    outputs.push(blank);

    let added = lantern(&[
        "epilogue",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--lantern",
        &l1,
        "--words",
        WORDS,
        "--by",
        "fixture-annotator",
    ])?;
    let report = one_object(&added)?;
    assert_eq!(
        sorted_keys(&report),
        [
            "added_at", "added_by", "id", "lantern", "ordinal", "session"
        ]
    );
    assert_eq!(report["lantern"], l1);
    assert_eq!(report["ordinal"], 1);
    let (head, entry) = head_of(&home)?;
    assert_eq!(report["id"], head);
    let EntryBody::Custom { custom_type, data } = entry.body else {
        return Err("the head is not a custom entry".into());
    };
    assert_eq!(custom_type, "lys.lantern_epilogue");
    assert_eq!(data.ok_or("no data")?["lantern"], l1);
    assert!(!String::from_utf8_lossy(&added.stdout).contains("cobalt"));
    outputs.push(added);

    let by_note = lantern(&["recall", "--home", home_arg, "--note", "cobalt"])?;
    let report = one_object(&by_note)?;
    assert_eq!(sorted_keys(&report), ["lanterns", "skipped"]);
    let rows = report["lanterns"].as_array().ok_or("no rows")?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.first().ok_or("no row")?["id"], l1);
    assert_eq!(report["skipped"].as_array().map(Vec::len), Some(0));
    outputs.push(by_note);

    let by_point = lantern(&[
        "recall",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--point",
        "e2",
    ])?;
    let report = one_object(&by_point)?;
    let ids: Vec<&Value> = report["lanterns"]
        .as_array()
        .ok_or("no rows")?
        .iter()
        .map(|row| &row["id"])
        .collect();
    assert_eq!(ids, [&Value::from(l1.as_str())]);
    outputs.push(by_point);

    let blank_words = lantern(&["recall", "--home", home_arg, "--note", ""])?;
    assert!(refused(&blank_words, 1)?.contains("words"));
    outputs.push(blank_words);

    let both = lantern(&[
        "recall",
        "--home",
        home_arg,
        "--note",
        "fold",
        "--session",
        FIXTURE,
        "--point",
        "e2",
    ])?;
    refused(&both, 2)?;
    outputs.push(both);

    let no_by = lantern(&[
        "light",
        "--home",
        home_arg,
        "--session",
        FIXTURE,
        "--point",
        "e2",
        "--note",
        NOTE,
    ])?;
    refused(&no_by, 2)?;
    outputs.push(no_by);

    assert_eq!(outputs.len(), 9);
    for output in &outputs {
        let all = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(!all.contains(TRANSCRIPT_LINE));
    }
    Ok(())
}

/// The names under a directory, or none when it is absent.
fn names_under(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, Box<dyn Error>>>()?;
    names.sort();
    Ok(names)
}

#[test]
fn an_absent_or_incomplete_home_is_refused_by_name_and_nothing_is_made() -> Gate {
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("nowhere");
    let incomplete = dir.path().join("partial");
    std::fs::create_dir(&incomplete)?;
    std::fs::write(incomplete.join("notes.txt"), "not a home\n")?;
    let mut runs = 0;
    for home in [&absent, &incomplete] {
        let home_arg = home.to_str().ok_or("home path is not text")?;
        let before = names_under(home)?;
        for args in [
            vec!["recall", "--home", home_arg, "--note", "fold"],
            vec![
                "recall",
                "--home",
                home_arg,
                "--session",
                FIXTURE,
                "--point",
                "e2",
            ],
            vec![
                "light",
                "--home",
                home_arg,
                "--session",
                FIXTURE,
                "--point",
                "e2",
                "--note",
                NOTE,
                "--by",
                "fixture-lighter",
            ],
            vec![
                "epilogue",
                "--home",
                home_arg,
                "--session",
                FIXTURE,
                "--lantern",
                "l1",
                "--words",
                WORDS,
                "--by",
                "fixture-annotator",
            ],
        ] {
            let output = lantern(&args)?;
            let stderr = refused(&output, 1)?;
            assert!(stderr.contains("sessions/"), "{stderr}");
            assert!(stderr.contains(home_arg), "{stderr}");
            assert_eq!(names_under(home)?, before);
            runs += 1;
        }
    }
    assert_eq!(runs, 8);
    assert!(!absent.exists());
    assert!(!incomplete.join("sessions").exists());
    assert!(!incomplete.join("blocks").exists());
    Ok(())
}
