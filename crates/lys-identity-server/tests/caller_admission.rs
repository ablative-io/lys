#![cfg(test)]
//! Lifecycle action admission requires Active and names refused states.

use std::error::Error;

use identity_contract::harness::Service;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::caller_admission::active_caller;
use lys_identity_server::routes::open_directory;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn a_person_is_admitted_only_when_active_and_other_states_are_named() -> TestResult {
    Service::start_with(|config| {
        let mut directory = open_directory(config)?;
        let binding = LoginBinding::new(&config.issuer, "lifecycle-person")?;
        let actor = Actor::new(binding.clone(), Provenance::new(AuthMethod::Oidc, 1));
        let (person, _) = directory.register_person(
            actor.clone(),
            OperationId::generate()?,
            Profile::new("Lifecycle person")?,
            1,
        )?;
        directory.bind_login(actor.clone(), OperationId::generate()?, person, binding, 2)?;
        let id = IdentityId::Person(person);
        let refused = active_caller(directory.projection()?, &actor)
            .err()
            .ok_or("Registered person admitted")?;
        assert_eq!(refused.name(), "inactive");
        assert!(refused.to_string().contains("registered"));
        directory.transition(
            actor.clone(),
            OperationId::generate()?,
            id,
            Transition::Activate,
            "",
            3,
        )?;
        assert_eq!(active_caller(directory.projection()?, &actor)?, id);
        for (transition, state) in [
            (Transition::Suspend, "suspended"),
            (Transition::Retire, "retired"),
        ] {
            directory.transition(
                actor.clone(),
                OperationId::generate()?,
                id,
                transition,
                "test refusal",
                4,
            )?;
            let error = active_caller(directory.projection()?, &actor)
                .err()
                .ok_or("inactive identity admitted")?;
            assert_eq!(error.name(), "inactive");
            assert!(error.to_string().contains(state));
            assert!(error.to_string().contains(&id.to_string()));
        }
        let unknown = Actor::new(
            LoginBinding::new(&config.issuer, "unknown")?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        assert_eq!(
            active_caller(directory.projection()?, &unknown)
                .err()
                .ok_or("unbound identity admitted")?
                .name(),
            "NoPerson"
        );
        Ok(())
    })
    .await?;
    Ok(())
}
