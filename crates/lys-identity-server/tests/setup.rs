//! First-run admission, a single durable setup event, replay and failure recovery.
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Fault, Harness, Service};
use lys_identity::{
    Actor, AuthMethod, IdentityError, IdentityId, LifecycleState, LoginBinding, OperationId,
    Profile, Provenance, Transition,
};
use serde_json::json;

type Result = std::result::Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "same@example.test".to_owned(),
    }
}
fn actor(at: u64) -> std::result::Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, at),
    ))
}

#[tokio::test]
async fn one_click_sets_up_only_the_verified_administrator_and_opens_personal_views() -> Result {
    let service = Service::start().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, before) = service.get("/me", Some(&admin)).await?;
    assert_eq!(status, 403);
    assert_eq!(before["refusal"], "SetupRequired");
    let body = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Tom"});
    let (status, answer) = service.post("/setup", Some(&admin), &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["receipt"]["change_kind"], 7);
    assert_eq!(answer["receipt"]["log"]["tree_size"], 1);
    assert_eq!(answer["receipt"]["actor"]["subject"], ADMINISTRATOR);
    assert_eq!(answer["person"], answer["receipt"]["identity"]);
    for route in ["/me", "/people", "/grants"] {
        let (status, read) = service.get(route, Some(&admin)).await?;
        assert_eq!(status, 200, "{route}: {read}");
        if route == "/me" {
            assert_eq!(read["person"]["state"], "active");
        }
    }
    let (status, repeated) = service.post("/setup", Some(&admin), &body).await?;
    assert_eq!(status, 200);
    assert_eq!(answer, repeated);
    let fresh =
        json!({"operation": OperationId::generate()?.to_string(), "display_name": "Someone else"});
    let (status, refused) = service.post("/setup", Some(&admin), &fresh).await?;
    assert_eq!(status, 409);
    assert_eq!(refused["refusal"], "AlreadyBootstrapped");
    Ok(())
}

#[tokio::test]
async fn visitors_and_caller_supplied_identity_claims_cannot_bootstrap() -> Result {
    let service = Service::start().await?;
    let other = service.sign_in(login("other-person")).await?;
    let body = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Other"});
    let (status, refused) = service.post("/setup", Some(&other), &body).await?;
    assert_eq!(status, 403);
    assert_eq!(refused["refusal"], "NotAdmitted");
    let (status, _) = service.post("/setup", None, &body).await?;
    assert_eq!(status, 401);
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    for field in ["issuer", "subject", "person", "state"] {
        let mut forged = body.clone();
        forged[field] = json!("attacker");
        let (status, _) = service.post("/setup", Some(&admin), &forged).await?;
        assert_eq!(status, 422, "{field}");
    }
    let (_, people) = service.get("/directory/people", Some(&admin)).await?;
    assert_eq!(people["people"], json!([]));
    Ok(())
}

#[test]
fn setup_recovers_every_append_outcome_and_reopens_as_one_active_bound_person() -> Result {
    let faults = [
        Fault::None,
        Fault::BeforeLeaf,
        Fault::LeafStoredWriteFailed,
        Fault::AfterLeaf,
        Fault::AfterLeafUnreadable,
    ];
    let mut legs = 0;
    for fault in faults {
        let harness = Harness::new(7)?;
        let mut directory = harness.open()?;
        let op = OperationId::generate()?;
        harness.fail(fault);
        let answer = directory.setup_person(actor(1)?, op, Profile::new("Tom")?, 2);
        if matches!(fault, Fault::BeforeLeaf | Fault::AfterLeafUnreadable) {
            assert!(answer.is_err());
        } else {
            assert!(answer.is_ok());
        }
        harness.fail(Fault::None);
        drop(directory);
        let mut directory = harness.open()?;
        let (person, receipt) = directory.setup_person(actor(3)?, op, Profile::new("Tom")?, 4)?;
        assert_eq!(receipt.coordinate().index, 0);
        let projection = directory.projection()?;
        assert_eq!(projection.records().count(), 1);
        assert_eq!(projection.person_for(actor(3)?.binding()), Some(person));
        let record = projection
            .record(IdentityId::Person(person))
            .ok_or("missing setup person")?;
        assert_eq!(record.state(), LifecycleState::Active);
        assert_eq!(record.events(), &[0]);
        directory.transition(
            actor(3)?,
            OperationId::generate()?,
            IdentityId::Person(person),
            Transition::Suspend,
            "test suspension",
            5,
        )?;
        let (_, retried) = directory.setup_person(actor(6)?, op, Profile::new("Tom")?, 7)?;
        assert_eq!(retried, receipt);
        assert_eq!(
            directory
                .record(IdentityId::Person(person))?
                .ok_or("missing person")?
                .state(),
            LifecycleState::Suspended
        );
        legs += 1;
    }
    assert_eq!(legs, 5);
    Ok(())
}
