//! Hiding credentials in what an upstream answers.
//!
//! Every hidden secret is found in one pass over the answer: at each place
//! the scan stands, a table says whether any secret starts with that byte,
//! and only then are the secrets that might start there compared, longest
//! first. Each occurrence becomes `[redacted]` and the scan resumes after
//! it. Secrets that never overlap one another are hidden exactly as one
//! pass per secret hid them. Where two overlap, the longest found at the
//! leftmost place is hidden whole, so no secret is left partly shown by an
//! order of passes.
//!
//! An answer is read whole, whatever its size, before any of it returns, so
//! a secret is never split between two pieces of it, and it is never cut
//! short.

use lys_secrets::{Secret, SecretsError};

/// What each hidden occurrence becomes.
pub(crate) const REDACTED: &[u8] = b"[redacted]";

/// The secrets one call hides.
pub(crate) struct Redactor<'s> {
    /// The secrets, longest first, none empty.
    needles: Vec<&'s [u8]>,
    /// Whether some secret starts with each byte.
    starts: [bool; 256],
}

impl<'s> Redactor<'s> {
    /// A redactor hiding each of `hidden`; an empty one hides nothing.
    pub(crate) fn new(hidden: &[&'s Secret]) -> Self {
        let mut needles: Vec<&'s [u8]> = hidden
            .iter()
            .map(|secret| secret.expose())
            .filter(|needle| !needle.is_empty())
            .collect();
        needles.sort_unstable_by_key(|needle| (std::cmp::Reverse(needle.len()), *needle));
        needles.dedup();
        let mut starts = [false; 256];
        for needle in &needles {
            if let Some(first) = needle.first() {
                starts[usize::from(*first)] = true;
            }
        }
        Self { needles, starts }
    }

    /// The length of the longest secret that starts at `at`, when one does.
    fn found_at(&self, haystack: &[u8], at: usize) -> Option<usize> {
        let rest = haystack.get(at..)?;
        let first = rest.first()?;
        if !self.starts[usize::from(*first)] {
            return None;
        }
        self.needles
            .iter()
            .find(|needle| rest.starts_with(needle))
            .map(|needle| needle.len())
    }

    /// `haystack` with every secret hidden, and how many places the one
    /// pass stood on, which is never more than `haystack` is long.
    pub(crate) fn redact_counted(&self, haystack: &[u8]) -> (Vec<u8>, usize) {
        let mut out = Vec::with_capacity(haystack.len());
        let mut at = 0;
        let mut stood = 0;
        while let Some(byte) = haystack.get(at) {
            stood += 1;
            if let Some(len) = self.found_at(haystack, at) {
                out.extend_from_slice(REDACTED);
                at += len;
            } else {
                out.push(*byte);
                at += 1;
            }
        }
        (out, stood)
    }

    /// `haystack` with every secret hidden.
    pub(crate) fn redact(&self, haystack: &[u8]) -> Vec<u8> {
        self.redact_counted(haystack).0
    }

    /// `text` with every secret hidden.
    pub(crate) fn redact_text(&self, text: &str) -> String {
        String::from_utf8_lossy(&self.redact(text.as_bytes())).into_owned()
    }

    /// Whether any secret occurs in `haystack`.
    pub(crate) fn contains(&self, haystack: &[u8]) -> bool {
        (0..haystack.len()).any(|at| self.found_at(haystack, at).is_some())
    }
}

/// The upstream's answer, read whole at any size, with every secret hidden.
/// A failure to read it is named with its words hidden.
pub(crate) async fn redacted_answer(
    upstream: reqwest::Response,
    redactor: &Redactor<'_>,
) -> Result<Vec<u8>, SecretsError> {
    let answer = upstream
        .bytes()
        .await
        .map_err(|error| SecretsError::Encoding {
            context: "upstream answer",
            reason: redactor.redact_text(&error.to_string()),
        })?;
    Ok(redactor.redact(&answer))
}
