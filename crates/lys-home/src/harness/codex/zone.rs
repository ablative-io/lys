//! The measured Codex version, the time zone a rollout is named in, and the
//! entry stamps a rollout carries (HOME-009 R1).
//!
//! Invariants:
//!
//! - Only [`MEASURED_VERSION`] is rendered. Any other version, a prefix of
//!   it included, is refused by name ([`HomeError::UnmeasuredCodexVersion`]):
//!   a rollout shape is measured from files that version wrote, never
//!   assumed to carry over.
//! - The zone is an explicit input. Nothing here reads the environment,
//!   falls back to UTC or asks the machine for its zone: an absent, empty or
//!   non-IANA-shaped name is refused ([`HomeError::UnnamedTimeZone`]), and an
//!   IANA-shaped name the database does not hold is refused
//!   ([`HomeError::UnknownTimeZone`]).
//! - The database is the one bundled into the build
//!   ([`TimeZoneDatabase::bundled`]); the system database is never read, so
//!   one zone name gives the same offsets on every machine.
//! - A stamp is parsed as RFC 3339, strictly; one that does not parse is
//!   refused naming the entry ([`HomeError::StampNotRfc3339`]).

use jiff::Timestamp;
use jiff::tz::{TimeZone, TimeZoneDatabase};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::HomeError;

/// The one Codex version whose rollout shape is measured.
pub const MEASURED_VERSION: &str = "0.156.0";

/// Refuse any version but the measured one, compared exactly.
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
/// digits, `_`, `+` and `-`, joined by single `/`, with no leading `/` or
/// `:`.
fn is_iana_shaped(name: &str) -> bool {
    !name.is_empty() && name.split('/').all(is_component)
}

/// One component of an IANA-shaped name: non-empty, of ASCII letters,
/// digits, `_`, `+` and `-`.
fn is_component(component: &str) -> bool {
    !component.is_empty() && component.bytes().all(is_name_byte)
}

/// Whether a byte may stand in an IANA-shaped name component.
fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-')
}

/// Resolve a zone name in the bundled database.
pub fn resolve_zone(zone: Option<&str>) -> Result<TimeZone, HomeError> {
    let Some(name) = zone else {
        return Err(HomeError::UnnamedTimeZone {
            value: "unset".to_owned(),
        });
    };
    if !is_iana_shaped(name) {
        return Err(HomeError::UnnamedTimeZone {
            value: name.to_owned(),
        });
    }
    // The database's own error names only the zone, which the refusal names.
    TimeZoneDatabase::bundled()
        .get(name)
        .ok()
        .ok_or_else(|| HomeError::UnknownTimeZone {
            zone: name.to_owned(),
        })
}

/// Parse an entry's stamp as RFC 3339.
pub fn parse_stamp(id: &str, stamp: &str) -> Result<Timestamp, HomeError> {
    // The parsers' own errors quote the stamp; the refusal names the entry.
    let refused = || HomeError::StampNotRfc3339 { id: id.to_owned() };
    let parsed = OffsetDateTime::parse(stamp, &Rfc3339)
        .ok()
        .ok_or_else(refused)?;
    Timestamp::from_nanosecond(parsed.unix_timestamp_nanos())
        .ok()
        .ok_or_else(refused)
}

/// An instant's wall-clock time in a zone, to the second, the fraction
/// truncated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    /// The year.
    pub year: i16,
    /// The month, 1 to 12.
    pub month: i8,
    /// The day of the month.
    pub day: i8,
    /// The hour, 0 to 23.
    pub hour: i8,
    /// The minute.
    pub minute: i8,
    /// The second, the fraction truncated.
    pub second: i8,
}

impl Local {
    /// The instant's wall-clock time in `zone`.
    #[must_use]
    pub fn of(instant: Timestamp, zone: &TimeZone) -> Self {
        let local = instant.to_zoned(zone.clone()).datetime();
        Self {
            year: local.year(),
            month: local.month(),
            day: local.day(),
            hour: local.hour(),
            minute: local.minute(),
            second: local.second(),
        }
    }

    /// The date with `sep` between its fields.
    fn date(self, sep: char) -> String {
        let (y, m, d) = (self.year, self.month, self.day);
        format!("{y:04}{sep}{m:02}{sep}{d:02}")
    }

    /// The time of day with `sep` between its fields, the fraction dropped.
    fn time(self, sep: char) -> String {
        let (h, m, s) = (self.hour, self.minute, self.second);
        format!("{h:02}{sep}{m:02}{sep}{s:02}")
    }

    /// `YYYY-MM-DDTHH:MM:SS`.
    #[must_use]
    pub fn iso(self) -> String {
        format!("{}T{}", self.date('-'), self.time(':'))
    }

    /// The date directories Codex files a rollout under: `YYYY/MM/DD`.
    #[must_use]
    pub fn date_dirs(self) -> String {
        self.date('/')
    }

    /// The date and time as a rollout's file name carries them:
    /// `YYYY-MM-DDTHH-MM-SS`.
    #[must_use]
    pub fn file_stamp(self) -> String {
        format!("{}T{}", self.date('-'), self.time('-'))
    }
}
