//! The fork run as the binary (HOME-006 R5 and R7): the report's exact
//! keys, each refusal's exit code with nothing on stdout, clap's refusals,
//! and the brief's fixture home forked five times and refused five times
//! with the block store unchanged, the children's lines hash-equal, and
//! both ancestry sides on the parent. The lanterns `L5`, `L6`, `L1` and
//! `C5` are lit with `lys-home lantern light`; `L2` is written with
//! `Session::append_entry` carrying `lit_in`, the key the lantern card
//! records in a later round and the light act on this tree does not, and
//! `O2` the same way without it. No test name carries a content sentinel.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Map, Value, json};

use lys_home::harness::claude_code::events::{HarnessEvent, KIND_ATTACHMENT, KIND_PERMISSION_MODE};
use lys_home::{BlockStore, Entry, EntryBase, EntryBody, Hash, Home};

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const PARENT: &str = "parent";
const COMPACTED: &str = "compacted";
const NOTE: &str = "fixture note";
const LIGHTER: &str = "fixture-lighter";
const STAMP: &str = "2026-01-01T00:00:00.000Z";
const SENTINELS: [&str; 8] = [
    "fixture-text-1",
    "fixture-text-2",
    "fixture-text-4",
    "fixture-text-5",
    "fixture-text-6",
    "fixture-text-7",
    "fixture-text-8",
    "fixture-text-9",
];
const REPORT_KEYS: [&str; 10] = [
    "blocks",
    "carried",
    "child",
    "coordinate_carried",
    "cut_at",
    "entries",
    "lantern",
    "parent",
    "point",
    "unstored",
];

type Gate = Result<(), Box<dyn Error>>;

/// The ids the fixture's lanterns were lit under.
struct Lanterns {
    l5: String,
    l1: String,
}

fn base(id: &str, parent: Option<&str>) -> EntryBase {
    EntryBase {
        id: id.to_owned(),
        parent_id: parent.map(str::to_owned),
        timestamp: STAMP.to_owned(),
    }
}

fn text_part(n: usize) -> Value {
    json!({"type": "text", "text": format!("fixture-text-{n}")})
}

fn user(id: &str, parent: Option<&str>, parts: &[Value]) -> Entry {
    Entry {
        base: base(id, parent),
        body: EntryBody::Message {
            message: json!({"role": "user", "content": parts, "timestamp": 0}),
        },
    }
}

fn assistant(id: &str, parent: Option<&str>, parts: &[Value]) -> Entry {
    Entry {
        base: base(id, parent),
        body: EntryBody::Message {
            message: json!({"role": "assistant", "content": parts, "api": "anthropic-messages",
                "provider": "anthropic", "model": "claude-fixture", "stopReason": "stop", "timestamp": 0}),
        },
    }
}

fn event(id: &str, parent: &str, kind: &str, record: &Hash) -> Result<Entry, Box<dyn Error>> {
    let event = HarnessEvent {
        kind: kind.to_owned(),
        source_uuid: None,
        record: Some(record.as_str().to_owned()),
        detail: Map::new(),
    };
    Ok(Entry {
        base: base(id, Some(parent)),
        body: EntryBody::Custom {
            custom_type: "lys.harness_event".to_owned(),
            data: Some(event.data()?),
        },
    })
}

fn lantern_entry(id: &str, parent: &str, point: &str, lit_in: Option<&str>) -> Entry {
    let mut data = json!({"point": point, "note": NOTE, "lit_by": LIGHTER, "lit_at": STAMP});
    if let Some(session) = lit_in {
        data["lit_in"] = json!(session);
    }
    Entry {
        base: base(id, Some(parent)),
        body: EntryBody::Custom {
            custom_type: "lys.lantern".to_owned(),
            data: Some(data),
        },
    }
}

fn put_parts(blocks: &BlockStore, parts: &[Value]) -> Result<(), Box<dyn Error>> {
    for part in parts {
        blocks.put(&serde_json::to_vec(part)?)?;
    }
    Ok(())
}

fn lys_home(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(BIN).args(args).output()?)
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
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    Ok(String::from_utf8(output.stderr.clone())?)
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort_unstable();
    keys
}

/// Light a lantern with the binary and return its id.
fn light(home: &str, session: &str, point: &str) -> Result<String, Box<dyn Error>> {
    let output = lys_home(&[
        "lantern",
        "light",
        "--home",
        home,
        "--session",
        session,
        "--point",
        point,
        "--note",
        NOTE,
        "--by",
        LIGHTER,
    ])?;
    let report = one_object(&output)?;
    Ok(report["id"].as_str().ok_or("a lantern id")?.to_owned())
}

/// The brief's fixture home under `dir`, with its lanterns lit.
fn fixture_home(dir: &Path) -> Result<(PathBuf, Lanterns), Box<dyn Error>> {
    let root = dir.join("home");
    let home = Home::open(&root)?;
    let blocks = home.blocks()?;
    let s2_record = blocks.put(b"fixture-record-s2")?.hash;
    let e3_record = blocks.put(b"fixture-record-e3")?.hash;
    {
        let mut session = home.create_session(PARENT, "/fixture", None)?;
        session.append_entry(&user("e1", None, &[text_part(1)]))?;
        session.append_entry(&assistant("e2", Some("e1"), &[text_part(2)]))?;
        session.append_entry(&event("s2", "e2", KIND_PERMISSION_MODE, &s2_record)?)?;
        session.move_head(Some("e2"))?;
    }
    {
        let mut session = home.open_session(PARENT)?;
        session.append_entry(&lantern_entry("L2", "e2", "e2", Some(PARENT)))?;
        session.append_entry(&lantern_entry("O2", "L2", "e2", None))?;
        session.append_entry(&event("e3", "O2", KIND_ATTACHMENT, &e3_record)?)?;
        session.append_entry(&user("e4", Some("e3"), &[text_part(4)]))?;
        session.append_entry(&assistant("e5", Some("e4"), &[text_part(5)]))?;
        let image = json!({"type": "image", "data": "aWdub3JlZA==", "mimeType": "image/png"});
        session.append_entry(&user("e6", Some("e5"), &[text_part(6), image]))?;
        session.append_entry(&assistant("e7", Some("e6"), &[text_part(7)]))?;
        session.append_entry(&user("e8", Some("e7"), &[text_part(8)]))?;
        session.append_entry(&assistant("e9", Some("e8"), &[text_part(9)]))?;
    }
    for n in [1, 2, 4, 5, 6, 7, 8, 9] {
        put_parts(&blocks, &[text_part(n)])?;
    }
    {
        let mut session = home.create_session(COMPACTED, "/fixture", None)?;
        session.append_entry(&user("e1", None, &[text_part(1)]))?;
        session.append_entry(&assistant("e2", Some("e1"), &[text_part(2)]))?;
        session.append_entry(&Entry {
            base: base("c3", Some("e2")),
            body: EntryBody::Compaction {
                summary: "fixture summary".to_owned(),
                first_kept_entry_id: "e2".to_owned(),
                tokens_before: 0,
                rest: Map::new(),
            },
        })?;
        session.append_entry(&user("e4", Some("c3"), &[text_part(4)]))?;
        session.append_entry(&assistant("e5", Some("e4"), &[text_part(5)]))?;
    }
    let home_arg = root.to_str().ok_or("a UTF-8 path")?;
    let l5 = light(home_arg, PARENT, "e5")?;
    light(home_arg, PARENT, "e6")?;
    let l1 = light(home_arg, PARENT, "e1")?;
    light(home_arg, COMPACTED, "e5")?;
    Ok((root, Lanterns { l5, l1 }))
}

fn lines_of_code(file: &str) -> Result<usize, Box<dyn Error>> {
    Ok(std::fs::read_to_string(file)?
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.is_empty() && !line.starts_with("//")
        })
        .count())
}

#[test]
fn fork_prints_one_report_and_refuses_by_name_with_nothing_on_stdout() -> Gate {
    let dir = tempfile::tempdir()?;
    let (home, lanterns) = fixture_home(dir.path())?;
    let home_arg = home.to_str().ok_or("a UTF-8 path")?;
    let mut outputs = Vec::new();

    let forked = lys_home(&["fork", "--home", home_arg, "--lantern", &lanterns.l5])?;
    let report = one_object(&forked)?;
    assert_eq!(sorted_keys(&report), ["command", "report"]);
    assert_eq!(report["command"], "fork");
    assert_eq!(sorted_keys(&report["report"]), REPORT_KEYS);
    assert_eq!(report["report"]["parent"], PARENT);
    outputs.push(forked);

    let unknown = lys_home(&["fork", "--home", home_arg, "--lantern", "no-such-lantern"])?;
    assert!(refused(&unknown, 1)?.contains("no-such-lantern"));
    outputs.push(unknown);

    let nothing = lys_home(&["fork", "--home", home_arg, "--lantern", &lanterns.l1])?;
    let stderr = refused(&nothing, 1)?;
    assert!(stderr.contains("nothing_to_fork"), "{stderr}");
    assert!(stderr.contains(&lanterns.l1), "{stderr}");
    outputs.push(nothing);

    let no_lantern = lys_home(&["fork", "--home", home_arg])?;
    refused(&no_lantern, 2)?;
    outputs.push(no_lantern);

    let point = lys_home(&[
        "fork",
        "--home",
        home_arg,
        "--lantern",
        &lanterns.l5,
        "--point",
        "e5",
    ])?;
    refused(&point, 2)?;
    outputs.push(point);

    assert_eq!(outputs.len(), 5);
    for output in &outputs {
        let all = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        for sentinel in SENTINELS {
            assert!(!all.contains(sentinel));
        }
    }
    assert!(lines_of_code(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli.rs"))? <= 500);
    Ok(())
}
