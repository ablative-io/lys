//! Typed grant records change only when account authority changes.

use std::sync::Arc;

use lys_identity::projection::accounts::Accounts;
use lys_identity::{IdentityError, LoginBinding, PersonId, Profile, ServiceAccountId};

use super::Created;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct GrantIndex {
    accounts: Arc<Accounts>,
    error: Option<IdentityError>,
}

impl GrantIndex {
    pub(super) fn hold(&mut self, created: &Created, retired: bool) {
        #[cfg(test)]
        crate::service_account_grants::tests::visited();
        if self.error.is_some() {
            return;
        }
        let checked = (|| -> Result<_, IdentityError> {
            Ok((
                created.id.parse::<ServiceAccountId>()?,
                created.owner.parse::<PersonId>()?,
                Profile::new(&created.name)?,
                LoginBinding::new(&created.by.provider, &created.by.subject)?,
            ))
        })();
        match checked {
            Ok((id, owner, profile, by)) => {
                Arc::make_mut(&mut self.accounts).put(id, owner, &profile, retired, &by);
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn accounts(&self) -> Result<Arc<Accounts>, IdentityError> {
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(Arc::clone(&self.accounts)),
        }
    }
}
