//! The translation's checks (HOME-009 R1): the one Codex version whose
//! rollout shape was measured, and the time zone the rollout's file name is
//! written in.
//!
//! The zone is an explicit input: nothing here reads the environment, falls
//! back to UTC or asks the machine for its zone. A name is resolved through
//! the time zone database bundled in the build (jiff's `tzdb-bundle-always`),
//! never the system's, so one name gives the same offsets on every machine.
//! A stamp is parsed as RFC 3339 and placed in the zone to the second, the
//! fraction dropped.

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::HomeError;

/// The one Codex version whose rollout shape was measured.
pub const MEASURED_VERSION: &str = "0.156.0";

/// Refuse any Codex version but the measured one, exactly; no prefix passes.
pub fn check_version(version: &str) -> Result<(), HomeError> {
    if version == MEASURED_VERSION {
        Ok(())
    } else {
        Err(HomeError::UnmeasuredCodexVersion {
            version: version.to_owned(),
        })
    }
}

/// Whether a name is IANA-shaped: one or more components of ASCII letters,
/// digits, `_`, `+` and `-`, joined by single `/`, so no leading `/` or `:`.
fn iana_shaped(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|component| {
            !component.is_empty()
                && component
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-'))
        })
}

/// The zone of a name from the bundled database. An absent or empty name,
/// or one not IANA-shaped, is refused as unnamed; a shaped name the database
/// does not hold is refused as unknown.
pub fn zone_of(name: Option<&str>) -> Result<TimeZone, HomeError> {
    let Some(name) = name else {
        return Err(HomeError::UnnamedTimeZone {
            value: "unset".to_owned(),
        });
    };
    if !iana_shaped(name) {
        return Err(HomeError::UnnamedTimeZone {
            value: name.to_owned(),
        });
    }
    // The database's own error names only the zone, which the refusal names.
    jiff::tz::db()
        .get(name)
        .ok()
        .ok_or_else(|| HomeError::UnknownTimeZone {
            zone: name.to_owned(),
        })
}

/// An entry's stamp parsed as RFC 3339; a stamp that does not parse is
/// refused naming the entry.
pub fn parse_stamp(entry: &str, stamp: &str) -> Result<OffsetDateTime, HomeError> {
    // The parser's own error would quote the stamp; the refusal names the entry.
    OffsetDateTime::parse(stamp, &Rfc3339)
        .ok()
        .ok_or_else(|| HomeError::StampNotRfc3339 {
            entry: entry.to_owned(),
        })
}

/// An entry's stamp as the local date and time in the zone, to the second.
pub fn local_time(entry: &str, stamp: &str, zone: &TimeZone) -> Result<DateTime, HomeError> {
    let instant = parse_stamp(entry, stamp)?;
    let seconds = Timestamp::from_second(instant.unix_timestamp())
        .ok()
        .ok_or_else(|| HomeError::StampNotRfc3339 {
            entry: entry.to_owned(),
        })?;
    Ok(seconds.to_zoned(zone.clone()).datetime())
}
