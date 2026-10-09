//! Account records are prepared at fold time and selected by indexed owner state.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Projection, Record};
use crate::{
    ConnectorId, IdentityId, LifecycleState, LoginBinding, MachineId, PersonId, Profile,
    ServiceAccountId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Account {
    owner: PersonId,
    retired: bool,
    states: [Record; 4],
}

/// The separately folded service-account, connector and machine records used
/// for grant decisions.
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
        self.insert(
            IdentityId::ServiceAccount(id),
            owner,
            profile,
            retired,
            created_by,
        );
    }

    /// Insert or update an approved app's connector, read from the apps log,
    /// answering to `approver`, the administrator who approved the app. The
    /// connector is active while its approver is and stops when they do; a
    /// connector whose app is `retired` is retired whatever its approver's state.
    pub fn put_connector(
        &mut self,
        id: ConnectorId,
        approver: PersonId,
        profile: &Profile,
        retired: bool,
        approved_by: &LoginBinding,
    ) {
        self.insert(
            IdentityId::Connector(id),
            approver,
            profile,
            retired,
            approved_by,
        );
    }

    /// Insert or update a machine, read from the connection codes, answering
    /// to `issuer`, the administrator who asked for the code it joined with
    /// (ACCESS-005 R1). The machine is active while its issuer is and stops
    /// when they do; a machine `retired`, because a later join of the same
    /// computer replaced it or the computer was retired, is retired whatever
    /// its issuer's state. `issued_by` is the issuer's login.
    pub fn put_machine(
        &mut self,
        id: MachineId,
        issuer: PersonId,
        profile: &Profile,
        retired: bool,
        issued_by: &LoginBinding,
    ) {
        self.insert(IdentityId::Machine(id), issuer, profile, retired, issued_by);
    }

    fn insert(
        &mut self,
        identity: IdentityId,
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
            identity,
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
