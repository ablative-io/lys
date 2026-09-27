//! Gates on the `lys.given_statement` entry: its shape and parent, the block
//! it names verifying over the record's canonical bytes, the head and the
//! given entry's line unchanged, and statements read back in file order.

use std::fmt::Write as _;
use std::path::Path;

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, verify_attestation_bytes};
use serde_json::Value;

use crate::harness::claude_code::given::ConfigSource;
use crate::record::blocks::{BlockStore, Hash};
use crate::record::entries::{CUSTOM_GIVEN_STATEMENT, EntryBody};
use crate::record::given::GivenRecord;
use crate::record::given_statement::GivenStatement;
use crate::record::given_tests::{resolution, session_with_a_render_event};
use crate::record::{Home, Session};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn key(dir: &Path) -> Result<Ed25519Identity, Box<dyn std::error::Error>> {
    Ok(Ed25519Identity::load_or_generate(&dir.join("test.key"))?)
}

/// The line of the session file whose entry has `id`.
fn line_of(session: &Session, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(session.file())?;
    let mut lines = text
        .lines()
        .filter(|line| serde_json::from_str::<Value>(line).is_ok_and(|v| v["id"] == id));
    let line = lines.next().ok_or("the entry's line")?.to_owned();
    assert!(lines.next().is_none(), "one line per id");
    Ok(line)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").expect("a String takes every write");
        s
    })
}

#[test]
fn a_signed_record_hangs_one_statement_under_its_given_entry() -> Outcome {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let blocks = Home::open(dir.path().join("home"))?.blocks()?;
    let record = GivenRecord::claude_code(resolution(ConfigSource::Template), Vec::new());
    let given = record.append_under(&mut session, &event)?;
    let head = session.head()?.map(str::to_owned);
    let head_hash = session.head_hash()?;
    let given_line = line_of(&session, &given)?;
    let canonical = record.canonical_bytes()?;
    let signed = GivenStatement::sign(&mut session, &blocks, &given, &canonical, &key)?;

    let entries = session.customs_everywhere(CUSTOM_GIVEN_STATEMENT)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id(), signed.entry);
    assert_eq!(entries[0].parent_id(), Some(given.as_str()));
    let EntryBody::Custom {
        data: Some(data), ..
    } = &entries[0].body
    else {
        return Err("a custom entry with data".into());
    };
    let object = data.as_object().ok_or("an object")?;
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["given", "statement"]);
    assert_eq!(data["given"], given.as_str());

    let block = blocks.get(&Hash::parse(data["statement"].as_str().ok_or("a hash")?)?)?;
    assert_eq!(block, signed.cose);
    let attestation = Attestation::from_cose_bytes(&block)?;
    assert_eq!(
        hex(&attestation.payload_hash),
        record.given_hash()?.as_str()
    );
    verify_attestation_bytes(&block, canonical.as_bytes())?;

    assert_eq!(session.head()?.map(str::to_owned), head);
    assert_eq!(session.head_hash()?, head_hash);
    assert_eq!(line_of(&session, &given)?, given_line);
    Ok(())
}

#[test]
fn two_signed_records_read_back_as_two_statements_in_file_order() -> Outcome {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let blocks: BlockStore = Home::open(dir.path().join("home"))?.blocks()?;
    let mut expected = Vec::new();
    for source in [ConfigSource::Template, ConfigSource::Home] {
        let record = GivenRecord::claude_code(resolution(source), Vec::new());
        let given = record.append_under(&mut session, &event)?;
        let signed = GivenStatement::sign(
            &mut session,
            &blocks,
            &given,
            &record.canonical_bytes()?,
            &key,
        )?;
        expected.push((signed.entry, given));
    }
    let all = GivenStatement::read_all(&session)?;
    assert_eq!(all.len(), 2);
    for ((id, statement), (entry, given)) in all.iter().zip(&expected) {
        assert_eq!(id, entry);
        assert_eq!(&statement.given, given);
    }
    Ok(())
}
