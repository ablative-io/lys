#![cfg(test)]

//! An AI admits a person (AGENTS-019 R2): POST /people/admit asked with an
//! approved app's agent pass is judged as the agent, by the agent's own
//! grants, exactly as a person's pass is judged as the person. Given
//! organisation.admit by its responsible person, the agent admits; without
//! it, the act is refused by name before any step and nothing is recorded.

use std::error::Error;

use identity_contract::harness::Service;
use lys_identity::OperationId;
use serde_json::json;

#[path = "people_admit_app/table.rs"]
mod table;
use table::{APP, BEA, STEPS, accounts_for, admit_with, bea, member_of, people, table};

type TestResult = Result<(), Box<dyn Error>>;

/// A new agent of Ada's, active, answering its id.
async fn agent_of(service: &Service, ada: &str) -> Result<String, Box<dyn Error>> {
    let (status, made) = service
        .post(
            "/agents",
            Some(ada),
            &json!({"operation": OperationId::generate()?.to_string(), "display_name": "Helper"}),
        )
        .await?;
    assert_eq!(status, 200, "{made}");
    let agent = made["agent"].as_str().ok_or("an agent")?.to_owned();
    let (status, active) = service
        .post(
            &format!("/identities/{agent}/transitions"),
            Some(ada),
            &json!({"operation": OperationId::generate()?.to_string(),
                "transition": "activate", "reason": ""}),
        )
        .await?;
    assert_eq!(status, 200, "{active}");
    Ok(agent)
}

/// Ada gives `agent` `relation` on her own organisation from her standing,
/// answering the grant.
async fn give(
    service: &Service,
    ada: &str,
    agent: &str,
    relation: &str,
) -> Result<String, Box<dyn Error>> {
    let (status, me) = service.get("/me", Some(ada)).await?;
    assert_eq!(status, 200, "{me}");
    let body = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "source": "administrator", "recipient": agent, "responsible": me["person"]["id"],
        "resource": {"kind": "organisation", "id": me["organisation"]["id"]},
        "relation": relation, "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null}
    });
    let (status, given) = service.post("/grants", Some(ada), &body).await?;
    assert_eq!(status, 200, "{given}");
    Ok(given["grant"].as_str().ok_or("a grant")?.to_owned())
}

/// The fixture app's pass for `agent`, proved by a credential Ada issues
/// for `grant`.
async fn agent_pass(
    service: &Service,
    ada: &str,
    agent: &str,
    grant: &str,
) -> Result<String, Box<dyn Error>> {
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 300;
    let (status, credential) = service
        .post(
            &format!("/grants/{grant}/tokens"),
            Some(ada),
            &json!({ "expires_at": expires_at }),
        )
        .await?;
    assert_eq!(status, 200, "{credential}");
    let credential = credential["token"].as_str().ok_or("a grant credential")?;
    let (status, answer) = service
        .post_carrying(
            &format!("/agents/{agent}/pass"),
            &[(lys_identity_server::grant_tokens::HEADER, credential)],
            serde_json::to_vec(&json!({ "audience": APP }))?,
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer["pass"].as_str().ok_or("a pass")?.to_owned())
}

#[tokio::test]
async fn an_agent_given_organisation_admit_admits_a_person_as_itself() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let agent = agent_of(&service, &ada).await?;
    let grant = give(&service, &ada, &agent, "administrator").await?;
    let pass = agent_pass(&service, &ada, &agent, &grant).await?;
    let before = people(&service, &ada).await?;
    let asked = bea(
        &OperationId::generate()?.to_string(),
        member_of("fixture_product.workspace"),
    );
    let (status, answer) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["receipt"]["completed"], json!(STEPS), "{answer}");
    assert!(answer["receipt"]["failed"].is_null(), "{answer}");
    // The act is the agent's own, through the app it holds the pass for.
    let actor = &answer["receipt"]["actor"];
    assert_eq!(actor["subject"], json!(agent), "{answer}");
    assert_eq!(actor["through_app"], APP, "{answer}");
    let first = answer["grant"].as_str().ok_or("a first grant")?;
    let (status, held) = service.get(&format!("/grants/{first}"), Some(&ada)).await?;
    assert_eq!(status, 200, "{held}");
    let text = held.to_string();
    for word in ["fixture_product.workspace", "ws-1", "member"] {
        assert!(text.contains(word), "the grant names {word}: {held}");
    }
    assert_eq!(people(&service, &ada).await?, before + 1);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    // Asked again with the same operation, the act replays.
    let (status, again) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["person"], answer["person"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    Ok(())
}

#[tokio::test]
async fn an_agent_without_organisation_admit_is_refused_before_any_step() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let agent = agent_of(&service, &ada).await?;
    // A grant that reads the organisation and admits nobody.
    let grant = give(&service, &ada, &agent, "beta").await?;
    let pass = agent_pass(&service, &ada, &agent, &grant).await?;
    let before = people(&service, &ada).await?;
    let asked = bea(
        &OperationId::generate()?.to_string(),
        member_of("fixture_product.workspace"),
    );
    let (status, refused) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("organisation.admit")),
        "the refusal names the act: {refused}"
    );
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    // A product kind the app does not own is refused as a person's pass is.
    let mut elsewhere = asked;
    elsewhere["grant"] = member_of("cambium.workspace");
    elsewhere["operation"] = json!(OperationId::generate()?.to_string());
    let (status, refused) = admit_with(&service, &pass, &elsewhere).await?;
    assert_eq!(
        (status, refused["refusal"].as_str()),
        (403, Some("not_your_app")),
        "{refused}"
    );
    Ok(())
}
