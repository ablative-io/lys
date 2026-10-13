//! Personal registration and key certification preserve the directory's audit and refusals.

use base64::Engine;
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn a_person_registers_and_activates_their_own_agent_with_the_same_registration_audit()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, seeded) = Service::start_with(|config| {
        Ok(
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "member"],
            )?,
        )
    })
    .await?;
    let member = service
        .sign_in(identity_contract::apps::login("member"))
        .await?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let body = json!({"operation":lys_identity::OperationId::generate()?.to_string(), "display_name":"Own new agent", "answers_to":seeded.people[1].id.to_string()});
    let (status, own) = service.post("/agents", Some(&member), &body).await?;
    assert_eq!(status, 200);
    let agent = own["agent"].as_str().ok_or("agent absent")?;
    let (status, view) = service
        .get(&format!("/agents/{agent}"), Some(&member))
        .await?;
    assert_eq!(status, 200);
    assert_eq!(view["person"]["id"], seeded.people[1].id.to_string());
    let (status, registered) = service.post("/agents", Some(&admin), &json!({
        "operation":lys_identity::OperationId::generate()?.to_string(), "display_name":"Admin new agent"
    })).await?;
    assert_eq!(status, 200);
    assert_eq!(
        own["receipt"]["change_kind"],
        registered["receipt"]["change_kind"]
    );
    let (status, receipt) = service
        .get(
            &format!(
                "/receipts/{}",
                own["receipt"]["log"]["index"]
                    .as_u64()
                    .ok_or("audit index absent")?
            ),
            None,
        )
        .await?;
    assert_eq!(status, 200);
    assert_eq!(receipt["receipt"]["identity"], agent);
    let (status, _) = service.post(&format!("/identities/{agent}/transitions"), Some(&member), &json!({
        "operation":lys_identity::OperationId::generate()?.to_string(), "transition":"activate"
    })).await?;
    assert_eq!(status, 200);
    let (status, refused) = service.post("/agents", Some(&member), &json!({
        "operation":lys_identity::OperationId::generate()?.to_string(), "display_name":"Foreign agent", "answers_to":seeded.people[0].id.to_string()
    })).await?;
    assert_eq!(status, 403);
    assert_eq!(refused["refusal"], "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn a_person_certifies_their_own_agents_proved_key_and_cannot_certify_another_persons_agent()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, seeded) = Service::start_with(|config| {
        Ok(
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "member"],
            )?,
        )
    })
    .await?;
    let member = service
        .sign_in(identity_contract::apps::login("member"))
        .await?;
    let agent = seeded.people[1].agents[0].id.to_string();
    let key = Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &service.dir.path().join("own-agent.key"),
    )?);
    let body = json!({"operation":lys_identity::OperationId::generate()?.to_string(),"request":base64::engine::general_purpose::STANDARD.encode(lys_core::ca::create_certificate_request(&key, &agent)?)});
    let path = format!("/agents/{agent}/certificates");
    let (status, issued) = service.post(&path, Some(&member), &body).await?;
    assert_eq!(status, 200);
    assert_eq!(issued["recorded"], body["operation"]);
    let (status, kept) = service.get(&path, Some(&member)).await?;
    assert_eq!(status, 200);
    assert_eq!(kept["certificates"], issued["certificates"]);
    let foreign = seeded.people[0].agents[0].id.to_string();
    let (status, refused) = service
        .post(
            &format!("/agents/{foreign}/certificates"),
            Some(&member),
            &body,
        )
        .await?;
    assert_eq!(status, 403);
    assert_eq!(refused["refusal"], "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn the_next_signed_agent_pass_contains_the_certified_key_thumbprint()
-> Result<(), Box<dyn std::error::Error>> {
    let mut table = crate::budgets_feed::entry_tests::fixture().await?;
    let dir = tempfile::tempdir()?;
    let key = Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("agent.key"),
    )?);
    let agent: lys_identity::AgentId = table.agent.parse()?;
    let request = lys_core::ca::create_certificate_request(&key, &table.agent)?;
    let service_key = Arc::new(lys_core::Ed25519Identity::load(
        &table.state.grant_setup.key_file,
    )?);
    let mut store = crate::certificates_store::CertificateStore::open(
        &dir.path().join("certificates"),
        service_key,
    )?;
    let person = crate::routes::with_directory(&table.state, |directory| {
        directory
            .projection()?
            .record(lys_identity::IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(crate::ServerError::NoPerson)
    })?;
    let issued = crate::certificates_issue::issued(
        &table.state,
        &crate::agent_sight::SeenAgent {
            agent,
            responsible: Some(person),
        },
        &request,
        lys_identity::OperationId::generate()?.to_string(),
        crate::session::now(),
    )?;
    store.issue(issued)?;
    let state = Arc::get_mut(&mut table.state).ok_or("fixture state shared")?;
    state.certificates = Some(std::sync::Mutex::new(store));
    let issuer_key = dir.path().join("issuer.key");
    std::fs::write(&issuer_key, [6_u8; 32])?;
    let provider = crate::provider::OpenIdProvider::open(
        &crate::provider::ProviderSettings {
            key_file: issuer_key,
            code_seconds: 120,
            pass_seconds: 3600,
            rights_bytes: None,
        },
        state.oidc.public_origin().to_owned(),
    )?;
    let pass = crate::provider::certificate_test_pass(state, &provider, agent)?;
    let payload = pass.split('.').nth(1).ok_or("payload absent")?;
    let claims: serde_json::Value =
        serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?)?;
    use sha2::Digest;
    let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(key.public_key_bytes()));
    assert_eq!(claims["agent_key_thumbprints"], json!([expected]));
    table.finish().await
}
