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
    let (chosen, receipt) =
        admitted_choices(&service, &administrator, &other, &own, &responsible).await?;
    let mut changed = chosen.clone();
    changed["answers_to"] = json!(own);
    let bytes = registration_bytes(&service, &receipt)?;
    refused(&service, &administrator, &changed, 409, "OperationReused").await?;
    assert_eq!(registration_bytes(&service, &receipt)?, bytes);
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
        let bytes = registration_bytes(&service, &receipt)?;
        let next = next_index(&service).await?;
        absent(&service, next).await?;
        let (status, replay) = service
            .post("/agents", Some(&administrator), &chosen)
            .await?;
        assert_eq!(status, 200, "{replay}");
        assert_eq!(replay, receipt);
        absent(&service, next).await?;
        assert_eq!(registration_bytes(&service, &receipt)?, bytes);
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
    let next = next_index(service).await?;
    absent(service, next).await?;
    let (actual, answer) = service.post("/agents", Some(cookie), body).await?;
    assert_eq!(actual, status, "{answer}");
    assert_eq!(answer["refusal"], code);
    absent(service, next).await?;
    Ok(())
}

async fn next_index(service: &Service) -> Result<u64, Box<dyn Error>> {
    let (status, page) = service.get("/receipts/0", None).await?;
    assert_eq!(status, 200, "{page}");
    page["checkpoint"]["tree_size"]
        .as_u64()
        .ok_or_else(|| "receipt checkpoint has no tree size".into())
}

async fn absent(service: &Service, index: u64) -> Result<(), Box<dyn Error>> {
    let (status, answer) = service.get(&format!("/receipts/{index}"), None).await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed");
    assert_eq!(
        answer["reason"],
        format!("RequestMalformed: the log holds no leaf {index}")
    );
    Ok(())
}

async fn admitted_choices(
    service: &Service,
    administrator: &str,
    other: &str,
    own: &str,
    responsible: &str,
) -> Result<(serde_json::Value, serde_json::Value), Box<dyn Error>> {
    let mut chosen = None;
    for (name, requested) in [
        ("Default owner", None),
        ("Explicit owner", Some(own)),
        ("Chosen owner", Some(responsible)),
    ] {
        let expected = requested.unwrap_or(own);
        let mut body = json!({
            "operation": OperationId::generate()?.to_string(),
            "display_name": name,
        });
        if let Some(id) = requested {
            body["answers_to"] = json!(id);
        }
        let (status, made) = service.post("/agents", Some(administrator), &body).await?;
        assert_eq!(status, 200, "{made}");
        assert_eq!(made["responsible"], expected);
        let agent = made["agent"].as_str().ok_or("registration has no agent")?;
        let viewer = if expected == responsible {
            other
        } else {
            administrator
        };
        let (status, seen) = service
            .get(&format!("/agents/{agent}"), Some(viewer))
            .await?;
        assert_eq!(status, 200, "{seen}");
        assert_eq!(seen["person"]["id"], expected);
        let next = next_index(service).await?;
        absent(service, next).await?;
        let (status, replayed) = service.post("/agents", Some(administrator), &body).await?;
        assert_eq!(status, 200, "{replayed}");
        assert_eq!(replayed["agent"], agent);
        assert_eq!(replayed["receipt"], made["receipt"]);
        absent(service, next).await?;
        if expected == responsible {
            chosen = Some((body, made));
        }
    }
    chosen.ok_or_else(|| "the chosen-person registration was not exercised".into())
}

type RegistrationBytes = (Vec<u8>, Vec<u8>);
fn registration_bytes(
    service: &Service,
    answer: &serde_json::Value,
) -> Result<RegistrationBytes, Box<dyn Error>> {
    let index = answer["receipt"]["log"]["index"]
        .as_u64()
        .ok_or("registration has no receipt index")?;
    let log = service.dir.path().join("log");
    Ok((
        std::fs::read(log.join("state.json"))?,
        std::fs::read(log.join("leaves").join(format!("{index:020}")))?,
    ))
}
