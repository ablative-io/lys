#![cfg(test)]

//! An AI admits a person (AGENTS-019 R2): POST /people/admit asked with an
//! approved app's agent pass is judged as the agent, by the agent's own
//! grants, exactly as a person's pass is judged as the person. Given
//! organisation.admit by its responsible person, the agent admits; without
//! it, the act is refused by name before any step and nothing is recorded.
//! What the agent did is on its person's dashboard, and on no one else's
//! (AGENTS-019 R5).

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::Service;
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::OperationId;
use lys_identity::grants::shipped_model;
use lys_identity_server::agent_signature::{HEADER, payload};
use serde_json::{Value, json};

#[path = "people_admit_app/table.rs"]
mod table;
use table::{APP, BEA, STEPS, accounts_for, admit_with, bea, member_of, people, table, table_on};

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

/// Lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// A key for `agent`, certified by Ada's session over it.
async fn certified(
    service: &Service,
    ada: &str,
    agent: &str,
) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &service.dir.path().join(format!("{agent}.key")),
    )?);
    let request = STANDARD.encode(create_certificate_request(&key, agent)?);
    let (status, issued) = service
        .post(
            &format!("/agents/{agent}/certificates"),
            Some(ada),
            &json!({"operation": OperationId::generate()?.to_string(), "request": request}),
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    Ok(key)
}

/// `agent`'s own signed MCP call, by its certified `key`, of POST `path`
/// with `body` through the change tool, answered as JSON-RPC.
async fn change_as(
    service: &Service,
    (agent, key): (&str, &Ed25519Identity),
    path: &str,
    body: &Value,
) -> Result<Value, Box<dyn Error>> {
    let message = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "change", "arguments": {"method": "POST", "path": path, "body": body}}});
    let bytes = serde_json::to_vec(&message)?;
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = hex(OperationId::generate()?.as_bytes());
    let cose = sign_attestation(&payload("POST", "/mcp", &bytes, at, &nonce), key).to_cose_bytes();
    let answer = reqwest::Client::new()
        .post(format!("{}/mcp", service.base))
        .header(HEADER, format!("{agent} {at} {nonce} {}", hex(&cose)))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .body(bytes)
        .send()
        .await?;
    Ok(answer.json().await?)
}

/// `cookie`'s dashboard: its agents and the lines acted for them.
async fn dashboard(service: &Service, cookie: &str) -> Result<(Value, Vec<Value>), Box<dyn Error>> {
    let (status, answer) = service.get("/dashboard", Some(cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    let lines = answer["acted_by_agents"]
        .as_array()
        .ok_or_else(|| format!("acted_by_agents is not a list: {answer}"))?
        .clone();
    Ok((answer["agents"].clone(), lines))
}

#[tokio::test]
async fn what_an_agent_did_for_its_person_is_on_their_dashboard_and_no_one_elses() -> TestResult {
    let (service, rauthy, ada) = table_on(&shipped_model(), true).await?;
    let agent = agent_of(&service, &ada).await?;
    let grant = give(&service, &ada, &agent, "administrator").await?;
    let pass = agent_pass(&service, &ada, &agent, &grant).await?;
    // The agent admits Bea for Ada.
    let asked = bea(
        &OperationId::generate()?.to_string(),
        member_of("fixture_product.workspace"),
    );
    let (status, admitted) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{admitted}");
    let bea_person = admitted["person"].as_str().ok_or("a person")?.to_owned();
    let bea_subject = admitted["subject"].as_str().ok_or("a subject")?.to_owned();

    // Ada passes the agent the change of Bea's email, from a root of her own,
    // and the agent changes it through its own signed MCP call.
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    let account = json!({"kind": "account", "id": bea_person});
    let (status, root) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({
            "operation": OperationId::generate()?.to_string(), "route": "api",
            "holder": me["person"]["id"], "resource": account, "relation": "owner",
            "pass_on": {"kind": "to", "actions": ["person.email.set"], "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null}}),
        )
        .await?;
    assert_eq!(status, 200, "{root}");
    let (status, passed) = service
        .post(
            "/grants",
            Some(&ada),
            &json!({
            "operation": OperationId::generate()?.to_string(), "route": "api",
            "source": root["grant"], "recipient": agent, "responsible": me["person"]["id"],
            "resource": account, "relation": "only.person.email.set",
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}}),
        )
        .await?;
    assert_eq!(status, 200, "{passed}");
    let key = certified(&service, &ada, &agent).await?;
    let reply = change_as(
        &service,
        (agent.as_str(), key.as_ref()),
        &format!("/directory/people/{bea_person}/account/email"),
        &json!({"email": "bea.new@example.test"}),
    )
    .await?;
    assert!(reply["error"].is_null(), "{reply}");
    assert_eq!(accounts_for(&rauthy, "bea.new@example.test")?, 1, "{reply}");

    // Ada's dashboard lists both acts, newest first, each naming the agent,
    // which her dashboard names Helper.
    let (agents, lines) = dashboard(&service, &ada).await?;
    assert!(
        agents
            .as_array()
            .is_some_and(|agents| agents.iter().any(|row| {
                row["agent"]["id"] == agent.as_str() && row["agent"]["display_name"] == "Helper"
            })),
        "{agents}"
    );
    let line = |kind: &str| {
        lines
            .iter()
            .position(|line| line["kind"] == kind && line["acting_agent"] == agent.as_str())
            .ok_or_else(|| format!("no {kind} line by the agent: {lines:?}"))
    };
    let (admission, email) = (line("organisation.admit")?, line("person.email.set")?);
    assert!(email < admission, "newest first: {lines:?}");
    let admission = &lines[admission];
    assert_eq!(admission["agent"], agent.as_str(), "{admission}");
    assert_eq!(admission["subject"]["kind"], "organisation", "{admission}");
    assert_eq!(
        admission["subject"]["id"], me["organisation"]["id"],
        "{admission}"
    );
    assert!(admission["index"].as_u64().is_some(), "{admission}");
    assert!(admission["at"].as_u64().is_some(), "{admission}");
    let email = &lines[email];
    assert_eq!(email["subject"], account, "{email}");
    assert!(
        email["sentence"]
            .as_str()
            .is_some_and(|words| !words.is_empty()),
        "{email}"
    );
    assert!(
        email["index"].as_u64() > admission["index"].as_u64(),
        "{email} {admission}"
    );

    // Bea's own dashboard shows none of it.
    let bea_cookie = service
        .sign_in(Login {
            subject: bea_subject,
            email: "bea.new@example.test".to_owned(),
        })
        .await?;
    let (_, theirs) = dashboard(&service, &bea_cookie).await?;
    assert!(
        theirs
            .iter()
            .all(|line| line["acting_agent"] != agent.as_str()),
        "{theirs:?}"
    );
    Ok(())
}
