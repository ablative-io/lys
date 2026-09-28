#![cfg(test)]
//! `lys/delegation/v1`, judged by `veraison/go-cose` — and, more to the
//! point, by the bytes go-cose says it signed over.
//!
//! # The hole this exists for, stated before anything is asserted
//!
//! `delegation_preimage` builds the RFC 9052 §4.4 `Sig_structure`.
//! `sign_delegation` signs it. `verify_delegation` rebuilds it *with the same
//! function* and checks the signature against that. The three share one
//! construction, so if the preimage is wrong — a mis-encoded head, a stray
//! trailing byte, an `external_aad` that is not `h''` — every signature this
//! crate produces is over the wrong bytes and every verification this crate
//! performs reproduces the same wrong bytes and agrees.
//!
//! **Every in-crate assertion of the form `verify_delegation(..).is_ok()` is
//! blind to that by construction, and so is every round trip through
//! `to_cose_bytes` and back.** So is an assertion that compares the preimage
//! against `cbor::sig_structure_bytes`, because that is the same crate's
//! encoder being asked whether it agrees with itself.
//!
//! What closes it is a reader that constructs the signed bytes from the
//! artifact **by its own rules** and hands them back verbatim.
//! `cose-conformance`'s `delegation-verify` does exactly that: a
//! [`cose.Verifier`] implemented in the scaffold is passed to go-cose's
//! `Sign1Message.Verify`, so go-cose calls it with the `ToBeSigned` *go-cose*
//! assembled, and the tool prints those bytes as hex before anything else. This
//! file compares them against `delegation_preimage`, byte for byte.
//!
//! That is not a stylistic preference for verbatim return. In increment 3 an
//! anchor signed `body ‖ "extension\n"` instead of `body`, and all eighteen
//! in-crate tests stayed green because lys's own parser discarded the trailing
//! line. The Go gate caught it because Go returned the signed text verbatim.
//! This is the same mechanism, applied to a COSE `Sig_structure`.
//!
//! # What this gate does NOT close
//!
//! Narrower than "the delegation format is now externally validated", and the
//! narrowness is the point:
//!
//! - **It is not the sole guard for most of the format's rules.** The measured
//!   drift table below shows which injections this gate catches *and* which the
//!   in-crate suite already catches. Where both fire, this gate is corroboration
//!   rather than the only witness.
//! - **It says nothing about the semantic rules.** Cross-subject and cross-kind
//!   replay (spec §3.3), unknown-value and invalid-pair rejection (§1.2, §2.3)
//!   and non-oracle failure (§3.5) are `verify_delegation`'s job and the Go tool
//!   has no opinion about any of them; it is handed a subject nowhere and never
//!   asked which error fired.
//!
//!   It does, however, read labels 1 and 4 independently and print both, which is
//!   what makes `subject_kind != role` checkable by a party that did not choose
//!   the numbering. That property is why `role` starts at 2.
//! - **It says nothing about whether the frozen bytes were the right ones.** It
//!   proves two implementations agree, not that they agree on a good format.
//!   That judgement is the specification's, and the vector in
//!   `delegation_vector/` is what pins the answer once it is made.
//!
//! # Measured drift injections
//!
//! Each was applied to a snapshot of `crates/lys-core/src/delegation/` in an
//! isolated worktree, measured, then restored with the restore verified by
//! SHA-256 against the snapshot. Recorded so nobody has to re-derive which
//! checks are load-bearing — and because one row of it changed this file's
//! design:
//!
//! | injection | in-crate `delegation::*` (46 tests) | this gate | `delegation_vector` |
//! |---|---|---|---|
//! | `CONTENT_TYPE` v1 → v2 | **3 fail** | **both fail** — go-cose refuses the content type | **3 of 5 fail** |
//! | payload map emits `{2, 1, 3, 4}` | **18 fail** | **both fail** — but see below | **3 of 5 fail** |
//! | `delegation_preimage` returns `Sig_structure ‖ 0x0a` | **2 fail**, 44 pass | **both fail** — go-cose reports `verification error` | **3 of 5 fail** |
//!
//! The third row is the one this file exists for. It is the increment-3 defect
//! reproduced exactly: lys signs bytes that are not the artifact's, and because
//! `sign_delegation`, `assemble_delegation` and `verify_delegation` all route
//! through the same `delegation_preimage`, **44 of the 46 in-crate delegation
//! tests stay green.** Neither of the two that fail is about the preimage; they
//! fail incidentally. go-cose fails it immediately, because go-cose builds the
//! `Sig_structure` from the artifact rather than from us.
//!
//! # The blind spot this gate had, found by measuring rather than by reasoning
//!
//! The second row originally read **"passes"** for this gate, and that was not a
//! prediction — it was measured. A key-order permutation applied inside
//! `payload_bytes` reaches the artifact *and* the preimage this file compares
//! against, go-cose hands the same permuted payload back verbatim, and the two
//! agree perfectly about the wrong encoding. **A verbatim return cannot see a
//! drift that is inside the bytes both parties read**, and no amount of care
//! about the comparison would have changed that.
//!
//! What closes it is a party with an opinion about what canonical *means*, so
//! `delegation-verify` now re-encodes both signature-covered maps with
//! `fxamacker/cbor`'s `CoreDetEncOptions` (RFC 8949 §4.2) and requires
//! byte-identity. That is a real second party for spec §3.4 — RFC 9052 §9 does
//! not mandate map key ordering, lys elects RFC 8949 §4.2, and before this the
//! rule was pinned outside `lys-core` by the frozen vector alone.
//!
//! # Which axis of independence this is, and which it is not
//!
//! **Implementation and language.** `veraison/go-cose` is a COSE library written
//! by other people, from RFC 9052, years before this format existed; it has
//! never heard of lys. `fxamacker/cbor` decodes the payload and judges its
//! canonicality from RFC 8949 §4.2. Neither was written to agree with this
//! crate.
//!
//! **Not platform, not toolchain, not custody.** One machine, one Go toolchain,
//! one vendored dependency resolution, one `GOPROXY=off` build of a pinned tree.
//! Two implementations agreeing here says nothing about a third environment, and
//! nothing at all about whether the vendored copy is the one upstream publishes.
//!
//! **Not encoding, either — that axis is elsewhere.** The independent encoder
//! and the fixed vector it produced live in `delegation_vector/`, whose hex is
//! written out as literals. This file's comparisons are all against values
//! *this* crate computed, so on its own it would be an implementation cross-check
//! with no anchor; the vector file is what pins the anchor.
//!
//! # What the Go tool is keyed on
//!
//! The root key handed to `delegation-verify` is the one **this test names**,
//! never the `kid` read back out of the artifact. That is the format's central
//! trap (spec §3.2): a delegation carries its signer's key in its own protected
//! header, so a verifier that reads the key out of the artifact accepts an
//! attacker's perfectly-signed delegation for anybody's origin. The scaffold
//! prints `kid` and never consults it; this file asserts the equality itself.
//!
//! # Availability
//!
//! Gated with the format it tests: with `unstable-anchor` off there is no
//! `delegation` module and this file compiles to nothing. It runs under
//! `--all-features`.
//!
//! # Environment contract
//!
//! See [`harness`] — vendored, network-free (`GOPROXY=off`), and a hard failure
//! rather than a skip when `LYS_REQUIRE_GO` is set. A missing Go toolchain on a
//! developer machine is a skip and is announced on stderr.
//!
//! [`cose.Verifier`]: https://pkg.go.dev/github.com/veraison/go-cose#Verifier

#![cfg(feature = "unstable-anchor")]

#[path = "../harness/mod.rs"]
mod harness;

use std::collections::BTreeMap;
use std::path::Path;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::Ed25519Identity;
use lys_core::delegation::{
    DelegationClaim, DelegationRole, DelegationSubjectKind, delegation_preimage, sign_delegation,
    verify_delegation,
};

mod refusals;
mod signing;

/// The root seed of the specification's fixed vector: bytes `0x00..=0x1f`.
const ROOT_SEED: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

/// The delegated key's seed: bytes `0x20..=0x3f`.
const DELEGATED_SEED: [u8; 32] = [
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
];

/// A second root key, used only to prove the Go tool verifies against the key
/// it is *told* rather than the one the artifact carries.
const OTHER_ROOT_SEED: [u8; 32] = *b"lys-delegation-gate-other-root!!";

/// The reserved name from the specification's vector. Not a production origin;
/// a committed constant naming a real one would be the mistake the format's
/// origin field exists to avoid.
const ORIGIN: &str = "example.test";

/// The identity for a fixed seed, via the public `load` route.
///
/// The `TempDir` is returned because it must outlive the read.
fn identity(seed: &[u8; 32]) -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    let key = Ed25519Identity::load(&path).unwrap();
    (dir, key)
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        write!(acc, "{byte:02x}").expect("writing to a String cannot fail");
        acc
    })
}

/// The four envelope lines the scaffold always prints.
const ENVELOPE_FIELDS: [&str; 4] = ["sig_structure", "protected", "payload", "kid"];

/// The `<name> <value>` lines the scaffold prints, as a map.
///
/// The scaffold pins no payload label set — it reports the labels it finds — so
/// this cannot assert a fixed line count. What it asserts instead is *internal
/// consistency*: the four envelope lines are all present, and the number of
/// `payload_<label>` lines equals the number of labels `payload_labels`
/// announced. A tool that printed half its fields would otherwise satisfy every
/// lookup the caller happened to make, and a tool that announced five labels
/// while printing four values would look like a passing parse.
fn parse_report(stdout: &[u8]) -> BTreeMap<String, String> {
    let text = String::from_utf8(stdout.to_vec()).expect("the report is ASCII");
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (name, value) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("unparseable report line {line:?}"));
        assert!(
            fields.insert(name.to_string(), value.to_string()).is_none(),
            "the scaffold printed {name} twice"
        );
    }
    for required in ENVELOPE_FIELDS {
        assert!(
            fields.contains_key(required),
            "delegation-verify did not report {required}: {fields:?}"
        );
    }
    let announced = fields
        .get("payload_labels")
        .unwrap_or_else(|| panic!("no payload_labels line: {fields:?}"));
    let label_count = announced.split(',').count();
    let value_lines = fields.keys().filter(|k| k.starts_with("payload_")).count() - 1;
    assert_eq!(
        label_count, value_lines,
        "the scaffold announced labels {announced} but printed {value_lines} values"
    );
    assert_eq!(
        fields.len(),
        ENVELOPE_FIELDS.len() + 1 + label_count,
        "unexpected extra lines in the report: {fields:?}"
    );
    fields
}

/// The payload labels this format version is expected to carry, and their
/// values, built from the claim **this test supplied**.
///
/// Written here as `("payload_<label>", "<type>:<value>")` pairs rather than
/// read back from the scaffold, so a field the encoder silently dropped or
/// re-typed is a disagreement rather than an absent comparison.
///
/// This is the one place in this file that knows the payload's shape. When the
/// format gains a label, exactly this function changes — the Go scaffold does
/// not, because it reports what it finds.
fn expected_payload_report(claim: &DelegationClaim) -> Vec<(String, String)> {
    vec![
        // Labels 1 and 2 are the typed subject, and the kind PRECEDES the value
        // it types. Both are reported as `uint`/`tstr` by a decoder that has
        // never heard of lys, so a kind emitted as a tstr — or a value emitted as
        // a uint — is a value disagreement here rather than a parse that works.
        (
            "payload_1".to_string(),
            format!("uint:{}", claim.subject_kind.wire_value()),
        ),
        (
            "payload_2".to_string(),
            format!("tstr:{}", to_hex(claim.subject_value.as_bytes())),
        ),
        (
            "payload_3".to_string(),
            format!("bstr:{}", to_hex(&claim.delegated_public_key)),
        ),
        // Label 4 is the role, whose vocabulary is offset past the subject
        // kind's — `operational` is 2, not 1 — so that no valid pair has
        // `subject_kind == role` and a transposition of labels 1 and 4 cannot be
        // byte-identical. This gate is where that becomes checkable by a party
        // that reads both labels independently.
        (
            "payload_4".to_string(),
            format!("uint:{}", claim.role.wire_value()),
        ),
        (
            "payload_5".to_string(),
            format!("uint:{}", claim.not_before_unix_ms),
        ),
        // Label 6, added by the adversarial review: `sequence`, the value that
        // orders delegations. Without it the format was replayable by an
        // attacker holding no key material — two issuances of one claim are
        // byte-identical, so a verbatim copy of a superseded delegation
        // resurrected a revoked key under last-wins-by-log-position.
        ("payload_6".to_string(), format!("uint:{}", claim.sequence)),
    ]
}

/// Builds the vendored `cose-conformance` tool, returning it with the
/// directories that must outlive it.
fn cose_tool(go: &Path) -> (tempfile::TempDir, std::path::PathBuf) {
    let workdir = tempfile::tempdir().unwrap();
    let bin = workdir.path().join("cosetool");
    build_go_tool(go, &workdir.path().join("gocache"), &bin);
    (workdir, bin)
}

/// The claims the sweep runs. Chosen so that both variable-length fields cross
/// every RFC 8949 head-width boundary they can reach, because a head written one
/// width too wide is still *decodable* and would round-trip through our own
/// encoder without complaint.
fn sweep_claims(delegated: [u8; 32]) -> Vec<DelegationClaim> {
    // `not_before` heads: immediate (<24), 1-byte, 2-byte, 4-byte, 8-byte, and
    // both sides of each boundary.
    let timestamps = [
        0u64,
        1,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        4_294_967_295,
        4_294_967_296,
        1_700_000_000_000,
        u64::MAX,
    ];
    // Subject-value lengths: the same boundaries for a tstr head, down to the
    // shortest value the format permits.
    //
    // The empty string used to lead this list — it is where a length-prefix bug
    // is easiest to hide — and it is gone because the format now **refuses** an
    // empty origin: a verifier whose configured origin is unset would otherwise
    // match one and accept it. `delegation::sign_tests` and
    // `delegation::encoding_tests` carry the dedicated rejection cases; a
    // one-character origin keeps the short-head coverage here.
    let subject_values = [
        "x".to_string(),
        "a".repeat(23),
        "b".repeat(24),
        "c".repeat(255),
        "d".repeat(256),
        ORIGIN.to_string(),
    ];
    // `sequence` head widths, swept alongside the rest: inline, one-byte,
    // two-byte, four-byte and eight-byte, so a head written one width too wide
    // has nowhere to hide in this field either.
    // `u64::MAX` is refused by the format (a sequence must have a successor),
    // so the widest head is swept at the largest issuable value.
    let sequences = [0u64, 23, 24, 255, 300, 65_536, u64::MAX - 1];

    // Both valid `(subject_kind, role)` pairs are swept, alternating, so the Go
    // gate reads each label pair on the same axis as everything else. Only the
    // pairs the format defines appear here: an invalid pair cannot be signed, and
    // its rejection is `lys-core`'s own gate rather than go-cose's.
    let pairs = [
        (DelegationSubjectKind::Domain, DelegationRole::Operational),
        (DelegationSubjectKind::Seat, DelegationRole::SpeaksFor),
    ];

    let mut claims = Vec::new();
    for (index, not_before_unix_ms) in timestamps.into_iter().enumerate() {
        for (offset, subject_value) in subject_values.iter().enumerate() {
            let (subject_kind, role) = pairs[(index + offset) % pairs.len()];
            claims.push(DelegationClaim {
                subject_kind,
                subject_value: subject_value.clone(),
                delegated_public_key: delegated,
                role,
                not_before_unix_ms,
                sequence: sequences[(index + offset) % sequences.len()],
            });
        }
    }
    claims
}

/// Decodes a hex field from the scaffold's report.
fn hex_bytes(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd-length hex {hex:?}");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex digit"))
        .collect()
}
