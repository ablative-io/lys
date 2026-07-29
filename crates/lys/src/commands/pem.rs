//! PEM framing for X.509 certificates (RFC 7468 `CERTIFICATE` label).
//!
//! The CLI stores certificates as PEM because that is the X.509 interop
//! norm — any standard tool can read them. `lys-core` speaks DER only, so
//! this module carries the encode/decode framing and nothing else: the DER
//! bytes pass through unchanged and uninterpreted. Decoding is strict about
//! structure (exact `BEGIN`/`END CERTIFICATE` lines framing a base64 body)
//! and every rejection names the reason.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::commands::error::{CliError, CliResult};

/// Opening boundary line of a PEM certificate.
const BEGIN_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----";
/// Closing boundary line of a PEM certificate.
const END_CERTIFICATE: &str = "-----END CERTIFICATE-----";
/// RFC 7468 recommends wrapping base64 at 64 characters per line.
const LINE_WIDTH: usize = 64;

/// Encodes certificate DER as a PEM `CERTIFICATE` block with a trailing
/// newline, base64 body wrapped at 64 characters.
pub fn encode_certificate(der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut out = String::with_capacity(body.len() + body.len() / LINE_WIDTH + 64);
    out.push_str(BEGIN_CERTIFICATE);
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
    out.push_str(END_CERTIFICATE);
    out.push('\n');
    out
}

/// Decodes a PEM `CERTIFICATE` block read from `path` back into DER bytes.
///
/// Requires exactly one certificate block: the first non-blank line must be
/// the `BEGIN CERTIFICATE` boundary, the last must be the `END CERTIFICATE`
/// boundary, and everything between must be valid base64. Leading and
/// trailing whitespace on each line is tolerated; anything else is rejected.
///
/// # Errors
///
/// Returns [`CliError::PemParse`] naming `path` and the specific structural
/// problem: non-UTF-8 content, missing boundaries, an empty body, or invalid
/// base64.
pub fn decode_certificate(pem_bytes: &[u8], path: &Path) -> CliResult<Vec<u8>> {
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
    if *first != BEGIN_CERTIFICATE {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("first line must be {BEGIN_CERTIFICATE:?}"),
        });
    }
    if lines.len() < 2 || *last != END_CERTIFICATE {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("last line must be {END_CERTIFICATE:?}"),
        });
    }

    let body: String = lines[1..lines.len() - 1].concat();
    if body.is_empty() {
        return Err(CliError::PemParse {
            path: path.to_path_buf(),
            reason: "certificate body is empty".to_string(),
        });
    }

    STANDARD.decode(&body).map_err(|source| CliError::PemParse {
        path: path.to_path_buf(),
        reason: format!("certificate body is not valid base64: {source}"),
    })
}

#[cfg(test)]
#[path = "pem_tests.rs"]
mod tests;
