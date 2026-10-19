//! The receipt written after the boot check and at a clean stop, each time
//! in one durable write, and the permission engine's projection compared
//! once the grants are open.

use std::collections::BTreeMap;
use std::sync::Arc;

use lys_identity::grants::RelationshipStore;
use lys_identity::stores_receipt::{
    self as receipt, ComparedBy, Entry, LogIdentity, Receipt, VERSION, Verdict,
};

use super::places::{Observed, Place, observe, places};
use super::{Checked, invalid};
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::AppState;

/// A service as it started: its router, the receipt's writer for a clean
/// stop, and the words of a refused start.
pub struct Started {
    /// The routes served.
    pub router: axum::Router,
    /// What writes the receipt at a clean stop; none for a refused start,
    /// whose stores were never opened.
    pub at_stop: Option<AtStop>,
    /// Why the service refused to start, as its startup line says it.
    pub refused: Option<String>,
}

/// What a clean stop writes the receipt from.
pub struct AtStop {
    config: Config,
    state: Arc<AppState>,
    checked: Checked,
}

impl AtStop {
    /// Write the receipt from every store as it stands now, in one durable
    /// write.
    ///
    /// # Errors
    /// A store's count unreadable or the receipt unwritable, by name.
    pub fn write(&self) -> Result<(), ServerError> {
        write(&self.config, &self.state, &self.checked)
    }
}

/// Write the receipt after the boot check, answering its writer for a
/// clean stop.
pub(crate) fn after_boot(
    config: &Config,
    state: &Arc<AppState>,
    checked: Checked,
) -> Result<AtStop, ServerError> {
    let at_stop = AtStop {
        config: config.clone(),
        state: Arc::clone(state),
        checked,
    };
    at_stop.write()?;
    Ok(at_stop)
}

fn poisoned(name: &str, error: &impl std::fmt::Display) -> ServerError {
    invalid(format!("the {name} store's lock is poisoned: {error}"))
}

/// A one-file store's records, counted by the store the service holds open.
fn records(state: &AppState, name: &str) -> Result<Option<u64>, ServerError> {
    let count = match name {
        "network" => state
            .network
            .as_ref()
            .map(|store| store.lock().map(|store| store.machines().len()))
            .transpose()
            .map_err(|error| poisoned(name, &error))?,
        "roles" => state
            .roles
            .as_ref()
            .map(|store| store.lock().map(|store| store.roles().len()))
            .transpose()
            .map_err(|error| poisoned(name, &error))?,
        "provisioning" => state
            .provisioning
            .as_ref()
            .map(|store| store.lock().map(|store| store.profiles().len()))
            .transpose()
            .map_err(|error| poisoned(name, &error))?,
        _ => None,
    };
    Ok(count.map(|count| u64::try_from(count).unwrap_or(u64::MAX)))
}

/// The grant revision the permission engine has projected and the grant
/// log's identity, when the engine is named and the grants are open.
fn projected(state: &AppState) -> Result<Option<(u64, Option<LogIdentity>)>, ServerError> {
    if state.grant_setup.spicedb.is_none() {
        return Ok(None);
    }
    let grants = state
        .grants
        .lock()
        .map_err(|error| poisoned("grants", &error))?;
    let Some(grants) = grants.as_ref() else {
        return Ok(None);
    };
    let revision = grants.relationships().revision()?;
    let identity =
        crate::grant_changes::recorded(&state.grant_setup.log_dir)?.map(|name| LogIdentity {
            identity: name.identity,
            epoch: name.epoch,
        });
    Ok(Some((revision, identity)))
}

fn entry(place: &Place, head: u64, digest: String, identity: Option<LogIdentity>) -> Entry {
    Entry {
        class: place.named.class,
        compared_by: place.named.compared_by,
        path: place.path.clone(),
        head,
        digest,
        read_at: receipt::now(),
        identity,
    }
}

/// A log or one-file store an operator accepted and that holds nothing
/// yet, counted as nothing so its acceptance is kept in the receipt and its
/// first records are counted from there.
fn accepted_at_nothing(checked: &Checked, place: &Place) -> Option<Entry> {
    (checked.accepted.contains_key(place.named.name)
        && matches!(
            place.named.compared_by,
            ComparedBy::LogHead | ComparedBy::JournalHead
        ))
    .then(|| entry(place, 0, String::new(), None))
}

/// `place` as it stands now; none when it holds nothing to count.
fn read(state: &AppState, place: &Place) -> Result<Option<Entry>, ServerError> {
    Ok(match (observe(place), place.named.compared_by) {
        (Observed::Found(found), ComparedBy::JournalHead) => {
            records(state, place.named.name)?.map(|head| entry(place, head, found.digest, None))
        }
        (Observed::Found(found), _) => Some(entry(place, found.head, found.digest, found.identity)),
        (Observed::Unread(_), ComparedBy::ProjectedRevision) => {
            projected(state)?.map(|(head, identity)| entry(place, head, String::new(), identity))
        }
        (Observed::Unread(_) | Observed::Absent, _) => None,
    })
}

fn write(config: &Config, state: &AppState, checked: &Checked) -> Result<(), ServerError> {
    let placed = places(config);
    let held = checked.receipt.as_ref();
    let carried = |name: &str| {
        held.and_then(|receipt| receipt.stores.get(name))
            .filter(|_| !checked.accepted.contains_key(name))
            .cloned()
    };
    let mut stores = BTreeMap::new();
    // A store this configuration leaves out, or one still lost, stays as
    // the receipt held it, so it is compared again when it is back.
    if let Some(receipt) = held {
        for (name, kept) in &receipt.stores {
            if !placed.iter().any(|place| place.named.name == name) {
                stores.insert(name.clone(), kept.clone());
            }
        }
    }
    for place in &placed {
        let name = place.named.name;
        let now = if checked.held.lost.contains_key(name) {
            carried(name)
        } else {
            read(state, place)?
                .or_else(|| carried(name))
                .or_else(|| accepted_at_nothing(checked, place))
        };
        if let Some(now) = now {
            stores.insert(name.to_owned(), now);
        }
    }
    let written = Receipt {
        version: VERSION,
        written_at: receipt::now(),
        stores,
    };
    receipt::write_receipt(&config.stores_receipt_file, &written)
        .map_err(|error| invalid(error.to_string()))
}

/// Compare the permission engine's projection with the grant log when the
/// grants are first opened, which is when the engine is first reached: the
/// boot itself asks no other service. Behind is projected forward before
/// anything is judged; ahead of the log's head, or a projection of another
/// log identity, refuses every grant act by name until its loss is accepted.
///
/// # Errors
/// `spicedb_projection_foreign` naming both revisions or identities; the
/// projection's own failure, by name.
pub(crate) fn projection(
    state: &AppState,
    grants: &mut lys_identity::grants::Grants<
        lys_log_store::FileLeafStore,
        crate::spicedb::Relationships,
    >,
) -> Result<(), ServerError> {
    if state.grant_setup.spicedb.is_none() {
        return Ok(());
    }
    let name = "permission-projection";
    let identity = crate::grant_changes::recorded(&state.grant_setup.log_dir)?
        .map(|recorded| recorded.identity);
    let projected = grants.relationships().revision()?;
    let head = grants.revision();
    match receipt::compare_projection(
        name,
        state.stores.projection.as_ref(),
        projected,
        head,
        identity.as_deref(),
    ) {
        Verdict::Kept => Ok(()),
        Verdict::Behind { projected, head } => {
            let reached = grants.project()?;
            (state.say)(&format!(
                "{name} was behind the grant log at revision {projected} of {head}; projected to {reached}"
            ));
            Ok(())
        }
        Verdict::Lost(_) if state.stores.lost_at(name).is_some() => Ok(()),
        Verdict::Lost(loss) => {
            let words = format!(
                "{}: {name}: the receipt holds {}; found {}. If what was found is all there is, run `lys identity accept-loss --store {name}` on {}",
                loss.refusal,
                loss.held,
                loss.found,
                super::computer()
            );
            (state.say)(&words);
            Err(invalid(words))
        }
    }
}
