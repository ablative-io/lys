#![cfg(test)]
//! Malformed notes: dash lookalikes, structural tampers, control characters,
//! the size cap, and malformed signature blobs and lines.

use super::*;

#[test]
fn dash_lookalikes_are_rejected() {
    let note = golden_note();
    assert_rejected(
        note.replacen('\u{2014}', "\u{2013}", 1).as_bytes(),
        "en dash U+2013",
    );
    assert_rejected(
        note.replacen('\u{2014}', "\u{2015}", 1).as_bytes(),
        "horizontal bar U+2015",
    );
    assert_rejected(
        note.replacen('\u{2014}', "--", 1).as_bytes(),
        "double hyphen",
    );
    assert_rejected(
        note.replacen("\u{2014} ", "\u{2014}", 1).as_bytes(),
        "em dash without following space",
    );
}

#[test]
fn structural_tampers_are_rejected() {
    let note = golden_note();
    assert_rejected(b"", "empty note");
    assert_rejected(GOLDEN_BODY.as_bytes(), "body with no signature block");
    assert_rejected(
        note.replacen("\n\n", "\n", 1).as_bytes(),
        "missing blank line",
    );
    assert_rejected(
        format!("{GOLDEN_BODY}\n").as_bytes(),
        "blank line but empty signature block",
    );
    assert_rejected(
        note.trim_end_matches('\n').to_string().as_bytes(),
        "signature block not newline-terminated",
    );
}

#[test]
fn control_characters_and_invalid_utf8_are_rejected() {
    let note = golden_note();
    assert_rejected(
        note.replacen("test\n3", "test\r\n3", 1).as_bytes(),
        "carriage return in body",
    );
    let mut invalid_utf8 = note.into_bytes();
    invalid_utf8[0] = 0xff;
    assert_rejected(&invalid_utf8, "invalid UTF-8 byte");
}

#[test]
fn oversized_note_is_rejected() {
    let mut oversized = golden_note().into_bytes();
    oversized.extend(std::iter::repeat_n(b'a', 1024 * 1024));
    assert_rejected(&oversized, "note above the 1 MiB cap");
}

/// Hand-signs an otherwise-valid golden-key note whose body is a single
/// `'a'`-line of exactly `body_len` bytes (including its trailing `'\n'`),
/// bypassing `sign_note`'s own size cap so verify-side behavior can be
/// tested in isolation.
fn hand_signed_note_with_body_len(body_len: usize) -> String {
    let (_dir, identity) = golden_identity();
    let mut body = "a".repeat(body_len - 1);
    body.push('\n');
    let signature = identity.sign(body.as_bytes());
    let mut blob = GOLDEN_KEY_ID.to_vec();
    blob.extend_from_slice(&signature);
    format!(
        "{body}\n\u{2014} {GOLDEN_NAME} {}\n",
        STANDARD.encode(&blob)
    )
}

/// Envelope overhead around the body for a single golden-key signature
/// line: blank-line `'\n'` (1) + em dash (3) + space (1) + name (20) +
/// space (1) + base64 of 68 blob bytes (92) + trailing `'\n'` (1).
const GOLDEN_NOTE_OVERHEAD: usize = 1 + 3 + 1 + GOLDEN_NAME.len() + 1 + 92 + 1;

#[test]
fn size_cap_boundary_is_exact_for_otherwise_valid_notes() {
    // Isolates the 1 MiB cap: both notes are fully valid except for size,
    // so the one-byte-over rejection can come only from the cap itself.
    let at_cap = hand_signed_note_with_body_len(MAX_NOTE_BYTES - GOLDEN_NOTE_OVERHEAD);
    assert_eq!(at_cap.len(), MAX_NOTE_BYTES);
    let body = verify_note(at_cap.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body.len(), MAX_NOTE_BYTES - GOLDEN_NOTE_OVERHEAD);

    let one_over = hand_signed_note_with_body_len(MAX_NOTE_BYTES - GOLDEN_NOTE_OVERHEAD + 1);
    assert_eq!(one_over.len(), MAX_NOTE_BYTES + 1);
    assert_rejected(
        one_over.as_bytes(),
        "otherwise-valid note one byte above the 1 MiB cap",
    );
}

#[test]
fn sign_note_refuses_bodies_that_would_exceed_the_cap() {
    // The emitted-note-re-verifies invariant, both sides of the boundary:
    // a body whose note lands exactly on the cap signs AND re-verifies;
    // one byte more and sign_note refuses instead of emitting a note that
    // verify_note would reject.
    let (_dir, identity) = golden_identity();
    let verifier = golden_verifier();

    let mut at_cap_body = "a".repeat(MAX_NOTE_BYTES - GOLDEN_NOTE_OVERHEAD - 1);
    at_cap_body.push('\n');
    let note = sign_note(&at_cap_body, GOLDEN_NAME, &identity).unwrap();
    assert_eq!(note.len(), MAX_NOTE_BYTES);
    assert_eq!(
        verify_note(note.as_bytes(), &verifier).unwrap(),
        at_cap_body
    );

    let mut over_body = "a".repeat(MAX_NOTE_BYTES - GOLDEN_NOTE_OVERHEAD);
    over_body.push('\n');
    let err = sign_note(&over_body, GOLDEN_NAME, &identity).unwrap_err();
    assert!(matches!(err, TrustError::CheckpointEncoding { .. }));
}

#[test]
fn malformed_signature_blobs_are_rejected() {
    let blob = STANDARD.decode(GOLDEN_SIG_BLOB_B64).unwrap();

    // Blob decoding to fewer than 5 bytes: structurally malformed line.
    let short = STANDARD.encode(&blob[..4]);
    assert_rejected(
        golden_note()
            .replacen(GOLDEN_SIG_BLOB_B64, &short, 1)
            .as_bytes(),
        "blob shorter than 5 bytes",
    );

    // Altered key ID: no candidate matches the verifier.
    let mut altered_id = blob.clone();
    altered_id[0] ^= 0xff;
    assert_rejected(
        golden_note()
            .replacen(GOLDEN_SIG_BLOB_B64, &STANDARD.encode(&altered_id), 1)
            .as_bytes(),
        "altered key ID",
    );

    // 63- and 65-byte signatures: candidate matches but never verifies.
    let sixty_three = STANDARD.encode(&blob[..4 + 63]);
    let mut long = blob.clone();
    long.push(0x00);
    let sixty_five = STANDARD.encode(&long);
    for (bad, label) in [(sixty_three, "63-byte"), (sixty_five, "65-byte")] {
        assert_rejected(
            golden_note()
                .replacen(GOLDEN_SIG_BLOB_B64, &bad, 1)
                .as_bytes(),
            label,
        );
    }

    // Bit-flipped signature body: full Ed25519 verification fails.
    let mut flipped = blob;
    flipped[10] ^= 0x01;
    assert_rejected(
        golden_note()
            .replacen(GOLDEN_SIG_BLOB_B64, &STANDARD.encode(&flipped), 1)
            .as_bytes(),
        "bit-flipped signature",
    );
}

#[test]
fn non_canonical_base64_signature_blobs_are_rejected() {
    // Unpadded re-encoding (padding stripped).
    let unpadded = GOLDEN_SIG_BLOB_B64.trim_end_matches('=');
    assert_rejected(
        golden_note()
            .replacen(GOLDEN_SIG_BLOB_B64, unpadded, 1)
            .as_bytes(),
        "unpadded base64",
    );

    // Non-canonical trailing bits in the final data character
    // ('U' -> 'V' sets a trailing bit that must be zero).
    let non_canonical = GOLDEN_SIG_BLOB_B64.replacen("8QU=", "8QV=", 1);
    assert_rejected(
        golden_note()
            .replacen(GOLDEN_SIG_BLOB_B64, &non_canonical, 1)
            .as_bytes(),
        "non-canonical trailing bits",
    );
}

#[test]
fn any_malformed_signature_line_rejects_the_whole_note() {
    // Go parity: a malformed second line rejects the note even though the
    // first line alone would verify.
    let note = golden_note();
    assert_rejected(
        format!("{note}garbage line\n").as_bytes(),
        "malformed second line",
    );
    assert_rejected(
        format!("{note}\u{2014} {GOLDEN_NAME} not*base64\n").as_bytes(),
        "second line with invalid base64",
    );
    assert_rejected(
        format!("{note}\u{2014} bad+name {GOLDEN_SIG_BLOB_B64}\n").as_bytes(),
        "second line with invalid name",
    );
}

#[test]
fn one_hundred_one_signature_lines_are_rejected_and_one_hundred_accepted() {
    let sig_line = format!("\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n");

    let hundred = format!("{GOLDEN_BODY}\n{}", sig_line.repeat(100));
    let body = verify_note(hundred.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body, GOLDEN_BODY);

    let hundred_one = format!("{GOLDEN_BODY}\n{}", sig_line.repeat(101));
    assert_rejected(hundred_one.as_bytes(), "101 signature lines");
}
