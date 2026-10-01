//! Administrator and link-audit authority require a live directory identity.

use std::error::Error;

use identity_contract::harness::Harness;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::admission::Admission;

type TestResult = Result<(), Box<dyn Error>>;

fn actor() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.example.test", "authority")?,
        Provenance::new(AuthMethod::Oidc, 1),
    ))
}

#[test]
fn an_unbound_configured_login_keeps_administrator_authority() -> TestResult {
    let actor = actor()?;
    let admission = Admission::new(Some(actor.binding().clone()), actor.binding().clone());
    assert!(
        admission
            .administrator(&lys_identity::projection::Projection::default(), &actor)
            .is_ok()
    );
    Ok(())
}

#[test]
fn a_suspended_administrator_has_no_administrator_authority() -> TestResult {
    let harness = Harness::new(29)?;
    let mut directory = harness.open()?;
    let actor = actor()?;
    let admission = Admission::new(Some(actor.binding().clone()), actor.binding().clone());
    let (person, _) = directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Authority")?,
        1,
    )?;
    admission.administrator(directory.projection()?, &actor)?;
    directory.transition(
        actor.clone(),
        OperationId::generate()?,
        IdentityId::Person(person),
        Transition::Suspend,
        "authority removed",
        2,
    )?;
    let error = admission
        .administrator(directory.projection()?, &actor)
        .err()
        .ok_or("suspended administrator admitted")?;
    assert_eq!(error.name(), "inactive");
    Ok(())
}

#[test]
fn an_unbound_configured_link_audit_login_keeps_its_authority() -> TestResult {
    let harness = Harness::new(30)?;
    let mut directory = harness.open()?;
    let actor = actor()?;
    let admission = Admission::new(None, actor.binding().clone());
    admission.link_audit_source(&actor)?;
    admission.link_audit_holder(directory.projection()?)?;
    Ok(())
}

#[test]
fn a_person_less_human_login_is_not_the_link_audit_source() -> TestResult {
    let harness = Harness::new(33)?;
    let mut directory = harness.open()?;
    let admission = Admission::new(None, actor()?.binding().clone());
    let human = Actor::new(
        LoginBinding::new("https://issuer.example.test", "someone-without-a-person")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    assert!(
        directory
            .projection()?
            .person_for(human.binding())
            .is_none()
    );
    let error = admission
        .link_audit_source(&human)
        .err()
        .ok_or("a person-less human was admitted as the link-audit source")?;
    assert_eq!(error.name(), "NotAdmitted");
    Ok(())
}

#[test]
fn a_suspended_link_audit_agent_cannot_borrow_its_active_persons_authority() -> TestResult {
    let harness = Harness::new(31)?;
    let mut directory = harness.open()?;
    let actor = actor()?;
    let admission = Admission::new(None, actor.binding().clone());
    let (person, _) = directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Authority")?,
        1,
    )?;
    let (agent, _) = directory.register_agent(
        actor.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Agent")?,
        2,
    )?;
    directory.transition(
        actor.clone(),
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        3,
    )?;
    admission.link_audit_agent(directory.projection()?, agent)?;
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Suspend,
        "authority removed",
        4,
    )?;
    assert!(
        admission
            .link_audit_agent(directory.projection()?, agent)
            .is_err()
    );
    Ok(())
}

#[test]
fn an_active_bound_administrator_is_admitted() -> TestResult {
    let harness = Harness::new(32)?;
    let mut directory = harness.open()?;
    let actor = actor()?;
    let admission = Admission::new(Some(actor.binding().clone()), actor.binding().clone());
    directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Authority")?,
        1,
    )?;
    assert!(admission.is_administrator(directory.projection()?, &actor)?);
    Ok(())
}
