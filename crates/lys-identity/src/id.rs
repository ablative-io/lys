//! Enduring identifiers for people, agents, service accounts and connectors.
//!
//! An identifier is sixteen bytes drawn from the operating system's secure
//! random source when the identity is registered, and it never changes. It is
//! never derived from an email, a display name or a login: those can change or
//! collide, and an identity must survive both (P1).

use std::fmt;
use std::str::FromStr;

use rand::TryRngCore;
use rand::rngs::OsRng;

use crate::error::IdentityError;

/// The length in bytes of every identifier this directory writes.
pub const ID_LEN: usize = 16;

/// Sixteen bytes from the operating system's secure random source.
pub(crate) fn random_bytes() -> Result<[u8; ID_LEN], IdentityError> {
    let mut bytes = [0u8; ID_LEN];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| IdentityError::RandomSourceUnavailable {
            reason: error.to_string(),
        })?;
    Ok(bytes)
}

/// Lowercase hex of `bytes`.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// The sixteen bytes a lowercase hex string of exactly 32 digits spells.
pub(crate) fn from_hex(text: &str) -> Option<[u8; ID_LEN]> {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        }
    }
    let raw = text.as_bytes();
    if raw.len() != ID_LEN * 2 {
        return None;
    }
    let mut bytes = [0u8; ID_LEN];
    for (slot, pair) in bytes.iter_mut().zip(raw.chunks_exact(2)) {
        *slot = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Some(bytes)
}

/// Parse `text` as `prefix` followed by 32 lowercase hex digits.
fn parse_prefixed(
    text: &str,
    prefix: &str,
    kind: &'static str,
) -> Result<[u8; ID_LEN], IdentityError> {
    text.strip_prefix(prefix)
        .and_then(from_hex)
        .ok_or_else(|| IdentityError::IdentifierMalformed {
            kind,
            text: text.to_owned(),
        })
}

/// A person's enduring identifier. Its text form is `person-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PersonId([u8; ID_LEN]);

/// An agent's enduring identifier. Its text form is `agent-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId([u8; ID_LEN]);

/// A service account's enduring id, retaining its creation operation's
/// existing `op-` text form. This is a distinct identity kind, never a person
/// or an agent even when another identifier has the same bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServiceAccountId([u8; ID_LEN]);

impl ServiceAccountId {
    /// The id assigned when the account was created.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }
    /// The bytes recorded by the service-account creation operation.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for ServiceAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op-{}", to_hex(&self.0))
    }
}

impl FromStr for ServiceAccountId {
    type Err = IdentityError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_prefixed(text, "op-", "service account").map(Self)
    }
}

/// An approved app's connector identity (ADR-126, DIRECTORY-048 R1). Its id is
/// made once from the secure random source and stored with the app's
/// approval; its text form is `connector-` and 32 hex digits. It is never a
/// person, an agent or a service account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectorId([u8; ID_LEN]);

impl ConnectorId {
    const PREFIX: &'static str = "connector-";

    /// A new identifier from the secure random source.
    pub fn generate() -> Result<Self, IdentityError> {
        random_bytes().map(Self)
    }

    /// The identifier these bytes are, as read back from a signed event.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes, as a signed event carries them.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for ConnectorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for ConnectorId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_prefixed(text, Self::PREFIX, "connector").map(Self)
    }
}

impl PersonId {
    const PREFIX: &'static str = "person-";

    /// A new identifier from the secure random source.
    pub fn generate() -> Result<Self, IdentityError> {
        random_bytes().map(Self)
    }

    /// The identifier these bytes are, as read back from a signed event.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes, as a signed event carries them.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl AgentId {
    const PREFIX: &'static str = "agent-";

    /// A new identifier from the secure random source.
    pub fn generate() -> Result<Self, IdentityError> {
        random_bytes().map(Self)
    }

    /// The identifier these bytes are, as read back from a signed event.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes, as a signed event carries them.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for PersonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for PersonId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_prefixed(text, Self::PREFIX, "person").map(Self)
    }
}

impl FromStr for AgentId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_prefixed(text, Self::PREFIX, "agent").map(Self)
    }
}

/// An identity Lys knows: a person, an agent, a service account or an
/// approved app's connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdentityId {
    /// A person.
    Person(PersonId),
    /// An agent.
    Agent(AgentId),
    /// A person-owned service account, authenticated independently.
    ServiceAccount(ServiceAccountId),
    /// An approved app's connector, answering to the administrator who
    /// approved the app.
    Connector(ConnectorId),
}

impl fmt::Display for IdentityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Person(id) => id.fmt(f),
            Self::Agent(id) => id.fmt(f),
            Self::ServiceAccount(id) => id.fmt(f),
            Self::Connector(id) => id.fmt(f),
        }
    }
}
