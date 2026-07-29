//! `lys key` subcommands: generate and inspect identity key files.
//!
//! Output discipline: only public material is ever printed — the Ed25519
//! verifying key and the derived X25519 public key, both as lowercase hex.
//! The 32-byte seed never leaves the key file.

use std::path::Path;

use lys_core::Ed25519Identity;
use lys_core::checkpoint::NoteVerifierKey;

use crate::commands::error::{CliError, CliResult};
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;

/// `lys key generate --out <path>`.
///
/// Generates a new Ed25519 identity key at `out` via
/// [`Ed25519Identity::load_or_generate`], which is safe under concurrent
/// callers and loads (rather than clobbers) an existing key file. Reports
/// which of the two happened, then prints the public key.
///
/// # Errors
///
/// Returns [`CliError::Trust`] if the key file cannot be created or an
/// existing file at `out` is not a valid 32-byte seed.
pub fn generate(out: &Path, json: bool) -> CliResult<()> {
    // Existence is checked before the call purely to report accurately
    // whether a key was generated or loaded; `load_or_generate` itself is
    // race-safe regardless.
    let existed = out.exists();
    let identity = Ed25519Identity::load_or_generate(out).map_err(CliError::from)?;
    let mut emit = Emitter::new(json);
    if existed {
        emit.note(&format!("loaded existing identity key: {}", out.display()));
    } else {
        emit.note(&format!("generated new identity key: {}", out.display()));
    }
    if emit.is_json() {
        emit.field(
            "identity key",
            "identity_key_path",
            out.display().to_string(),
        );
        // Whether the key was created or already present is the one thing a
        // caller cannot infer from the file afterwards, so it is a field
        // rather than only a sentence.
        emit.field("generated", "generated", !existed);
    }
    emit.field(
        "public key (ed25519)",
        "public_key_ed25519",
        hex_lower(&identity.public_key_bytes()),
    );
    emit.finish();
    Ok(())
}

/// `lys key inspect --key <path> [--note-name <name>]`.
///
/// Loads an existing identity key file and prints the Ed25519 public key
/// and the derived X25519 public key (used for sealed payload key
/// agreement), both as lowercase hex. When `note_name` is given, also
/// prints the signed-note verifier key line for that name — the name must
/// equal the log origin this key signs checkpoints for, because `lys`
/// verifiers enforce `checkpoint origin == verifier-key name`. Without a
/// name there is nothing truthful to print, so the line is omitted.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the file does not exist,
/// [`CliError::Trust`] if it cannot be read or is not a valid 32-byte seed,
/// or if `note_name` violates the signed-note key-name rules (non-empty,
/// no whitespace, no `'+'`).
pub fn inspect(
    key: &Path,
    note_name: Option<&str>,
    ssh: bool,
    allowed_signers: Option<&str>,
    json: bool,
) -> CliResult<()> {
    let identity = load_identity(key)?;
    let mut emit = Emitter::new(json);
    emit.field(
        "identity key",
        "identity_key_path",
        key.display().to_string(),
    );
    emit.field(
        "public key (ed25519)",
        "public_key_ed25519",
        hex_lower(&identity.public_key_bytes()),
    );
    emit.field(
        "public key (x25519)",
        "public_key_x25519",
        hex_lower(&identity.x25519_public_key()),
    );
    if let Some(name) = note_name {
        let verifier =
            NoteVerifierKey::new(name, identity.public_key_bytes()).map_err(CliError::from)?;
        emit.field(
            "verifier key (signed-note)",
            "verifier_key",
            verifier.to_spec(),
        );
    }
    if ssh {
        emit.field(
            "public key (openssh)",
            "public_key_openssh",
            lys_core::keys::ssh::openssh_public_key(&identity.public_key_bytes()),
        );
    }
    if let Some(principal) = allowed_signers {
        let line =
            lys_core::keys::ssh::allowed_signers_line(principal, &identity.public_key_bytes())
                .map_err(CliError::from)?;
        emit.field("allowed_signers", "allowed_signers", line);
    }
    emit.finish();
    Ok(())
}

/// Load an identity from an existing key file, refusing to generate one.
///
/// Consuming subcommands (`key inspect`, `attest`, `ca issue`, `seal`,
/// `open`) go through [`Ed25519Identity::load`], which can never mint key
/// material — signing with a key the operator never created is the failure
/// this guard exists to prevent, and the load-only constructor closes it
/// with no check-then-act window. The existence pre-check remains solely to
/// produce the friendlier [`CliError::KeyFileMissing`] message with its
/// `lys key generate` remedy.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if `key` does not exist, or
/// [`CliError::Trust`] if the file cannot be read or is invalid.
pub fn load_identity(key: &Path) -> CliResult<Ed25519Identity> {
    if !key.exists() {
        return Err(CliError::KeyFileMissing {
            path: key.to_path_buf(),
        });
    }
    Ed25519Identity::load(key).map_err(CliError::from)
}
