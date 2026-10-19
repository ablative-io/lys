//! The receipt and acceptance files: read, written durably, and the one
//! road a removal of records takes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Receipt, VERSION, after, write_durably};

/// A receipt or acceptance file that cannot be read or written, by name.
#[derive(Debug, thiserror::Error)]
pub enum ReceiptError {
    /// `stores_receipt_unreadable`: the file is there and does not read; it
    /// is never taken for an absent one.
    #[error("stores_receipt_unreadable: {path} does not read: {reason}")]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// `stores_receipt_unwritable`: the durable write failed.
    #[error("stores_receipt_unwritable: {path} could not be written: {reason}")]
    Unwritable {
        /// The file.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// `removal_exceeds_receipt`: a removal names more records than the
    /// receipt counts for the store.
    #[error(
        "removal_exceeds_receipt: removing {removed} records from {store}, which the receipt counts at {held}"
    )]
    RemovalExceeds {
        /// The store.
        store: String,
        /// The receipt's head.
        held: u64,
        /// What the removal names.
        removed: u64,
    },
}

impl ReceiptError {
    /// The refusal's stable name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unreadable { .. } => "stores_receipt_unreadable",
            Self::Unwritable { .. } => "stores_receipt_unwritable",
            Self::RemovalExceeds { .. } => "removal_exceeds_receipt",
        }
    }
}

fn unreadable(path: &Path, reason: &impl ToString) -> ReceiptError {
    ReceiptError::Unreadable {
        path: path.to_owned(),
        reason: reason.to_string(),
    }
}

fn unwritable(path: &Path, reason: &impl ToString) -> ReceiptError {
    ReceiptError::Unwritable {
        path: path.to_owned(),
        reason: reason.to_string(),
    }
}

fn read_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, ReceiptError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(path, &error)),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| unreadable(path, &error))
}

fn write_file<T: Serialize>(path: &Path, value: &T) -> Result<(), ReceiptError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| unwritable(path, &error))?;
    bytes.push(b'\n');
    write_durably(path, &bytes).map_err(|error| unwritable(path, &error))
}

/// The receipt at `path`; none when no receipt was ever written there.
///
/// # Errors
/// `stores_receipt_unreadable` for a file that is there and does not read,
/// or of another version.
pub fn read_receipt(path: &Path) -> Result<Option<Receipt>, ReceiptError> {
    let receipt: Option<Receipt> = read_file(path)?;
    match receipt {
        Some(receipt) if receipt.version != VERSION => Err(unreadable(
            path,
            &format!(
                "it is version {}; this build reads version {VERSION}",
                receipt.version
            ),
        )),
        other => Ok(other),
    }
}

/// Write `receipt` to `path` in one durable write.
///
/// # Errors
/// `stores_receipt_unwritable` naming the failure.
pub fn write_receipt(path: &Path, receipt: &Receipt) -> Result<(), ReceiptError> {
    write_file(path, receipt)
}

/// Lower `store`'s head in the receipt at `path` by `removed`, durably,
/// then run `remove`: the one road every removal of records takes. A store
/// the receipt does not name is removed from without a change to it. A
/// crash after the receipt is written and before `remove` ends leaves the
/// store at or above its receipt, which a boot serves.
///
/// # Errors
/// `removal_exceeds_receipt` when `removed` is more than the receipt counts,
/// the receipt's read or write failure, or `remove`'s own.
pub fn remove_records<T, E: From<ReceiptError>>(
    path: &Path,
    store: &str,
    removed: u64,
    remove: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    if let Some(mut receipt) = read_receipt(path)?
        && let Some(entry) = receipt.stores.get_mut(store)
    {
        entry.head =
            entry
                .head
                .checked_sub(removed)
                .ok_or_else(|| ReceiptError::RemovalExceeds {
                    store: store.to_owned(),
                    held: entry.head,
                    removed,
                })?;
        entry.read_at = super::now();
        receipt.written_at.clone_from(&entry.read_at);
        write_receipt(path, &receipt)?;
    }
    remove()
}

/// What an entry of the acceptance file records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The operator's act, `lys identity accept-loss`: the store is served
    /// at what was found.
    Accepted,
    /// A put-back moved records aside; recorded for the operator, it
    /// accepts nothing.
    RolledBack,
}

/// One entry of the acceptance file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    /// Accepted or rolled back.
    pub kind: Kind,
    /// The store.
    pub store: String,
    /// What the receipt held, or the records before a put-back.
    pub held: String,
    /// What was found, or the records after a put-back.
    pub found: String,
    /// When, RFC 3339 UTC.
    pub at: String,
    /// The login's user name, or the installer for a put-back.
    pub by: String,
}

/// The acceptance file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptances {
    /// [`VERSION`].
    pub version: u64,
    /// Every entry, oldest first.
    pub entries: Vec<Acceptance>,
}

/// Every entry of the acceptance file at `path`; none when none was written.
///
/// # Errors
/// `stores_receipt_unreadable` for a file that is there and does not read.
pub fn read_acceptances(path: &Path) -> Result<Vec<Acceptance>, ReceiptError> {
    Ok(read_file::<Acceptances>(path)?
        .map(|file| file.entries)
        .unwrap_or_default())
}

/// Append `entries` to the acceptance file at `path` in one durable write.
///
/// # Errors
/// The read or write failure, by name.
pub fn append_acceptances(path: &Path, entries: &[Acceptance]) -> Result<(), ReceiptError> {
    let mut file = read_file::<Acceptances>(path)?.unwrap_or(Acceptances {
        version: VERSION,
        entries: Vec::new(),
    });
    file.entries.extend_from_slice(entries);
    write_file(path, &file)
}

/// The newest operator acceptance of `store` made after `read_at`.
#[must_use]
pub fn accepted_since<'a>(
    entries: &'a [Acceptance],
    store: &str,
    read_at: &str,
) -> Option<&'a Acceptance> {
    entries.iter().rev().find(|entry| {
        entry.kind == Kind::Accepted && entry.store == store && after(&entry.at, read_at)
    })
}

/// Each store's loss epoch: the `at` of its newest operator acceptance.
#[must_use]
pub fn loss_epochs(entries: &[Acceptance]) -> BTreeMap<String, String> {
    let mut epochs = BTreeMap::new();
    for entry in entries.iter().filter(|entry| entry.kind == Kind::Accepted) {
        epochs.insert(entry.store.clone(), entry.at.clone());
    }
    epochs
}
