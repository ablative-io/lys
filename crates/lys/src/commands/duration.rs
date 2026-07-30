//! Validity-window parsing for `ca issue`.
//!
//! # Why this exists
//!
//! `lys_core`'s issuance API has always taken a [`Duration`], so the library
//! could mint a certificate valid for thirty minutes from the day it landed.
//! Only the CLI could not ask for one: `--validity-days` is a whole number of
//! days with a floor of 1. That is a real gap for short-lived scoped grants,
//! where the *point* is that the window is minutes to hours — a grant that has
//! to last a day is not short-lived.
//!
//! So `--validity` accepts a suffixed count and `--validity-days` stays exactly
//! as it was. Nothing is removed and no existing invocation changes meaning;
//! this module only widens what can be asked for.
//!
//! # Invariants
//!
//! - Exactly one unit, no compound specs. `1h30m` is refused rather than
//!   guessed at, because the alternative is a parser that silently reads
//!   `1h30m` as one hour and calls it success.
//! - A zero-length window is refused. It would produce a certificate whose
//!   `notBefore` equals its `notAfter` — valid at no instant, and a confusing
//!   way to discover that.
//! - Arithmetic is overflow-checked, so a spec like `99999999999999999999d`
//!   fails with a message rather than wrapping into a short window. A validity
//!   window that silently became *shorter* than asked is the dangerous
//!   direction, since it looks like it worked.
//! - No new dependency. A duration crate would be more general than the two
//!   flags need, and a trust tool pays for every dependency it takes.

use std::time::Duration;

use crate::commands::error::{CliError, CliResult};

/// Seconds in one day, shared with the `--validity-days` conversion.
pub const SECONDS_PER_DAY: u64 = 86_400;

/// Accepted unit suffixes and the seconds each represents.
const UNITS: &[(char, u64)] = &[('s', 1), ('m', 60), ('h', 3_600), ('d', SECONDS_PER_DAY)];

/// Resolves the validity window from the two mutually exclusive flags.
///
/// Clap enforces that exactly one is supplied, so the "neither" arm is
/// unreachable through the CLI. It still returns an error rather than panicking:
/// the guarantee lives in an argument-group declaration one edit away from being
/// broken, and a panic in a trust tool is a worse outcome than a message.
///
/// # Errors
///
/// Returns [`CliError::InvalidValidity`] if `spec` is malformed (see
/// [`parse_validity`]) or if neither flag was supplied.
pub fn validity_window(days: Option<u32>, spec: Option<&str>) -> CliResult<Duration> {
    match (days, spec) {
        // `--validity-days` keeps its own arithmetic: `u32` days cannot
        // overflow `u64` seconds, so there is nothing to check here.
        (Some(days), _) => Ok(Duration::from_secs(u64::from(days) * SECONDS_PER_DAY)),
        (None, Some(spec)) => parse_validity(spec),
        (None, None) => Err(CliError::InvalidValidity {
            spec: String::new(),
            reason: "no validity window given: pass --validity or --validity-days".to_string(),
        }),
    }
}

/// Parses a validity window: a positive count followed by one unit suffix,
/// `s`, `m`, `h`, or `d`. For example `90s`, `30m`, `12h`, `7d`.
///
/// # Errors
///
/// Returns [`CliError::InvalidValidity`] if the spec is empty, carries no unit
/// suffix or an unknown one, has a non-digit in its count, is zero, or overflows
/// when converted to seconds.
pub fn parse_validity(spec: &str) -> CliResult<Duration> {
    let invalid = |reason: &str| CliError::InvalidValidity {
        spec: spec.to_string(),
        reason: reason.to_string(),
    };

    let mut chars = spec.chars();
    let unit = chars
        .next_back()
        .ok_or_else(|| invalid("empty; expected a count and a unit, e.g. 30m"))?;
    let count = chars.as_str();

    let seconds_per_unit = UNITS
        .iter()
        .find(|(suffix, _)| *suffix == unit)
        .map(|(_, seconds)| *seconds)
        .ok_or_else(|| {
            invalid(
                "unknown unit: expected one of s (seconds), m (minutes), h (hours), \
                 d (days), and exactly one — compound specs like 1h30m are not accepted",
            )
        })?;

    if count.is_empty() {
        return Err(invalid("no count before the unit, e.g. 30m"));
    }
    // Checked before `parse`, so `1_000m`, `+5m` and `1h30m` are refused here
    // with an explanation rather than by a bare integer-parse error.
    if !count.chars().all(|c| c.is_ascii_digit()) {
        return Err(invalid(
            "the count must be ASCII digits only, followed by exactly one unit",
        ));
    }

    let count: u64 = count
        .parse()
        .map_err(|_err| invalid("count is too large to represent"))?;
    let seconds = count
        .checked_mul(seconds_per_unit)
        .ok_or_else(|| invalid("window is too long to represent in seconds"))?;
    if seconds == 0 {
        return Err(invalid(
            "a zero-length window is valid at no instant; ask for at least 1s",
        ));
    }
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
#[path = "duration_tests.rs"]
mod tests;
