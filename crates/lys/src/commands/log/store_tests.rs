#![cfg(test)]
//! Gates on the CLI's translation of storage failures.
//!
//! The layout, write-once and integrity rules are gated in `lys-log-store`
//! itself — duplicating them here would test the same code twice and leave the
//! actual new surface, the [`From<StoreError>`] mapping, uncovered.
//!
//! That mapping is worth its own tests because it can fail *silently*: a
//! mis-routed variant still produces an error and still exits 1, so a broken
//! mapping looks exactly like a working one until an operator reads a message
//! that has lost its remedy.

use std::path::Path;

use lys_log_store::{LeafStore, StoreError};

use super::*;

const ORIGIN: &str = "example.com/lys/store-map-test";

fn init(dir: &Path) {
    store_init(dir, ORIGIN).unwrap();
}

use super::init as store_init;

#[test]
fn an_uninitialized_directory_keeps_its_remedy() {
    let tmp = tempfile::tempdir().unwrap();
    let err = open(&tmp.path().join("nope")).unwrap_err();
    assert!(matches!(err, CliError::LogDirMissing { .. }), "{err}");
    assert!(err.to_string().contains("lys log init"), "{err}");
}

#[test]
fn reinitialization_explains_that_the_origin_is_pinned() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init(&dir);
    let err = store_init(&dir, "example.com/other").unwrap_err();
    assert!(matches!(err, CliError::LogDirInvalid { .. }), "{err}");
    let message = err.to_string();
    assert!(message.contains("already initialized"), "{message}");
    assert!(message.contains("pinned at init"), "{message}");
}

#[test]
fn a_rejected_origin_stays_a_trust_error() {
    let tmp = tempfile::tempdir().unwrap();
    let err = store_init(&tmp.path().join("log"), "has space").unwrap_err();
    assert!(matches!(err, CliError::Trust(_)), "{err}");
}

/// The first segment file of the log at `dir` (LYSLOGSTORE-008 R1).
fn first_segment(dir: &Path) -> std::path::PathBuf {
    dir.join("leaves")
        .join("segments")
        .join(format!("{:020}", 0))
}

#[test]
fn corruption_keeps_the_stores_specific_reason_and_path() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init(&dir);
    std::fs::write(
        dir.join("leaves").join("segments").join("stray.txt"),
        b"junk",
    )
    .unwrap();
    let err = open(&dir).unwrap_err();
    match err {
        CliError::LogDirInvalid {
            ref path,
            ref reason,
        } => {
            assert_eq!(path, &dir);
            assert!(reason.contains("unexpected entry"), "{reason}");
        }
        other => panic!("expected LogDirInvalid, got {other}"),
    }
}

/// A variant with no CLI-specific framing must still reach the operator with
/// its own words — the catch-all forwards, it does not flatten.
#[test]
fn an_unframed_store_failure_is_forwarded_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init(&dir);
    let log = open(&dir).unwrap();
    let store_err = log.prefix_tree(4).unwrap_err();
    let expected = store_err.to_string();
    let cli_err = CliError::from(store_err);
    assert!(matches!(cli_err, CliError::LogStore(_)), "{cli_err}");
    assert_eq!(cli_err.to_string(), expected, "transparent, not reworded");
    assert!(expected.contains("leaves a gap"), "{expected}");
}

/// The mapping must not swallow an I/O failure's context, which is the part
/// that names the file and the operation.
#[test]
fn an_io_failure_keeps_its_context() {
    let cli_err = CliError::from(StoreError::Io {
        context: "failed to write leaf file /x/leaves/00000000000000000000".to_string(),
        source: std::io::Error::other("device is on fire"),
    });
    assert!(matches!(cli_err, CliError::Io { .. }), "{cli_err}");
    let message = cli_err.to_string();
    assert!(message.contains("failed to write leaf file"), "{message}");
    assert!(message.contains("device is on fire"), "{message}");
}

/// Opening a log with an unfinished append must succeed and cut it. The
/// notice itself goes to stderr, which this layer owns; what is asserted here
/// is that the CLI does not turn a recoverable log into a failure, and that
/// nothing acknowledged is lost.
#[test]
fn an_unfinished_append_still_opens_through_the_cli_path() {
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init(&dir);
    let mut log = open(&dir).unwrap();
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    // A torn tail: bytes of a record that never reached its flush.
    let mut segment = std::fs::OpenOptions::new()
        .append(true)
        .open(first_segment(&dir))
        .unwrap();
    segment.write_all(&[7, 0, 0, 0, b'l', b'e', b'a']).unwrap();
    drop(segment);

    let recovered = open(&dir).unwrap();
    assert_eq!(recovered.tree().len(), 2);
    assert_eq!(recovered.recovered_to(), None);
    assert_eq!(recovered.store().extent(), 2);
    assert_eq!(recovered.store().pinned().tree_size, 2);
    // The writable open names what it cut; the cut itself is what the
    // read-only open below no longer finds.
    assert_eq!(
        recovered.store().unfinished_tail().map(|tail| tail.bytes),
        Some(7)
    );
    let read_only = lys_log_store::FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read_only.extent(), 2);
    assert!(
        read_only.unfinished_tail().is_none(),
        "the writable open cut it"
    );
}

/// A tampered leaf must still read as a corrupt *log directory*, naming the
/// path — this regressed when the integrity check moved into a backend-agnostic
/// library whose error deliberately carries no path, and it was the integration
/// suite that caught it.
#[test]
fn a_tampered_leaf_reads_as_an_invalid_log_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init(&dir);
    let mut log = open(&dir).unwrap();
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    // Byte 4 of the first segment is the first byte of leaf 0, inside the
    // pinned prefix; flipping it fails that record's CRC.
    let path = first_segment(&dir);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4] ^= 0x01;
    std::fs::write(&path, bytes).unwrap();

    let err = open(&dir).unwrap_err();
    match err {
        CliError::LogDirInvalid {
            ref path,
            ref reason,
        } => {
            assert_eq!(path, &dir);
            // The specific discrepancy survives the reframing.
            assert!(reason.starts_with("corrupt record: "), "{reason}");
        }
        other => panic!("expected LogDirInvalid, got {other}"),
    }
    assert!(
        err.to_string().contains("log directory invalid"),
        "the operator-facing shape must be preserved: {err}"
    );
}
