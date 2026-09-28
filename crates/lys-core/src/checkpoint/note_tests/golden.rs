#![cfg(test)]
//! The golden note: exact signing bytes, the key id, the signature over the
//! body, and verification of the golden and round-tripped notes.

use super::*;

// --- Golden vectors ---

#[test]
fn sign_note_emits_exact_golden_bytes() {
    let (_dir, identity) = golden_identity();
    let note = sign_note(GOLDEN_BODY, GOLDEN_NAME, &identity).unwrap();
    assert_eq!(note, golden_note());
}

#[test]
fn signature_line_prefix_is_exact_em_dash_bytes() {
    // Assert on the raw note bytes, not via string contains: the four
    // bytes after the blank line must be E2 80 94 20 (U+2014, space).
    let note = golden_note();
    let bytes = note.as_bytes();
    let sig_line_start = GOLDEN_BODY.len() + 1;
    assert_eq!(
        &bytes[sig_line_start..sig_line_start + 4],
        &[0xe2, 0x80, 0x94, 0x20]
    );
}

#[test]
fn key_id_matches_golden_and_hand_computed_sha256() {
    let (_dir, identity) = golden_identity();
    let pubkey = identity.public_key_bytes();
    assert_eq!(
        crate::hex_lower(&pubkey),
        "0cfd0fd81b16accbc5230cf45cba9d4b937d827c6c8dbd44a144e5aba571b9e2"
    );

    let id = key_id(GOLDEN_NAME, &pubkey).unwrap();
    assert_eq!(id, GOLDEN_KEY_ID);

    // Recompute SHA-256(name ‖ 0x0A ‖ 0x01 ‖ pubkey) by hand.
    let mut hasher = Sha256::new();
    hasher.update(GOLDEN_NAME.as_bytes());
    hasher.update([0x0a]);
    hasher.update([0x01]);
    hasher.update(pubkey);
    let digest = hasher.finalize();
    assert_eq!(&digest[..4], &id);
}

#[test]
fn key_id_rejects_invalid_name() {
    let (_dir, identity) = golden_identity();
    let err = key_id("bad name", &identity.public_key_bytes()).unwrap_err();
    assert!(matches!(err, TrustError::VerifierKey { .. }));
}

#[test]
fn signature_covers_body_including_trailing_newline_golden_hex() {
    let (_dir, identity) = golden_identity();
    let signature = identity.sign(GOLDEN_BODY.as_bytes());
    assert_eq!(crate::hex_lower(&signature), GOLDEN_SIG_HEX);

    // The golden blob is exactly key ID ‖ that signature.
    let blob = STANDARD.decode(GOLDEN_SIG_BLOB_B64).unwrap();
    assert_eq!(&blob[..4], &GOLDEN_KEY_ID);
    assert_eq!(&blob[4..], &signature);
}

#[test]
fn verify_note_accepts_golden_and_returns_body() {
    let body = verify_note(golden_note().as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body, GOLDEN_BODY);
}

#[test]
fn verify_checkpoint_accepts_golden_and_parses_body() {
    let body = verify_checkpoint(golden_note().as_bytes(), &golden_verifier()).unwrap();
    assert_eq!(body.origin(), GOLDEN_NAME);
    assert_eq!(body.tree_size(), 3);
    let (root, count) = body.to_root().to_parts();
    assert_eq!(
        crate::hex_lower(&root),
        "cf763a041c81ceef1578a6083f75c61bef2e0014f2a3e683a97fcfca5be7f19a"
    );
    assert_eq!(count, 3);
}

#[test]
fn sign_then_verify_round_trips_for_other_bodies() {
    let (_dir, identity) = golden_identity();
    let verifier = golden_verifier();
    for body in [
        "example.com/lys/test\n0\n47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\n",
        "example.com/lys/test\n1\nMF31n5WQw8msY9KydDw4jjeSRJB4zr9/s9vmRxZDsrc=\n",
    ] {
        let note = sign_note(body, GOLDEN_NAME, &identity).unwrap();
        assert_eq!(verify_note(note.as_bytes(), &verifier).unwrap(), body);
    }
}

// --- sign_note preconditions ---

#[test]
fn sign_note_rejects_invalid_bodies_and_names() {
    let (_dir, identity) = golden_identity();
    // (a) of the trailing-newline boundary: body lacking the trailing
    // newline is refused at signing time.
    let no_newline = GOLDEN_BODY.trim_end_matches('\n');
    for (body, name) in [
        ("", GOLDEN_NAME),
        (no_newline, GOLDEN_NAME),
        ("body\nwith\n\nblank line\n", GOLDEN_NAME),
        ("body\rwith carriage return\n", GOLDEN_NAME),
        ("body\twith tab\n", GOLDEN_NAME),
        (GOLDEN_BODY, "bad name"),
        (GOLDEN_BODY, "bad+name"),
        (GOLDEN_BODY, ""),
    ] {
        let err = sign_note(body, name, &identity).unwrap_err();
        assert!(
            matches!(err, TrustError::CheckpointEncoding { .. }),
            "body: {body:?}, name: {name:?}"
        );
    }
}

// --- Trailing-newline boundary, both directions ---

#[test]
fn signature_over_body_without_trailing_newline_is_rejected() {
    // (b): construct a signature over the body WITHOUT its trailing
    // newline, splice it into an otherwise-valid note. Verification signs
    // the body WITH the newline, so this must fail.
    let (_dir, identity) = golden_identity();
    let body_without_newline = GOLDEN_BODY.trim_end_matches('\n');
    let signature = identity.sign(body_without_newline.as_bytes());
    let mut blob = GOLDEN_KEY_ID.to_vec();
    blob.extend_from_slice(&signature);
    let spliced = format!(
        "{GOLDEN_BODY}\n\u{2014} {GOLDEN_NAME} {}\n",
        STANDARD.encode(&blob)
    );
    let err = verify_note(spliced.as_bytes(), &golden_verifier()).unwrap_err();
    assert!(matches!(err, TrustError::NoteVerification));
}

#[test]
fn golden_note_with_body_final_newline_removed_is_rejected() {
    // (c): flipping the body's final '\n' off the golden note collapses
    // the blank-line separator; the note must be rejected.
    let tampered = golden_note().replacen("=\n\n", "=\n", 1);
    let err = verify_note(tampered.as_bytes(), &golden_verifier()).unwrap_err();
    assert!(matches!(err, TrustError::NoteVerification));
}
