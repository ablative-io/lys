//! The seat file: one JSON file, mode 0600, that holds an agent's private key
//! where the agent runs, with what certifies it (AGENTS-017 R5).
//!
//! One shape is read by every reader: Lys's `lys mcp` bridge, the
//! `lys agent enrol` command that writes it, and an app's door, whose
//! `SeatFile` names the same members (`version`, `doorUrl`, `handle`, `device`,
//! `privateKeyPem`, `lysUrl`, `lysAgentId`). The member spellings are that
//! reader's, so a rename here is a seat that will not open. Lys adds
//! `certificatePem` and `serial`; a reader that does not know them ignores them.
//!
//! Invariants:
//! - `version` is [`SEAT_FILE_VERSION`]; a file of any other version, or of
//!   none, is refused naming what it carries and the version wanted, never read
//!   as if it were this one.
//! - `privateKeyPem` is a PKCS#8 PEM of one Ed25519 seed and nothing else; the
//!   key never appears in `Debug` output or in an error, and the text is wiped
//!   from memory when the value is dropped.
//! - The door's members (`doorUrl`, `handle`, `device`) are written only when
//!   all three are known, and are read as absent when a Lys-only seat omits them.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::keys::Ed25519Identity;

/// The seat file shape this reader and writer speak, the door's version 1.
pub const SEAT_FILE_VERSION: u32 = 1;

/// PKCS#8 v1 DER for an Ed25519 private key, before its 32-byte seed
/// (RFC 8410 section 7).
const PKCS8_ED25519_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];
/// The PEM label of a PKCS#8 private key (RFC 7468 section 10).
const PRIVATE_KEY_LABEL: &str = "PRIVATE KEY";
/// RFC 7468 recommends wrapping base64 at 64 characters per line.
const LINE_WIDTH: usize = 64;

/// Why a seat file or its key could not be read or written. No variant carries
/// key material.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SeatFileError {
    /// The file is not the JSON of a seat file.
    #[error("seat_file_unreadable: the seat file is not a seat file's JSON: {reason}")]
    Unreadable {
        /// What the JSON reader said, without the file's text.
        reason: String,
    },
    /// The file carries a version this reader does not speak, or none.
    #[error(
        "seat_file_version_unknown: the seat file {found}; this lys reads version {}: add \"version\": {} if it is a seat file of that shape",
        SEAT_FILE_VERSION,
        SEAT_FILE_VERSION
    )]
    VersionUnknown {
        /// The version the file carries, or that it carries none.
        found: VersionFound,
    },
    /// The private key is not a PKCS#8 PEM of an Ed25519 seed.
    #[error("seat_file_key_unreadable: {reason}")]
    KeyUnreadable {
        /// Which part of the key could not be read.
        reason: &'static str,
    },
}

/// The version a seat file carries, as a refusal says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionFound(pub Option<u64>);

impl fmt::Display for VersionFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(version) => write!(f, "is version {version}"),
            None => f.write_str("carries no version"),
        }
    }
}

/// The members of a seat file a door names: all three, or none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorSeat {
    /// The base URL of the app's door the seat enrolled at.
    pub door_url: String,
    /// The registry handle the seat answers as.
    pub handle: String,
    /// The device label carried on the seat's session.
    pub device: String,
}

/// A seat file, as written and as read.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatFile {
    /// Always [`SEAT_FILE_VERSION`] once read.
    pub version: u32,
    /// The door's base URL, absent for a seat no door founded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door_url: Option<String>,
    /// The registry handle, absent for a seat no door founded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    /// The device label, absent for a seat no door founded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// The agent's Ed25519 private key, PKCS#8 PEM. Never printed.
    pub private_key_pem: String,
    /// The Lys API that certified the key, absent in a file written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lys_url: Option<String>,
    /// The Lys agent the key's certificate names, absent in a file written
    /// before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lys_agent_id: Option<String>,
    /// The certificate Lys issued over the key, PEM, when Lys issued one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_pem: Option<String>,
    /// The certificate's serial in Lys's certificate log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
}

impl fmt::Debug for SeatFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeatFile")
            .field("version", &self.version)
            .field("door_url", &self.door_url)
            .field("handle", &self.handle)
            .field("device", &self.device)
            .field("private_key_pem", &"[REDACTED]")
            .field("lys_url", &self.lys_url)
            .field("lys_agent_id", &self.lys_agent_id)
            .field("certificate_pem", &self.certificate_pem)
            .field("serial", &self.serial)
            .finish()
    }
}

impl Drop for SeatFile {
    fn drop(&mut self) {
        self.private_key_pem.zeroize();
    }
}

impl SeatFile {
    /// A seat file holding `seed` as its key, for the agent `lys_agent_id`
    /// certified by the Lys at `lys_url`, with the door's members when a door
    /// is named.
    #[must_use]
    pub fn new(
        seed: &Zeroizing<[u8; 32]>,
        (lys_url, lys_agent_id): (&str, &str),
        door: Option<DoorSeat>,
    ) -> Self {
        let (door_url, handle, device) = door.map_or((None, None, None), |door| {
            (Some(door.door_url), Some(door.handle), Some(door.device))
        });
        Self {
            version: SEAT_FILE_VERSION,
            door_url,
            handle,
            device,
            private_key_pem: private_key_pem(seed).to_string(),
            lys_url: Some(lys_url.to_owned()),
            lys_agent_id: Some(lys_agent_id.to_owned()),
            certificate_pem: None,
            serial: None,
        }
    }

    /// Read a seat file from its JSON text.
    ///
    /// # Errors
    /// [`SeatFileError::Unreadable`] when the text is not a seat file's JSON,
    /// [`SeatFileError::VersionUnknown`] naming the version found (or none)
    /// when it is not [`SEAT_FILE_VERSION`].
    pub fn from_json(text: &str) -> Result<Self, SeatFileError> {
        // The reader's position and kind, never the text it read: the text holds the key.
        let unreadable = |error: &serde_json::Error| SeatFileError::Unreadable {
            reason: format!(
                "line {} column {}: {:?}",
                error.line(),
                error.column(),
                error.classify()
            ),
        };
        let members: serde_json::Value =
            serde_json::from_str(text).map_err(|error| unreadable(&error))?;
        let found = members.get("version").map(serde_json::Value::as_u64);
        match found {
            Some(Some(version)) if version == u64::from(SEAT_FILE_VERSION) => {}
            Some(None) => {
                return Err(SeatFileError::Unreadable {
                    reason: "version is not a whole number".to_owned(),
                });
            }
            other => {
                return Err(SeatFileError::VersionUnknown {
                    found: VersionFound(other.flatten()),
                });
            }
        }
        serde_json::from_value(members).map_err(|error| unreadable(&error))
    }

    /// The file's JSON text, the key inside it, wiped when dropped.
    ///
    /// # Errors
    /// [`SeatFileError::Unreadable`] when the value cannot be written as JSON,
    /// which a seat file's plain members never cause.
    pub fn to_json(&self) -> Result<Zeroizing<String>, SeatFileError> {
        serde_json::to_string_pretty(self)
            .map(Zeroizing::new)
            .map_err(|error| SeatFileError::Unreadable {
                reason: error.to_string(),
            })
    }

    /// The door's members, when the file names all three.
    #[must_use]
    pub fn door(&self) -> Option<DoorSeat> {
        Some(DoorSeat {
            door_url: self.door_url.clone()?,
            handle: self.handle.clone()?,
            device: self.device.clone()?,
        })
    }

    /// The identity the file's key is.
    ///
    /// # Errors
    /// [`SeatFileError::KeyUnreadable`] when `privateKeyPem` is not a PKCS#8
    /// PEM of an Ed25519 seed.
    pub fn identity(&self) -> Result<Ed25519Identity, SeatFileError> {
        identity_from_pem(&self.private_key_pem)
    }
}

/// `seed` as a PKCS#8 PEM `PRIVATE KEY`, wiped when dropped.
#[must_use]
pub fn private_key_pem(seed: &Zeroizing<[u8; 32]>) -> Zeroizing<String> {
    let mut der = Zeroizing::new(Vec::with_capacity(PKCS8_ED25519_PREFIX.len() + seed.len()));
    der.extend_from_slice(&PKCS8_ED25519_PREFIX);
    der.extend_from_slice(seed.as_slice());
    let body = Zeroizing::new(STANDARD.encode(der.as_slice()));
    let mut pem = Zeroizing::new(String::with_capacity(body.len() + 64));
    pem.push_str("-----BEGIN ");
    pem.push_str(PRIVATE_KEY_LABEL);
    pem.push_str("-----\n");
    for (index, character) in body.chars().enumerate() {
        if index > 0 && index % LINE_WIDTH == 0 {
            pem.push('\n');
        }
        pem.push(character);
    }
    pem.push_str("\n-----END ");
    pem.push_str(PRIVATE_KEY_LABEL);
    pem.push_str("-----\n");
    pem
}

/// The identity a PKCS#8 PEM of an Ed25519 seed holds.
///
/// # Errors
/// [`SeatFileError::KeyUnreadable`] when the text is not PEM, or the PEM is
/// not a PKCS#8 Ed25519 private key.
pub fn identity_from_pem(pem: &str) -> Result<Ed25519Identity, SeatFileError> {
    let body: Zeroizing<String> = Zeroizing::new(
        pem.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("-----"))
            .collect(),
    );
    let der = Zeroizing::new(
        STANDARD
            .decode(body.as_bytes())
            .map_err(|_not_pem| SeatFileError::KeyUnreadable {
                reason: "the key is not PEM",
            })?,
    );
    let seed = der
        .strip_prefix(&PKCS8_ED25519_PREFIX)
        .and_then(|seed| <[u8; 32]>::try_from(seed).ok())
        .ok_or(SeatFileError::KeyUnreadable {
            reason: "the key is not a PKCS#8 ed25519 private key",
        })?;
    Ok(Ed25519Identity::from_seed(&Zeroizing::new(seed)))
}

#[cfg(test)]
#[path = "seat_file_tests.rs"]
mod tests;
