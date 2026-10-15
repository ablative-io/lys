//! A fresh system authentication decision is required for recovery issuance.

use super::error::{ErrorKind, IdentityError, IdentityResult};

/// The person must answer a system-owned authentication prompt.
pub trait Presence {
    /// Authenticate once, without reusing an earlier authentication decision.
    ///
    /// # Errors
    /// Returns cancellation, failed authentication or unavailable presence by name.
    fn authenticate(&self) -> IdentityResult<()>;
}

/// LocalAuthentication on macOS; other platforms refuse recovery issuance.
pub struct SystemPresence;

pub(crate) fn refused(kind: ErrorKind, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "authenticate", "administrator recovery", detail)
}

#[cfg(target_os = "macos")]
#[path = "presence_macos.rs"]
mod macos;

impl Presence for SystemPresence {
    fn authenticate(&self) -> IdentityResult<()> {
        #[cfg(target_os = "macos")]
        {
            macos::authenticate()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(refused(
                ErrorKind::PresenceUnavailable,
                "administrator recovery requires macOS device-owner authentication",
            ))
        }
    }
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn answer(success: bool, output: &[u8]) -> IdentityResult<()> {
    match (success, output) {
        (true, b"authenticated\n") => Ok(()),
        (false, b"cancelled\n") => Err(refused(
            ErrorKind::PresenceCancelled,
            "the system authentication prompt was cancelled",
        )),
        (false, b"refused\n") => Err(refused(
            ErrorKind::PresenceRefused,
            "the system did not authenticate the device owner",
        )),
        _ => Err(refused(
            ErrorKind::PresenceUnavailable,
            "the system authentication helper could not prove device-owner authentication",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorKind, answer};

    #[test]
    fn only_an_authenticated_helper_answer_authorizes_recovery() {
        assert!(answer(true, b"authenticated\n").is_ok());
        for (success, output, kind) in [
            (
                false,
                b"cancelled\n".as_slice(),
                ErrorKind::PresenceCancelled,
            ),
            (false, b"refused\n".as_slice(), ErrorKind::PresenceRefused),
            (
                true,
                b"cancelled\n".as_slice(),
                ErrorKind::PresenceUnavailable,
            ),
            (
                false,
                b"authenticated\n".as_slice(),
                ErrorKind::PresenceUnavailable,
            ),
            (true, b"".as_slice(), ErrorKind::PresenceUnavailable),
        ] {
            let result = answer(success, output);
            assert_eq!(result.err().map(|error| error.kind()), Some(kind));
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn platforms_without_local_authentication_refuse_instead_of_skipping_presence() {
        use super::{Presence, SystemPresence};
        assert_eq!(
            SystemPresence
                .authenticate()
                .err()
                .map(|error| error.kind()),
            Some(ErrorKind::PresenceUnavailable)
        );
    }
}
