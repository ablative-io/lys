//! Registered lifecycle regression coverage.

use std::error::Error;

use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::caller_admission::active_caller;
use lys_identity_server::routes::open_directory;

type TestResult = Result<(), Box<dyn Error>>;

async fn agent_pair(person_active: bool, agent_active: bool) -> TestResult {
    Service::start_with(|config| {
        let mut directory = open_directory(config)?;
        let binding = LoginBinding::new(&config.issuer, "registered-responsible-person")?;
        let admin = Actor::new(
            LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        let (person, _) = directory.register_person(
            admin.clone(),
            OperationId::generate()?,
            Profile::new("Responsible person")?,
            1,
        )?;
        directory.bind_login(
            admin.clone(),
            OperationId::generate()?,
            person,
            binding.clone(),
            2,
        )?;
        let (agent, _) = directory.register_agent(
            admin.clone(),
            OperationId::generate()?,
            person,
            Profile::new("Agent")?,
            3,
        )?;
        for (id, active) in [
            (IdentityId::Person(person), person_active),
            (IdentityId::Agent(agent), agent_active),
        ] {
            if active {
                directory.transition(
                    admin.clone(),
                    OperationId::generate()?,
                    id,
                    Transition::Activate,
                    "fixture active control",
                    4,
                )?;
            }
        }
        let actor = Actor::new(binding, Provenance::by_agent(agent, 5));
        let answer = active_caller(directory.projection()?, &actor);
        if person_active && agent_active {
            assert_eq!(answer?, IdentityId::Agent(agent));
        } else {
            let error = answer
                .err()
                .ok_or("Registered agent or responsible person admitted")?;
            let refused = if agent_active {
                IdentityId::Person(person)
            } else {
                IdentityId::Agent(agent)
            };
            assert_eq!(error.name(), "inactive");
            assert!(error.to_string().contains(&refused.to_string()));
            assert!(error.to_string().contains("registered"));
        }
        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn registered_agent_is_not_an_active_caller() -> TestResult {
    agent_pair(true, false).await
}

#[tokio::test]
async fn active_agent_of_registered_person_is_not_an_active_caller() -> TestResult {
    agent_pair(false, true).await
}

#[tokio::test]
async fn active_agent_of_active_person_remains_admitted() -> TestResult {
    agent_pair(true, true).await
}
