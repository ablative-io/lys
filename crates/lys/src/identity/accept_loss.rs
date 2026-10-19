//! `lys identity accept-loss --store <name>...`: the operator's recorded act,
//! and the only way past a store Lys found short of its receipt
//! (DIRECTORY-107 R2); and the same comparison every identity act makes
//! before it changes anything.
//!
//! The receipt, `install/stores.json`, is the service's; this binary reads
//! it and never writes it. An acceptance names one store each, what the
//! receipt held, what was found and who accepted it, and is appended to
//! `install/stores.accepted.json` in one durable write. A store not named is
//! not accepted. The service's next boot serves each accepted store at what
//! was found, and every refusal its loss causes says when it was lost.
//!
//! Before install, upgrade, start and stop change anything, every store the
//! receipt names that this binary can read without opening it for writing
//! (an append-only log by its origin and head, a key by its digest, a join
//! file by its presence) is compared by the receipt's rule, and a loss no
//! operator accepted refuses the act by store name. A one-file store and the
//! permission engine's projection are counted by the service alone.

use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_identity::stores_receipt::{
    self as receipt, Acceptance, ComparedBy, Entry, Found, Kind, Receipt, Verdict,
};
use lys_log_store::{FileLeafStore, LeafStore, StoreError};

use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::install::layout::Layout;
use crate::commands::output::Emitter;

fn receipt_path(layout: &Layout) -> PathBuf {
    layout.stores_receipt()
}

fn unreadable(path: &Path, error: &receipt::ReceiptError) -> IdentityError {
    IdentityError::new(
        ErrorKind::StoresReceiptUnreadable,
        "read stores receipt",
        "install",
        error.to_string(),
    )
    .at(path)
}

fn held(layout: &Layout) -> IdentityResult<(Option<Receipt>, Vec<Acceptance>)> {
    let path = receipt_path(layout);
    let read = receipt::read_receipt(&path).map_err(|error| unreadable(&path, &error))?;
    let accepted_path = receipt::acceptances_beside(&path);
    let accepted = receipt::read_acceptances(&accepted_path)
        .map_err(|error| unreadable(&accepted_path, &error))?;
    Ok((read, accepted))
}

/// What is where an entry keeps its store, as this binary reads it.
enum Seen {
    /// Nothing is there.
    Absent,
    /// The store, at this head.
    Found(Found),
    /// Something this binary cannot read without the service; the service
    /// compares it at its next start.
    Unread,
}

/// What is where `entry` keeps its store, read without writing anything.
fn observe(entry: &Entry) -> Seen {
    let found = |head: u64, digest: String| {
        Seen::Found(Found {
            head,
            digest,
            identity: None,
        })
    };
    let path = &entry.path;
    match entry.compared_by {
        ComparedBy::ProjectedRevision => Seen::Unread,
        _ if !path.exists() => Seen::Absent,
        ComparedBy::LogHead => match FileLeafStore::open_read_only(path) {
            Ok(store) => found(store.extent(), receipt::digest(store.origin().as_bytes())),
            Err(StoreError::NotInitialized { .. }) => Seen::Absent,
            Err(_) => Seen::Unread,
        },
        ComparedBy::Digest => Ed25519Identity::load(path).map_or(Seen::Unread, |key| {
            found(0, receipt::digest(&key.public_key_bytes()))
        }),
        ComparedBy::Presence => found(0, String::new()),
        ComparedBy::JournalHead => Seen::Unread,
    }
}

/// Whether the receipt counts `store` with no operator acceptance since:
/// such a store is never made anew.
///
/// # Errors
/// The receipt or the acceptance file there and unreadable, by name.
pub fn counts(layout: &Layout, store: &str) -> IdentityResult<bool> {
    let (read, accepted) = held(layout)?;
    Ok(read
        .as_ref()
        .and_then(|read| read.stores.get(store))
        .is_some_and(|entry| receipt::accepted_since(&accepted, store, &entry.read_at).is_none()))
}

/// Refuse `act` by store name, before it changes anything, when a store
/// the receipt names is found short, of another origin, or absent, and no
/// operator accepted the loss.
///
/// # Errors
/// `store_lost` naming every such store, both heads and the command; the
/// receipt or acceptance file unreadable, by name.
pub fn check_before(layout: &Layout, act: &'static str) -> IdentityResult<()> {
    let (Some(read), accepted) = held(layout)? else {
        return Ok(());
    };
    let mut lost = Vec::new();
    for (name, entry) in &read.stores {
        let found = match observe(entry) {
            Seen::Absent => None,
            Seen::Found(found) => Some(found),
            Seen::Unread => continue,
        };
        if let Verdict::Lost(loss) = receipt::compare(name, entry, found.as_ref())
            && receipt::accepted_since(&accepted, name, &entry.read_at).is_none()
        {
            lost.push(loss);
        }
    }
    if lost.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = lost.iter().map(|loss| loss.store.as_str()).collect();
    let stores = lost
        .iter()
        .map(|loss| {
            format!(
                "{} ({}: the receipt holds {}; found {})",
                loss.store, loss.refusal, loss.held, loss.found
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Err(IdentityError::new(
        ErrorKind::StoreLost,
        act,
        names.join(", "),
        format!(
            "{stores}; nothing was changed. If what was found is all there is, run `lys identity accept-loss --store {}` first",
            names.join(" --store ")
        ),
    )
    .at(&receipt_path(layout)))
}

/// Record `stores` as accepted at what is found now, by `by`, answering
/// each acceptance written. A name Lys keeps no store by is refused and
/// nothing is written.
///
/// # Errors
/// `store_lost` for a name that is no store; the acceptance file's read or
/// write failure, by name.
pub fn accept(layout: &Layout, stores: &[String], by: &str) -> IdentityResult<Vec<Acceptance>> {
    let (read, _) = held(layout)?;
    if let Some(unknown) = stores.iter().find(|name| receipt::store(name).is_none()) {
        return Err(IdentityError::new(
            ErrorKind::StoreLost,
            "accept loss",
            unknown.clone(),
            "Lys keeps no store by this name; nothing was accepted",
        ));
    }
    let at = receipt::now();
    let written: Vec<Acceptance> = stores
        .iter()
        .map(|name| {
            let entry = read.as_ref().and_then(|read| read.stores.get(name));
            let held = entry.map_or_else(
                || "nothing counted".to_owned(),
                |entry| format!("head {} ({})", entry.head, entry.path.display()),
            );
            let found = match entry.map(observe) {
                Some(Seen::Found(found)) => format!("head {}", found.head),
                Some(Seen::Absent) => "nothing".to_owned(),
                Some(Seen::Unread) | None => "what the service finds at its next start".to_owned(),
            };
            Acceptance {
                kind: Kind::Accepted,
                store: name.clone(),
                held,
                found,
                at: at.clone(),
                by: by.to_owned(),
            }
        })
        .collect();
    let path = receipt::acceptances_beside(&receipt_path(layout));
    receipt::append_acceptances(&path, &written).map_err(|error| unreadable(&path, &error))?;
    Ok(written)
}

/// Runs `lys identity accept-loss`.
///
/// # Errors
/// As [`accept`]; a store named by no `--store` is refused before anything
/// is written.
pub fn run(root: Option<PathBuf>, stores: &[String], json: bool) -> IdentityResult<()> {
    let layout = match root {
        Some(root) => Layout::at(root),
        None => Layout::discover()?,
    };
    if stores.is_empty() {
        return Err(IdentityError::new(
            ErrorKind::StoreLost,
            "accept loss",
            "--store",
            "name each store to accept with --store; nothing was accepted",
        ));
    }
    let by = std::env::var("USER").unwrap_or_else(|_| "the login".to_owned());
    let mut emitter = Emitter::new(json);
    let written = accept(&layout, stores, &by)?;
    for accepted in &written {
        emitter.note(&said(accepted));
    }
    if emitter.is_json() {
        let lines: Vec<String> = written.iter().map(said).collect();
        emitter.field("accepted", "accepted", lines);
    }
    emitter.finish();
    Ok(())
}

/// One acceptance as it is printed.
#[must_use]
pub fn said(accepted: &Acceptance) -> String {
    format!(
        "accepted the loss of {}: the receipt held {}; found {}; at {} by {}",
        accepted.store, accepted.held, accepted.found, accepted.at, accepted.by
    )
}
