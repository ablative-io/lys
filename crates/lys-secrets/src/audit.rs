//! The audit log. Every issue, use, refusal, drop and rotation is one line,
//! signed by the broker's audit key and appended through lys-log-store. The
//! log's head is anchored outside the store and log directories, so a log
//! restored from an older copy is refused at start.

use std::fs;
use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_log_store::{FileLeafStore, Log};
use serde::{Deserialize, Serialize};

use crate::encoding::{Canonical, Reader, hex, sha256};
use crate::error::SecretsError;
use crate::fsutil::{ensure_outside, io, write_atomic};
use crate::keys::StoreKey;

const LINE_DOMAIN: &str = "lys-secrets/audit-line/v2";
const SIGNED_DOMAIN: &str = "lys-secrets/audit-signed/v1";
const ANCHOR_DOMAIN: &str = "lys-secrets/audit-anchor/v1";

/// What an audit line records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    /// A handle was issued.
    Issue,
    /// A handle was used, or a use was refused.
    Use,
    /// A handle was dropped.
    Drop,
    /// A secret was sealed into the store.
    Seal,
    /// The store key was rotated.
    Rotation,
    /// A secret's use moved to its next account.
    NextAccount,
    /// An admitted call's outcome, with what it spent.
    Settlement,
    /// A sealed record was read, or a read refused.
    SealedRead,
}

impl AuditKind {
    /// The kind as the line writes it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::Use => "use",
            Self::Drop => "drop",
            Self::Seal => "seal",
            Self::Rotation => "rotation",
            Self::NextAccount => "next_account",
            Self::Settlement => "settlement",
            Self::SealedRead => "sealed_read",
        }
    }

    fn parse(text: &[u8]) -> Option<Self> {
        match text {
            b"issue" => Some(Self::Issue),
            b"use" => Some(Self::Use),
            b"drop" => Some(Self::Drop),
            b"seal" => Some(Self::Seal),
            b"rotation" => Some(Self::Rotation),
            b"next_account" => Some(Self::NextAccount),
            b"settlement" => Some(Self::Settlement),
            b"sealed_read" => Some(Self::SealedRead),
            _ => None,
        }
    }
}

/// One audit line. No field is a raw handle, a digest, a credential or a key
/// byte: the log can be read and replayed by anyone who holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLine {
    /// What happened.
    pub kind: AuditKind,
    /// The broker's clock, in milliseconds since the epoch.
    pub at_ms: i64,
    /// The handle id, when a handle is involved.
    pub handle: Option<String>,
    /// The identity acted for.
    pub identity: Option<String>,
    /// The secret's name, never its value.
    pub secret: Option<String>,
    /// The call's operation id, in hex.
    pub operation: Option<String>,
    /// A keyed mark of the request a use was for, in hex. Only the broker
    /// can compute it, so it tells a reused operation id without saying
    /// what the request was.
    pub request: Option<String>,
    /// The lease's use count after this line.
    pub uses: Option<u64>,
    /// The spend reserved (on a use) or settled (on a settlement), for a
    /// lease with a spend cap.
    pub spend: Option<u64>,
    /// The outcome: `issued`, `admitted`, a refusal's name, `dropped`.
    pub outcome: String,
}

/// A line as the log holds it, with its index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedLine {
    /// Its leaf index.
    pub index: u64,
    /// The line.
    pub line: AuditLine,
}

#[derive(Debug, Serialize, Deserialize)]
struct Anchor {
    len: u64,
    last_digest: String,
    signature: String,
}

/// The append-only, signed audit log.
pub struct AuditLog {
    log: Log<FileLeafStore>,
    anchor: PathBuf,
    verifying_key: [u8; 32],
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("origin", &self.log.origin())
            .field("len", &self.log.tree().len())
            .finish_non_exhaustive()
    }
}

impl AuditLog {
    /// Makes a new log in `dir`, anchored at `anchor`.
    ///
    /// # Errors
    ///
    /// `KeyFileMisplaced` when the anchor lies in a guarded directory, and
    /// `Log` when the log cannot be made.
    pub fn create(
        dir: &Path,
        origin: &str,
        anchor: &Path,
        guarded: &[&Path],
        key: &StoreKey,
    ) -> Result<Self, SecretsError> {
        fs::create_dir_all(dir).map_err(io(format!("creating {}", dir.display())))?;
        ensure_outside(anchor, guarded)?;
        let log = Log::open(FileLeafStore::create(dir, origin)?)?;
        let audit = Self {
            log,
            anchor: anchor.to_path_buf(),
            verifying_key: key.verifying_key(),
        };
        audit.write_anchor(key.identity())?;
        Ok(audit)
    }

    /// Opens the log in `dir` and checks it against its anchor and every
    /// line's signature.
    ///
    /// # Errors
    ///
    /// `LogBehindAnchor`-shaped `AuditLineMissing` when the log is shorter
    /// than its anchor, `AuditSignatureInvalid` when the anchor or a line
    /// does not verify, and `Log`.
    pub fn open(
        dir: &Path,
        anchor: &Path,
        guarded: &[&Path],
        key: &StoreKey,
    ) -> Result<Self, SecretsError> {
        ensure_outside(anchor, guarded)?;
        let log = Log::open(FileLeafStore::open(dir)?)?;
        let audit = Self {
            log,
            anchor: anchor.to_path_buf(),
            verifying_key: key.verifying_key(),
        };
        audit.check_anchor()?;
        audit.replay()?;
        Ok(audit)
    }

    /// The number of lines.
    pub fn len(&self) -> u64 {
        self.log.tree().len()
    }

    /// Whether the log holds no line.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The public key every line is verified against.
    pub fn verifying_key(&self) -> [u8; 32] {
        self.verifying_key
    }

    /// Signs and appends `line`, then moves the anchor to the new head.
    ///
    /// # Errors
    ///
    /// `Encoding`, `Log` and `Io`.
    pub fn append(&mut self, line: &AuditLine, key: &Ed25519Identity) -> Result<u64, SecretsError> {
        let body = encode_line(line)?;
        let signature = key.sign(&body);
        let mut signed = Canonical::new(SIGNED_DOMAIN)?;
        signed.field(&body)?.field(&signature)?;
        let (index, _root) = self.log.append(&signed.into_bytes())?;
        self.write_anchor(key)?;
        Ok(index)
    }

    /// Every line, each signature verified against the audit key.
    ///
    /// # Errors
    ///
    /// `AuditLineMissing`, `AuditLineUnreadable` and `AuditSignatureInvalid`.
    pub fn replay(&self) -> Result<Vec<RecordedLine>, SecretsError> {
        (0..self.len())
            .map(|index| {
                let bytes = self
                    .log
                    .leaf_bytes(index)
                    .ok_or(SecretsError::AuditLineMissing {
                        index,
                        len: self.len(),
                    })?;
                let line = decode_signed(index, bytes, &self.verifying_key)?;
                Ok(RecordedLine { index, line })
            })
            .collect()
    }

    fn head_digest(&self) -> Result<[u8; 32], SecretsError> {
        match self.len().checked_sub(1) {
            None => Ok([0u8; 32]),
            Some(last) => {
                self.log
                    .leaf_bytes(last)
                    .map(sha256)
                    .ok_or(SecretsError::AuditLineMissing {
                        index: last,
                        len: self.len(),
                    })
            }
        }
    }

    fn anchor_body(len: u64, digest: &[u8; 32]) -> Result<Vec<u8>, SecretsError> {
        let mut body = Canonical::new(ANCHOR_DOMAIN)?;
        body.number(len)?.field(digest)?;
        Ok(body.into_bytes())
    }

    fn write_anchor(&self, key: &Ed25519Identity) -> Result<(), SecretsError> {
        let digest = self.head_digest()?;
        let body = Self::anchor_body(self.len(), &digest)?;
        let anchor = Anchor {
            len: self.len(),
            last_digest: hex(&digest),
            signature: hex(&key.sign(&body)),
        };
        let bytes = serde_json::to_vec_pretty(&anchor).map_err(|error| SecretsError::Encoding {
            context: "audit anchor",
            reason: error.to_string(),
        })?;
        write_atomic(&self.anchor, &bytes)
    }

    fn check_anchor(&self) -> Result<(), SecretsError> {
        let bytes = fs::read(&self.anchor)
            .map_err(io(format!("reading anchor {}", self.anchor.display())))?;
        let anchor: Anchor =
            serde_json::from_slice(&bytes).map_err(|error| SecretsError::AuditLineUnreadable {
                index: 0,
                reason: format!("the anchor does not read: {error}"),
            })?;
        let digest: [u8; 32] = crate::encoding::unhex(&anchor.last_digest)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(SecretsError::AuditSignatureInvalid { index: anchor.len })?;
        let signature = crate::encoding::unhex(&anchor.signature).unwrap_or_default();
        let body = Self::anchor_body(anchor.len, &digest)?;
        Ed25519Identity::verify(&self.verifying_key, &body, &signature)
            .map_err(|_invalid| SecretsError::AuditSignatureInvalid { index: anchor.len })?;
        if self.len() < anchor.len {
            return Err(SecretsError::AuditLineMissing {
                index: anchor.len,
                len: self.len(),
            });
        }
        let at_anchor = match anchor.len.checked_sub(1) {
            None => [0u8; 32],
            Some(last) => {
                self.log
                    .leaf_bytes(last)
                    .map(sha256)
                    .ok_or(SecretsError::AuditLineMissing {
                        index: last,
                        len: self.len(),
                    })?
            }
        };
        if at_anchor != digest {
            return Err(SecretsError::AuditSignatureInvalid { index: anchor.len });
        }
        Ok(())
    }
}

fn optional(encoding: &mut Canonical, value: Option<&str>) -> Result<(), SecretsError> {
    match value {
        None => encoding.field(&[0]),
        Some(text) => {
            let mut bytes = Vec::with_capacity(text.len() + 1);
            bytes.push(1);
            bytes.extend_from_slice(text.as_bytes());
            encoding.field(&bytes)
        }
    }
    .map(|_encoding| ())
}

fn read_optional(reader: &mut Reader<'_>) -> Result<Option<String>, SecretsError> {
    let field = reader.field()?;
    match field.split_first() {
        Some((0, [])) => Ok(None),
        Some((1, text)) => {
            String::from_utf8(text.to_vec())
                .map(Some)
                .map_err(|_utf8| SecretsError::Encoding {
                    context: "audit line",
                    reason: "a text field is not UTF-8".to_owned(),
                })
        }
        _ => Err(SecretsError::Encoding {
            context: "audit line",
            reason: "an optional field is malformed".to_owned(),
        }),
    }
}

fn encode_line(line: &AuditLine) -> Result<Vec<u8>, SecretsError> {
    let mut encoding = Canonical::new(LINE_DOMAIN)?;
    encoding.field(line.kind.label().as_bytes())?;
    encoding.field(&line.at_ms.to_be_bytes())?;
    optional(&mut encoding, line.handle.as_deref())?;
    optional(&mut encoding, line.identity.as_deref())?;
    optional(&mut encoding, line.secret.as_deref())?;
    optional(&mut encoding, line.operation.as_deref())?;
    optional(&mut encoding, line.request.as_deref())?;
    optional(
        &mut encoding,
        line.uses.map(|uses| uses.to_string()).as_deref(),
    )?;
    optional(
        &mut encoding,
        line.spend.map(|spend| spend.to_string()).as_deref(),
    )?;
    encoding.field(line.outcome.as_bytes())?;
    Ok(encoding.into_bytes())
}

fn decode_signed(index: u64, bytes: &[u8], key: &[u8; 32]) -> Result<AuditLine, SecretsError> {
    let unreadable = |reason: String| SecretsError::AuditLineUnreadable { index, reason };
    let mut signed = Reader::new(bytes, "signed audit line");
    if signed.field()? != SIGNED_DOMAIN.as_bytes() {
        return Err(unreadable("not a signed audit line".to_owned()));
    }
    let body = signed.field()?;
    let signature = signed.field()?;
    Ed25519Identity::verify(key, body, signature)
        .map_err(|_invalid| SecretsError::AuditSignatureInvalid { index })?;
    let mut reader = Reader::new(body, "audit line");
    if reader.field()? != LINE_DOMAIN.as_bytes() {
        return Err(unreadable("not an audit line".to_owned()));
    }
    let kind =
        AuditKind::parse(reader.field()?).ok_or_else(|| unreadable("unknown kind".to_owned()))?;
    let at: [u8; 8] = reader
        .field()?
        .try_into()
        .map_err(|_length| unreadable("the time is not 8 bytes".to_owned()))?;
    let handle = read_optional(&mut reader)?;
    let identity = read_optional(&mut reader)?;
    let secret = read_optional(&mut reader)?;
    let operation = read_optional(&mut reader)?;
    let request = read_optional(&mut reader)?;
    let uses = read_optional(&mut reader)?
        .map(|text| {
            text.parse::<u64>()
                .map_err(|_number| unreadable("the use count is not a number".to_owned()))
        })
        .transpose()?;
    let spend = read_optional(&mut reader)?
        .map(|text| {
            text.parse::<u64>()
                .map_err(|_number| unreadable("the spend is not a number".to_owned()))
        })
        .transpose()?;
    let outcome = String::from_utf8(reader.field()?.to_vec())
        .map_err(|_utf8| unreadable("the outcome is not UTF-8".to_owned()))?;
    Ok(AuditLine {
        kind,
        at_ms: i64::from_be_bytes(at),
        handle,
        identity,
        secret,
        operation,
        request,
        uses,
        spend,
        outcome,
    })
}
