//! Account records are prepared at fold time and selected by indexed owner state.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Projection, Record};
use crate::{IdentityId, LifecycleState, LoginBinding, PersonId, Profile, ServiceAccountId};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Account {
    owner: PersonId,
    retired: bool,
    states: [Record; 4],
}

/// The separately folded service-account records used for grant decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Accounts {
    records: BTreeMap<IdentityId, Arc<Account>>,
}

impl Accounts {
    /// Insert or update a checked account without reading any directory history.
    pub fn put(
        &mut self,
        id: ServiceAccountId,
        owner: PersonId,
        profile: &Profile,
        retired: bool,
        created_by: &LoginBinding,
    ) {
        let states = [
            LifecycleState::Registered,
            LifecycleState::Active,
            LifecycleState::Suspended,
            LifecycleState::Retired,
        ]
        .map(|state| Record {
            profile: profile.clone(),
            state,
            responsible: Some(owner),
            reports_to: None,
            reporting_gap: None,
            bindings: Vec::new(),
            registered_by: created_by.clone(),
            events: Vec::new(),
        });
        self.records.insert(
            IdentityId::ServiceAccount(id),
            Arc::new(Account {
                owner,
                retired,
                states,
            }),
        );
    }

    pub(super) fn record(&self, id: IdentityId, directory: &Projection) -> Option<&Record> {
        let account = self.records.get(&id)?;
        let owner = directory.records.get(&IdentityId::Person(account.owner))?;
        let state = if account.retired {
            LifecycleState::Retired
        } else {
            owner.state()
        };
        let index = match state {
            LifecycleState::Registered => 0,
            LifecycleState::Active => 1,
            LifecycleState::Suspended => 2,
            LifecycleState::Retired => 3,
        };
        account.states.get(index)
    }

    pub(super) fn records<'a>(
        &'a self,
        directory: &'a Projection,
    ) -> impl Iterator<Item = (&'a IdentityId, &'a Record)> {
        self.records
            .keys()
            .filter_map(move |id| self.record(*id, directory).map(|record| (id, record)))
    }
}
