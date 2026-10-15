use super::Table;
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use lys_core::{Ed25519Identity, attestation::sign_attestation, ca::create_certificate_request};
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "app_person_fixture_tests.rs"]
mod fixture;
use fixture::{TestResult, operation};

#[tokio::test]
async fn a_cambium_person_pass_registers_activates_certifies_and_signs_as_the_agent() -> TestResult
{
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    let (status, registered) = table
        .send(
            "POST",
            "/agents",
            &pass,
            &json!({"operation": operation()?, "display_name": "Cambium seat"}),
            &[],
        )
        .await?;
    assert_eq!(status, 200, "{registered}");
    let agent = registered["agent"].as_str().ok_or("agent absent")?;
    assert_eq!(registered["receipt"]["identity"], agent);
    assert_eq!(registered["responsible"], table.person.to_string());
    assert_eq!(
        registered["reports_to"],
        json!({"kind": "person", "id": table.person.to_string()})
    );
    assert_eq!(table.state_of(agent)?, "registered");
    let path = format!("/identities/{agent}/transitions");
    let (status, activated) = table
        .send(
            "POST",
            &path,
            &pass,
            &json!({"operation": operation()?, "transition": "activate"}),
            &[],
        )
        .await?;
    assert_eq!(status, 200, "{activated}");
    assert_eq!(activated["receipt"]["identity"], agent);
    assert_eq!(table.state_of(agent)?, "active");
    let key = Arc::new(Ed25519Identity::ephemeral());
    let serial = operation()?;
    let request = STANDARD.encode(create_certificate_request(&key, agent)?);
    let (status, certified) = table
        .send(
            "POST",
            &format!("/agents/{agent}/certificates"),
            &pass,
            &json!({"operation": serial, "request": request}),
            &[],
        )
        .await?;
    assert_eq!(status, 200, "{certified}");
    assert_eq!(certified["agent"], agent);
    assert_eq!(certified["recorded"], serial);
    let certificate = certified["certificates"]
        .as_array()
        .ok_or("certificates absent")?
        .iter()
        .find(|certificate| certificate["serial"] == serial)
        .ok_or("certificate absent")?;
    let der = STANDARD.decode(certificate["der"].as_str().ok_or("DER absent")?)?;
    assert_eq!(
        lys_core::ca::certificate_subject_public_key(&der)?,
        key.public_key_bytes()
    );
    crate::certificates_issue::with_store(&table.state, |store| {
        assert!(store.certificate(&serial).is_some());
        Ok(())
    })?;
    let path = "/mcp";
    let body = b"agent request";
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = "1234567890abcdef1234567890abcdef";
    let payload = crate::agent_signature::payload("POST", path, body, at, nonce);
    let signature = crate::routes::hex(&sign_attestation(&payload, &key).to_cose_bytes());
    let mut headers = HeaderMap::new();
    headers.insert(
        crate::agent_signature::HEADER,
        format!("{agent} {at} {nonce} {signature}").parse()?,
    );
    let signed = crate::routes::with_directory(&table.state, |directory| {
        crate::agent_signature::signed_agent(
            &table.state,
            directory.projection()?,
            &headers,
            ("POST", path, body),
        )
    })?;
    assert_eq!(signed.map(|id| id.to_string()).as_deref(), Some(agent));
    Ok(())
}

#[tokio::test]
async fn every_approved_app_audience_can_register_an_own_agent() -> TestResult {
    let table = Table::app_fixture().await?;
    for app in ["cambium", "fixture_product", "lys"] {
        let answer = table
            .send(
                "POST",
                "/agents",
                &table.pass(app, false, true)?,
                &json!({"operation": operation()?, "display_name": app}),
                &[],
            )
            .await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
        assert_eq!(answer.1["responsible"], table.person.to_string());
    }
    Ok(())
}

#[path = "app_person_refusals_tests.rs"]
mod refusals;
