//! A secret added, its value replaced and the secret retired, each by its
//! owner's word under an operation id the owner made once for that change,
//! as scope and recipients are (see `owner`): a resend of the same change
//! answers the outcome recorded the first time and applies nothing, and the
//! same id sent with another change is refused.
//!
//! The value is never part of what a change is recorded by: the digest an
//! operation id is held to covers the secret's name and what the change
//! says of it (its class and route, or that it is replaced or retired),
//! never a byte of the value, so no audit line carries anything a value
//! could be guessed from. A retired name is never sealed again.

use crate::error::SecretsError;
use crate::handle::HandleId;
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::Broker;
use super::owner::{ADDED, Admission, OwnerChanged, REPLACED, RETIRED};
use super::scope::with_via;

/// The class a person may add a value under, as its outcome names it.
fn addable(class: EntryClass) -> Result<&'static str, SecretsError> {
    match class {
        EntryClass::Credential => Ok("credential"),
        EntryClass::Key => Ok("key"),
        EntryClass::Memory | EntryClass::OAuth => Err(SecretsError::InvalidName {
            what: "class",
            name: format!("{class:?}").to_lowercase(),
            reason: "is not a class a value is added or replaced under here: a credential or a key is",
        }),
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// Seals `value` as the new secret `name` of `class`, owned by `owner`,
    /// under `operation`, asked through the screen service `via`. `route`
    /// is what the caller binds the secret to, in words, so the same id
    /// sent with another route is refused.
    ///
    /// # Errors
    ///
    /// `OperationMissing`, `OperationReused`, `InvalidName` for a class a
    /// person does not add, `SecretRetired`, `SecretExists`, and the
    /// store's and the audit log's refusals.
    pub fn add_secret(
        &mut self,
        owner: &str,
        (name, class, value): (&str, EntryClass, &Secret),
        route: &str,
        (via, operation): (Option<&str>, Option<&str>),
    ) -> Result<OwnerChanged, SecretsError> {
        let label = addable(class)?;
        let change = format!("{ADDED}{label} {route}");
        let call = match self.owner_admission(name, operation, &change)? {
            Admission::Repeated(outcome) => return Ok(OwnerChanged::Repeated { outcome }),
            Admission::Fresh(call) => call,
        };
        self.store.add(&self.store_key, name, class, owner, value)?;
        let outcome = with_via(format!("{ADDED}{label}"), via);
        self.record_owner_change(owner, name, &outcome, Some(&call))?;
        Ok(OwnerChanged::Applied)
    }

    /// Replaces the value of `name`, as its owner, under `operation`.
    /// Answers the change and the sequence the secret now stands at.
    ///
    /// # Errors
    ///
    /// `OperationMissing`, `OperationReused`, `LendingNotPermitted` when
    /// `owner` does not own it, `InvalidName` for a class a value is not
    /// replaced under here, and the store's and the audit log's refusals.
    pub fn replace_secret(
        &mut self,
        owner: &str,
        (name, value): (&str, &Secret),
        (via, operation): (Option<&str>, Option<&str>),
    ) -> Result<(OwnerChanged, u64), SecretsError> {
        let change = format!("{REPLACED}value");
        let call = match self.owner_admission(name, operation, &change)? {
            Admission::Repeated(outcome) => {
                // Only its owner reads where the secret stands, also when the
                // operation is one already answered.
                self.owns(owner, name)?;
                let sequence = self.store.entry(name).map_or(0, |entry| entry.sequence);
                return Ok((OwnerChanged::Repeated { outcome }, sequence));
            }
            Admission::Fresh(call) => call,
        };
        self.owns(owner, name)?;
        let class = self
            .store
            .entry(name)
            .map(|entry| entry.class)
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: name.to_owned(),
            })?;
        addable(class)?;
        let view = self.store.replace(&self.store_key, name, value)?;
        let outcome = with_via(format!("{REPLACED}{}", view.sequence), via);
        self.record_owner_change(owner, name, &outcome, Some(&call))?;
        Ok((OwnerChanged::Applied, view.sequence))
    }

    /// Retires `name`, as its owner, under `operation`: every handle still
    /// live on it is dropped first, each with its own audit line, then its
    /// sealings, accounts and settings leave the store and its name is kept
    /// as retired. Answers the change and when the name was retired.
    ///
    /// # Errors
    ///
    /// `OperationMissing`, `OperationReused`, `LendingNotPermitted` when
    /// `owner` does not own it, and the store's and the audit log's
    /// refusals.
    pub fn retire_secret(
        &mut self,
        owner: &str,
        name: &str,
        (via, operation): (Option<&str>, Option<&str>),
    ) -> Result<(OwnerChanged, i64), SecretsError> {
        let change = format!("{RETIRED}{name}");
        let call = match self.owner_admission(name, operation, &change)? {
            Admission::Repeated(outcome) => {
                // Only the one who retired it reads when; anyone else is
                // answered as the owner check answers.
                let at = match self.store.retired(name) {
                    Some(retired) if retired.by == owner => retired.at_ms,
                    _ => {
                        self.owns(owner, name)?;
                        0
                    }
                };
                return Ok((OwnerChanged::Repeated { outcome }, at));
            }
            Admission::Fresh(call) => call,
        };
        if self.store.retired(name).is_some() {
            return Err(SecretsError::SecretRetired {
                name: name.to_owned(),
            });
        }
        self.owns(owner, name)?;
        let live: Vec<String> = self
            .handles
            .values()
            .filter(|record| record.secret == name && !record.dropped)
            .map(|record| record.id.clone())
            .collect();
        for id in live {
            self.drop_handle(&HandleId::from_text(&id))?;
        }
        let at = (self.clock)();
        self.store.retire(name, owner, at)?;
        let outcome = with_via(format!("{RETIRED}{name}"), via);
        self.record_owner_change(owner, name, &outcome, Some(&call))?;
        Ok((OwnerChanged::Applied, at))
    }
}
