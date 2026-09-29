#![cfg(test)]
//! Existing sessions cannot outlive suspension, including administrator authority.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Provenance, Transition,
};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::routes::open_directory;
use lys_identity_server::session::{Sessions, now};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;
const PERSON: &str = "lifecycle-person";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "lifecycle@example.test".to_owned(),
    }
}

#[tokio::test]
async fn suspended_and_retired_saved_administrator_sessions_refuse_by_state() -> TestResult {
    for transition in [Transition::Suspend, Transition::Retire] {
        let (service, (cookie, agent)) = Service::start_with(|config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, PERSON])?;
            let person = seeded.people[0].id;
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, now()),
            );
            let sessions = Sessions::open(
                config.sessions_file.clone().ok_or("no sessions path")?,
                config.session_seconds,
                config.secure_cookie,
            )?;
            let cookie = sessions.begin(actor.clone())?;
            let mut directory = open_directory(config)?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Person(person),
                transition,
                "authority withdrawn",
                now(),
            )?;
            Ok((cookie, seeded.people[0].agents[0].id.to_string()))
        })
        .await?;
        for path in [
            "/me".to_owned(),
            "/identities".to_owned(),
            "/apps".to_owned(),
            "/grants/model".to_owned(),
            format!("/agents/{agent}/runtime/sessions"),
        ] {
            let (status, body) = service.get(&path, Some(&cookie)).await?;
            assert_eq!(status, 403, "{path}: {body}");
            assert_eq!(body["refusal"], "inactive", "{path}: {body}");
        }
        let before = service.log_size().await?;
        let response = service.post("/people", Some(&cookie), &json!({"operation": OperationId::generate()?.to_string(), "display_name": "must not be written"})).await?;
        assert_eq!(response.0, 403, "{}", response.1);
        assert_eq!(response.1["refusal"], "inactive");
        assert_eq!(service.log_size().await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn suspension_ends_every_session_durably_and_reinstatement_does_not_restore_them()
-> TestResult {
    let (mut service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, PERSON])?)).await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let first = service.sign_in(login(PERSON)).await?;
    let second = service.sign_in(login(PERSON)).await?;
    let person = seeded.people[1].id;
    let path = format!("/identities/{person}/transitions");
    let suspend = json!({"operation": OperationId::generate()?.to_string(), "transition": "suspend", "reason": "authority withdrawn"});
    let response = service.post(&path, Some(&admin), &suspend).await?;
    assert_eq!(response.0, 200, "{}", response.1);
    let response = service
        .get(
            &format!("/directory/people/{person}/sessions"),
            Some(&admin),
        )
        .await?;
    assert_eq!(response.0, 200, "{}", response.1);
    assert_eq!(response.1["sessions"], json!([]));
    let response = service
        .post(
            &path,
            Some(&admin),
            &json!({"operation": OperationId::generate()?.to_string(), "transition": "reinstate"}),
        )
        .await?;
    assert_eq!(response.0, 200, "{}", response.1);
    service.restart().await?;
    for cookie in [&first, &second] {
        let response = service.get("/me", Some(cookie)).await?;
        assert_eq!(response.0, 401, "{}", response.1);
        assert_eq!(response.1["refusal"], "NotSignedIn");
    }
    let fresh = service.sign_in(login(PERSON)).await?;
    assert_eq!(service.get("/me", Some(&fresh)).await?.0, 200);
    Ok(())
}
