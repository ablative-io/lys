#![cfg(test)]
//! Which signature lines count: other names, key ids, failed and duplicate
//! candidates, extension lines, the body split, and checkpoint tampers.

use super::*;

// --- Candidate semantics ---

#[test]
fn signature_under_different_name_is_filtered_not_accepted() {
    // Origin-confusion half 1: the same key signing under a different
    // keyname is filtered out (different name AND different key ID).
    let (_, identity) = golden_identity();
    let note = sign_note(GOLDEN_BODY, "other.example/log", &identity).unwrap();
    assert_rejected(note.as_bytes(), "signature under a different keyname");
}

#[test]
fn matching_key_id_alone_is_never_authentication() {
    // A candidate whose (name, key ID) match but whose signature is
    // garbage must not be accepted: key IDs are filters, the full Ed25519
    // check decides — and its failure rejects the whole note.
    let mut blob = GOLDEN_KEY_ID.to_vec();
    blob.extend_from_slice(&[0u8; 64]);
    let forged = format!(
        "{GOLDEN_BODY}\n\u{2014} {GOLDEN_NAME} {}\n",
        STANDARD.encode(&blob)
    );
    assert_rejected(forged.as_bytes(), "matching key ID with garbage signature");
}

#[test]
fn failed_known_key_signature_rejects_despite_later_valid_line() {
    // Go parity (note.Open returns InvalidSignatureError; C2SP: a failed
    // known-key signature rejects the whole note): a garbage candidate
    // matching the verifier's (name, key ID) rejects even though a fully
    // valid signature line follows it.
    let mut garbage_blob = GOLDEN_KEY_ID.to_vec();
    garbage_blob.extend_from_slice(&[0u8; 64]);
    let note = format!(
        "{GOLDEN_BODY}\n\u{2014} {GOLDEN_NAME} {}\n\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n",
        STANDARD.encode(&garbage_blob)
    );
    assert_rejected(
        note.as_bytes(),
        "failed known-key signature before a valid line",
    );
}

#[test]
fn duplicate_lines_after_a_verifying_first_candidate_are_skipped() {
    // Go parity (the `seen` map): once the first matching candidate
    // verifies, later lines by the same signer — even garbage ones — are
    // skipped and the note is accepted.
    let mut garbage_blob = GOLDEN_KEY_ID.to_vec();
    garbage_blob.extend_from_slice(&[0u8; 64]);
    let note = format!(
        "{GOLDEN_BODY}\n\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n\u{2014} {GOLDEN_NAME} {}\n",
        STANDARD.encode(&garbage_blob)
    );
    let body = verify_note(note.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body, GOLDEN_BODY);
}

#[test]
fn golden_signature_spliced_onto_different_body_is_rejected() {
    let other_body = "example.com/lys/test\n2\nYKU+7Q3oepDI5ZQnxZxGJTwzp2oJUCpRgBMAknt+a9w=\n";
    let spliced = format!("{other_body}\n\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n");
    assert_rejected(spliced.as_bytes(), "golden signature over a different body");
}

// --- Extension lines and smuggling ---

#[test]
fn resigned_extension_line_is_tolerated_by_checkpoint_verification() {
    // A timestamp-like fourth line WITH a re-signed note is accepted as an
    // extension line by design; the parsed body ignores it.
    let (_, identity) = golden_identity();
    let body_with_extension = format!("{GOLDEN_BODY}1234567890\n");
    let note = sign_note(&body_with_extension, GOLDEN_NAME, &identity).unwrap();
    let body = verify_checkpoint(note.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body.tree_size(), 3);
    assert_eq!(body.encode(), GOLDEN_BODY);
}

#[test]
fn unsigned_extension_line_breaks_the_signature() {
    // The same fourth line inserted WITHOUT re-signing: the signature no
    // longer covers the body text and the note is rejected.
    let tampered = golden_note().replacen("=\n\n", "=\n1234567890\n\n", 1);
    assert_rejected(tampered.as_bytes(), "unsigned extension line");
}

#[test]
fn em_dash_line_inside_body_stays_signed_content() {
    // Signature-line smuggling: a body line that LOOKS like a signature
    // line remains part of the signed body (split happens at the LAST
    // blank line), and the note still verifies with the line intact.
    let (_, identity) = golden_identity();
    let smuggled = format!("{GOLDEN_BODY}\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n");
    let note = sign_note(&smuggled, GOLDEN_NAME, &identity).unwrap();
    let body = verify_note(note.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body, smuggled, "smuggled line must remain in the body");
}

#[test]
fn verify_note_splits_at_the_last_blank_line() {
    // Go parity for the split point: Go note.Sign requires only a trailing
    // newline, so a Go-signed note may carry a body CONTAINING a blank
    // line, and Go note.Open splits at bytes.LastIndex("\n\n"). lys
    // sign_note refuses such bodies, so hand-sign one here: verify_note
    // must split at the LAST blank line and return the body intact.
    let (_, identity) = golden_identity();
    let blank_line_body = "A\n\nB\n";
    let signature = identity.sign(blank_line_body.as_bytes());
    let mut blob = GOLDEN_KEY_ID.to_vec();
    blob.extend_from_slice(&signature);
    let note = format!(
        "{blank_line_body}\n\u{2014} {GOLDEN_NAME} {}\n",
        STANDARD.encode(&blob)
    );
    let body = verify_note(note.as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(
        body, blank_line_body,
        "the body's own blank line must stay inside the signed body"
    );
}

// --- verify_checkpoint binding and parse collapse ---

#[test]
fn checkpoint_origin_must_equal_verifier_name() {
    // Origin-confusion half 2: an honest key, a validly signed note, but
    // the body's origin differs from the verifier's name (R1 binding).
    let (_, identity) = golden_identity();
    let foreign_body = "other.example/log\n3\nz3Y6BByBzu8VeKYIP3XGG+8uABTyo+aDqX/Pylvn8Zo=\n";
    let note = sign_note(foreign_body, GOLDEN_NAME, &identity).unwrap();

    // The note itself verifies (signed under the golden keyname)...
    verify_note(note.as_bytes(), &golden_verifier()).unwrap();
    // ...but checkpoint verification enforces origin == verifier name.
    let err = verify_checkpoint(note.as_bytes(), &golden_verifier()).unwrap_err();
    assert!(matches!(err, TrustError::NoteVerification));
}

#[test]
fn unparseable_body_collapses_to_note_verification() {
    let (_, identity) = golden_identity();
    // Valid note, but the body is not a checkpoint (one line only).
    let note = sign_note("not-a-checkpoint\n", GOLDEN_NAME, &identity).unwrap();
    let err = verify_checkpoint(note.as_bytes(), &golden_verifier()).unwrap_err();
    assert!(matches!(err, TrustError::NoteVerification));
}

#[test]
fn tree_size_tamper_in_checkpoint_note_is_rejected() {
    // Splice a different tree size into the golden note without
    // re-signing: signature breaks.
    let tampered = golden_note().replacen("\n3\n", "\n4\n", 1);
    assert_rejected(tampered.as_bytes(), "tree-size line tamper");

    // Leading-zero tree size WITH a valid re-sign: rejected by the strict
    // body parse inside verify_checkpoint.
    let (_, identity) = golden_identity();
    let leading_zero_body = GOLDEN_BODY.replacen("\n3\n", "\n03\n", 1);
    let note = sign_note(&leading_zero_body, GOLDEN_NAME, &identity).unwrap();
    let err = verify_checkpoint(note.as_bytes(), &golden_verifier()).unwrap_err();
    assert!(matches!(err, TrustError::NoteVerification));
}

#[test]
fn root_hash_tamper_in_checkpoint_note_is_rejected() {
    let tampered = golden_note().replacen("z3Y6", "z3Y7", 1);
    assert_rejected(tampered.as_bytes(), "root-hash line tamper");
}
