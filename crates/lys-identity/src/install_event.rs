//! The install event: a change to the install as a whole, recorded in the
//! directory's log beside the identity events.
//!
//! An identity event names one identity and the human the service
//! authenticated. An install event names neither. Its subject is the install
//! itself, and its actor is the directory service at start, named by the build
//! that recorded it: no person asked for it and no login is claimed for it.
//! The one install change is an issuer move. A deployment whose sign-in
//! service changes the issuer name it signs tokens under records that move
//! once, and every login bound under the earlier issuer is bound under the new
//! one from that leaf on. No earlier leaf is rewritten: the history says when
//! the move happened and which build recorded it.
//!
//! The body has the identity event's seven keys, under version 3:
//! 1 version, 2 operation id, 3 actor `{1: method 4, 2: build}`, 4 subject
//! `{1: kind 4}`, 5 recorded-at, 6 change kind 10 and 7 change
//! `{1: from, 2: to}`. docs/design/identity/IDENTITY-EVENTS.md gives the same
//! table. Decoding is as strict as the identity event's: only the canonical
//! bytes of a valid event decode.

use ciborium::Value;
use sha2::{Digest, Sha256};

use crate::binding::LoginBinding;
use crate::encoding::{
    as_bytes, as_text, as_uint, bytes, cbor, fields, malformed, map, text, uint,
};
use crate::error::IdentityError;
use crate::id::ID_LEN;
use crate::operation::OperationId;

/// The body version an install event is written under.
pub const INSTALL_EVENT_VERSION: u64 = 3;

/// The actor's method code: the directory service, at its own start.
const DIRECTORY_SERVICE: u64 = 4;
/// The subject's kind code: the install as a whole.
const INSTALL: u64 = 4;
/// The change kind code of an issuer move.
const ISSUER_MOVED: u64 = 10;

/// A change to the install as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstallChange {
    /// The sign-in service's issuer name moved: every login bound under
    /// `from` is bound under `to` from this event on.
    IssuerMoved {
        /// The issuer the logins were bound under.
        from: String,
        /// The issuer they are bound under from now on.
        to: String,
    },
}

/// A change to the install, recorded by the directory service itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstallEvent {
    operation: OperationId,
    build: String,
    recorded_at: u64,
    change: InstallChange,
}

/// Refuse an issuer that no login binding could carry.
fn check_issuer(issuer: &str) -> Result<(), IdentityError> {
    LoginBinding::new(issuer, "subject").map(|_| ())
}

impl InstallEvent {
    /// The issuer move from `from` to `to`, recorded by the directory service
    /// running `build` at `recorded_at`. Its operation id is the move's own,
    /// so the same move is never recorded twice.
    pub fn issuer_moved(
        from: &str,
        to: &str,
        build: &str,
        recorded_at: u64,
    ) -> Result<Self, IdentityError> {
        let change = InstallChange::IssuerMoved {
            from: from.to_owned(),
            to: to.to_owned(),
        };
        Self::new(Self::operation_of(&change), build, recorded_at, change)
    }

    /// The operation id a change is recorded under: the first 16 bytes of
    /// SHA-256 over its kind and fields, so recording it again finds it.
    pub fn operation_of(change: &InstallChange) -> OperationId {
        let InstallChange::IssuerMoved { from, to } = change;
        let mut hasher = Sha256::new();
        hasher.update(b"lys/identity-directory/issuer-moved\0");
        hasher.update(from.as_bytes());
        hasher.update([0]);
        hasher.update(to.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        let mut id = [0u8; ID_LEN];
        id.copy_from_slice(&digest[..ID_LEN]);
        OperationId::from_bytes(id)
    }

    fn new(
        operation: OperationId,
        build: &str,
        recorded_at: u64,
        change: InstallChange,
    ) -> Result<Self, IdentityError> {
        if build.is_empty() || build.trim() != build {
            return Err(IdentityError::ChangeMismatch {
                reason: "an install event names its build, unpadded and not empty",
            });
        }
        let InstallChange::IssuerMoved { from, to } = &change;
        check_issuer(from)?;
        check_issuer(to)?;
        if from == to {
            return Err(IdentityError::ChangeMismatch {
                reason: "an issuer move names two different issuers",
            });
        }
        if operation != Self::operation_of(&change) {
            return Err(IdentityError::ChangeMismatch {
                reason: "an install event's operation id is its change's own",
            });
        }
        Ok(Self {
            operation,
            build: build.to_owned(),
            recorded_at,
            change,
        })
    }

    /// The operation id the event is recorded under.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The build of the directory service that recorded the event.
    pub fn build(&self) -> &str {
        &self.build
    }

    /// When the service recorded the event, in seconds since the Unix epoch.
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    pub fn change(&self) -> &InstallChange {
        &self.change
    }
}

/// The canonical body bytes of `event`.
pub fn encode(event: &InstallEvent) -> Vec<u8> {
    let InstallChange::IssuerMoved { from, to } = &event.change;
    let mut out = Vec::new();
    map(&mut out, 7);
    uint(&mut out, 1);
    uint(&mut out, INSTALL_EVENT_VERSION);
    uint(&mut out, 2);
    bytes(&mut out, event.operation.as_bytes());
    uint(&mut out, 3);
    map(&mut out, 2);
    uint(&mut out, 1);
    uint(&mut out, DIRECTORY_SERVICE);
    uint(&mut out, 2);
    text(&mut out, &event.build);
    uint(&mut out, 4);
    map(&mut out, 1);
    uint(&mut out, 1);
    uint(&mut out, INSTALL);
    uint(&mut out, 5);
    uint(&mut out, event.recorded_at);
    uint(&mut out, 6);
    uint(&mut out, ISSUER_MOVED);
    uint(&mut out, 7);
    map(&mut out, 2);
    uint(&mut out, 1);
    text(&mut out, from);
    uint(&mut out, 2);
    text(&mut out, to);
    out
}

/// The install event a body's bytes name, refused unless they are its
/// canonical encoding.
pub fn decode(body: &[u8]) -> Result<InstallEvent, IdentityError> {
    const SHAPE: &str = "an install event is a map of keys 1 to 7";
    let [
        version,
        operation,
        actor,
        subject,
        recorded_at,
        kind,
        change,
    ] = fields::<7>(cbor(body, SHAPE)?, SHAPE)?;
    if as_uint(&version, SHAPE)? != INSTALL_EVENT_VERSION {
        return Err(malformed("an install event is version 3"));
    }
    let [method, build] = fields::<2>(actor, "an install actor is a map of keys 1 and 2")?;
    if as_uint(&method, "an install actor's method is a code")? != DIRECTORY_SERVICE {
        return Err(malformed(
            "an install actor is the directory service, code 4",
        ));
    }
    let [subject] = fields::<1>(subject, "an install subject is a map of key 1")?;
    if as_uint(&subject, "an install subject is a code")? != INSTALL {
        return Err(malformed("an install subject is the install, code 4"));
    }
    if as_uint(&kind, "an install change kind is a code")? != ISSUER_MOVED {
        return Err(malformed(
            "the one install change kind is the issuer move, 10",
        ));
    }
    let [from, to] = fields::<2>(change, "an issuer move is a map of keys 1 and 2")?;
    let operation = <[u8; ID_LEN]>::try_from(as_bytes(operation, "an operation id is 16 bytes")?)
        .ok()
        .ok_or_else(|| malformed("an operation id is 16 bytes"))?;
    let event = InstallEvent::new(
        OperationId::from_bytes(operation),
        &as_text(build, "a build is text")?,
        as_uint(&recorded_at, "a recorded time is seconds")?,
        InstallChange::IssuerMoved {
            from: as_text(from, "an issuer is text")?,
            to: as_text(to, "an issuer is text")?,
        },
    )?;
    if encode(&event) != body {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(event)
}

/// The body version a body names in its first entry, before its shape is read.
pub(crate) fn body_version(body: &[u8]) -> Result<Option<u64>, IdentityError> {
    const SHAPE: &str = "the body is a map of keys 1 to 7";
    if let Value::Map(pairs) = cbor(body, SHAPE)?
        && let Some((_, version)) = pairs.first()
    {
        return Ok(Some(as_uint(version, SHAPE)?));
    }
    Ok(None)
}

#[cfg(test)]
#[path = "install_event_tests.rs"]
mod tests;
