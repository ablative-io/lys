//! Gates on the strict verification, on a home built as a render leaves
//! it: clean as built; each session reason from the one fault that causes
//! it, with the index's bytes unchanged and nothing rebuilt or written; a
//! corrupted block and a corrupted template each named by hash and never
//! by content.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

use crate::harness::claude_code::launch::{LaunchArgs, render_launch};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::verify::{Reason, SessionReason, verify_home, verify_sessions};

const SESSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);
const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);
const CORRUPTED: &[u8] = b"corrupted-block-bytes";

type Gate = Result<(), Box<dyn Error>>;

/// The fixture home and the render that built it.
struct Fixture {
    /// The home's directory.
    root: TempDir,
    /// The render's out directory.
    out: TempDir,
    /// The home.
    home: Home,
    /// The render's report.
    report: Value,
}

impl Fixture {
    /// Remove both directories, reporting a failure to remove either.
    fn close(self) -> Gate {
        self.root.close()?;
        self.out.close()?;
        Ok(())
    }
}

/// The fixture home: the launch session copied in as `fixture`, then one
/// render-launch of the launch template into a second empty directory.
fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let out = tempfile::tempdir()?;
    let home = Home::open(root.path())?;
    std::fs::copy(SESSION, home.session_path("fixture")?)?;
    let report = render_launch(&LaunchArgs {
        home: root.path().to_path_buf(),
        session: "fixture".to_owned(),
        template: PathBuf::from(TEMPLATE),
        uuid: "00000000-0000-4000-8000-000000000019".to_owned(),
        cwd: "/fixture".to_owned(),
        model: "claude-fixture".to_owned(),
        version: "2.1.283".to_owned(),
        out: out.path().to_path_buf(),
        key: None,
    })?;
    let sessions = root.path().join("sessions");
    assert!(sessions.join("fixture.index.jsonl").is_file());
    assert!(sessions.join("fixture.head").is_file());
    assert_eq!(files_under(&root.path().join("templates"))?.len(), 1);
    assert!(!files_under(&root.path().join("blocks"))?.is_empty());
    Ok(Fixture {
        root,
        out,
        home,
        report,
    })
}

/// Every file under `dir`, recursively.
fn files_under(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(files_under(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
}

fn index_file(home: &Home) -> PathBuf {
    home.root().join("sessions").join("fixture.index.jsonl")
}

fn head_file(home: &Home) -> PathBuf {
    home.root().join("sessions").join("fixture.head")
}

fn sha256_of(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(Hash::of(&std::fs::read(path)?).as_str().to_owned())
}

fn only_reason(home: &Home, reason: Reason) -> Gate {
    let found = verify_sessions(home)?;
    assert_eq!(
        found,
        vec![SessionReason {
            session: "fixture".to_owned(),
            reason,
        }]
    );
    Ok(())
}

#[test]
fn the_home_as_the_render_left_it_verifies_clean() -> Gate {
    let f = fixture()?;
    let home = &f.home;
    let found = verify_home(home)?;
    assert!(found.sessions.is_empty(), "{found:?}");
    assert!(found.bad_blocks.is_empty(), "{found:?}");
    assert!(found.bad_templates.is_empty(), "{found:?}");
    assert!(found.is_clean());
    f.close()
}

#[test]
fn a_line_appended_past_the_index_is_index_not_this_file_and_the_index_is_untouched() -> Gate {
    let f = fixture()?;
    let home = &f.home;
    let head = std::fs::read_to_string(head_file(home))?.trim().to_owned();
    let entry = Entry {
        base: EntryBase {
            id: "appended-directly".to_owned(),
            parent_id: Some(head),
            timestamp: "2026-01-01T00:00:09.000Z".to_owned(),
        },
        body: EntryBody::Custom {
            custom_type: "lys.fixture".to_owned(),
            data: None,
        },
    };
    let mut line = serde_json::to_string(&entry)?;
    line.push('\n');
    let session = home.session_path("fixture")?;
    let mut bytes = std::fs::read(&session)?;
    bytes.extend_from_slice(line.as_bytes());
    std::fs::write(&session, bytes)?;
    let before = sha256_of(&index_file(home))?;
    only_reason(home, Reason::IndexNotThisFile)?;
    assert_eq!(sha256_of(&index_file(home))?, before);
    f.close()
}

#[test]
fn an_index_that_is_not_json_is_index_not_this_file_and_is_untouched() -> Gate {
    let f = fixture()?;
    let home = &f.home;
    std::fs::write(index_file(home), b"not json")?;
    let before = sha256_of(&index_file(home))?;
    only_reason(home, Reason::IndexNotThisFile)?;
    assert_eq!(sha256_of(&index_file(home))?, before);
    f.close()
}

#[test]
fn a_removed_index_is_index_missing_and_is_not_rebuilt() -> Gate {
    let f = fixture()?;
    let home = &f.home;
    std::fs::remove_file(index_file(home))?;
    only_reason(home, Reason::IndexMissing)?;
    assert!(!index_file(home).exists());
    f.close()
}

#[test]
fn a_removed_head_is_head_missing_and_a_head_off_the_index_is_head_not_indexed() -> Gate {
    let f = fixture()?;
    let home = &f.home;
    std::fs::remove_file(head_file(home))?;
    only_reason(home, Reason::HeadMissing)?;
    assert!(!head_file(home).exists());
    std::fs::write(head_file(home), b"nope\n")?;
    only_reason(home, Reason::HeadNotIndexed)?;
    f.close()
}

#[test]
fn a_corrupted_block_and_a_corrupted_template_are_named_by_hash_only() -> Gate {
    let f = fixture()?;
    let (home, report) = (&f.home, &f.report);
    let event = report["event"].as_str().ok_or("the render's event id")?;
    let text = std::fs::read_to_string(home.session_path("fixture")?)?;
    let mut records = Vec::new();
    for line in text.lines() {
        let value: Value = serde_json::from_str(line)?;
        if value["id"] == event && value["data"]["kind"] == "template_render" {
            records.push(
                value["data"]["record"]
                    .as_str()
                    .ok_or("a record")?
                    .to_owned(),
            );
        }
    }
    assert_eq!(records.len(), 1);
    let manifest = records.remove(0);
    assert_eq!(report["manifest"], manifest.as_str());
    let block = home
        .root()
        .join("blocks")
        .join(&manifest[..2])
        .join(&manifest);
    std::fs::write(&block, CORRUPTED)?;
    let found = verify_home(home)?;
    assert_eq!(found.bad_blocks, vec![manifest]);
    assert!(found.bad_templates.is_empty());
    assert!(!serde_json::to_string(&found)?.contains("corrupted-block-bytes"));

    let templates = files_under(&home.root().join("templates"))?;
    assert_eq!(templates.len(), 1);
    let template = &templates[0];
    let name = template
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("a template name")?
        .to_owned();
    std::fs::write(template, CORRUPTED)?;
    let found = verify_home(home)?;
    assert_eq!(found.bad_templates, vec![name]);
    assert!(!serde_json::to_string(&found)?.contains("corrupted-block-bytes"));
    f.close()
}
