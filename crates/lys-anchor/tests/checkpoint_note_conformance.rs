#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The anchor's checkpoints, judged by `golang.org/x/mod/sumdb/note`.
//!
//! # Which axis of independence this is, and which it is not
//!
//! **Algorithm and toolchain.** The judge is a different language, a different
//! implementation and a different author: Go's reference signed-note code,
//! vendored and pinned in `lys-core`'s scaffold, which has never heard of an
//! anchor. It agrees or it does not, and no amount of symmetry inside this
//! workspace can make it agree wrongly.
//!
//! **Not platform.** One machine, one toolchain resolution, one dependency
//! graph. Two implementations agreeing here says nothing about a third
//! environment.
//!
//! **Not the Merkle root.** Go checks the note envelope — the key name, the key
//! ID, the signature over the body bytes — and takes the body as opaque text.
//! That the root line is the RFC 6962 root of the log is a separate claim, and
//! it is checked in `anchor/checkpoint_tests.rs` against a value computed with
//! a shell pipeline rather than with any Rust at all.
//!
//! # The strongest available claim is byte-identity, so it is the one made
//!
//! Ed25519 signing is deterministic, so for one body, one name and one seed
//! there is exactly one correct note. Every case below therefore does more than
//! ask Go to accept the anchor's note: it has Go *produce* the note from the
//! same inputs and compares the two byte for byte. An acceptance can be
//! satisfied by a verifier that is too permissive; byte-identity cannot.
//!
//! # What only this gate catches
//!
//! One rule has no other guard, and it was found by drifting for it: **the
//! anchor must sign exactly the body it reports, with nothing appended.**
//! C2SP checkpoints permit trailing extension lines, and `lys-core`'s parser
//! tolerates and discards them — so an anchor that quietly signed
//! `body ‖ "extension\n"` would still hand back a `PublishedCheckpoint` whose
//! `body` parsed, verified and matched every in-crate expectation. Every one of
//! the eighteen tests in the library stayed green under exactly that change.
//! What fails is the comparison below against the body Go hands back, because
//! Go returns the signed text verbatim and does not discard anything.
//!
//! That is the shape of the risk generally: what a producer *reports* and what
//! it *signed* are two different values, and only a reader that re-derives the
//! signed bytes from the artifact can tell them apart.
//!
//! # The refusal case comes last, and by then the tool has already accepted
//!
//! A test made only of refusals cannot tell a working verifier from one that
//! rejects everything. Every acceptance in the sweep runs first, so by the time
//! the tampered note is offered, this tool has been shown to accept four
//! genuine ones.

mod harness;

use std::path::Path;

use lys_anchor::{AcceptAll, Anchor, AnchorConfig, FileSigner, Signer};
use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_log_store::{FileLeafStore, Log};
use tempfile::TempDir;

/// The origin the store is created under, and therefore the note key name.
const ORIGIN: &str = "example.com/lys/anchor-go-gate";

const GENESIS: &[u8] = b"lys-anchor go-gate genesis fixture";

/// `lys-core`'s Go-conformance fixture seed, and its hex form for the Go tool's
/// `sign` mode. A fixed seed is what makes byte-identity checkable at all.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";
const FIXTURE_SEED_HEX: &str = "6c79732d676f2d636f6e666f726d616e63652d746573742d736565642d303121";

/// How many tree sizes the sweep covers.
const SIZES: u64 = 4;

fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

fn open_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

#[test]
fn go_sumdb_note_reproduces_the_anchors_checkpoints_byte_for_byte() {
    let Some(go) = harness::go_or_skip("lys-anchor checkpoint note conformance") else {
        return;
    };
    let gocache_dir = TempDir::new().unwrap();
    let bin_dir = TempDir::new().unwrap();
    let bin = bin_dir.path().join("notetool");
    harness::build_go_tool(
        &go,
        harness::GoScaffold::Note,
        &gocache_dir.path().join("gocache"),
        &bin,
    );

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(
        store,
        GENESIS,
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap();

    // The verifier key a stranger would be handed: the origin they were told,
    // and the anchor's public key. Built here from the literal this test gave
    // the store, not from anything the anchor reported back.
    let spec = NoteVerifierKey::new(ORIGIN, open_anchor(dir).signer().public_key())
        .unwrap()
        .to_spec();

    let mut accepted = 0;
    let mut last_note = String::new();
    let mut bodies = Vec::new();
    for size in 1..=SIZES {
        if size > 1 {
            let mut log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
            log.append(format!("entry at size {size}").as_bytes())
                .unwrap();
        }
        let anchor = open_anchor(dir);
        let published = anchor.publish_checkpoint().unwrap();
        let body = published.body.encode();

        // Rust -> Go: the reference verifier accepts, and hands back exactly
        // the body bytes the anchor signed.
        let (ok, returned) =
            harness::run_built_tool(&bin, &["verify", &spec], published.note.as_bytes());
        assert!(
            ok,
            "Go note.Open rejected the anchor's checkpoint at tree size {size}"
        );
        assert_eq!(
            returned,
            body.as_bytes(),
            "Go returned a different body at tree size {size}"
        );

        // Go -> Rust: Go signs the same body under the same name and seed, and
        // the two notes are the same bytes. This is the assertion an
        // over-permissive verifier cannot satisfy.
        let (ok, go_note) =
            harness::run_built_tool(&bin, &["sign", ORIGIN, FIXTURE_SEED_HEX], body.as_bytes());
        assert!(ok, "Go note.Sign failed at tree size {size}");
        assert_eq!(
            go_note,
            published.note.as_bytes(),
            "the anchor's note and Go's note must be byte-identical at tree size {size}"
        );

        last_note = published.note;
        bodies.push(body);
        accepted += 1;
    }

    // Count what fired: a loop that ran zero times satisfies every assertion
    // inside it, and a Go gate that never spawned would look exactly like one
    // that passed.
    assert_eq!(accepted, SIZES);

    // And it swept four *different* checkpoints rather than the same one four
    // times, which four identical Go agreements would also have satisfied.
    // Stated as distinctness rather than as "the size was N": what the size
    // committed to is `checkpoint_tests.rs`'s rule, and one rule guarded in two
    // places is a rule neither place proves.
    bodies.sort_unstable();
    bodies.dedup();
    assert_eq!(u64::try_from(bodies.len()).unwrap(), SIZES);

    // Negative parity, offered only now that this tool has accepted four
    // genuine notes: one character of the signed root line, altered. The edit
    // is keyed on the root line's own first character rather than on any value
    // this test knows in advance, so it stays a tamper whatever the log holds
    // and whatever the tree size happens to be — a tamper string that had to
    // match a size would make this case a second guard of a rule that already
    // has one.
    let mut lines: Vec<&str> = last_note.split('\n').collect();
    let root_line = lines[2];
    let flipped = format!(
        "{}{}",
        if root_line.starts_with('A') { 'B' } else { 'A' },
        &root_line[1..]
    );
    lines[2] = &flipped;
    let tampered = lines.join("\n");
    assert_ne!(tampered, last_note, "the tamper must change a byte");

    let (ok, _stdout) = harness::run_built_tool(&bin, &["verify", &spec], tampered.as_bytes());
    assert!(!ok, "Go accepted a checkpoint whose root line was altered");

    let verifier = NoteVerifierKey::new(ORIGIN, open_anchor(dir).signer().public_key()).unwrap();
    assert!(
        verify_checkpoint(tampered.as_bytes(), &verifier).is_err(),
        "lys must refuse a checkpoint whose root line was altered, exactly like Go"
    );
}
