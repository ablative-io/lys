//! The audit log. Every issue, use, refusal, drop and rotation is one line,
//! signed by the broker's audit key and appended through lys-log-store. The
//! log's head is anchored outside the store and log directories, so a log
//! restored from an older copy is refused at start.
//!
//! A start does not read the whole log. The log opens from the signed
//! snapshot its owner last wrote, and hands the owner that snapshot's state
//! and only the lines after it, each with its signature checked. A snapshot
//! that cannot be believed is refused by name in [`AuditLog::start`], and
//! then every line is read.

use std::fs;
use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_log_store::{FileLeafStore, FrontierLog, SnapshotRefusal, Start, Tail, start};
use serde::{Deserialize, Serialize};

use crate::encoding::{Canonical, hex, sha256};
use crate::error::SecretsError;
use crate::fsutil::{ensure_outside, io, write_atomic};
use crate::keys::StoreKey;

const LINE_DOMAIN: &str = "lys-secrets/audit-line/v2";
const SIGNED_DOMAIN: &str = "lys-secrets/audit-signed/v1";
const ANCHOR_DOMAIN: &str = "lys-secrets/audit-anchor/v1";
/// The kind of state the log's snapshot carries.
pub const STATE_DOMAIN: &str = "lys-secrets/audit-state/v1";

mod codec;
use codec::{decode_signed, encode_line};

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
    /// An OAuth access token was refreshed, or a grant revoked upstream.
    Refresh,
    /// A seat's own login was handed to it at spawn.
    SpawnLogin,
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
            Self::Refresh => "refresh",
            Self::SpawnLogin => "spawn_login",
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
            b"refresh" => Some(Self::Refresh),
            b"spawn_login" => Some(Self::SpawnLogin),
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

/// What a start hands the log's owner to fold.
#[derive(Debug)]
pub struct Opened {
    /// The owner's state from the snapshot. `None` when the snapshot was
    /// refused, and then `lines` holds every line.
    pub state: Option<Vec<u8>>,
    /// The lines after the state, in order, each signature verified.
    pub lines: Vec<RecordedLine>,
}

/// The append-only, signed audit log.
pub struct AuditLog {
    log: FrontierLog<FileLeafStore>,
    dir: PathBuf,
    anchor: PathBuf,
    verifying_key: [u8; 32],
    /// The digest of the last line, zero for a log that holds none.
    head: [u8; 32],
    start: Start,
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("origin", &self.log.origin())
            .field("len", &self.log.len())
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
        let (log, _tail) = FrontierLog::open(FileLeafStore::create(dir, origin)?)?;
        let audit = Self {
            log,
            dir: dir.to_path_buf(),
            anchor: anchor.to_path_buf(),
            verifying_key: key.verifying_key(),
            head: [0u8; 32],
            start: Start::Rebuilt {
                refusal: SnapshotRefusal::Missing,
                replayed: 0,
            },
        };
        audit.write_anchor(key.identity())?;
        Ok(audit)
    }

    /// Opens the log in `dir` from its snapshot and checks it against its
    /// anchor. Only the lines after the snapshot are read, and each one's
    /// signature is checked. When the snapshot is refused every line is.
    ///
    /// # Errors
    ///
    /// `LogBehindAnchor`-shaped `AuditLineMissing` when the log is shorter
    /// than its anchor, `AuditSignatureInvalid` when the anchor or a line
    /// read does not verify, and `Log`.
    pub fn open(
        dir: &Path,
        anchor: &Path,
        guarded: &[&Path],
        key: &StoreKey,
    ) -> Result<(Self, Opened), SecretsError> {
        ensure_outside(anchor, guarded)?;
        let started = start(
            FileLeafStore::open(dir)?,
            STATE_DOMAIN,
            &key.verifying_key(),
        )?;
        let audit = Self {
            log: started.log,
            dir: dir.to_path_buf(),
            anchor: anchor.to_path_buf(),
            verifying_key: key.verifying_key(),
            head: [0u8; 32],
            start: started.start,
        };
        audit.opened(started.state, &started.tail)
    }

    /// Opens the log again from its first line, because its owner could not
    /// read the state the snapshot carries.
    ///
    /// # Errors
    ///
    /// As [`AuditLog::open`].
    pub fn rebuild(self, reason: String) -> Result<(Self, Opened), SecretsError> {
        let Self {
            log,
            dir,
            anchor,
            verifying_key,
            ..
        } = self;
        drop(log);
        let (log, tail) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
        let audit = Self {
            log,
            dir,
            anchor,
            verifying_key,
            head: [0u8; 32],
            start: Start::Rebuilt {
                refusal: SnapshotRefusal::StateUnreadable { reason },
                replayed: u64::try_from(tail.leaves.len()).unwrap_or(u64::MAX),
            },
        };
        audit.opened(None, &tail)
    }

    fn opened(
        mut self,
        state: Option<Vec<u8>>,
        tail: &Tail,
    ) -> Result<(Self, Opened), SecretsError> {
        self.head = self.digest_before(self.len())?;
        self.check_anchor()?;
        let lines = (tail.from..)
            .zip(&tail.leaves)
            .map(|(index, bytes)| {
                let line = decode_signed(index, bytes, &self.verifying_key)?;
                Ok(RecordedLine { index, line })
            })
            .collect::<Result<_, SecretsError>>()?;
        Ok((self, Opened { state, lines }))
    }

    /// How the log was started: from its snapshot, or from every line and
    /// why.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Writes a signed snapshot of `state`, which must be the owner's fold
    /// of every line the log holds now. Answers the size it is bound to.
    ///
    /// # Errors
    ///
    /// `Log`.
    pub fn write_snapshot(
        &mut self,
        state: &[u8],
        key: &Ed25519Identity,
    ) -> Result<u64, SecretsError> {
        Ok(self.log.write_snapshot(STATE_DOMAIN, state, key)?)
    }

    /// The number of lines.
    pub fn len(&self) -> u64 {
        self.log.len()
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
        let bytes = signed.into_bytes();
        let (index, _leaf_hash) = self.log.append(&bytes)?;
        self.head = sha256(&bytes);
        self.write_anchor(key)?;
        Ok(index)
    }

    /// Every line, each signature verified against the audit key. This reads
    /// the whole log, and is no part of a start.
    ///
    /// # Errors
    ///
    /// `AuditLineMissing`, `AuditLineUnreadable` and `AuditSignatureInvalid`.
    pub fn replay(&self) -> Result<Vec<RecordedLine>, SecretsError> {
        self.lines_from(0)
    }

    /// Every line from `from` on, each signature verified.
    ///
    /// # Errors
    ///
    /// As [`AuditLog::replay`].
    pub fn lines_from(&self, from: u64) -> Result<Vec<RecordedLine>, SecretsError> {
        (from..self.len())
            .map(|index| {
                let bytes = self.leaf(index)?;
                let line = decode_signed(index, &bytes, &self.verifying_key)?;
                Ok(RecordedLine { index, line })
            })
            .collect()
    }

    fn leaf(&self, index: u64) -> Result<Vec<u8>, SecretsError> {
        self.log
            .leaf_bytes(index)?
            .ok_or(SecretsError::AuditLineMissing {
                index,
                len: self.len(),
            })
    }

    /// The digest of the last of the first `len` lines, read from the log,
    /// zero when there is none.
    fn digest_before(&self, len: u64) -> Result<[u8; 32], SecretsError> {
        match len.checked_sub(1) {
            None => Ok([0u8; 32]),
            Some(last) => Ok(sha256(&self.leaf(last)?)),
        }
    }

    fn anchor_body(len: u64, digest: &[u8; 32]) -> Result<Vec<u8>, SecretsError> {
        let mut body = Canonical::new(ANCHOR_DOMAIN)?;
        body.number(len)?.field(digest)?;
        Ok(body.into_bytes())
    }

    fn write_anchor(&self, key: &Ed25519Identity) -> Result<(), SecretsError> {
        let digest = self.head;
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
        let signature = crate::encoding::unhex(&anchor.signature)
            .ok_or(SecretsError::AuditSignatureInvalid { index: anchor.len })?;
        let body = Self::anchor_body(anchor.len, &digest)?;
        Ed25519Identity::verify(&self.verifying_key, &body, &signature)
            .map_err(|_invalid| SecretsError::AuditSignatureInvalid { index: anchor.len })?;
        if self.len() < anchor.len {
            return Err(SecretsError::AuditLineMissing {
                index: anchor.len,
                len: self.len(),
            });
        }
        let at_anchor = if anchor.len == self.len() {
            self.head
        } else {
            self.digest_before(anchor.len)?
        };
        if at_anchor != digest {
            return Err(SecretsError::AuditSignatureInvalid { index: anchor.len });
        }
        Ok(())
    }
}
