#![cfg(test)]

use std::cell::Cell;
use std::error::Error;

use lys_identity::event::{Change, IdentityEvent};
use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, PersonId, Profile,
    Provenance, ServiceAccountId,
};

use crate::read_views::Login;
use crate::service_accounts_state::{Created, Held, Line};

thread_local! {
    static COPIES: Cell<usize> = const { Cell::new(0) };
    static VISITS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn copied(base: &Projection, view: &Projection) {
    COPIES.set(
        base.records()
            .filter(|(id, record)| {
                view.record(**id)
                    .is_none_or(|other| !std::ptr::eq(*record, other))
            })
            .count(),
    );
}

pub(super) fn visited() {
    VISITS.set(VISITS.get() + 1);
}

fn directory() -> Result<(Projection, PersonId), Box<dyn Error>> {
    let mut projection = Projection::new();
    let actor = Actor::new(
        LoginBinding::new("issuer", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    for index in 0..256u128 {
        let person = PersonId::from_bytes((index + 1).to_be_bytes());
        let change = if index == 0 {
            Change::SetupPerson {
                profile: Profile::new("owner")?,
            }
        } else {
            Change::RegisterPerson {
                profile: Profile::new("person")?,
            }
        };
        projection.apply(
            &IdentityEvent::new(
                OperationId::from_bytes((index + 1).to_be_bytes()),
                actor.clone(),
                IdentityId::Person(person),
                1,
                change,
            )?,
            u64::try_from(index)?,
        )?;
    }
    let first = PersonId::from_bytes(1u128.to_be_bytes());
    Ok((projection, first))
}

#[test]
fn a_grant_request_shares_directory_records_and_does_not_walk_unrelated_accounts()
-> Result<(), Box<dyn Error>> {
    let (directory, owner) = directory()?;
    let mut held = Held::default();
    for index in 0..1024u128 {
        held.hold(Line::Created(Created {
            id: ServiceAccountId::from_bytes(index.to_be_bytes()).to_string(),
            owner: owner.to_string(),
            name: "service".to_owned(),
            description: String::new(),
            by: Login {
                provider: "issuer".to_owned(),
                subject: "owner".to_owned(),
            },
            at: 1,
        }))?;
    }
    COPIES.set(0);
    VISITS.set(0);
    let view = super::expanded(&directory, &held)?;
    let record = view
        .record(IdentityId::ServiceAccount(ServiceAccountId::from_bytes(
            1023u128.to_be_bytes(),
        )))
        .ok_or("selected account missing")?;
    assert_eq!(record.responsible(), Some(owner));
    assert_eq!(record.state(), LifecycleState::Active);
    assert_eq!(
        (COPIES.get(), VISITS.get()),
        (0, 0),
        "copied directory records and visited unrelated service accounts"
    );
    Ok(())
}
