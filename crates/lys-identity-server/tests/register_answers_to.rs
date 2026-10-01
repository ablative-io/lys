//! Registration preserves the selected responsible person and the caller's authority.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{OperationId, PersonId};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

#[tokio::test]
async fn registration_respects_the_chosen_person_and_the_callers_authority()
-> Result<(), Box<dyn Error>> {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, "other"])?))
            .await?;
    let administrator = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let other = service
        .sign_in(Login {
            subject: "other".to_owned(),
            email: "other@example.test".to_owned(),
        })
        .await?;
    let own = seeded.people[0].id.to_string();
    let responsible = seeded.people[1].id.to_string();
    let unknown = PersonId::generate()?.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    for requested in [
        None,
        Some(&own),
        Some(&responsible),
        Some(&unknown),
        Some(&agent),
    ] {
        let mut body = json!({
            "operation": OperationId::generate()?.to_string(),
            "display_name": "Not admitted",
        });
        if let Some(id) = requested {
            body["answers_to"] = json!(id);
        }
        refused(&service, &other, &body, 403, "NotAdmitted").await?;
    }
    for (name, requested) in [
        ("Default owner", None),
        ("Explicit owner", Some(&own)),
        ("Chosen owner", Some(&responsible)),
    ] {
        let expected = requested.unwrap_or(&own);
        let mut body = json!({
            "operation": OperationId::generate()?.to_string(),
            "display_name": name,
        });
        if let Some(id) = requested {
            body["answers_to"] = json!(id);
        }
        let (status, made) = service.post("/agents", Some(&administrator), &body).await?;
        assert_eq!(status, 200, "{made}");
        assert_eq!(made["responsible"], *expected);
        let agent = made["agent"].as_str().ok_or("registration has no agent")?;
        let viewer = if expected == &responsible {
            &other
        } else {
            &administrator
        };
        let (status, seen) = service
            .get(&format!("/agents/{agent}"), Some(viewer))
            .await?;
        assert_eq!(status, 200, "{seen}");
        assert_eq!(seen["person"]["id"], *expected);
        let before = service.log_size().await?;
        let (status, replayed) = service.post("/agents", Some(&administrator), &body).await?;
        assert_eq!(status, 200, "{replayed}");
        assert_eq!(replayed["agent"], agent);
        assert_eq!(replayed["receipt"], made["receipt"]);
        assert_eq!(service.log_size().await?, before);
    }
    for (id, status, code) in [
        (&unknown, 404, "IdentityUnknown"),
        (&agent, 409, "IdentifierMalformed"),
    ] {
        let body = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Invalid responsibility", "answers_to": id});
        refused(&service, &administrator, &body, status, code).await?;
    }
    for transition in ["suspend", "retire"] {
        let body = json!({"operation": OperationId::generate()?.to_string(), "transition": transition, "reason": "Responsibility withdrawn"});
        let (status, answer) = service
            .post(
                &format!("/identities/{responsible}/transitions"),
                Some(&administrator),
                &body,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        let body = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Inactive responsibility", "answers_to": responsible});
        refused(&service, &administrator, &body, 403, "inactive").await?;
    }
    Ok(())
}

async fn refused(
    service: &Service,
    cookie: &str,
    body: &serde_json::Value,
    status: u16,
    code: &str,
) -> Result<(), Box<dyn Error>> {
    let before = service.log_size().await?;
    let (actual, answer) = service.post("/agents", Some(cookie), body).await?;
    assert_eq!(actual, status, "{answer}");
    assert_eq!(answer["refusal"], code);
    assert_eq!(service.log_size().await?, before);
    Ok(())
}
