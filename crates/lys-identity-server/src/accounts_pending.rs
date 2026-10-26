//! Pending mailbox proofs are published before mail and withdrawn by the
//! request's drop. A newer generation can never be withdrawn by an older one.

use crate::error::ServerError;
use crate::sign_in_providers::SignInProviders;
use super::PendingEmail;

pub(super) fn changes(api: &SignInProviders) -> Result<std::sync::MutexGuard<'_, std::collections::BTreeMap<String, PendingEmail>>, ServerError> {
    api.email_changes.lock().map_err(|error| ServerError::SignInProvidersUnavailable { reason:error.to_string() })
}

pub(super) struct Sending<'a> {
    api: &'a SignInProviders,
    id: &'a str,
    digest: String,
    completed: bool,
}

impl<'a> Sending<'a> {
    pub(super) fn new(api: &'a SignInProviders, id: &'a str, pending: PendingEmail) -> Result<Self, ServerError> {
        let digest = pending.digest.clone();
        changes(api)?.insert(id.to_owned(), pending);
        Ok(Self { api, id, digest, completed:false })
    }
    pub(super) fn complete(mut self) -> Result<(), ServerError> {
        let mut held = changes(self.api)?;
        if let Some(pending) = held.get_mut(self.id) && pending.digest == self.digest {
            pending.sent = true;
        }
        self.completed = true;
        Ok(())
    }
}

impl Drop for Sending<'_> {
    fn drop(&mut self) {
        if self.completed { return; }
        match changes(self.api) {
            Ok(mut held) => {
                if held.get(self.id).is_some_and(|pending| pending.digest == self.digest) { held.remove(self.id); }
            }
            Err(error) => tracing::error!("email_change_withdrawal_failed: {error}"),
        }
    }
}
