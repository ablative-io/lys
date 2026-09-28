#![cfg(test)]
//! An import's recorded bytes are unchanged (HOME-020 R9): the multi-result
//! fixture imported through the import command, its session file masked, and
//! the SHA-256 of the masked bytes pinned at the parent commit the change is
//! built on. The mask replaces two things and nothing else: the header line's
//! timestamp value, by `T`, and every run of exactly 32 lowercase hex digits
//! with no hex digit beside it (the importer's fresh ids), by `#` and its
//! order of first appearance counting from 0. Import determinism is not
//! attempted; the ids and the header's timestamp are drawn as they always
//! were, and only the mask makes two imports comparable.
//!
//! This file uses only what the crate offered at the parent commit, so it
//! runs unchanged in a worktree of that commit and must pass there too.

use std::path::PathBuf;

use sha2::{Digest, Sha256};

use lys_home::cli::{Cli, Command, run};

/// The fixture imported.
const MULTI_RESULT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/multi_result.jsonl"
);

/// SHA-256 of the fixture's session file masked as the module states, as
/// the parent commit writes it. Derived without running the importer, by
/// re-deriving each line the importer writes for the fixture from the
/// importer's source; written here, never computed from the import it
/// checks.
const PINNED_MASKED_SHA256: &str =
    "b99cb81a1e46f881b9228cb449347f59dc428560b955b3513816f91313e699dd";

/// Where the header's timestamp value begins.
const TIMESTAMP: &str = "\"timestamp\":\"";

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
}

/// Import the fixture into a fresh home as session `multi` through the
/// import command, and read the session file back.
fn imported() -> String {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let report = run(Cli {
        command: Command::Import {
            home: home.clone(),
            claude_code: PathBuf::from(MULTI_RESULT),
            session: "multi".into(),
        },
    })
    .unwrap();
    assert_eq!(report["report"]["records"], 6);
    std::fs::read_to_string(home.join("sessions").join("multi.jsonl")).unwrap()
}

/// The text with the first line's timestamp value replaced by `T` and each
/// fresh id by its order of first appearance.
fn mask(text: &str) -> String {
    let line_end = text.find('\n').unwrap_or(text.len());
    let stamped = match text[..line_end].find(TIMESTAMP) {
        Some(at) => {
            let start = at + TIMESTAMP.len();
            let end = start + text[start..].find('"').unwrap();
            format!("{}T{}", &text[..start], &text[end..])
        }
        None => text.to_owned(),
    };
    let mut ids: Vec<String> = Vec::new();
    let mut out = String::with_capacity(stamped.len());
    let mut run = String::new();
    for c in stamped.chars() {
        if c.is_ascii_hexdigit() {
            run.push(c);
        } else {
            flush(&mut run, &mut ids, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut ids, &mut out);
    out
}

/// End a run of hex digits: exactly 32 lowercase ones become `#` and their
/// order of first appearance; any other run is kept as it stands.
fn flush(run: &mut String, ids: &mut Vec<String>, out: &mut String) {
    if run.len() == 32 && !run.bytes().any(|b| b.is_ascii_uppercase()) {
        let n = if let Some(n) = ids.iter().position(|id| *id == *run) {
            n
        } else {
            ids.push(run.clone());
            ids.len() - 1
        };
        out.push('#');
        out.push_str(&n.to_string());
    } else {
        out.push_str(run);
    }
    run.clear();
}

#[test]
fn the_imported_fixture_masked_hashes_to_the_value_pinned_at_the_parent_commit() {
    let text = imported();
    assert_eq!(sha256_hex(mask(&text).as_bytes()), PINNED_MASKED_SHA256);
}

#[test]
fn the_mask_numbers_each_fresh_id_by_its_first_appearance() {
    let line = r#"{"id":"0123456789abcdef0123456789abcdef","parentId":"0123456789abcdef0123456789abcdef","x":"fedcba9876543210fedcba9876543210"}"#;
    assert_eq!(mask(line), r##"{"id":"#0","parentId":"#0","x":"#1"}"##);
}

#[test]
fn the_mask_leaves_a_block_hash_and_a_hyphenated_uuid_unchanged() {
    let hash = sha256_hex(b"a block");
    assert_eq!(hash.len(), 64);
    let uuid = "11111111-1111-4111-8111-111111111111";
    let text = format!("{{\"hash\":\"{hash}\",\"uuid\":\"{uuid}\"}}");
    assert_eq!(mask(&text), text);
}

#[test]
fn two_imports_differ_in_their_raw_bytes_and_agree_once_masked() {
    let (a, b) = (imported(), imported());
    assert_ne!(sha256_hex(a.as_bytes()), sha256_hex(b.as_bytes()));
    let (ma, mb) = (mask(&a), mask(&b));
    assert_eq!(ma, mb);
    assert_eq!(sha256_hex(ma.as_bytes()), sha256_hex(mb.as_bytes()));
}
