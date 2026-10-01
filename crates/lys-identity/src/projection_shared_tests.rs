//! Request snapshots share every index and retain their prior records after a write.

use std::error::Error;
use std::sync::Arc;

use super::Projection;
use crate::{
    Actor, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance,
};

#[test]
fn request_snapshots_share_every_directory_index_and_isolate_later_writes()
-> Result<(), Box<dyn Error>> {
    let mut directory = Projection::new();
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let first = IdentityId::Person(PersonId::from_bytes([1; 16]));
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([1; 16]),
            actor.clone(),
            first,
            1,
            Change::SetupPerson {
                profile: Profile::new("owner")?,
            },
        )?,
        0,
    )?;
    let view = directory.shared();
    assert!(Arc::ptr_eq(&directory.records, &view.records));
    assert!(Arc::ptr_eq(&directory.people, &view.people));
    assert!(Arc::ptr_eq(
        &directory.agents_by_person,
        &view.agents_by_person
    ));
    assert!(Arc::ptr_eq(
        &directory.reporting_children,
        &view.reporting_children
    ));
    assert!(Arc::ptr_eq(&directory.bindings, &view.bindings));
    assert!(Arc::ptr_eq(&directory.agent_bindings, &view.agent_bindings));
    assert!(Arc::ptr_eq(&directory.operations, &view.operations));
    assert!(Arc::ptr_eq(&directory.link_sources, &view.link_sources));
    assert!(Arc::ptr_eq(&directory.accounts, &view.accounts));
    let second = IdentityId::Person(PersonId::from_bytes([2; 16]));
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([2; 16]),
            actor,
            second,
            2,
            Change::RegisterPerson {
                profile: Profile::new("other")?,
            },
        )?,
        1,
    )?;
    assert!(directory.record(second).is_some());
    assert!(view.record(second).is_none());
    assert_eq!(view.records().count(), 1);
    assert_eq!(view.operation(OperationId::from_bytes([2; 16])), None);
    Ok(())
}
