//! The launch record: one kind of signed directory event, kept on the
//! directory's launch-record log and read by its id.
//!
//! A launch record names its own id, the enduring agent id, the machine, the
//! executable, its recorded arguments, the working directory, the profile
//! version, the credential ids, the person who gave it, when, and, for a
//! start given again from a kept record, the record it was copied from. It
//! carries no credential value. Its encoding, and the withdrawal's beside
//! it, are written in docs/design/identity/IDENTITY-EVENTS.md.
//!
//! Each event is a `COSE_Sign1` message in the shape of `lys/identity-event/v1`,
//! with its own content type in the protected header, so neither kind is
//! ever read as the other or as an identity event. Reading is strict: a
//! message is refused unless re-encoding what it names gives back its very
//! bytes, and every failure of a message is the one refusal, whatever it was.

use ciborium::Value;
use lys_core::Ed25519Identity;

use crate::encoding::{MAJOR_ARRAY, MAJOR_NEGATIVE, MAJOR_TAG, bytes, head, map, text, uint};
use crate::id::{from_hex, random_bytes, to_hex};
use crate::start::error::StartError;
use crate::start::withdrawal::Withdrawal;

/// The content type of a launch record's protected header.
pub const LAUNCH_RECORD_CONTENT_TYPE: &str = "application/vnd.lys.launch-record.v1+cbor";

/// The content type of a withdrawal's protected header.
pub const WITHDRAWAL_CONTENT_TYPE: &str = "application/vnd.lys.launch-withdrawal.v1+cbor";

/// The largest event the launch-record log writes or reads, in bytes.
pub const MAX_EVENT_BYTES: usize = 64 * 1024;

/// The version both kinds' bodies carry at key 1.
pub(crate) const VERSION: u64 = 1;

const ID_PREFIX: &str = "launch-";
const COSE_SIGN1_TAG: u64 = 18;
const CBOR_NULL: u8 = 0xf6;
const REFUSED: &str = "the message is not a launch event this service signed";

/// A kept launch record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRecord {
    /// The record's own id.
    pub id: String,
    /// The enduring agent id.
    pub agent: String,
    /// The machine the command is for.
    pub machine: String,
    /// The executable the profile version records.
    pub executable: String,
    /// The arguments the profile version records, in order.
    pub arguments: Vec<String>,
    /// The working directory the profile version records.
    pub working_directory: String,
    /// The profile version.
    pub profile_version: String,
    /// The ids of the virtual credentials the credentials check handed on.
    pub credential_ids: Vec<String>,
    /// The person who gave it.
    pub given_by: String,
    /// When, in seconds since the Unix epoch.
    pub given_at: u64,
    /// The record it was copied from, when it was given again from a kept one.
    pub copied_from: Option<String>,
}

/// A new launch record id: `launch-` and 32 lowercase hex digits from the
/// secure random source.
pub fn new_launch_record_id() -> Result<String, StartError> {
    let bytes = random_bytes().map_err(|error| StartError::Unavailable {
        reason: error.to_string(),
    })?;
    Ok(format!("{ID_PREFIX}{}", to_hex(&bytes)))
}

/// Whether `text` is in the launch record id's grammar: `launch-` and 32
/// lowercase hex digits.
pub fn is_launch_record_id(text: &str) -> bool {
    text.strip_prefix(ID_PREFIX).and_then(from_hex).is_some()
}

fn texts(out: &mut Vec<u8>, items: &[String]) {
    head(out, MAJOR_ARRAY, items.len() as u64);
    for item in items {
        text(out, item);
    }
}

impl LaunchRecord {
    /// The canonical body: a map of keys 1 to 12, as IDENTITY-EVENTS.md gives them.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        map(&mut out, 12);
        uint(&mut out, 1);
        uint(&mut out, VERSION);
        for (key, value) in [
            (2, &self.id),
            (3, &self.agent),
            (4, &self.machine),
            (5, &self.executable),
        ] {
            uint(&mut out, key);
            text(&mut out, value);
        }
        uint(&mut out, 6);
        texts(&mut out, &self.arguments);
        uint(&mut out, 7);
        text(&mut out, &self.working_directory);
        uint(&mut out, 8);
        text(&mut out, &self.profile_version);
        uint(&mut out, 9);
        texts(&mut out, &self.credential_ids);
        uint(&mut out, 10);
        text(&mut out, &self.given_by);
        uint(&mut out, 11);
        uint(&mut out, self.given_at);
        uint(&mut out, 12);
        match &self.copied_from {
            Some(source) => text(&mut out, source),
            None => out.push(CBOR_NULL),
        }
        out
    }

    fn decode(body: &[u8]) -> Result<Self, String> {
        let mut values = fields(cbor(body)?, 12)?.into_iter();
        let mut next = || values.next().ok_or_else(|| REFUSED.to_owned());
        if as_uint(&next()?)? != VERSION {
            return Err(REFUSED.to_owned());
        }
        let record = Self {
            id: as_text(next()?)?,
            agent: as_text(next()?)?,
            machine: as_text(next()?)?,
            executable: as_text(next()?)?,
            arguments: as_texts(next()?)?,
            working_directory: as_text(next()?)?,
            profile_version: as_text(next()?)?,
            credential_ids: as_texts(next()?)?,
            given_by: as_text(next()?)?,
            given_at: as_uint(&next()?)?,
            copied_from: match next()? {
                Value::Null => None,
                other => Some(as_text(other)?),
            },
        };
        if !is_launch_record_id(&record.id) || record.encode() != body {
            return Err(REFUSED.to_owned());
        }
        Ok(record)
    }
}

pub(crate) fn cbor(bytes: &[u8]) -> Result<Value, String> {
    ciborium::from_reader(bytes).map_err(|_unread| REFUSED.to_owned())
}

/// The values of a map whose keys are exactly 1 to `count`, in order.
pub(crate) fn fields(value: Value, count: u64) -> Result<Vec<Value>, String> {
    let Value::Map(pairs) = value else {
        return Err(REFUSED.to_owned());
    };
    if pairs.len() as u64 != count {
        return Err(REFUSED.to_owned());
    }
    let mut values = Vec::with_capacity(pairs.len());
    for (expected, (key, value)) in (1..).zip(pairs) {
        if as_uint(&key)? != expected {
            return Err(REFUSED.to_owned());
        }
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn as_uint(value: &Value) -> Result<u64, String> {
    match value {
        Value::Integer(integer) => u64::try_from(*integer).map_err(|_negative| REFUSED.to_owned()),
        _ => Err(REFUSED.to_owned()),
    }
}

pub(crate) fn as_text(value: Value) -> Result<String, String> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(REFUSED.to_owned()),
    }
}

fn as_texts(value: Value) -> Result<Vec<String>, String> {
    match value {
        Value::Array(items) => items.into_iter().map(as_text).collect(),
        _ => Err(REFUSED.to_owned()),
    }
}

pub(crate) fn as_byte_string(value: Value) -> Result<Vec<u8>, String> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        _ => Err(REFUSED.to_owned()),
    }
}

/// The protected header naming `EdDSA`, `content_type` and the key id.
fn protected_header(content_type: &str, kid: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 3);
    uint(&mut out, 1);
    head(&mut out, MAJOR_NEGATIVE, 7);
    uint(&mut out, 3);
    text(&mut out, content_type);
    uint(&mut out, 4);
    bytes(&mut out, kid);
    out
}

fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_ARRAY, 4);
    text(&mut out, "Signature1");
    bytes(&mut out, protected);
    bytes(&mut out, &[]);
    bytes(&mut out, payload);
    out
}

fn cose_sign1(protected: &[u8], payload: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    head(&mut out, MAJOR_ARRAY, 4);
    bytes(&mut out, protected);
    map(&mut out, 0);
    bytes(&mut out, payload);
    bytes(&mut out, signature);
    out
}

/// One verified event of the launch-record log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchEvent {
    /// A launch record.
    Launch(LaunchRecord),
    /// A withdrawal.
    Withdrawal(Withdrawal),
}

impl LaunchEvent {
    /// Sign the event with the directory service's key: the whole message is
    /// the log leaf.
    pub(crate) fn seal(&self, key: &Ed25519Identity) -> Result<Vec<u8>, StartError> {
        let (content_type, body) = match self {
            Self::Launch(record) => (LAUNCH_RECORD_CONTENT_TYPE, record.encode()),
            Self::Withdrawal(withdrawal) => (WITHDRAWAL_CONTENT_TYPE, withdrawal.encode()),
        };
        let protected = protected_header(content_type, &key.public_key_bytes());
        let signature = key.sign(&sig_structure(&protected, &body));
        let message = cose_sign1(&protected, &body, &signature);
        if message.len() > MAX_EVENT_BYTES {
            return Err(StartError::Unavailable {
                reason: format!(
                    "the event is {} bytes, over the {MAX_EVENT_BYTES} the launch-record log reads",
                    message.len()
                ),
            });
        }
        Ok(message)
    }
}

/// Verify `message` against the directory service's public key and read the
/// event it carries. Every failure is the one refusal, whatever it was.
pub fn verify_launch_event(
    message: &[u8],
    service_key: &[u8; 32],
) -> Result<LaunchEvent, String> {
    let refused = || REFUSED.to_owned();
    if message.len() > MAX_EVENT_BYTES {
        return Err(refused());
    }
    let Value::Tag(COSE_SIGN1_TAG, inner) = cbor(message)? else {
        return Err(refused());
    };
    let Value::Array(parts) = *inner else {
        return Err(refused());
    };
    let [protected, unprotected, payload, signature] =
        <[Value; 4]>::try_from(parts).map_err(|_parts| refused())?;
    if unprotected != Value::Map(Vec::new()) {
        return Err(refused());
    }
    let protected = as_byte_string(protected)?;
    let payload = as_byte_string(payload)?;
    let signature = as_byte_string(signature)?;
    let content_type = [LAUNCH_RECORD_CONTENT_TYPE, WITHDRAWAL_CONTENT_TYPE]
        .into_iter()
        .find(|kind| protected == protected_header(kind, service_key))
        .ok_or_else(refused)?;
    Ed25519Identity::verify(
        service_key,
        &sig_structure(&protected, &payload),
        &signature,
    )
    .map_err(|_forged| refused())?;
    if cose_sign1(&protected, &payload, &signature) != message {
        return Err(refused());
    }
    if content_type == LAUNCH_RECORD_CONTENT_TYPE {
        LaunchRecord::decode(&payload).map(LaunchEvent::Launch)
    } else {
        Withdrawal::decode(&payload).map(LaunchEvent::Withdrawal)
    }
}
