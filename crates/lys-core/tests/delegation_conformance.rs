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
//!   `delegation_vector.rs` is what pins the answer once it is made.
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
//! and the fixed vector it produced live in `delegation_vector.rs`, whose hex is
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
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::collections::BTreeMap;
use std::path::Path;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::Ed25519Identity;
use lys_core::delegation::{
    DelegationClaim, DelegationRole, DelegationSubjectKind, delegation_preimage, sign_delegation,
    verify_delegation,
};

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
        let _ = write!(acc, "{byte:02x}");
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

/// go-cose accepts what lys signs, and — the assertion this file exists for —
/// the bytes go-cose signed over are byte-identical to `delegation_preimage`'s.
#[test]
fn go_cose_signs_over_exactly_the_bytes_delegation_preimage_builds() {
    let Some(go) = go_or_skip("go-cose delegation conformance") else {
        return;
    };
    let (_workdir, bin) = cose_tool(&go);

    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    let root_public_key = root.public_key_bytes();
    let root_hex = to_hex(&root_public_key);
    let delegated_public_key = delegated.public_key_bytes();

    let claims = sweep_claims(delegated_public_key);
    let mut cases = 0usize;
    let mut wide_timestamp_cases = 0usize;
    let mut long_subject_cases = 0usize;
    let mut distinct_sequences = std::collections::BTreeSet::new();
    let mut distinct_pairs = std::collections::BTreeSet::new();

    for claim in &claims {
        let artifact = sign_delegation(&root, claim).unwrap();
        let preimage = delegation_preimage(&root_public_key, claim);

        let (ok, stdout) =
            run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &artifact);
        assert!(
            ok,
            "go-cose rejected a delegation lys signed (subject {} bytes, \
             not_before {})",
            claim.subject_value.len(),
            claim.not_before_unix_ms
        );
        let report = parse_report(&stdout);

        // ---- THE assertion. Not "go-cose said yes" — which bytes it said yes
        // to. An implementation that has never seen lys reassembled the
        // Sig_structure from the artifact and reached the same 165-or-so bytes
        // `delegation_preimage` produced.
        assert_eq!(
            report["sig_structure"],
            to_hex(&preimage),
            "go-cose signed over different bytes than delegation_preimage built \
             (subject {} bytes, not_before {})",
            claim.subject_value.len(),
            claim.not_before_unix_ms
        );

        // ---- The preimage's own shape, read out of the bytes go-cose returned
        // rather than restated here. `protected` comes back bstr-wrapped,
        // exactly as it sits inside the Sig_structure.
        let protected = hex_bytes(&report["protected"]);
        let payload = hex_bytes(&report["payload"]);
        // 12 = 0x84 (4-array) + 0x6A + "Signature1".
        assert!(
            preimage[12..].starts_with(&protected),
            "the protected bucket go-cose read is not the one inside our preimage"
        );
        // RFC 9052 §4.4's third element. Pinned here because an unpinned
        // `external_aad` sits INSIDE the signed bytes: two conforming
        // implementations would produce different signatures over one claim and
        // neither would be wrong.
        assert_eq!(
            preimage[12 + protected.len()],
            0x40,
            "external_aad must be the empty byte string h''"
        );
        assert!(
            preimage.ends_with(&payload),
            "the payload go-cose read is not the one inside our preimage"
        );

        // ---- Every payload field the scaffold found, with its CBOR type, so a
        // type or a dropped field surfaces as a value disagreement rather than
        // as a parse that happens to work. The label list is compared too: a
        // field this format is supposed to carry and does not is a
        // `payload_labels` mismatch here, not a lookup nobody made.
        let expected = expected_payload_report(claim);
        let expected_labels = expected
            .iter()
            .map(|(name, _)| name.trim_start_matches("payload_"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            report["payload_labels"], expected_labels,
            "the payload does not carry the labels this format version defines"
        );
        let mut payload_fields_checked = 0usize;
        for (name, value) in &expected {
            assert_eq!(report[name], *value, "payload field {name} disagrees");
            payload_fields_checked += 1;
        }
        assert_eq!(
            payload_fields_checked, 6,
            "the payload comparison must cover every field this version defines"
        );
        // The two fields whose wire values must differ, read out of what the Go
        // decoder reported rather than out of our own claim. If the vocabularies
        // were renumbered so that `subject_kind == role`, a transposition of the
        // two labels would become undetectable and this assertion is where the
        // second party notices.
        assert_ne!(
            report["payload_1"], report["payload_4"],
            "subject_kind and role must not share a wire value, or transposing \
             them is invisible on the wire"
        );

        // ---- And `kid`: compared HERE, by this test, against the key it named.
        // The scaffold never consults it — see the module docs.
        assert_eq!(report["kid"], root_hex);

        if claim.not_before_unix_ms > 65_535 {
            wide_timestamp_cases += 1;
        }
        if claim.subject_value.len() > 255 {
            long_subject_cases += 1;
        }
        distinct_sequences.insert(claim.sequence);
        distinct_pairs.insert((claim.subject_kind.wire_value(), claim.role.wire_value()));
        cases += 1;
    }

    // Count what fired. A loop that ran zero times satisfies every assertion
    // inside it, and a Go gate that never spawned looks exactly like one that
    // passed.
    assert_eq!(cases, 72, "12 timestamps x 6 subject values");
    // Both valid pairs reached the Go gate. A sweep that only ever emitted the
    // anchor's pair would leave the seat one unread by any second party.
    assert_eq!(
        distinct_pairs,
        std::collections::BTreeSet::from([(1u64, 2u64), (2, 3)]),
        "both valid (subject_kind, role) pairs must have been read by go-cose"
    );
    // And the sweep reached every `sequence` head width, so the newest payload
    // field is covered on the same axis as the rest rather than only at 300.
    assert_eq!(
        distinct_sequences.len(),
        7,
        "every sequence head width must have appeared in the sweep"
    );
    // And the sweep reached the shapes where a head width can be wrong at all.
    // A four- or eight-byte head written one width too wide is still perfectly
    // decodable, so nothing but a byte comparison against another
    // implementation discriminates — and a sweep confined to small values would
    // leave that undiscriminated while looking thorough.
    assert_eq!(
        wide_timestamp_cases, 30,
        "cases whose not_before needs a four- or eight-byte CBOR head"
    );
    assert_eq!(
        long_subject_cases, 12,
        "cases whose subject value needs a two-byte tstr length"
    );
}

/// Decodes a hex field from the scaffold's report.
fn hex_bytes(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd-length hex {hex:?}");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex digit"))
        .collect()
}

/// go-cose refuses what it should refuse.
///
/// A gate made only of successes cannot tell a working verifier from a broken
/// one, and a gate made only of refusals cannot tell a working verifier from one
/// that refuses everything — so the control is first and the refusals follow.
#[test]
fn go_cose_refuses_a_flipped_signature_and_the_wrong_root_key() {
    let Some(go) = go_or_skip("go-cose delegation negatives") else {
        return;
    };
    let (_workdir, bin) = cose_tool(&go);

    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    let (_other_dir, other_root) = identity(&OTHER_ROOT_SEED);
    let root_public_key = root.public_key_bytes();
    let other_root_public_key = other_root.public_key_bytes();
    assert_ne!(
        root_public_key, other_root_public_key,
        "the two root keys must differ or the wrong-key case is a no-op"
    );
    let root_hex = to_hex(&root_public_key);
    let other_hex = to_hex(&other_root_public_key);

    let claim = DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: ORIGIN.to_string(),
        delegated_public_key: delegated.public_key_bytes(),
        role: DelegationRole::Operational,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: 300,
    };
    let honest = sign_delegation(&root, &claim).unwrap();

    // ---- Positive control, FIRST. Every case below asserts a refusal, and a
    // tool that refused everything would satisfy all of them at once. That is
    // not hypothetical in this repository: drifting a Go content-type constant
    // by one character once left a conformance test made of refusals entirely
    // green.
    let (ok, stdout) = run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &honest);
    assert!(
        ok,
        "the control failed: go-cose refused an honest delegation, so no \
         refusal below means anything"
    );
    let report = parse_report(&stdout);
    assert_eq!(
        report["sig_structure"],
        to_hex(&delegation_preimage(&root_public_key, &claim)),
        "the control failed: go-cose exited 0 without agreeing about the \
         signed bytes"
    );

    // ---- A flipped bit in the signature. The last 64 bytes of the artifact are
    // the signature; the assertion below proves the flip landed there rather
    // than trusting the layout.
    let mut flipped = honest.clone();
    let sig_start = flipped.len() - 64;
    flipped[sig_start] ^= 0x01;
    assert_ne!(flipped, honest, "the flip must change the artifact");
    assert_eq!(
        flipped[..sig_start],
        honest[..sig_start],
        "the flip must land in the signature, not in the signed bytes"
    );
    let (ok, _stdout) = run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &flipped);
    assert!(
        !ok,
        "go-cose accepted a delegation with a flipped signature bit"
    );
    assert!(
        verify_delegation(
            &flipped,
            &root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_err(),
        "lys accepted a delegation with a flipped signature bit"
    );

    // ---- The central trap (spec §3.2), from the Go side. The artifact is
    // untouched and perfectly signed; only the key the verifier was TOLD to use
    // changes. A tool that read the key out of `kid` would accept this.
    let (ok, _stdout) = run_built_tool(&bin, &["delegation-verify", other_hex.as_str()], &honest);
    assert!(
        !ok,
        "go-cose accepted an honest delegation against a root key that did not \
         sign it — it must be verifying against `kid` rather than against the \
         key it was given"
    );

    // ---- And the same trap from the lys side, so the two implementations are
    // shown to refuse it for the same reason rather than only the Go one being
    // checked. A delegation genuinely signed by the OTHER root key is
    // cryptographically perfect and vouches for nothing.
    let attacker = sign_delegation(&other_root, &claim).unwrap();
    let (ok, _stdout) = run_built_tool(&bin, &["delegation-verify", other_hex.as_str()], &attacker);
    assert!(
        ok,
        "the control failed: an attacker's delegation must be internally valid, \
         otherwise the refusal below is about malformedness rather than about trust"
    );
    assert!(
        verify_delegation(
            &attacker,
            &root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_err(),
        "lys accepted a delegation signed by a root key the caller does not trust"
    );
    assert!(
        verify_delegation(
            &attacker,
            &other_root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_ok(),
        "the control failed: the attacker's own delegation must verify under \
         the attacker's own key"
    );
}
