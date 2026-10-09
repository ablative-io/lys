#![cfg(test)]

use std::cell::Cell;
use std::error::Error;

use lys_identity::event::{Change, IdentityEvent};
use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, PersonId, Profile,
    Provenance, ServiceAccountId, Transition,
};

use crate::read_views::Login;
use crate::service_accounts_state::{Created, Held, Line, Retired};

/// The projection over `held` beside apps that hold no connector.
fn expanded(directory: &Projection, held: &Held) -> Result<Projection, crate::ServerError> {
    super::expanded(directory, held, &crate::apps_state::Held::default(), &[])
}

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

pub(crate) fn visited() {
    VISITS.set(VISITS.get() + 1);
}

fn account(owner: PersonId) -> Created {
    Created {
        id: ServiceAccountId::from_bytes(1u128.to_be_bytes()).to_string(),
        owner: owner.to_string(),
        name: "service".to_owned(),
        description: String::new(),
        by: Login {
            provider: "https://issuer.test".to_owned(),
            subject: "owner".to_owned(),
        },
        at: 1,
    }
}

fn change_owner(
    directory: &mut Projection,
    owner: PersonId,
    transition: Transition,
    index: u64,
) -> Result<(), Box<dyn Error>> {
    let identity = IdentityId::Person(owner);
    let from = directory.record(identity).ok_or("owner missing")?.state();
    let to = match transition {
        Transition::Suspend => LifecycleState::Suspended,
        Transition::Reinstate | Transition::Activate => LifecycleState::Active,
        Transition::Retire => LifecycleState::Retired,
    };
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes(u128::from(index).to_be_bytes()),
            Actor::new(
                LoginBinding::new("https://issuer.test", "owner")?,
                Provenance::new(AuthMethod::Oidc, index),
            ),
            identity,
            index,
            Change::Transition {
                transition,
                from,
                to,
                reason: "owner lifecycle".to_owned(),
            },
        )?,
        index,
    )?;
    Ok(())
}

#[test]
fn indexed_accounts_follow_owner_lifecycle_without_rebuilding_accounts()
-> Result<(), Box<dyn Error>> {
    let (mut directory, owner) = directory()?;
    let mut held = Held::default();
    let created = account(owner);
    let identity = IdentityId::ServiceAccount(created.id.parse()?);
    held.hold(Line::Created(created))?;
    let active = expanded(&directory, &held)?;
    VISITS.set(0);
    for (index, transition, state) in [
        (300, Transition::Suspend, LifecycleState::Suspended),
        (301, Transition::Reinstate, LifecycleState::Active),
        (302, Transition::Retire, LifecycleState::Retired),
    ] {
        change_owner(&mut directory, owner, transition, index)?;
        let view = expanded(&directory, &held)?;
        let record = view.record(identity).ok_or("account missing")?;
        assert_eq!(record.state(), state);
        assert_eq!(record.responsible(), Some(owner));
        assert_eq!(
            active
                .record(identity)
                .ok_or("snapshot account missing")?
                .state(),
            LifecycleState::Active
        );
    }
    assert_eq!(VISITS.get(), 0, "owner transitions rebuilt account records");
    Ok(())
}

#[test]
fn old_account_snapshots_rebuild_the_index_and_retirement_keeps_prior_views()
-> Result<(), Box<dyn Error>> {
    let (directory, owner) = directory()?;
    let created = account(owner);
    let identity = IdentityId::ServiceAccount(created.id.parse()?);
    let old = serde_json::to_vec(&serde_json::json!({
        "format": "lys-service-accounts-state/v1",
        "held": { "accounts": [{ "created": created, "retired": null }] }
    }))?;
    let mut held = Held::decode(&old)?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&held.encode()?)?,
        serde_json::from_slice::<serde_json::Value>(&old)?
    );
    let active = expanded(&directory, &held)?;
    held.hold(Line::Retired(Retired {
        operation: OperationId::from_bytes(2u128.to_be_bytes()).to_string(),
        account: created.id,
        by: created.by,
        at: 2,
    }))?;
    let reopened = Held::decode(&held.encode()?)?;
    let retired = expanded(&directory, &reopened)?;
    assert_eq!(
        active
            .record(identity)
            .ok_or("active account missing")?
            .state(),
        LifecycleState::Active
    );
    assert_eq!(
        retired
            .record(identity)
            .ok_or("retired account missing")?
            .state(),
        LifecycleState::Retired
    );
    Ok(())
}

#[test]
fn malformed_stored_account_provenance_refuses_the_grant_projection_by_name()
-> Result<(), Box<dyn Error>> {
    let (directory, owner) = directory()?;
    let mut created = account(owner);
    created.by.provider = "invalid-provider".to_owned();
    let mut held = Held::default();
    held.hold(Line::Created(created))?;
    assert!(matches!(
        expanded(&directory, &held),
        Err(crate::error::ServerError::Identity(
            lys_identity::IdentityError::BindingMalformed { .. }
        ))
    ));
    Ok(())
}

fn directory() -> Result<(Projection, PersonId), Box<dyn Error>> {
    let mut projection = Projection::new();
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
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
                provider: "https://issuer.test".to_owned(),
                subject: "owner".to_owned(),
            },
            at: 1,
        }))?;
    }
    COPIES.set(0);
    VISITS.set(0);
    let view = expanded(&directory, &held)?;
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

/// The apps holding the app `notes`, approved with its connector answering
/// to `approver`, and retired when `retired`; with the connector's identity.
fn connected(
    approver: PersonId,
    retired: bool,
) -> Result<(crate::apps_state::Held, IdentityId), Box<dyn Error>> {
    use crate::apps_state::{Approved, By, Client, Connected, Decided, Line, Registered};
    let by = By::Operator {
        login: Login {
            provider: "https://issuer.test".to_owned(),
            subject: "owner".to_owned(),
        },
    };
    let op = |index: u128| OperationId::from_bytes(index.to_be_bytes()).to_string();
    let connector = lys_identity::ConnectorId::from_bytes(7u128.to_be_bytes());
    let mut lines = vec![
        Line::Registered(Registered {
            operation: op(1),
            app: "notes".to_owned(),
            name: "Notes".to_owned(),
            redirects: Vec::new(),
            schema: serde_json::json!({"kinds": {"notes_page": {"relations": {"reader": ["read"]}}}}),
            service_account: None,
            by: by.clone(),
            at: 1,
        }),
        Line::Approved(Approved {
            operation: op(2),
            app: "notes".to_owned(),
            client: Client {
                client_id: "notes".to_owned(),
                secret_sha256: "cd".repeat(32),
            },
            binding: None,
            by: by.clone(),
            at: 2,
        }),
        Line::Connector(Connected {
            operation: op(2),
            app: "notes".to_owned(),
            connector: connector.to_string(),
            approver: approver.to_string(),
            by: by.clone(),
            at: 2,
        }),
    ];
    if retired {
        lines.push(Line::Retired(Decided {
            operation: op(3),
            app: "notes".to_owned(),
            reason: "done".to_owned(),
            by,
            at: 3,
        }));
    }
    let mut apps = crate::apps_state::Held::default();
    for line in lines {
        apps.hold(line)?;
    }
    Ok((apps, IdentityId::Connector(connector)))
}

/// DIRECTORY-080 R1 (box 12.4): the engine judges an app's connector as an
/// account beside the service accounts, retired with its app whatever its
/// approver's state; admission refuses anything but Active by its state.
#[test]
fn a_retired_apps_connector_is_judged_retired() -> Result<(), Box<dyn Error>> {
    let (directory, owner) = directory()?;
    let (apps, identity) = connected(owner, true)?;
    let view = super::expanded(&directory, &Held::default(), &apps, &[])?;
    let record = view.record(identity).ok_or("connector missing")?;
    assert_eq!(record.state(), LifecycleState::Retired);
    assert_eq!(record.responsible(), Some(owner));
    let (apps, identity) = connected(owner, false)?;
    let view = super::expanded(&directory, &Held::default(), &apps, &[])?;
    let record = view.record(identity).ok_or("connector missing")?;
    assert_eq!(record.state(), LifecycleState::Active);
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.4): a connector stops when its approver is
/// suspended and acts again once they are reinstated, with nothing held for
/// it but the apps' line.
#[test]
fn a_connector_follows_its_approver_through_suspension_and_reinstatement()
-> Result<(), Box<dyn Error>> {
    let (mut directory, owner) = directory()?;
    let (apps, identity) = connected(owner, false)?;
    for (index, transition, state) in [
        (300, Transition::Suspend, LifecycleState::Suspended),
        (301, Transition::Reinstate, LifecycleState::Active),
    ] {
        change_owner(&mut directory, owner, transition, index)?;
        let view = super::expanded(&directory, &Held::default(), &apps, &[])?;
        let record = view.record(identity).ok_or("connector missing")?;
        assert_eq!(record.state(), state, "{transition:?}");
        assert_eq!(record.responsible(), Some(owner));
    }
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.4): with no service accounts configured, the
/// projection still holds every app's connector.
#[test]
fn connectors_are_judged_without_service_accounts() -> Result<(), Box<dyn Error>> {
    let (directory, owner) = directory()?;
    let (apps, identity) = connected(owner, false)?;
    let connectors = crate::apps_connector::with_connectors(std::sync::Arc::default(), &apps)?;
    let view = directory.with_accounts(connectors);
    assert_eq!(
        view.record(identity).ok_or("connector missing")?.state(),
        LifecycleState::Active
    );
    Ok(())
}
