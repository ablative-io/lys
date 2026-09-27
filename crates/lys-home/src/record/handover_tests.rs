#![cfg(test)]
//! Gates on the handover (HOME-015): the refusals by name, the canon's
//! rule, the letter taken from the outgoing session and refused before
//! anything is written, the successor home written with the letter copied
//! whole, the `handover` subcommand, and the successor rendered for its own
//! model and for another. The fixture home holds one session `outgoing`
//! whose root-to-head path is `u1`, `a1`, `a2`, `u2`, `a3`, `x1`, with `b1`
//! a side branch off `a1`. No test name carries a content sentinel.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cli::{Cli, Command, run};
use crate::error::HomeError;
use crate::harness::claude_code::render::{RenderReport, RenderTarget, render_claude_code};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::canon::{Inherited, add_authored, load};
use crate::record::entries::{CUSTOM_INHERITED, Entry, EntryBase, EntryBody, INHERITED_FROM};
use crate::record::handover::{HandoverReport, handover};
use crate::record::index::{Index, read_head};
use crate::record::reader::SessionReader;

type Gate = Result<(), Box<dyn Error>>;

/// The fixture's outgoing session.
const OUTGOING: &str = "outgoing";
/// The prefixes of the fixture's content, none of which a report or an
/// error may carry.
const SENTINELS: [&str; 3] = ["fixture-text-", "fixture-thinking-", "fixture-signature-"];

fn text(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

fn thinking(id: &str) -> Value {
    json!({"type": "thinking", "thinking": format!("fixture-thinking-{id}"),
        "thinkingSignature": format!("fixture-signature-{id}")})
}

fn entry(id: &str, parent: Option<&str>, message: Value) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            timestamp: format!("fixture-time-{id}"),
        },
        body: EntryBody::Message { message },
    }
}

fn user(id: &str, parent: Option<&str>) -> Entry {
    let message = json!({"role": "user", "content": [text(&format!("fixture-text-{id}"))]});
    entry(id, parent, message)
}

fn assistant(id: &str, parent: &str, part: &Value, who: [&str; 3]) -> Entry {
    let [provider, api, model] = who;
    let message = json!({"role": "assistant", "content": [part], "provider": provider,
        "api": api, "model": model});
    entry(id, Some(parent), message)
}

const ANTHROPIC: [&str; 3] = ["anthropic", "anthropic-messages", "claude-fixture-model"];

/// A fresh directory holding the fixture home at `home/`; returns the
/// directory and the home's root.
fn fixture() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    let mut s = home.create_session(OUTGOING, "/work/outgoing", None)?;
    let body = |id: &str| text(&format!("fixture-text-{id}"));
    s.append_entry(&user("u1", None))?;
    s.append_entry(&assistant("a1", "u1", &thinking("a1"), ANTHROPIC))?;
    s.append_entry(&assistant("a2", "a1", &body("a2"), ANTHROPIC))?;
    s.append_entry(&user("u2", Some("a2")))?;
    s.append_entry(&assistant("a3", "u2", &body("a3"), ANTHROPIC))?;
    s.append_entry(&assistant("b1", "a1", &thinking("b1"), ANTHROPIC))?;
    let authored = ["authored"; 3];
    s.append_entry(&assistant("x1", "a3", &body("x1"), authored))?;
    Ok((dir, root))
}

/// Every file under `root` by its path relative to `root`, with its SHA-256.
fn snapshot(root: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for item in std::fs::read_dir(&dir)? {
            let path = item?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let name = path.strip_prefix(root)?.to_string_lossy().into_owned();
                files.insert(name, Hash::of(&std::fs::read(&path)?).to_string());
            }
        }
    }
    Ok(files)
}

fn carries_no_content(shown: &str) -> bool {
    SENTINELS.iter().all(|s| !shown.contains(s))
}

/// The rule-less data R2 names.
fn ruleless(rule: Option<&str>) -> Inherited {
    Inherited {
        authored: false,
        from_session: Some("s".into()),
        from_entries: vec!["e".into()],
        provider: "p".into(),
        api: "a".into(),
        model: "m".into(),
        curated_at: "t".into(),
        curated_by: "b".into(),
        rule: rule.map(str::to_owned),
    }
}

fn inherited_entry(id: &str) -> Result<Entry, Box<dyn Error>> {
    Ok(Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: None,
            timestamp: "t".to_owned(),
        },
        body: EntryBody::Custom {
            custom_type: CUSTOM_INHERITED.to_owned(),
            data: Some(serde_json::to_value(ruleless(None))?),
        },
    })
}

/// A canon file of a header line and one rule-less `lys.inherited` entry `k1`.
fn ruleless_canon(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("canon.jsonl");
    let header =
        json!({"type": "session", "version": 2, "id": "canon", "timestamp": "t", "cwd": "/canon"});
    let entry = serde_json::to_string(&inherited_entry("k1")?)?;
    std::fs::write(&path, format!("{header}\n{entry}\n"))?;
    Ok(path)
}

/// A render of the fixture's cwd for `model` into `out`.
fn target(model: &str, out: &Path, canon: Option<PathBuf>) -> RenderTarget {
    RenderTarget {
        session_id: "render-target".into(),
        cwd: "/work/outgoing".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(out.to_path_buf()),
        canon,
    }
}

fn sha(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(Hash::of(&std::fs::read(path)?).to_string())
}

#[test]
fn each_refusal_begins_with_its_name_and_names_what_it_refuses() {
    let (s, id) = (|| OUTGOING.to_owned(), |id: &str| id.to_owned());
    let cases = [
        (
            HomeError::LetterNotAssistant {
                session: s(),
                id: id("u1"),
            },
            "letter_not_assistant",
            &["u1"][..],
        ),
        (
            HomeError::LetterNotContiguous {
                session: s(),
                id: id("a3"),
            },
            "letter_not_contiguous",
            &["a3"],
        ),
        (
            HomeError::LetterAuthored {
                session: s(),
                id: id("x1"),
            },
            "letter_authored",
            &["x1"],
        ),
        (
            HomeError::LetterWithoutThinking {
                session: s(),
                ids: vec![id("a3"), id("a4")],
            },
            "letter_without_thinking",
            &["a3", "a4"],
        ),
        (
            HomeError::SuccessorNotEmpty {
                path: PathBuf::from("successor-dir"),
            },
            "successor_not_empty",
            &["successor-dir"],
        ),
        (
            HomeError::CanonExampleWithoutRule { id: id("k1") },
            "canon_example_without_rule",
            &["k1", "`rule`"],
        ),
    ];
    for (error, name, parts) in cases {
        let shown = error.to_string();
        assert!(shown.starts_with(name), "{shown}");
        assert!(parts.iter().all(|part| shown.contains(part)), "{shown}");
    }
    assert_eq!(INHERITED_FROM, "inherited from ");
}

#[test]
fn an_inherited_rule_is_written_when_present_and_skipped_when_absent() -> Gate {
    assert_eq!(
        serde_json::to_string(&ruleless(Some("r")))?,
        r#"{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b","rule":"r"}"#
    );
    assert_eq!(
        serde_json::to_string(&ruleless(None))?,
        r#"{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b"}"#
    );
    Ok(())
}

#[test]
fn the_canon_refuses_an_example_without_a_rule_on_load_add_and_render() -> Gate {
    let (dir, root) = fixture()?;
    let canon = ruleless_canon(dir.path())?;
    let refused =
        |r: &HomeError| matches!(r, HomeError::CanonExampleWithoutRule { id } if id == "k1");
    let loaded = load(&canon);
    assert!(matches!(&loaded, Err(e) if refused(e)), "{loaded:?}");

    let turns = dir.path().join("turns.txt");
    std::fs::write(&turns, "user: q\nassistant: a\n")?;
    let before = sha(&canon)?;
    let added = add_authored(&canon, &turns, "r", "b");
    assert!(matches!(&added, Err(e) if refused(e)), "{added:?}");
    assert_eq!(sha(&canon)?, before);

    let home = Home::open(&root)?;
    let session = home.open_session(OUTGOING)?;
    let out = dir.path().join("rendered.jsonl");
    let target = target("claude-fixture-model", &out, Some(canon));
    let rendered = render_claude_code(&session, &target, None);
    assert!(matches!(&rendered, Err(e) if refused(e)), "{rendered:?}");
    assert!(!out.exists());
    Ok(())
}

#[test]
fn a_ruleless_inherited_entry_in_a_session_of_a_home_reads_cleanly() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("inheritor", "/work", None)?;
    session.append_entry(&inherited_entry("h1")?)?;
    drop(session);
    let reader = SessionReader::open(home.session_path("inheritor")?)?;
    let EntryBody::Custom {
        data: Some(data), ..
    } = reader.entry("h1")?.body
    else {
        return Err("h1 is not a custom entry with data".into());
    };
    let inherited: Inherited = serde_json::from_value(data)?;
    assert_eq!(inherited.rule, None);
    Ok(())
}

/// A letter of ids.
fn ids(letter: &[&str]) -> Vec<String> {
    letter.iter().map(|id| (*id).to_owned()).collect()
}

/// The refusal cases but the non-empty successor: the from session, the
/// letter's ids and the refusal expected in its debug form, split by `|`.
const CASES: [&str; 9] = [
    r#"outgoing|u1|LetterNotAssistant { session: "outgoing", id: "u1" }"#,
    r#"outgoing|a1 a3|LetterNotContiguous { session: "outgoing", id: "a3" }"#,
    r#"outgoing|a2 a1|LetterNotContiguous { session: "outgoing", id: "a1" }"#,
    r#"outgoing|a1 b1|LetterNotContiguous { session: "outgoing", id: "b1" }"#,
    r#"outgoing|x1|LetterAuthored { session: "outgoing", id: "x1" }"#,
    r#"outgoing|a3|LetterWithoutThinking { session: "outgoing", ids: ["a3"] }"#,
    r#"outgoing||LetterWithoutThinking { session: "outgoing", ids: [] }"#,
    r#"outgoing|nope|UnknownEntry { session: "outgoing", id: "nope" }"#,
    r#"no-such-session|a1 a2|UnknownSession { session: "no-such-session" }"#,
];

#[test]
fn every_refusal_is_named_before_anything_is_written() -> Gate {
    let (dir, root) = fixture()?;
    let before = snapshot(&root)?;
    let mut ran = 0;
    for (n, case) in CASES.iter().enumerate() {
        let mut fields = case.split('|');
        let (from, letter) = (fields.next().unwrap_or(""), fields.next().unwrap_or(""));
        let successor = dir.path().join(format!("successor-{n}"));
        let letter: Vec<String> = letter.split_whitespace().map(str::to_owned).collect();
        let result = handover(&root, from, &letter, &successor);
        let Err(error) = result else {
            return Err(format!("case {n} was not refused: {result:?}").into());
        };
        assert_eq!(
            Some(format!("{error:?}").as_str()),
            fields.next(),
            "case {n}"
        );
        assert!(carries_no_content(&error.to_string()), "case {n}");
        assert!(!successor.exists(), "case {n} created the successor");
        assert_eq!(
            snapshot(&root)?,
            before,
            "case {n} changed the outgoing home"
        );
        ran += 1;
    }
    let kept = dir.path().join("kept");
    std::fs::create_dir(&kept)?;
    std::fs::write(kept.join("keep"), "kept")?;
    let kept_before = snapshot(&kept)?;
    let result = handover(&root, OUTGOING, &ids(&["a1", "a2"]), &kept);
    assert!(
        matches!(&result, Err(HomeError::SuccessorNotEmpty { path }) if *path == kept),
        "{result:?}"
    );
    assert_eq!(snapshot(&kept)?, kept_before);
    assert_eq!(kept_before.len(), 1);
    assert_eq!(snapshot(&root)?, before);
    ran += 1;
    assert_eq!(ran, 10);
    Ok(())
}

/// The successor file's lines after the header, as JSON values.
fn successor_lines(file: &Path) -> Result<(Value, Vec<Value>), Box<dyn Error>> {
    let text = std::fs::read_to_string(file)?;
    let mut lines = text.lines().map(serde_json::from_str::<Value>);
    let header = lines.next().ok_or("no header")??;
    Ok((header, lines.collect::<Result<_, _>>()?))
}

fn source_message(root: &Path, id: &str) -> Result<Value, Box<dyn Error>> {
    let reader = Home::read(root)?.read_session(OUTGOING)?;
    let EntryBody::Message { message } = reader.entry(id)?.body else {
        return Err(format!("{id} is not a message").into());
    };
    Ok(message)
}

/// Hand `[a1, a2]` of the fixture to `successor` and check what was written.
fn hand_over_and_check(root: &Path, successor: &Path) -> Result<HandoverReport, Box<dyn Error>> {
    let before = snapshot(root)?;
    let report = handover(root, OUTGOING, &ids(&["a1", "a2"]), successor)?;
    assert_eq!(snapshot(root)?, before);
    let (header, entries) = successor_lines(&report.file)?;
    let id = header["id"].as_str().ok_or("no id")?;
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    assert!(id.len() == 32 && id.bytes().all(hex), "{id}");
    assert_eq!(header["cwd"], "/work/outgoing");
    assert!(header.get("parentSession").is_none());
    assert_eq!(
        report.file,
        successor.join("sessions").join(format!("{id}.jsonl"))
    );
    assert_eq!(entries.len(), 4);
    let mark = &entries[0];
    assert_eq!(
        (mark["type"].as_str(), mark["customType"].as_str()),
        (Some("custom"), Some(CUSTOM_INHERITED))
    );
    assert!(mark["parentId"].is_null());
    assert_eq!(
        mark["data"],
        json!({"authored": false, "from_session": "outgoing", "from_entries": ["a1", "a2"],
            "provider": "anthropic", "api": "anthropic-messages", "model": "claude-fixture-model",
            "curated_at": "fixture-time-a2", "curated_by": "outgoing"})
    );
    assert!(mark["data"].get("rule").is_none());
    for (n, (id, parent)) in [("a1", &mark["id"]), ("a2", &json!("a1"))]
        .into_iter()
        .enumerate()
    {
        let copy = &entries[n + 1];
        assert_eq!((&copy["id"], &copy["parentId"]), (&json!(id), parent));
        assert_eq!(copy["timestamp"], format!("fixture-time-{id}"));
        assert_eq!(copy["message"], source_message(root, id)?);
    }
    let signature = entries[1]["message"]["content"][0]["thinkingSignature"]
        .as_str()
        .ok_or("no signature")?;
    assert_eq!(signature.as_bytes(), b"fixture-signature-a1");
    let info = &entries[3];
    let named = json!({"type": "session_info", "id": info["id"], "parentId": "a2",
        "timestamp": info["timestamp"], "name": "inherited from outgoing"});
    assert_eq!(*info, named);
    let (_, index, _) = Index::read(&report.file)?;
    assert_eq!(
        read_head(&report.file, &index)?.as_deref(),
        info["id"].as_str()
    );
    assert_eq!(std::fs::read_dir(successor.join("blocks"))?.count(), 0);
    Ok(report)
}

#[test]
fn the_letter_is_copied_whole_into_a_new_successor_home() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let report = hand_over_and_check(&root, &successor)?;
    let shown = serde_json::to_value(&report)?;
    let whole = json!({"successor_home": successor, "session": report.session,
        "file": report.file, "inherited": true});
    assert_eq!(shown, whole);
    let (header, _) = successor_lines(&report.file)?;
    assert_eq!(header["id"], report.session.as_str());
    assert!(carries_no_content(&shown.to_string()));
    Ok(())
}

#[test]
fn an_empty_successor_directory_takes_the_same_four_entries() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("empty");
    std::fs::create_dir(&successor)?;
    let report = hand_over_and_check(&root, &successor)?;
    assert!(report.inherited);
    Ok(())
}

#[test]
fn the_handover_subcommand_prints_its_report_and_refuses_by_name() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let argv = |letter: &str| {
        let line = format!(
            "lys-home handover --home {} --from outgoing --letter {letter} --successor {}",
            root.display(),
            successor.display()
        );
        line.split(' ').map(str::to_owned).collect::<Vec<String>>()
    };
    let refused = run(<Cli as clap::Parser>::parse_from(argv("x1")));
    let Err(error) = refused else {
        return Err("x1 was not refused".into());
    };
    let shown = error.to_string();
    assert!(
        shown.starts_with("letter_authored") && shown.contains("x1"),
        "{shown}"
    );
    assert!(!successor.exists());
    let value = run(<Cli as clap::Parser>::parse_from(argv("a1 a2")))?;
    assert_eq!(value["command"], "handover");
    assert_eq!(value["report"]["inherited"], true);
    let file = PathBuf::from(value["report"]["file"].as_str().ok_or("no file")?);
    assert!(file.is_file());
    assert_eq!(
        file.file_stem().and_then(|s| s.to_str()),
        value["report"]["session"].as_str()
    );
    let cli = <Cli as clap::Parser>::parse_from(argv("a1"));
    assert!(matches!(cli.command, Command::Handover(_)));
    assert_eq!(cli.command.refusal_status(), 1);
    let missing = "lys-home handover --home h --from s --successor d".split(' ');
    let missing = <Cli as clap::Parser>::try_parse_from(missing);
    let Err(missing) = missing else {
        return Err("a handover without --letter was accepted".into());
    };
    assert!(missing.to_string().contains("--letter"), "{missing}");
    Ok(())
}

/// A render of the successor: its directory, report, records and the source part.
type Rendered = (TempDir, RenderReport, Vec<Value>, Value);

/// Render the fixture successor of `[a1, a2]` for `model`, returning the
/// report, the records written and the source `a1` thinking part.
fn render_successor(model: &str) -> Result<Rendered, Box<dyn Error>> {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let report = handover(&root, OUTGOING, &ids(&["a1", "a2"]), &successor)?;
    let session = Home::open(&successor)?.open_session(&report.session)?;
    let target = target(model, &dir.path().join("rendered.jsonl"), None);
    let rendered = render_claude_code(&session, &target, None)?;
    let text = std::fs::read_to_string(&rendered.path)?;
    let records = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    let part = source_message(&root, "a1")?["content"][0].clone();
    Ok((dir, rendered, records, part))
}

#[test]
fn a_successor_rendered_for_another_model_carries_its_thinking_as_text() -> Gate {
    let (_dir, report, records, part) = render_successor("claude-other-model")?;
    let counts = [
        report.thinking_as_text,
        report.thinking_kept,
        report.dropped,
    ];
    assert_eq!(counts, [1, 0, 1]);
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0]["message"]["content"],
        json!([{"type": "text", "text": "fixture-thinking-a1"}])
    );
    assert!(
        records
            .iter()
            .all(|r| !r.to_string().contains("\"signature\""))
    );
    let account: Value = serde_json::from_slice(&std::fs::read(&report.loss_path)?)?;
    let dropped = account["dropped"].as_array().ok_or("no dropped")?;
    assert_eq!(dropped.len(), 1);
    assert_eq!(
        dropped[0]["hash"],
        Hash::of(&serde_json::to_vec(&part)?).to_string()
    );
    assert_eq!(
        dropped[0]["reason"],
        "signed thinking rendered as text: different provider, api or model"
    );
    Ok(())
}

#[test]
fn a_successor_rendered_for_its_own_model_keeps_its_signed_thinking() -> Gate {
    let (_dir, report, records, _) = render_successor("claude-fixture-model")?;
    assert_eq!((report.thinking_kept, report.dropped), (1, 0));
    let part = &records[0]["message"]["content"][0];
    assert_eq!(
        (&part["type"], &part["signature"]),
        (&json!("thinking"), &json!("fixture-signature-a1"))
    );
    Ok(())
}
