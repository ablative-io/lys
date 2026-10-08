//! Instance-owned UTC time for creation and expiry decisions.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

/// The UTC instant a provider supplies without requiring consumers to name its dependency.
pub type UtcInstant = DateTime<Utc>;

/// A failure to obtain or represent a clock reading.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClockError {
    /// The provider could not supply a reading.
    #[error("clock unavailable: {reason}")]
    Unavailable {
        /// The provider's reason for refusing the read.
        reason: String,
    },
    /// The instant cannot be represented by the receiving time type.
    #[error("clock unavailable: instant is outside the supported time range")]
    InstantOutOfRange {
        /// The original integer conversion failure, when one produced the refusal.
        #[source]
        source: Option<std::num::TryFromIntError>,
    },
}

/// A UTC clock supplied when an owner is constructed.
pub trait Clock: std::fmt::Debug + Send + Sync {
    /// Returns this instance's current UTC instant.
    ///
    /// # Errors
    ///
    /// Returns a named clock failure when a reading cannot be supplied.
    fn now(&self) -> Result<DateTime<Utc>, ClockError>;

    /// Returns this instance's current whole unsigned Unix second.
    ///
    /// # Errors
    ///
    /// Propagates the provider's failure and refuses pre-epoch instants.
    fn unix_seconds(&self) -> Result<u64, ClockError> {
        unix_seconds(self.now()?)
    }
}

/// The explicit clock choice owned by a constructed service or command.
///
/// The production variant requires no provider allocation. Supplied providers
/// are moved into their owner and are never replaced after a read failure.
#[derive(Debug, Default)]
pub enum ClockSource {
    /// Read the operating system clock.
    #[default]
    System,
    /// Read the provider supplied by the owner at construction.
    Supplied(Arc<dyn Clock>),
}

impl Clock for ClockSource {
    fn now(&self) -> Result<DateTime<Utc>, ClockError> {
        match self {
            Self::System => SystemClock.now(),
            Self::Supplied(clock) => clock.now(),
        }
    }
}

/// The production clock, read directly from the operating system.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Result<DateTime<Utc>, ClockError> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => utc_from_duration(duration, false),
            Err(error) => utc_from_duration(error.duration(), true),
        }
    }
}

/// Converts a UTC instant to the existing whole unsigned Unix seconds.
///
/// # Errors
///
/// Returns [`ClockError::InstantOutOfRange`] for a pre-epoch instant.
pub fn unix_seconds(instant: DateTime<Utc>) -> Result<u64, ClockError> {
    u64::try_from(instant.timestamp()).map_err(|source| ClockError::InstantOutOfRange {
        source: Some(source),
    })
}

fn utc_from_duration(duration: Duration, before_epoch: bool) -> Result<DateTime<Utc>, ClockError> {
    let seconds =
        i64::try_from(duration.as_secs()).map_err(|source| ClockError::InstantOutOfRange {
            source: Some(source),
        })?;
    let nanos = duration.subsec_nanos();
    let (seconds, nanos) = if before_epoch {
        let seconds = seconds
            .checked_neg()
            .ok_or(ClockError::InstantOutOfRange { source: None })?;
        if nanos == 0 {
            (seconds, 0)
        } else {
            (
                seconds
                    .checked_sub(1)
                    .ok_or(ClockError::InstantOutOfRange { source: None })?,
                1_000_000_000 - nanos,
            )
        }
    } else {
        (seconds, nanos)
    };
    DateTime::from_timestamp(seconds, nanos).ok_or(ClockError::InstantOutOfRange { source: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_conversion_preserves_epoch_and_fractional_seconds() {
        assert_eq!(
            utc_from_duration(Duration::ZERO, false)
                .unwrap()
                .timestamp(),
            0
        );
        let instant = utc_from_duration(Duration::new(7, 123_456_789), false).unwrap();
        assert_eq!(instant.timestamp(), 7);
        assert_eq!(instant.timestamp_subsec_nanos(), 123_456_789);
    }

    #[test]
    fn system_conversion_preserves_negative_fractional_seconds() {
        let instant = utc_from_duration(Duration::new(7, 123_456_789), true).unwrap();
        assert_eq!(instant.timestamp(), -8);
        assert_eq!(instant.timestamp_subsec_nanos(), 876_543_211);
        assert_eq!(
            utc_from_duration(Duration::from_secs(7), true)
                .unwrap()
                .timestamp(),
            -7
        );
        assert!(matches!(
            unix_seconds(instant),
            Err(ClockError::InstantOutOfRange { .. })
        ));
        let error = unix_seconds(instant).unwrap_err();
        assert!(
            std::error::Error::source(&error)
                .is_some_and(<(dyn std::error::Error + 'static)>::is::<std::num::TryFromIntError>)
        );
    }

    #[test]
    fn system_conversion_refuses_unrepresentable_seconds_without_zero() {
        for before_epoch in [false, true] {
            assert!(matches!(
                utc_from_duration(Duration::MAX, before_epoch),
                Err(ClockError::InstantOutOfRange { .. })
            ));
            let error = utc_from_duration(Duration::MAX, before_epoch).unwrap_err();
            assert!(
                std::error::Error::source(&error).is_some_and(
                    <(dyn std::error::Error + 'static)>::is::<std::num::TryFromIntError>
                )
            );
            assert!(matches!(
                utc_from_duration(Duration::from_secs(i64::MAX.unsigned_abs()), before_epoch),
                Err(ClockError::InstantOutOfRange { .. })
            ));
        }
    }
}
