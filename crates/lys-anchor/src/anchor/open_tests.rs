#![cfg(test)]
//! Gates on creating and opening an anchor.
//!
//! # Where the second party comes from
//!
//! Nothing here is checked against this crate's own idea of what it did. Every
//! case is keyed on something written down before `lys-anchor` existed:
//!
//! - **The store contract**, as prose. Write-once, contiguity, "the pin only
//!   advances", and "exactly one interrupted append is repaired and reported"
//!   are stated in `lys-log-store`'s trait and module docs. The tampering and
//!   recovery cases below assert what that prose promises, not what this code
//!   happens to do.
//! - **The bytes on disk.** Every case reopens through a *fresh*
//!   [`FileLeafStore`] handle, so the value being asserted came back off the
//!   filesystem rather than out of the object that wrote it. An `Anchor` that
//!   cached what it was handed and returned that would pass a same-handle test
//!   and fails these.
//! - **The caller's own strings.** The origin cases assert the value the test
//!   supplied to `FileLeafStore::create`, over more than one origin, so a
//!   constant or a fallback would have to be right for two different inputs at
//!   once.
//!
//! # Every refusal here opens with a positive control
//!
//! A test made only of refusals cannot tell a working check from one that
//! rejects everything: "it refused" is satisfied by both. So each negative case
//! below first asserts that the *unmodified* setup is accepted, and only then
//! introduces the one difference under test.

use std::path::{Path, PathBuf};

use lys_log_store::{FileLeafStore, FrontierLog, LeafStore, StoreError};
use tempfile::TempDir;

use crate::admission::{AcceptAll, AdmissionPolicy, MaxSize, NotAdmitted, SubmitterContext};
use crate::keys::FileSigner;
use crate::wire::Submission;

use super::*;

const ORIGIN: &str = "example.com/lys/anchor-test";
const GENESIS: &[u8] = b"genesis bytes chosen by the caller, not by lys-anchor";

/// `lys-core`'s Go-conformance fixture seed. Tests use a fixed seed rather than
/// a generated one so nothing here depends on randomness, and `FileSigner` is
/// exercised through the one path it offers: a key file that already exists.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Writes the fixture key into `dir` and loads a signer over it. The log store
/// enumerates only its `leaves/` subdirectory, so a key file alongside it is
/// invisible to the store.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// The first segment file, per the layout `FileLeafStore` documents
/// (LYSLOGSTORE-008 R1): a record is its u32 LE length, the leaf bytes, the
/// pin and a CRC, so byte 4 of the first segment is the first byte of leaf 0.
///
/// Only the damage cases need this: editing a leaf underneath the log, and a
/// tail torn before its flush, are states no API offers because no API is
/// allowed to offer them.
fn first_segment(dir: &Path) -> PathBuf {
    dir.join("leaves")
        .join("segments")
        .join(format!("{:020}", 0))
}

/// Creates a store and an anchor over it, returning the anchor.
fn create_anchor(
    dir: &Path,
    origin: &str,
    genesis: &[u8],
) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    let store = FileLeafStore::create(dir, origin).unwrap();
    Anchor::create(
        store,
        genesis,
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// Reopens the anchor at `dir` through a fresh store handle.
fn reopen(dir: &Path) -> AnchorResult<Anchor<FileLeafStore, FileSigner, AcceptAll>> {
    Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
}

#[test]
fn genesis_lands_at_index_zero_and_survives_a_reopen() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let created = create_anchor(dir, ORIGIN, GENESIS);
    assert_eq!(created.tree_size(), 1);
    assert_eq!(created.recovered_to(), None);
    drop(created);

    // The claim is about what is on disk, so it is read back through a store
    // handle that never saw the append.
    let store = FileLeafStore::open(dir).unwrap();
    assert_eq!(store.extent(), 1);
    assert_eq!(store.leaf(0).unwrap().as_deref(), Some(GENESIS));
    assert_eq!(store.leaf(1).unwrap(), None);

    let reopened =
        Anchor::open(store, signer(dir), AcceptAll, AnchorConfig::unconfigured()).unwrap();
    assert_eq!(reopened.tree_size(), 1);
    assert_eq!(reopened.recovered_to(), None);
}

#[test]
fn opening_a_log_with_no_genesis_leaf_is_refused() {
    let accepted = TempDir::new().unwrap();
    let refused = TempDir::new().unwrap();

    // Positive control: this instrument accepts a log that has a genesis leaf.
    create_anchor(accepted.path(), ORIGIN, GENESIS);
    assert!(reopen(accepted.path()).is_ok());

    // The one difference: the store was created and nothing was appended.
    FileLeafStore::create(refused.path(), ORIGIN).unwrap();
    match reopen(refused.path()) {
        Err(AnchorError::Genesis(GenesisError::NoGenesisLeaf { origin })) => {
            assert_eq!(origin, ORIGIN);
        }
        other => panic!("expected NoGenesisLeaf for an extent-0 log, got {other:?}"),
    }
}

#[test]
fn the_origin_is_the_one_the_store_was_created_with() {
    // Two origins, so a constant, a default or a fallback would have to be
    // correct for both inputs at once.
    let origins = [
        "example.com/lys/anchor-alpha",
        "somewhere.else.invalid/a/different/log",
    ];
    assert_ne!(origins[0], origins[1]);

    let mut reported = Vec::new();
    for origin in origins {
        let tmp = TempDir::new().unwrap();
        let created = create_anchor(tmp.path(), origin, GENESIS);
        assert_eq!(created.origin(), origin);
        drop(created);

        // And again off the disk, where `log.json` is the only record of it.
        let reopened = reopen(tmp.path()).unwrap();
        assert_eq!(reopened.origin(), origin);
        reported.push(reopened.origin().to_string());
    }

    // Count what fired: a loop that ran zero or one times would satisfy every
    // assertion above without comparing two different origins at all.
    assert_eq!(reported.len(), origins.len());
    assert_ne!(reported[0], reported[1]);
}

#[test]
fn a_tampered_genesis_leaf_is_refused_at_open() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_anchor(dir, ORIGIN, GENESIS);

    // Positive control: untampered, this opens.
    assert!(reopen(dir).is_ok());

    // The one difference: one byte of leaf 0, edited underneath the log. A
    // second leaf behind it keeps the genesis record from being the last, so
    // the open cannot read the damage as a torn tail; the record's own CRC is
    // what catches it.
    {
        let (mut log, _tail) = FrontierLog::open(FileLeafStore::open(dir).unwrap()).unwrap();
        log.append(b"a second leaf behind genesis").unwrap();
    }
    assert!(reopen(dir).is_ok(), "two whole records open");
    let path = first_segment(dir);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4] ^= 0x01;
    std::fs::write(&path, &bytes).unwrap();

    match reopen(dir) {
        Err(AnchorError::Store(StoreError::CorruptRecord { offset, .. })) => {
            assert_eq!(offset, 0, "the damaged record is the first");
        }
        other => panic!("expected CorruptRecord for an edited leaf 0, got {other:?}"),
    }
}

#[test]
fn an_unfinished_append_is_cut_and_reported_rather_than_swallowed() {
    use std::io::Write;
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_anchor(dir, ORIGIN, GENESIS);

    // Positive control: a clean log reports no recovery and no cut, so `Some`
    // below cannot be an artefact of an accessor always answering the same way.
    assert_eq!(reopen(dir).unwrap().recovered_to(), None);
    assert!(
        FileLeafStore::open_read_only(dir)
            .unwrap()
            .unfinished_tail()
            .is_none()
    );

    // Fabricate the one state a crash can leave: bytes of a record that never
    // reached its flush, after the last whole record. The append writes the
    // leaf and its pin as one act precisely so this is the only divergence.
    let mut segment = std::fs::OpenOptions::new()
        .append(true)
        .open(first_segment(dir))
        .unwrap();
    segment.write_all(&[7, 0, 0, 0, b'l', b'e', b'a']).unwrap();
    drop(segment);

    let recovered = reopen(dir).unwrap();
    assert_eq!(recovered.recovered_to(), None, "nothing acknowledged moved");
    assert_eq!(recovered.tree_size(), 1);

    // The cut was durable, and it is reported exactly once: the next open finds
    // a whole log. An anchor that re-reported it would be describing a cut
    // that no longer happened.
    let store = FileLeafStore::open_read_only(dir).unwrap();
    assert_eq!(store.extent(), 1);
    assert!(store.unfinished_tail().is_none());
    assert_eq!(reopen(dir).unwrap().recovered_to(), None);
}

#[test]
fn creating_genesis_over_a_log_that_already_has_leaves_is_refused() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Positive control: the first create is accepted, so the refusal below is
    // about the log's state and not about `create` refusing everything.
    let created = create_anchor(dir, ORIGIN, GENESIS);
    assert_eq!(created.tree_size(), 1);
    drop(created);

    // What leaf 0 holds is the *other* case's rule, so this one records the
    // state it observes rather than asserting the genesis bytes again. A
    // failure here is then attributable to the refusal having written
    // something, and not to a change in what `create` stores.
    let before = FileLeafStore::open(dir).unwrap().leaf(0).unwrap();

    let store = FileLeafStore::open(dir).unwrap();
    match Anchor::create(
        store,
        GENESIS,
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    ) {
        Err(AnchorError::Genesis(GenesisError::GenesisAlreadyWritten { origin, tree_size })) => {
            assert_eq!(origin, ORIGIN);
            assert_eq!(tree_size, 1);
        }
        other => panic!("expected GenesisAlreadyWritten over an occupied log, got {other:?}"),
    }

    // And it refused without writing: nothing was appended, and leaf 0 is
    // byte-for-byte what it was before the refused call.
    let after = FileLeafStore::open(dir).unwrap();
    assert_eq!(after.extent(), 1);
    assert_eq!(after.leaf(0).unwrap(), before);
}

#[test]
fn genesis_is_written_even_under_a_policy_that_would_refuse_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The control that makes this test mean anything: this policy really does
    // refuse these exact bytes. Without it the assertion below would pass for a
    // policy that admits everything, which is most of them.
    let refusing = MaxSize::new(0);
    assert_eq!(
        refusing.admit(
            &Submission { statement: GENESIS },
            &SubmitterContext::Unidentified
        ),
        Err(NotAdmitted),
        "the fixture policy must refuse the genesis bytes"
    );

    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let anchor = Anchor::create(
        store,
        GENESIS,
        signer(dir),
        refusing,
        AnchorConfig::unconfigured(),
    )
    .expect("creating an anchor must not consult its admission policy");
    assert_eq!(anchor.tree_size(), 1);
    assert_eq!(anchor.policy(), &MaxSize::new(0));
    drop(anchor);

    // Read back through a handle that never saw the append: genesis is on disk,
    // verbatim.
    let store = FileLeafStore::open(dir).unwrap();
    assert_eq!(store.extent(), 1);
    assert_eq!(store.leaf(0).unwrap().as_deref(), Some(GENESIS));
}
