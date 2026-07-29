//! PEM framing for X.509 certificates and PKCS#10 certificate-signing
//! requests (RFC 7468 `CERTIFICATE` and `CERTIFICATE REQUEST` labels).
//!
//! The CLI stores both as PEM because that is the interop norm — any standard
//! tool can read them. `lys-core` speaks DER only, so this module carries the
//! encode/decode framing and nothing else: the DER bytes pass through
//! unchanged and uninterpreted. Decoding is strict about structure (exact
//! `BEGIN`/`END` lines framing a base64 body) and every rejection names the
//! reason.
//!
//! Decoding requires the **exact** label it was asked for. A certificate and a
//! request are both signed DER structures, and accepting either where the
//! other is expected would hand a parser an artifact from the wrong protocol;
//! the label is the cheapest place to refuse that.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::commands::error::{CliError, CliResult};

/// RFC 7468 label for an X.509 certificate.
const CERTIFICATE_LABEL: &str = "CERTIFICATE";
/// RFC 7468 label for a PKCS#10 certificate-signing request.
const CERTIFICATE_REQUEST_LABEL: &str = "CERTIFICATE REQUEST";
/// RFC 7468 recommends wrapping base64 at 64 characters per line.
const LINE_WIDTH: usize = 64;

/// Encodes certificate DER as a PEM `CERTIFICATE` block with a trailing
/// newline, base64 body wrapped at 64 characters.
pub fn encode_certificate(der: &[u8]) -> String {
    encode_labelled(CERTIFICATE_LABEL, der)
}

/// Encodes request DER as a PEM `CERTIFICATE REQUEST` block, the label
/// `openssl req` and every other standard tool expects.
pub fn encode_certificate_request(der: &[u8]) -> String {
    encode_labelled(CERTIFICATE_REQUEST_LABEL, der)
}

/// Encodes `der` as a PEM block under `label`.
fn encode_labelled(label: &str, der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut out = String::with_capacity(body.len() + body.len() / LINE_WIDTH + 64);
    out.push_str("-----BEGIN ");
    out.push_str(label);
    out.push_str("-----");
    out.push('\n');
    for (index, character) in body.chars().enumerate() {
        if index > 0 && index % LINE_WIDTH == 0 {
            out.push('\n');
        }
        out.push(character);
    }
    if !body.is_empty() {
        out.push('\n');
    }
    out.push_str("-----END ");
    out.push_str(label);
    out.push_str("-----");
    out.push('\n');
    out
}

/// Decodes a PEM `CERTIFICATE` block read from `path` back into DER bytes.
///
/// Requires exactly one certificate block: the first non-blank line must be
/// the `BEGIN CERTIFICATE` boundary, the last must be the `END CERTIFICATE`
/// boundary, and everything between must be valid base64. Leading and
/// trailing whitespace on each line is tolerated; anything else is rejected —
/// including a `CERTIFICATE REQUEST` block.
///
/// # Errors
///
/// Returns [`CliError::PemParse`] naming `path` and the specific structural
/// problem: non-UTF-8 content, missing boundaries, an empty body, or invalid
/// base64.
pub fn decode_certificate(pem_bytes: &[u8], path: &Path) -> CliResult<Vec<u8>> {
    decode_labelled(CERTIFICATE_LABEL, pem_bytes, path)
}

/// Decodes a PEM `CERTIFICATE REQUEST` block read from `path` back into DER
/// bytes. A `CERTIFICATE` block is rejected.
///
/// # Errors
///
/// As [`decode_certificate`], against the request label.
pub fn decode_certificate_request(pem_bytes: &[u8], path: &Path) -> CliResult<Vec<u8>> {
    decode_labelled(CERTIFICATE_REQUEST_LABEL, pem_bytes, path)
}

/// Decodes a PEM block that must carry exactly `label`.
fn decode_labelled(label: &str, pem_bytes: &[u8], path: &Path) -> CliResult<Vec<u8>> {
    let begin_boundary = format!("-----BEGIN {label}-----");
    let end_boundary = format!("-----END {label}-----");
    let text = std::str::from_utf8(pem_bytes).map_err(|source| CliError::PemParse {
        path: path.to_path_buf(),
        reason: format!("file is not UTF-8 text: {source}"),
    })?;

    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: "file is empty".to_string(),
        });
    };
    if *first != begin_boundary {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("first line must be {begin_boundary:?}"),
        });
    }
    if lines.len() < 2 || *last != end_boundary {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("last line must be {end_boundary:?}"),
        });
    }

    let body: String = lines[1..lines.len() - 1].concat();
    if body.is_empty() {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("{label} body is empty"),
        });
    }

    STANDARD.decode(&body).map_err(|source| CliError::PemParse {
        path: path.to_path_buf(),
        reason: format!("{label} body is not valid base64: {source}"),
    })
}

#[cfg(test)]
#[path = "pem_tests.rs"]
mod tests;
