//! Signing and verifying C2SP signed notes, mirroring the Go `sumdb/note`
//! reference implementation line-for-line.
//!
//! # Invariants
//!
//! - The signature line prefix is the U+2014 em dash followed by one ASCII
//!   space — bytes `E2 80 94 20`, never `--` or a lookalike.
//! - The Ed25519 signature covers the body **including its trailing
//!   `'\n'`**, excluding the blank line and signature lines.
//! - Key ID = first 4 bytes of `SHA-256(keyname ‖ 0x0A ‖ 0x01 ‖ pubkey)`.
//! - [`sign_note`] enforces preconditions (valid name; body non-empty,
//!   `'\n'`-terminated, no `"\n\n"`, no ASCII control character below
//!   `0x20` other than `'\n'`) that guarantee the emitted note always
//!   re-verifies under [`verify_note`] AND under Go `note.Open`.
//! - [`verify_note`] splits at the LAST `"\n\n"` (Go `bytes.LastIndex`),
//!   rejects any malformed signature line, caps at 100 signature lines
//!   (Go's exact cap) and 1 MiB total (a `lys` defensive cap), and treats
//!   key IDs as candidate FILTERS only — full strict Ed25519 verification
//!   decides, and any one fully-verifying candidate accepts (C2SP
//!   semantics).
//! - Every verification failure collapses to the single non-oracle
//!   [`TrustError::NoteVerification`] value.
//!
//! [`TrustError::NoteVerification`]: crate::error::TrustError::NoteVerification

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

use crate::error::{TrustError, TrustResult};
use crate::keys::Ed25519Identity;

use super::body::CheckpointBody;
use super::verifier_key::{NoteVerifierKey, validate_note_name};

/// Defensive cap on total note size (1 MiB). Go has no cap; real
/// checkpoints are a few hundred bytes, so no legitimate note is affected.
const MAX_NOTE_BYTES: usize = 1024 * 1024;

/// Maximum number of signature lines, adopted verbatim from Go
/// (`note.Open` rejects past 100).
const MAX_SIGNATURE_LINES: usize = 100;

/// The signature-line prefix: U+2014 em dash then one ASCII space
/// (bytes `E2 80 94 20`, Go `sigPrefix`).
const SIG_PREFIX: &str = "\u{2014} ";

/// Computes the signed-note key ID:
/// `SHA-256(name ‖ 0x0A ‖ 0x01 ‖ pubkey)[..4]`.
///
/// The `0x01` is the Ed25519 algorithm byte; the key ID therefore binds
/// the key name, the algorithm, and the public key together.
///
/// # Errors
///
/// Returns [`TrustError::VerifierKey`] if `name` violates the key-name
/// rules.
pub fn key_id(name: &str, public_key: &[u8; 32]) -> TrustResult<[u8; 4]> {
    validate_note_name(name)?;
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update([0x0a]);
    hasher.update([0x01]);
    hasher.update(public_key);
    let digest = hasher.finalize();
    let mut id = [0u8; 4];
    id.copy_from_slice(&digest[..4]);
    Ok(id)
}

/// Signs `body` under `name` and emits the complete note text:
/// body ‖ blank line ‖ one signature line.
///
/// The signature line is
/// `"\u{2014} {name} {base64(key_id ‖ signature)}\n"` and the Ed25519
/// signature covers the body bytes including the trailing `'\n'`.
///
/// # Errors
///
/// Returns [`TrustError::CheckpointEncoding`] if the name is invalid, or
/// if the body is empty, does not end with `'\n'`, contains `"\n\n"`, or
/// contains an ASCII control character below `0x20` other than `'\n'`.
/// These preconditions guarantee the emitted note always re-verifies under
/// [`verify_note`] and under Go `note.Open`.
pub fn sign_note(body: &str, name: &str, identity: &Ed25519Identity) -> TrustResult<String> {
    validate_note_name(name).map_err(|e| TrustError::CheckpointEncoding {
        reason: format!("invalid note key name: {e}"),
    })?;
    if body.is_empty() {
        return Err(TrustError::CheckpointEncoding {
            reason: "note body must not be empty".to_string(),
        });
    }
    if !body.ends_with('\n') {
        return Err(TrustError::CheckpointEncoding {
            reason: "note body must end with a newline".to_string(),
        });
    }
    if body.contains("\n\n") {
        return Err(TrustError::CheckpointEncoding {
            reason: "note body must not contain a blank line".to_string(),
        });
    }
    if body.chars().any(|c| c < ' ' && c != '\n') {
        return Err(TrustError::CheckpointEncoding {
            reason: "note body must not contain ASCII control characters other than newline"
                .to_string(),
        });
    }
    let id =
        key_id(name, &identity.public_key_bytes()).map_err(|e| TrustError::CheckpointEncoding {
            reason: format!("invalid note key name: {e}"),
        })?;
    let signature = identity.sign(body.as_bytes());
    let mut blob = Vec::with_capacity(4 + 64);
    blob.extend_from_slice(&id);
    blob.extend_from_slice(&signature);
    Ok(format!(
        "{body}\n{SIG_PREFIX}{name} {}\n",
        STANDARD.encode(&blob)
    ))
}

/// A parsed candidate signature line: the signer name, the declared 4-byte
/// key ID, and the remaining signature bytes.
struct SignatureLine<'a> {
    name: &'a str,
    key_id: [u8; 4],
    signature: Vec<u8>,
}

/// Parses and verifies an untrusted note; returns the verified body text
/// (including its trailing newline).
///
/// Mirrors Go `note.Open`: total-size cap, UTF-8 with no ASCII control
/// characters below `0x20` except `'\n'`, split at the last `"\n\n"`,
/// every signature line structurally valid (any malformed line rejects
/// the whole note), at most 100 signature lines. Candidate lines matching
/// the verifier's `(name, key ID)` are checked with strict Ed25519
/// verification over the body bytes; any one success accepts.
///
/// # Errors
///
/// Returns [`TrustError::NoteVerification`] on every failure mode — size,
/// UTF-8, structure, no candidate, bad signature — with no distinguishing
/// detail (non-oracle).
pub fn verify_note(note_bytes: &[u8], verifier: &NoteVerifierKey) -> TrustResult<String> {
    let (body, signature_lines) = parse_note(note_bytes)?;
    let public_key = verifier.public_key();
    let mut seen: Vec<&[u8]> = Vec::new();
    for line in &signature_lines {
        if line.name != verifier.name() || line.key_id != verifier.key_id() {
            continue;
        }
        // Deduplicate identical candidate signatures (Go skips re-checking
        // an already-seen signature blob for the same key).
        if seen.contains(&line.signature.as_slice()) {
            continue;
        }
        seen.push(&line.signature);
        if line.signature.len() == 64
            && Ed25519Identity::verify(&public_key, body.as_bytes(), &line.signature).is_ok()
        {
            return Ok(body.to_string());
        }
    }
    Err(TrustError::NoteVerification)
}

/// [`verify_note`] followed by [`CheckpointBody::parse`] and the
/// origin-binding check `body.origin() == verifier.name()`.
///
/// The binding check closes origin confusion: a key that signs two logs
/// cannot have a checkpoint for one log accepted by a verifier configured
/// for the other, because `lys` verifiers are named by the origin they
/// trust.
///
/// # Errors
///
/// Returns [`TrustError::NoteVerification`] on any failure, including body
/// parsing and the binding check (non-oracle).
pub fn verify_checkpoint(
    note_bytes: &[u8],
    verifier: &NoteVerifierKey,
) -> TrustResult<CheckpointBody> {
    let body_text = verify_note(note_bytes, verifier)?;
    let body = CheckpointBody::parse(&body_text).map_err(|_err| TrustError::NoteVerification)?;
    if body.origin() != verifier.name() {
        return Err(TrustError::NoteVerification);
    }
    Ok(body)
}

/// Structural parse of a note: returns the body text (with its trailing
/// newline) and every parsed signature line. Any structural violation is
/// [`TrustError::NoteVerification`].
fn parse_note(note_bytes: &[u8]) -> TrustResult<(&str, Vec<SignatureLine<'_>>)> {
    if note_bytes.len() > MAX_NOTE_BYTES {
        return Err(TrustError::NoteVerification);
    }
    let text = std::str::from_utf8(note_bytes).map_err(|_err| TrustError::NoteVerification)?;
    if text.chars().any(|c| c < ' ' && c != '\n') {
        return Err(TrustError::NoteVerification);
    }
    // Split at the LAST blank line (Go bytes.LastIndex): the body keeps its
    // trailing '\n'; the signature block is everything after the blank line
    // and must be non-empty and '\n'-terminated.
    let split = text.rfind("\n\n").ok_or(TrustError::NoteVerification)?;
    let body = &text[..=split];
    let sig_block = &text[split + 2..];
    if sig_block.is_empty() || !sig_block.ends_with('\n') {
        return Err(TrustError::NoteVerification);
    }
    let mut lines = Vec::new();
    for line in sig_block[..sig_block.len() - 1].split('\n') {
        lines.push(parse_signature_line(line)?);
        if lines.len() > MAX_SIGNATURE_LINES {
            return Err(TrustError::NoteVerification);
        }
    }
    Ok((body, lines))
}

/// Parses one signature line: `"\u{2014} <name> <base64 blob>"` where the
/// blob decodes canonically to at least 5 bytes (4-byte key ID plus a
/// non-empty signature). Any violation is [`TrustError::NoteVerification`].
fn parse_signature_line(line: &str) -> TrustResult<SignatureLine<'_>> {
    let rest = line
        .strip_prefix(SIG_PREFIX)
        .ok_or(TrustError::NoteVerification)?;
    let (name, blob_b64) = rest.split_once(' ').ok_or(TrustError::NoteVerification)?;
    if blob_b64.is_empty() || validate_note_name(name).is_err() {
        return Err(TrustError::NoteVerification);
    }
    let blob = STANDARD
        .decode(blob_b64)
        .map_err(|_err| TrustError::NoteVerification)?;
    if blob.len() < 5 {
        return Err(TrustError::NoteVerification);
    }
    let mut id = [0u8; 4];
    id.copy_from_slice(&blob[..4]);
    Ok(SignatureLine {
        name,
        key_id: id,
        signature: blob[4..].to_vec(),
    })
}

#[cfg(test)]
#[path = "note_tests.rs"]
mod tests;
