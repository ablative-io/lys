//! Every store held against its receipt (DIRECTORY-107 R2, R4): compared
//! at boot before anything is opened or served, written after the boot
//! check and at a clean stop, and the losses an operator accepted carried
//! into every refusal they cause.
//!
//! At boot, before the directory is opened, each store the receipt names is
//! read where the configuration keeps it, without creating or writing
//! anything, and compared by the rule its kind keeps
//! (`lys_identity::stores_receipt`). A whole store found short, of another
//! origin or identity, or absent refuses the boot: the service then serves
//! `GET /health` and the sign-in page alone (`boot_refused`). An own store
//! found so is left out of the configuration the service opens, so it is
//! never made anew empty and only its routes refuse, with the loss in their
//! words. A store with an operator's acceptance newer than its receipt entry
//! is served at what was found, and the acceptance's instant becomes its loss
//! epoch; an accepted loss on the grant log advances the log's identity to
//! the next epoch first, so every consumer that read the earlier one is told
//! it was rolled back.
//!
//! Invariants: the receipt is written only here, each time in one durable
//! write, and an own store still lost is written back as the receipt held
//! it, so it stays refused until accepted. No flag, environment variable or
//! configuration key skips the comparison.

use std::collections::BTreeMap;

use lys_identity::stores_receipt::{
    self as receipt, Acceptance, Class, ComparedBy, Entry, Loss, Receipt, Verdict,
};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;

#[path = "stores_receipt/places.rs"]
pub mod places;
#[path = "stores_receipt/words.rs"]
pub(crate) mod words;

use places::{Observed, Place, observe, places};

fn invalid(reason: impl Into<String>) -> ServerError {
    ServerError::ConfigInvalid {
        reason: reason.into(),
    }
}

fn unreadable(error: &receipt::ReceiptError) -> ServerError {
    invalid(error.to_string())
}

/// The losses this service holds while it serves: each store's loss epoch
/// and each own store still lost, with this computer's name for the words.
#[derive(Debug, Clone, Default)]
pub struct Held {
    /// Each store's loss epoch, the instant of its newest acceptance.
    pub(crate) epochs: BTreeMap<String, String>,
    /// Each own store lost and not accepted.
    pub(crate) lost: BTreeMap<String, Loss>,
    /// This computer's name, where `accept-loss` is run; read only once a
    /// loss is found.
    pub(crate) computer: String,
    /// The permission engine's projection as the receipt holds it, compared
    /// when the grants are first opened.
    pub(crate) projection: Option<Entry>,
}

impl Held {
    /// The loss epoch of `store`, when an operator accepted a loss of it.
    #[must_use]
    pub fn lost_at(&self, store: &str) -> Option<&str> {
        self.epochs.get(store).map(String::as_str)
    }

    /// Whether there is nothing to say: no epoch and no own store lost.
    fn quiet(&self) -> bool {
        self.epochs.is_empty() && self.lost.is_empty()
    }
}

/// This computer's own name, as the network screens offer it.
pub(crate) fn computer() -> String {
    crate::network_api::host_name().unwrap_or_else(|| "the computer Lys runs on".to_owned())
}

/// What the boot check found.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    receipt: Option<Receipt>,
    /// Whole stores lost and not accepted: the boot is refused by them.
    pub(crate) refused: Vec<Loss>,
    /// Stores lost and accepted since the receipt read them, by name.
    accepted: BTreeMap<String, Acceptance>,
    /// Every entry of the acceptance file, for the stores compared once
    /// they are opened.
    acceptances: Vec<Acceptance>,
    held: Held,
}

impl Checked {
    /// Compare every store the receipt names with what is where the
    /// configuration keeps it, before anything is opened, saying each loss.
    ///
    /// # Errors
    /// The receipt or the acceptance file there and unreadable, by name.
    pub fn before_open(config: &Config, say: &Say) -> Result<Self, ServerError> {
        let held_receipt = receipt::read_receipt(&config.stores_receipt_file)
            .map_err(|error| unreadable(&error))?;
        let acceptances = receipt::read_acceptances(&config.stores_accepted_file)
            .map_err(|error| unreadable(&error))?;
        let mut checked = Self {
            held: Held {
                epochs: receipt::loss_epochs(&acceptances),
                lost: BTreeMap::new(),
                computer: String::new(),
                projection: None,
            },
            ..Self::default()
        };
        if let Some(held) = &held_receipt {
            for place in places(config) {
                let Some(entry) = held.stores.get(place.named.name) else {
                    continue;
                };
                let verdict = match observe(&place) {
                    Observed::Unread(_) => continue,
                    Observed::Absent => receipt::compare(place.named.name, entry, None),
                    // A one-file store's records are counted once it is
                    // opened (`journal_head`); here it is present.
                    Observed::Found(_) if entry.compared_by == ComparedBy::JournalHead => continue,
                    Observed::Found(found) => identity_lost(&place, entry, &found)
                        .unwrap_or_else(|| receipt::compare(place.named.name, entry, Some(&found))),
                };
                if let Verdict::Lost(loss) = verdict {
                    checked.lost(&place, entry, loss, &acceptances, say);
                }
            }
        }
        checked.receipt = held_receipt;
        checked.acceptances = acceptances;
        Ok(checked)
    }

    /// Record `loss` of `place`: accepted, refusing the boot, or refusing
    /// the store's own routes.
    fn lost(
        &mut self,
        place: &Place,
        entry: &Entry,
        loss: Loss,
        acceptances: &[Acceptance],
        say: &Say,
    ) {
        let name = place.named.name;
        if self.held.computer.is_empty() {
            self.held.computer = computer();
        }
        if let Some(accepted) = receipt::accepted_since(acceptances, name, &entry.read_at) {
            say(&format!(
                "{name}: the loss {} accepted at {} is served at what was found ({})",
                loss.refusal, accepted.at, loss.found
            ));
            self.accepted.insert(name.to_owned(), accepted.clone());
            return;
        }
        let words = format!(
            "{name} {}: the receipt holds {}; found {}",
            loss.refusal, loss.held, loss.found
        );
        match place.named.class {
            Class::Whole => {
                say(&format!("refused_to_start: {words}"));
                self.refused.push(loss);
            }
            Class::Own => {
                say(&format!(
                    "{words}; its routes refuse until the loss is accepted"
                ));
                self.held.lost.insert(name.to_owned(), loss);
            }
        }
    }

    /// The configuration the service opens: every own store still lost left
    /// out, so none is made anew empty; an accepted loss on the grant log
    /// advanced to its next epoch first.
    ///
    /// # Errors
    /// The grant log identity's advance, by name.
    pub fn opened(&self, config: &Config, say: &Say) -> Result<Config, ServerError> {
        if self.accepted.contains_key("grant-log")
            && let Some(held) = self
                .entry("grant-log")
                .and_then(|entry| entry.identity.clone())
        {
            let advanced = crate::grant_changes::rolled_back(
                &config.grant_log_dir,
                &config.grant_log_origin,
                &lys_identity::grants::change_stream::LogName {
                    identity: held.identity,
                    epoch: held.epoch,
                },
            )?;
            say(&format!(
                "grant log {} advanced from epoch {} to {} past its accepted loss",
                advanced.new.identity, advanced.old.epoch, advanced.new.epoch
            ));
        }
        let mut opened = config.clone();
        for name in self.held.lost.keys() {
            match name.as_str() {
                "network" | "network-joins" => opened.network_file = None,
                "roles" => opened.roles_file = None,
                "provisioning" => opened.provisioning_file = None,
                "homes" => opened.homes_dir = None,
                "requests" | "mcp-requests" => opened.requests_dir = None,
                "runtime" => opened.runtime_dir = None,
                "service-accounts" => opened.service_accounts_dir = None,
                "reviews" => opened.reviews_dir = None,
                "teams" => opened.teams_dir = None,
                "stops" => opened.stops_dir = None,
                "budgets" => opened.budgets_dir = None,
                "policies" => opened.policies_dir = None,
                "goals" => opened.goals_dir = None,
                "words" => opened.words_dir = None,
                "variables" => opened.variables_dir = None,
                "schedules" => opened.schedules_dir = None,
                _ => opened.runner_socket = None,
            }
        }
        Ok(opened)
    }

    fn entry(&self, name: &str) -> Option<&Entry> {
        self.receipt.as_ref().and_then(|held| held.stores.get(name))
    }

    /// Whether the receipt counts `name`, so it is never made anew empty.
    #[must_use]
    pub fn counts(&self, name: &str) -> bool {
        self.entry(name).is_some() && !self.accepted.contains_key(name)
    }

    /// Whether the one-file store `name`, opened holding `records`, stands
    /// at or past its receipt; one short and not accepted is recorded lost
    /// and left out, answering `false`.
    pub fn journal_head(&mut self, name: &str, records: usize, say: &Say) -> bool {
        let Some(entry) = self.entry(name).cloned() else {
            return true;
        };
        let records = u64::try_from(records).unwrap_or(u64::MAX);
        if records >= entry.head || self.accepted.contains_key(name) {
            return true;
        }
        if let Some(accepted) = receipt::accepted_since(&self.acceptances, name, &entry.read_at) {
            say(&format!(
                "{name}: the loss accepted at {} is served at what was found ({records} records)",
                accepted.at
            ));
            self.accepted.insert(name.to_owned(), accepted.clone());
            return true;
        }
        let loss = Loss {
            store: name.to_owned(),
            refusal: "store_lost",
            held: format!("{} records", entry.head),
            found: format!("{records} records"),
        };
        say(&format!(
            "{name} store_lost: the receipt holds {}; found {}; its routes refuse until the loss is accepted",
            loss.held, loss.found
        ));
        if self.held.computer.is_empty() {
            self.held.computer = computer();
        }
        self.held.lost.insert(name.to_owned(), loss);
        false
    }

    /// The one-file stores as opened, each one short of its receipt and not
    /// accepted recorded lost and left out.
    pub fn file_stores(
        &mut self,
        (network, roles, provisioning): crate::file_stores::FileStores,
        say: &Say,
    ) -> crate::file_stores::FileStores {
        let network =
            network.filter(|store| self.journal_head("network", store.machines().len(), say));
        let roles = roles.filter(|store| self.journal_head("roles", store.roles().len(), say));
        let provisioning = provisioning
            .filter(|store| self.journal_head("provisioning", store.profiles().len(), say));
        (network, roles, provisioning)
    }

    /// The losses the service holds while it serves.
    #[must_use]
    pub fn held(&self) -> Held {
        let mut held = self.held.clone();
        held.projection = self.entry("permission-projection").cloned();
        held
    }
}

/// The grant log's identity found changed or absent where the receipt holds
/// one.
fn identity_lost(place: &Place, entry: &Entry, found: &receipt::Found) -> Option<Verdict> {
    let held = entry.identity.as_ref()?;
    if found.identity.is_some() {
        return None;
    }
    Some(Verdict::Lost(Loss {
        store: place.named.name.to_owned(),
        refusal: "grant_log_identity_changed",
        held: format!("identity {} at epoch {}", held.identity, held.epoch),
        found: "no identity recorded beside the grant log".to_owned(),
    }))
}

#[path = "stores_receipt/writing.rs"]
mod writing;
pub use writing::{AtStop, Started};
pub(crate) use writing::{after_boot, projection};
