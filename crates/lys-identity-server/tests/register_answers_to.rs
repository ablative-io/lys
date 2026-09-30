use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
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
    let refused = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": "Wrong responsibility",
        "answers_to": own,
    });
    let (status, answer) = service.post("/agents", Some(&other), &refused).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");

    for (cookie, name) in [
        (&administrator, "Chosen by administrator"),
        (&other, "Chosen for self"),
    ] {
        let body = json!({
            "operation": OperationId::generate()?.to_string(),
            "display_name": name,
            "answers_to": responsible,
        });
        let (status, made) = service.post("/agents", Some(cookie), &body).await?;
        assert_eq!(status, 200, "{made}");
        assert_eq!(made["responsible"], responsible);
        let agent = made["agent"].as_str().ok_or("registration has no agent")?;
        let (status, seen) = service
            .get(&format!("/agents/{agent}"), Some(&other))
            .await?;
        assert_eq!(status, 200, "{seen}");
        assert_eq!(seen["person"]["id"], responsible);
        let (status, replayed) = service.post("/agents", Some(cookie), &body).await?;
        assert_eq!(status, 200, "{replayed}");
        assert_eq!(replayed["agent"], agent);
        assert_eq!(replayed["receipt"], made["receipt"]);
    }
    Ok(())
}
