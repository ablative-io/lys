//! The receipt of every store Lys keeps (DIRECTORY-107 R2): one file outside
//! `data/`, `install/stores.json`, naming each store by the head it is
//! honestly compared by, and the operator's acceptances of a loss beside it,
//! `install/stores.accepted.json`.
//!
//! Each store is compared by what can only move forward unless a record was
//! lost: an append-only log by its origin digest and head index, a one-file
//! store by the records it holds (none of them retires a record by removing
//! it; its format digest is kept to read, since a first write moves an older
//! whole-file layout to the journal), the permission engine's projection by the grant
//! revision it has projected, and the install's keys and join files by their
//! digest or their presence. A live set's size is never compared: a revoke
//! lowers it honestly.
//!
//! Invariants: the receipt is written only by the service, in one durable
//! write (a new file, synced, renamed over the old, its folder synced); the
//! `lys` binary reads it and writes acceptances only. A removal that lowers a
//! store's head takes [`remove_records`], which lowers the receipt's head
//! before the records go, so a crash between the two leaves the store at or
//! above its receipt; a removal that bypasses it is a loss at the next boot.
//! A store with an acceptance is never accepted by a rollback the installer
//! recorded: only an entry of kind `accepted`, written by
//! `lys identity accept-loss`, lets a boot serve a store found short.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The version of the receipt and acceptance files this build writes and reads.
pub const VERSION: u64 = 1;

/// What a lost store takes with it, and so how much a loss refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// Every route depends on it: a loss refuses the boot.
    Whole,
    /// Only its own routes depend on it: a loss refuses those routes alone.
    Own,
}

/// How a store is compared with its receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparedBy {
    /// An append-only log: its origin digest and its head index.
    LogHead,
    /// A one-file store: its format digest and the records it holds.
    JournalHead,
    /// The permission engine: the grant revision it has projected, of the
    /// grant log identity named.
    ProjectedRevision,
    /// A key: the digest of its public half.
    Digest,
    /// A join file or folder: its presence.
    Presence,
}

/// A grant log's recorded identity and epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogIdentity {
    /// The identity the log was recorded under.
    pub identity: String,
    /// Its reset epoch.
    pub epoch: u64,
}

/// One store as the receipt holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Whole or own.
    pub class: Class,
    /// How it is compared.
    pub compared_by: ComparedBy,
    /// Where it is kept.
    pub path: PathBuf,
    /// The head it was read at: a log's index, a one-file store's records,
    /// the projected revision; zero for a key or a join file.
    pub head: u64,
    /// The origin or format digest of a log or one-file store, the key's
    /// public digest; empty for presence alone.
    pub digest: String,
    /// When it was read, RFC 3339 UTC.
    pub read_at: String,
    /// The grant log's identity, for the grant log and its projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<LogIdentity>,
}

impl Entry {
    /// Whether this entry counts its store as nothing: a log or one-file
    /// store an operator accepted at nothing, before it was first written.
    /// Such an entry has head 0 and no digest.
    #[must_use]
    pub fn counts_nothing(&self) -> bool {
        self.head == 0
            && self.digest.is_empty()
            && matches!(
                self.compared_by,
                ComparedBy::LogHead | ComparedBy::JournalHead
            )
    }
}

/// The receipt: every store by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// [`VERSION`].
    pub version: u64,
    /// When it was written, RFC 3339 UTC.
    pub written_at: String,
    /// Each store, by its name in [`STORES`].
    pub stores: BTreeMap<String, Entry>,
}

/// What was found where a store is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The head found, as [`Entry::head`] counts it.
    pub head: u64,
    /// The digest found, as [`Entry::digest`] names it.
    pub digest: String,
    /// The grant log identity found, for the grant log.
    pub identity: Option<LogIdentity>,
}

/// A store named in Lys's table, with its class and what it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Named {
    /// Its name in the receipt and in `--store`.
    pub name: &'static str,
    /// Whole or own.
    pub class: Class,
    /// How it is compared.
    pub compared_by: ComparedBy,
    /// What its records are, in a person's words: `grant records`.
    pub records: &'static str,
}

const fn named(
    name: &'static str,
    class: Class,
    compared_by: ComparedBy,
    records: &'static str,
) -> Named {
    Named {
        name,
        class,
        compared_by,
        records,
    }
}

/// Every store the service opens at startup, its class and how it is
/// compared. The service maps each name to the place its configuration
/// gives; a store opened at startup and missing here fails the service's
/// own test.
///
/// Whole: every route depends on it (the directory, the grants and their
/// identity, the certificates, the organisation, the service key, the
/// permission engine's projection), or every start opens it and would make
/// it anew, so it cannot be left out while the rest serves (the apps, whose
/// log holds the model every grant is judged by; the stop cord; the runner
/// acts; the seats and their imports; the launch records; the connected
/// apps). Own: the configuration can leave it out, and only its own routes
/// refuse while it is lost.
pub const STORES: &[Named] = &[
    named(
        "directory-log",
        Class::Whole,
        ComparedBy::LogHead,
        "directory records",
    ),
    named(
        "organisation",
        Class::Whole,
        ComparedBy::LogHead,
        "organisation records",
    ),
    named(
        "grant-log",
        Class::Whole,
        ComparedBy::LogHead,
        "grant records",
    ),
    named(
        "certificates",
        Class::Whole,
        ComparedBy::LogHead,
        "certificate records",
    ),
    named(
        "agent-keys",
        Class::Whole,
        ComparedBy::Presence,
        "certificate records",
    ),
    named(
        "service-key",
        Class::Whole,
        ComparedBy::Digest,
        "signing key",
    ),
    named(
        "permission-projection",
        Class::Whole,
        ComparedBy::ProjectedRevision,
        "grant records",
    ),
    named("apps", Class::Whole, ComparedBy::LogHead, "app records"),
    named(
        "cord",
        Class::Whole,
        ComparedBy::LogHead,
        "stop-cord records",
    ),
    named(
        "runner-acts",
        Class::Whole,
        ComparedBy::LogHead,
        "runner act records",
    ),
    named("seats", Class::Whole, ComparedBy::LogHead, "seat records"),
    named(
        "seat-imports",
        Class::Whole,
        ComparedBy::LogHead,
        "seat import records",
    ),
    named(
        "launch-records",
        Class::Whole,
        ComparedBy::LogHead,
        "launch records",
    ),
    named(
        "mcp-connected-apps",
        Class::Whole,
        ComparedBy::Presence,
        "connected app records",
    ),
    named(
        "network",
        Class::Own,
        ComparedBy::JournalHead,
        "computer records",
    ),
    named(
        "network-joins",
        Class::Own,
        ComparedBy::Presence,
        "computer records",
    ),
    named("roles", Class::Own, ComparedBy::JournalHead, "role records"),
    named(
        "provisioning",
        Class::Own,
        ComparedBy::JournalHead,
        "provisioning records",
    ),
    named("homes", Class::Own, ComparedBy::Presence, "agent homes"),
    named(
        "requests",
        Class::Own,
        ComparedBy::LogHead,
        "access request records",
    ),
    named(
        "mcp-requests",
        Class::Own,
        ComparedBy::LogHead,
        "tool request records",
    ),
    named(
        "runtime",
        Class::Own,
        ComparedBy::LogHead,
        "runtime records",
    ),
    named(
        "service-accounts",
        Class::Own,
        ComparedBy::LogHead,
        "service account records",
    ),
    named("reviews", Class::Own, ComparedBy::LogHead, "review records"),
    named("teams", Class::Own, ComparedBy::LogHead, "team records"),
    named("stops", Class::Own, ComparedBy::LogHead, "stop records"),
    named("budgets", Class::Own, ComparedBy::LogHead, "budget records"),
    named(
        "policies",
        Class::Own,
        ComparedBy::LogHead,
        "policy records",
    ),
    named("goals", Class::Own, ComparedBy::LogHead, "goal records"),
    named("words", Class::Own, ComparedBy::LogHead, "word records"),
    named(
        "variables",
        Class::Own,
        ComparedBy::LogHead,
        "variable records",
    ),
    named(
        "schedules",
        Class::Own,
        ComparedBy::LogHead,
        "schedule records",
    ),
    named(
        "runner-machine-key",
        Class::Own,
        ComparedBy::Digest,
        "computer records",
    ),
    named(
        "runner-join",
        Class::Own,
        ComparedBy::Presence,
        "computer records",
    ),
    named(
        "runner-server-key",
        Class::Own,
        ComparedBy::Presence,
        "computer records",
    ),
    named(
        "runner-owners",
        Class::Own,
        ComparedBy::Presence,
        "computer records",
    ),
];

/// The named store `name`, if Lys keeps one by that name.
#[must_use]
pub fn store(name: &str) -> Option<&'static Named> {
    STORES.iter().find(|named| named.name == name)
}

/// The lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Now, RFC 3339 UTC to the microsecond: a receipt and an acceptance made
/// in the same second are still ordered.
#[must_use]
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// Whether the RFC 3339 instant `later` is after `earlier`; an instant that
/// does not read is never after anything.
#[must_use]
pub fn after(later: &str, earlier: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(later),
        chrono::DateTime::parse_from_rfc3339(earlier),
    ) {
        (Ok(later), Ok(earlier)) => later > earlier,
        _ => false,
    }
}

#[path = "stores_receipt_compare.rs"]
mod compare;
pub use compare::{Loss, Verdict, compare, compare_projection};

#[path = "stores_receipt_files.rs"]
mod files;
pub use files::{
    Acceptance, Acceptances, Kind, ReceiptError, accepted_since, append_acceptances, loss_epochs,
    read_acceptances, read_receipt, remove_records, write_receipt,
};

/// The receipt at `path` beside which the acceptances are kept.
#[must_use]
pub fn acceptances_beside(receipt: &Path) -> PathBuf {
    receipt.with_file_name("stores.accepted.json")
}

/// Write `bytes` to `path` durably: a new file beside it, written and
/// synced, renamed over `path`, and the folder synced.
///
/// # Errors
/// The I/O failure, naming nothing secret.
pub fn write_durably(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let folder = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| std::io::Error::other("the file has no folder"))?;
    std::fs::create_dir_all(folder)?;
    let mut name = path.as_os_str().to_os_string();
    name.push(".writing");
    let writing = PathBuf::from(name);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&writing)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&writing, path)?;
    std::fs::File::open(folder)?.sync_all()
}

#[cfg(test)]
#[path = "stores_receipt_tests.rs"]
mod tests;
