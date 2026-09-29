//! Every refusal and failure the broker names. Each message names the act
//! that answers it.

use std::path::PathBuf;

mod accounts;
mod bounds;
mod lease;
mod lending;
mod name;
mod oauth;
mod owner;
mod revocation;
mod service;

pub use accounts::AccountsRefusal;
pub use bounds::BoundsRefusal;
pub use lease::{LeaseRefusal, ListRefusal};
pub use lending::LendingRefusal;
pub use oauth::OAuthRefusal;
pub use owner::OwnerChangeRefusal;
pub use revocation::RevocationRefusal;
pub use service::ServiceRefusal;

/// A named refusal or failure of the secrets broker.
#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    /// No store key was supplied, or the supplied key file does not exist.
    #[error("StoreKeyMissing: no store key at {path:?} (act: supply the store key file at start)")]
    StoreKeyMissing {
        /// The key file asked for, if one was named.
        path: Option<PathBuf>,
    },
    /// A key file lies inside the store or the log directory.
    #[error(
        "KeyFileMisplaced: key file {key} lies inside {dir} (act: move the key file outside the store and log directories and supply its new location)"
    )]
    KeyFileMisplaced {
        /// The key file.
        key: PathBuf,
        /// The directory it lies inside.
        dir: PathBuf,
    },
    /// A key file is readable or writable by someone other than its owner.
    #[error(
        "KeyFilePermissions: key file {path} has mode {mode:o}, not 600 (act: chmod 600 the key file)"
    )]
    KeyFilePermissions {
        /// The key file.
        path: PathBuf,
        /// Its permission bits.
        mode: u32,
    },
    /// A new key file was asked for where one already exists.
    #[error("KeyFileExists: {path} already exists (act: name a new key file location)")]
    KeyFileExists {
        /// The existing file.
        path: PathBuf,
    },
    /// The supplied store key is not the store's current key.
    #[error(
        "StoreKeyMismatch: the store is sealed under key {expected} and key {presented} was supplied (act: supply the store's current key)"
    )]
    StoreKeyMismatch {
        /// The store's current key id.
        expected: String,
        /// The supplied key's id.
        presented: String,
    },
    /// The supplied store key was rotated out.
    #[error(
        "StoreKeyRetired: key {key_id} was rotated out of this store (act: use the current key)"
    )]
    StoreKeyRetired {
        /// The retired key's id.
        key_id: String,
    },
    /// No store exists in the directory.
    #[error("StoreNotFound: no store at {path} (act: create the store first)")]
    StoreNotFound {
        /// The directory.
        path: PathBuf,
    },
    /// A store already exists in the directory.
    #[error("StoreAlreadyExists: a store already exists at {path} (act: open it instead)")]
    StoreAlreadyExists {
        /// The directory.
        path: PathBuf,
    },
    /// The persisted index has not yet been reconciled and durably synced.
    #[error("IndexUnresolved: {reason} (act: reconcile the index before confirming custody)")]
    IndexUnresolved { reason: String },
    /// The store's index could not be read.
    #[error("StoreCorrupt: {reason} (act: restore the store from its current copy)")]
    StoreCorrupt {
        /// What was wrong.
        reason: String,
    },
    /// A sealed entry opened to a binding other than the entry asked for.
    #[error(
        "EntryBindingMismatch: the sealed entry for {entry} is bound to another entry (act: restore the store from its current copy)"
    )]
    EntryBindingMismatch {
        /// The entry asked for.
        entry: String,
    },
    /// A sealed entry is older than the sequence the store records for it.
    #[error(
        "EntryRolledBack: entry {entry} holds sequence {found}, older than the recorded {recorded} (act: restore the store from its current copy or write the entry again)"
    )]
    EntryRolledBack {
        /// The entry.
        entry: String,
        /// The sequence the store's index records.
        recorded: u64,
        /// The sequence the sealed entry carries.
        found: u64,
    },
    /// A sealed entry did not open under the store key.
    #[error(
        "EntryUnsealFailed: entry {entry} does not open under the current store key (act: restore the store from its current copy)"
    )]
    EntryUnsealFailed {
        /// The entry.
        entry: String,
    },
    /// A secret of that name already exists.
    #[error("SecretExists: a secret named {name} already exists (act: choose another name)")]
    SecretExists {
        /// The name.
        name: String,
    },
    /// No secret of that name exists.
    #[error("SecretUnknown: no secret named {name} (act: add the secret first)")]
    SecretUnknown {
        /// The name.
        name: String,
    },
    /// The secret holds no account of that name.
    #[error(
        "AccountUnknown: secret {secret} holds no account {account} (act: name one of its accounts)"
    )]
    AccountUnknown {
        /// The secret.
        secret: String,
        /// The account.
        account: String,
    },
    /// The secret already holds an account of that name.
    #[error(
        "AccountExists: secret {secret} already holds account {account} (act: choose another account name)"
    )]
    AccountExists {
        /// The secret.
        secret: String,
        /// The account.
        account: String,
    },
    /// No account of the secret can serve.
    #[error(transparent)]
    Accounts(#[from] AccountsRefusal),
    /// A name is empty, too long, or holds a control character.
    #[error(
        "InvalidName: {what} {name:?} {reason} (act: use a non-empty printable name of at most 128 bytes)"
    )]
    InvalidName {
        /// What was being named.
        what: &'static str,
        /// The name given.
        name: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A handle's lifetime is zero or longer than the broker allows.
    #[error(
        "InvalidLifetime: a handle lives more than 0 and at most {max_ms} ms, and {asked_ms} ms was asked (act: ask for a lifetime within the limit)"
    )]
    InvalidLifetime {
        /// The lifetime asked for, in milliseconds.
        asked_ms: u128,
        /// The longest lifetime allowed, in milliseconds.
        max_ms: u128,
    },
    /// No issued handle matches the one presented.
    #[error(
        "HandleUnknown: the presented handle was not issued by this broker (act: present a handle the broker issued)"
    )]
    HandleUnknown,
    /// The handle's lifetime has passed.
    #[error(
        "HandleExpired: handle {handle} expired at {expired_at_ms} ms (act: ask the grant's owner for a new handle)"
    )]
    HandleExpired {
        /// The handle id.
        handle: String,
        /// When it expired, in unix milliseconds.
        expired_at_ms: i64,
    },
    /// The handle has been dropped.
    #[error(
        "HandleDropped: handle {handle} was dropped (act: ask the grant's owner for a new handle)"
    )]
    HandleDropped {
        /// The handle id.
        handle: String,
    },
    /// The handle was presented by an identity other than its holder.
    #[error(
        "HandleWrongIdentity: handle {handle} is bound to another identity than {presenter} (act: present the handle from the identity it was issued to)"
    )]
    HandleWrongIdentity {
        /// The handle id.
        handle: String,
        /// The identity that presented it.
        presenter: String,
    },
    /// The presentation carries no signature.
    #[error(
        "PresentationUnsigned: the presentation for handle {handle} carries no signature (act: sign the presentation with the key registered for the handle's identity)"
    )]
    PresentationUnsigned {
        /// The handle id.
        handle: String,
    },
    /// The presentation's signature did not verify against the holder's key.
    #[error(
        "PresentationInvalid: the presentation for handle {handle} does not verify (act: sign the presentation with the key registered for the handle's identity)"
    )]
    PresentationInvalid {
        /// The handle id.
        handle: String,
    },
    /// The presentation's operation id was already used under this handle.
    #[error(
        "PresentationReplayed: operation {operation} was already presented under handle {handle} (act: sign a new presentation with a new operation id)"
    )]
    PresentationReplayed {
        /// The handle id.
        handle: String,
        /// The operation id, hex.
        operation: String,
    },
    /// The presentation's signed time is too far from the broker's clock.
    #[error(
        "PresentationStale: the presentation was signed {skew_ms} ms from the broker's clock, beyond {limit_ms} ms (act: sign the presentation again with the current time)"
    )]
    PresentationStale {
        /// The distance between the signed time and the broker's clock.
        skew_ms: i64,
        /// The largest distance allowed.
        limit_ms: i64,
    },
    /// The operation id is shorter than 16 bytes.
    #[error(
        "OperationIdTooShort: an operation id of {len} bytes was sent (act: send an operation id of at least 16 random bytes)"
    )]
    OperationIdTooShort {
        /// Its length.
        len: usize,
    },
    /// The permission check did not permit the use.
    #[error(
        "PermissionDenied: {holder} may not use {secret}: {reason} (act: ask the resource's owner for the permission)"
    )]
    PermissionDenied {
        /// The holder.
        holder: String,
        /// The secret.
        secret: String,
        /// The permission check's reason.
        reason: String,
    },
    /// An audit log leaf is not a signed line.
    #[error(
        "AuditLineUnreadable: audit leaf {index} is not a signed line: {reason} (act: restore the audit log from its current copy)"
    )]
    AuditLineUnreadable {
        /// The leaf index.
        index: u64,
        /// Why.
        reason: String,
    },
    /// An audit line's signature does not verify against the audit key.
    #[error(
        "AuditSignatureInvalid: audit line {index} is not signed by the broker's audit key (act: restore the audit log from its current copy)"
    )]
    AuditSignatureInvalid {
        /// The leaf index.
        index: u64,
    },
    /// An audit line's inclusion proof does not verify against the log root.
    #[error(
        "AuditProofInvalid: the inclusion proof of audit line {index} does not verify: {reason} (act: restore the audit log from its current copy)"
    )]
    AuditProofInvalid {
        /// The leaf index.
        index: u64,
        /// Why.
        reason: String,
    },
    /// No audit line has that index.
    #[error(
        "AuditLineMissing: no audit line {index}; the log holds {len} (act: name a line the log holds)"
    )]
    AuditLineMissing {
        /// The index asked for.
        index: u64,
        /// The number of lines.
        len: u64,
    },
    /// The lease's uses are spent.
    #[error(
        "LeaseExhausted: handle {handle} has no uses left (act: ask the grant's owner for more uses)"
    )]
    LeaseExhausted {
        /// The handle id.
        handle: String,
    },
    /// The lease's window has closed.
    #[error(
        "LeaseWindowClosed: handle {handle} was presented after its window (act: ask the grant's owner for a new lease within the grant's window)"
    )]
    LeaseWindowClosed {
        /// The handle id.
        handle: String,
    },
    /// The grant a handle would be issued under traces to no person.
    #[error(
        "NoPersonRoot: the access for {holder} traces to no person (act: have a person grant the access)"
    )]
    NoPersonRoot {
        /// The holder asked for.
        holder: String,
    },
    /// An operation id was sent again with a different request.
    #[error(
        "OperationIdReused: operation {operation} was sent with another request (act: send a new operation id)"
    )]
    OperationIdReused {
        /// The operation id.
        operation: String,
    },
    /// A sealed record read by an identity without the `read` relation.
    #[error(
        "NoRelation: {identity} holds no read relation on {record} (act: ask the record's owner for the relation)"
    )]
    NoRelation {
        /// The asking identity.
        identity: String,
        /// The record's name.
        record: String,
    },
    /// A sealed record read after the identity's `read` relation was removed.
    #[error(
        "RelationRemoved: the read relation of {identity} on {record} was removed (act: ask the record's owner to restore the relation)"
    )]
    RelationRemoved {
        /// The asking identity.
        identity: String,
        /// The record's name.
        record: String,
    },
    /// A read of a key or credential, which is used and never read.
    #[error(
        "KeyNotReadable: {record} is used through the proxy and never read (act: use the key through the proxy)"
    )]
    KeyNotReadable {
        /// The record's name.
        record: String,
    },
    /// A handle asked for on a memory record, which is read and never used.
    #[error(
        "MemoryNotUsable: {record} is a memory record, read by name and never used through a handle (act: read it under the read relation)"
    )]
    MemoryNotUsable {
        /// The record's name.
        record: String,
    },
    /// A derived or issued handle refused by the rules of lending.
    #[error(transparent)]
    Lending(#[from] LendingRefusal),
    /// A lease refused for its bounds.
    #[error(transparent)]
    Lease(#[from] BoundsRefusal),
    /// An OAuth grant's refresh refused.
    #[error(transparent)]
    OAuth(#[from] OAuthRefusal),
    /// A provider revocation's confirmation refused.
    #[error(transparent)]
    Revocation(#[from] RevocationRefusal),
    /// A screen service's request on a person's behalf refused.
    #[error(transparent)]
    Service(#[from] ServiceRefusal),
    /// An owner change refused for its operation id.
    #[error(transparent)]
    OwnerChange(#[from] OwnerChangeRefusal),
    /// A scope written as none of the three kinds.
    #[error(
        "InvalidScope: {text:?} is not a scope (act: name it as personal:<person>, team:<name> or organisation:<name>)"
    )]
    InvalidScope {
        /// The text given.
        text: String,
    },
    /// A read of a record that is not there to be read.
    #[error("NotFound: no record by that name (act: ask for a record by a name you can discover)")]
    NotFound,
    /// A reservation would take a capped lease past its cap.
    #[error(
        "SpendCapReached: handle {handle} has {left} of its spend cap {cap} left and asked to reserve {asked} (act: ask the grant's owner to raise the spend cap)"
    )]
    SpendCapReached {
        /// The handle id.
        handle: String,
        /// The lease's cap.
        cap: u64,
        /// What is left after settled spend and open reservations.
        left: u64,
        /// The reservation asked for.
        asked: u64,
    },
    /// A use of a capped lease named no amount to reserve.
    #[error(
        "ReservationMissing: handle {handle} has a spend cap and the call reserved nothing (act: send the amount the call may spend)"
    )]
    ReservationMissing {
        /// The handle id.
        handle: String,
    },
    /// Another broker holds the store.
    #[error("StoreLocked: another broker holds the store at {path} (act: stop the other broker)")]
    StoreLocked {
        /// The store directory.
        path: PathBuf,
    },
    /// The broker's state lock was poisoned by a panic in another thread.
    #[error(
        "StatePoisoned: a thread panicked while holding the broker's state (act: restart the broker)"
    )]
    StatePoisoned,
    /// The permission check could not be set up or changed.
    #[error("Grants: {0} (act: correct the grant request)")]
    Grants(Box<lys_identity::grants::GrantError>),
    /// The directory of identities refused or failed.
    #[error("Identity: {0} (act: correct the directory request)")]
    Identity(Box<lys_identity::IdentityError>),
    /// The secure random source failed.
    #[error(
        "Random: the secure random source failed: {reason} (act: retry; if it persists, check the operating system's entropy source)"
    )]
    Random {
        /// Its report.
        reason: String,
    },
    /// An upstream answered more than the proxy carries back. Nothing of
    /// it is returned: an answer is never cut short.
    #[error(
        "AnswerTooLarge: the upstream answered more than {limit} bytes (act: ask the upstream for less, or page the answer)"
    )]
    AnswerTooLarge {
        /// The most the proxy carries back, in bytes.
        limit: usize,
    },
    /// Bytes could not be encoded or decoded.
    #[error("Encoding: {context}: {reason}")]
    Encoding {
        /// What was being encoded or decoded.
        context: &'static str,
        /// What went wrong.
        reason: String,
    },
    /// A filesystem operation failed.
    #[error("Io: {context}: {source}")]
    Io {
        /// What was being done.
        context: String,
        /// The operating system's report.
        source: std::io::Error,
    },
    /// A lys-core primitive failed.
    #[error("Trust: {0}")]
    Trust(Box<lys_core::TrustError>),
    /// The audit log's storage failed.
    #[error("Log: {0}")]
    Log(Box<lys_log_store::StoreError>),
}

impl From<lys_core::TrustError> for SecretsError {
    fn from(error: lys_core::TrustError) -> Self {
        Self::Trust(Box::new(error))
    }
}

impl From<lys_log_store::StoreError> for SecretsError {
    fn from(error: lys_log_store::StoreError) -> Self {
        Self::Log(Box::new(error))
    }
}

impl From<lys_identity::grants::GrantError> for SecretsError {
    fn from(error: lys_identity::grants::GrantError) -> Self {
        Self::Grants(Box::new(error))
    }
}

impl From<lys_identity::IdentityError> for SecretsError {
    fn from(error: lys_identity::IdentityError) -> Self {
        Self::Identity(Box::new(error))
    }
}
