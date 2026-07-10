//! `lys attest` — sign an attestation over a payload file.
//!
//! Reads the payload, signs it with the identity at `--key` via
//! [`lys_core::attestation::sign_attestation`], and writes the resulting
//! tagged `COSE_Sign1` artifact to `--out` as raw bytes (conventional
//! extension `.cose`, media type `application/cose`). The file is the
//! exact `lys/attestation/v2` wire artifact — no CLI-invented framing — so
//! it verifies with any off-the-shelf COSE library and drops verbatim into
//! the transparency log's raw-leaf path.

use std::path::Path;

use lys_core::attestation::sign_attestation;

use crate::commands::error::CliResult;
use crate::commands::files::{read_file, write_file};
use crate::commands::hex::hex_lower;
use crate::commands::key::load_identity;

/// `lys attest --key <path> --payload <file> --out <file>`.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the key file does not exist,
/// [`CliError::Trust`] if it is invalid, and [`CliError::Io`] if the
/// payload cannot be read or the artifact cannot be written.
///
/// [`CliError::KeyFileMissing`]: crate::commands::error::CliError::KeyFileMissing
/// [`CliError::Trust`]: crate::commands::error::CliError::Trust
/// [`CliError::Io`]: crate::commands::error::CliError::Io
pub fn run(key: &Path, payload: &Path, out: &Path) -> CliResult<()> {
    let identity = load_identity(key)?;
    let payload_bytes = read_file(payload, "payload file")?;
    let attestation = sign_attestation(&payload_bytes, &identity);
    write_file(out, &attestation.to_cose_bytes(), "attestation file")?;
    println!("attested payload: {}", payload.display());
    println!(
        "payload hash (sha256): {}",
        hex_lower(&attestation.payload_hash)
    );
    println!(
        "signer public key (ed25519): {}",
        hex_lower(&attestation.signer_public_key)
    );
    println!("signed at (unix ms): {}", attestation.timestamp);
    println!(
        "attestation written: {} (COSE_Sign1, application/cose)",
        out.display()
    );
    Ok(())
}
