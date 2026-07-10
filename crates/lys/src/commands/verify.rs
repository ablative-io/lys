//! `lys verify` — verify a `COSE_Sign1` attestation artifact against a
//! payload.
//!
//! Reads the raw artifact bytes written by `lys attest`, re-reads the
//! candidate payload, and delegates to
//! [`lys_core::attestation::verify_attestation_bytes`]. Success prints the
//! verified attestation details and exits 0; any failure — malformed or
//! non-canonical artifact, tampered payload, tampered timestamp, forged
//! signature, wrong signer key — exits 1 with a single indistinguishable
//! message, matching the library's deliberate non-oracle behaviour. There
//! is no separate parse error for the attestation file: an unparseable
//! artifact is indistinguishable from an unverifiable one by design.

use std::path::Path;

use lys_core::TrustError;
use lys_core::attestation::verify_attestation_bytes;

use crate::commands::error::{CliError, CliResult};
use crate::commands::files::read_file;
use crate::commands::hex::hex_lower;

/// `lys verify --attestation <file> --payload <file>`.
///
/// # Errors
///
/// Returns [`CliError::Io`] if either file cannot be read,
/// [`CliError::VerificationFailed`] — the single generic message — if the
/// artifact is malformed or non-canonical, the payload does not match, or
/// the signature is invalid, and [`CliError::Trust`] for any other library
/// failure.
pub fn run(attestation_path: &Path, payload_path: &Path) -> CliResult<()> {
    let artifact_bytes = read_file(attestation_path, "attestation file")?;
    let payload = read_file(payload_path, "payload file")?;
    match verify_attestation_bytes(&artifact_bytes, &payload) {
        Ok(attestation) => {
            println!("attestation verified");
            println!(
                "signer public key (ed25519): {}",
                hex_lower(&attestation.signer_public_key)
            );
            println!(
                "payload hash (sha256): {}",
                hex_lower(&attestation.payload_hash)
            );
            println!("signed at (unix ms): {}", attestation.timestamp);
            Ok(())
        }
        Err(TrustError::InvalidSignature) => Err(CliError::VerificationFailed),
        Err(other) => Err(CliError::Trust(other)),
    }
}
