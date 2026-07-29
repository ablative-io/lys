//! OpenSSH public-key and `allowed_signers` text forms for a lys Ed25519 key.
//!
//! # Why this exists
//!
//! A lys identity is an Ed25519 keypair, and so is an SSH signing key. Git
//! can sign commits and tags with an SSH key (`gpg.format = ssh`) and verify
//! them against an `allowed_signers` file, which means a lys identity can
//! sign commits that `git verify-commit` checks on plain GitHub — no
//! accounts, no third-party service, no lys installed on the verifier's
//! machine. That is the same "a stranger can check this with standard
//! tooling" property the rest of this crate exists for, obtained for the
//! price of an encoding.
//!
//! # Invariants
//!
//! - The public-key blob is the RFC 4253 §6.6 / RFC 8709 §4 form: the string
//!   `"ssh-ed25519"` followed by the 32-byte key, each length-prefixed with
//!   a big-endian `u32`, then standard base64 with padding. Every
//!   `ssh-ed25519` key therefore begins `AAAAC3NzaC1lZDI1NTE5AAAAI`, which
//!   is asserted in the tests as a cross-check against an external constant
//!   rather than only against our own output.
//! - A principal is non-empty and free of whitespace. `allowed_signers` is a
//!   whitespace-separated line format, so a principal containing a space
//!   would silently shift every following field — the exact class of quiet
//!   corruption this crate refuses elsewhere.
//! - Only public material is handled here. Nothing in this module can emit a
//!   private key, and there is deliberately no private-key export: an
//!   OpenSSH private key file is a second at-rest copy of the seed, and this
//!   crate keeps exactly one.
//!
//! # This is not a lys wire format
//!
//! These encodings are OpenSSH's, not ours. They carry no domain-separation
//! tag and nothing here freezes under the lys versioning rules — if OpenSSH
//! changes its text form, the correct response is to follow it, because the
//! whole value of this module is agreeing with somebody else's parser.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::{TrustError, TrustResult};

/// The SSH public-key algorithm name for Ed25519 (RFC 8709 §4).
const SSH_ED25519: &[u8] = b"ssh-ed25519";

/// The signature namespace Git uses for commit and tag signatures.
///
/// `allowed_signers` entries are scoped to a namespace; `git` is the one
/// `git verify-commit` and `git verify-tag` present.
const GIT_NAMESPACE: &str = "git";

/// Encodes a length-prefixed SSH string into `out` (RFC 4251 §5).
fn push_ssh_string(out: &mut Vec<u8>, bytes: &[u8]) {
    // Every input here is a fixed 11-byte name or a 32-byte key, so the cast
    // cannot truncate; it is written as a checked conversion anyway so a
    // future caller with a larger input fails loudly rather than silently
    // encoding a wrapped length.
    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
}

/// Returns the OpenSSH one-line public-key form of an Ed25519 public key.
///
/// The result is `ssh-ed25519 <base64 blob>`, byte-compatible with
/// `authorized_keys`, `allowed_signers`, and `ssh-keygen -l`. No comment
/// field is appended; callers that want one can add it, since a comment is
/// free text that no parser interprets.
#[must_use]
pub fn openssh_public_key(public_key: &[u8; 32]) -> String {
    let mut blob = Vec::with_capacity(51);
    push_ssh_string(&mut blob, SSH_ED25519);
    push_ssh_string(&mut blob, public_key);
    format!("ssh-ed25519 {}", STANDARD.encode(&blob))
}

/// Returns an `allowed_signers` line binding `principal` to this key for
/// Git signatures.
///
/// The form is `<principal> namespaces="git" ssh-ed25519 <base64 blob>`.
/// Write it to a file and point Git at it with
/// `git config gpg.ssh.allowedSignersFile <path>`; `git verify-commit` then
/// checks commits signed by this key with no lys tooling present.
///
/// The namespace is pinned to `git` rather than left open. An unscoped entry
/// authorises this key for every SSH signature namespace, including ones the
/// operator never intended, and a signing key for commits is not
/// automatically a signing key for anything else.
///
/// # Errors
///
/// Returns [`TrustError::KeyManagement`] if `principal` is empty or contains
/// whitespace. `allowed_signers` is whitespace-separated, so such a
/// principal would shift every following field and produce a file that
/// parses into something other than what was written.
pub fn allowed_signers_line(principal: &str, public_key: &[u8; 32]) -> TrustResult<String> {
    if principal.is_empty() {
        return Err(TrustError::KeyManagement {
            reason: "allowed_signers principal must not be empty".to_string(),
        });
    }
    if principal.chars().any(char::is_whitespace) {
        return Err(TrustError::KeyManagement {
            reason: "allowed_signers principal must not contain whitespace: the file format is \
                     whitespace-separated, so a principal with a space would shift every \
                     following field"
                .to_string(),
        });
    }
    Ok(format!(
        "{principal} namespaces=\"{GIT_NAMESPACE}\" {}",
        openssh_public_key(public_key)
    ))
}

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod tests;
