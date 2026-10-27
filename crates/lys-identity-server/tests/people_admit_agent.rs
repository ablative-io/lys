#![cfg(test)]

//! An AI admits a person (AGENTS-019 R2): POST /people/admit asked with an
//! approved app's agent pass is judged as the agent, by the agent's own
//! grants, exactly as a person's pass is judged as the person. Given
//! organisation.admit by its responsible person, the agent admits; without
//! it, the act is refused by name before any step and nothing is recorded.
//! What the agent did is on its person's dashboard, and on no one else's
//! (AGENTS-019 R5). An administrator's agent reads which schema each
//! approved app registered, and nothing more of the apps (27 October 2026).

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

#[path = "../src/mcp_test_answer.rs"]
mod mcp_answer;
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
    agent: (&str, &Ed25519Identity),
    path: &str,
    body: &Value,
) -> Result<Value, Box<dyn Error>> {
    let arguments = json!({"method": "POST", "path": path, "body": body});
    call_as(service, agent, "change", &arguments).await
}

/// `agent`'s own signed MCP read of `path`: the relayed route's status and
/// answer.
async fn read_as(
    service: &Service,
    agent: (&str, &Ed25519Identity),
    path: &str,
) -> Result<(u64, Value), Box<dyn Error>> {
    let answer = call_as(service, agent, "read", &json!({"path": path})).await?;
    let relayed = mcp_answer::body(&answer["result"]);
    let status = relayed["status"]
        .as_u64()
        .ok_or_else(|| format!("no status: {answer}"))?;
    Ok((status, relayed["body"].clone()))
}

/// `agent`'s own signed MCP call, by its certified `key`, of `tool` with
/// `arguments`, answered as JSON-RPC.
async fn call_as(
    service: &Service,
    (agent, key): (&str, &Ed25519Identity),
    tool: &str,
    arguments: &Value,
) -> Result<Value, Box<dyn Error>> {
    let message = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}});
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

/// `agent`'s signed MCP call moving Bea's sign-in to a new record named
/// Annabel, as the move from People asks it.
async fn move_as(
    service: &Service,
    agent: (&str, &Ed25519Identity),
    (person, subject): (&str, &str),
) -> Result<Value, Box<dyn Error>> {
    let body = json!({"operation": OperationId::generate()?.to_string(),
        "issuer": service.issuer.issuer(), "subject": subject,
        "target": {"display_name": "Annabel"}});
    change_as(
        service,
        agent,
        &format!("/people/{person}/logins/move"),
        &body,
    )
    .await
}

#[tokio::test]
async fn an_agent_given_organisation_administer_moves_a_sign_in_to_its_person() -> TestResult {
    let (service, _rauthy, ada) = table_on(&shipped_model(), true).await?;
    let agent = agent_of(&service, &ada).await?;
    let grant = give(&service, &ada, &agent, "administrator").await?;
    let pass = agent_pass(&service, &ada, &agent, &grant).await?;
    let asked = bea(
        &OperationId::generate()?.to_string(),
        member_of("fixture_product.workspace"),
    );
    let (status, admitted) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{admitted}");
    let person = admitted["person"].as_str().ok_or("a person")?;
    let subject = admitted["subject"].as_str().ok_or("a subject")?;

    // An agent that may admit people, and not administer them, is refused
    // the move, and Bea keeps her sign-in.
    let only_admits = agent_of(&service, &ada).await?;
    give(&service, &ada, &only_admits, "only.organisation.admit").await?;
    let only_admits_key = certified(&service, &ada, &only_admits).await?;
    let refused = move_as(
        &service,
        (only_admits.as_str(), only_admits_key.as_ref()),
        (person, subject),
    )
    .await?;
    assert!(refused.to_string().contains("NotHeld"), "{refused}");
    let (_, kept) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    assert!(kept["logins"].to_string().contains(subject), "{kept}");

    // The administrator agent moves it, as itself, and Bea's record no
    // longer holds it.
    let key = certified(&service, &ada, &agent).await?;
    let reply = move_as(&service, (agent.as_str(), key.as_ref()), (person, subject)).await?;
    assert!(reply["error"].is_null(), "{reply}");
    assert!(
        reply.to_string().contains(&agent),
        "the act is the agent's: {reply}"
    );
    let (_, source) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    assert!(!source["logins"].to_string().contains(subject), "{source}");
    Ok(())
}

/// The grant `id` as Ada lists it.
async fn listed(service: &Service, ada: &str, id: &str) -> Result<Value, Box<dyn Error>> {
    let (status, grants) = service.get("/grants", Some(ada)).await?;
    assert_eq!(status, 200, "{grants}");
    grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|each| each["id"] == id)
        .cloned()
        .ok_or_else(|| "the grant is not listed".into())
}

#[tokio::test]
async fn an_administrator_agent_reads_an_approved_apps_schema_and_a_lesser_one_is_refused()
-> TestResult {
    let (service, _rauthy, ada) = table_on(&shipped_model(), true).await?;
    let path = format!("/apps/{APP}/schema");

    // An agent that may admit people, and holds no reading standing, is
    // refused the read by the act's name.
    let only_admits = agent_of(&service, &ada).await?;
    give(&service, &ada, &only_admits, "only.organisation.admit").await?;
    let only_admits_key = certified(&service, &ada, &only_admits).await?;
    let lesser = (only_admits.as_str(), only_admits_key.as_ref());
    let (status, refused) = read_as(&service, lesser, &path).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");
    assert_eq!(refused["fields"]["action"], "read", "{refused}");
    assert_eq!(refused["fields"]["resource_kind"], "app", "{refused}");

    // The administrator agent reads the schema the app registered, by its
    // standing, and the use is recorded against the standing grant.
    let agent = agent_of(&service, &ada).await?;
    let grant = give(&service, &ada, &agent, "administrator").await?;
    let key = certified(&service, &ada, &agent).await?;
    let administrator = (agent.as_str(), key.as_ref());
    let (status, schema) = read_as(&service, administrator, &path).await?;
    assert_eq!(status, 200, "{schema}");
    assert_eq!(schema["app"], APP, "{schema}");
    assert_eq!(schema["version"], 1, "{schema}");
    let kinds = schema["schema"]["kinds"]
        .as_object()
        .ok_or_else(|| format!("no kinds: {schema}"))?;
    for kind in ["workspace", "channel", "doc"] {
        assert!(kinds.contains_key(&format!("{APP}.{kind}")), "{schema}");
    }
    let held = listed(&service, &ada, &grant).await?;
    assert_eq!(held["last_use"]["seen"], true, "{held}");

    // Reading opens no write: checking a change stays the administrator's
    // and the app's, refused at the door for want of a declared scope.
    let check = json!({"schema": schema["schema"]});
    let reply = change_as(&service, administrator, &format!("{path}/check"), &check).await?;
    let relayed = mcp_answer::body(&reply["result"]);
    assert_eq!(relayed["status"], 403, "{relayed}");
    assert_eq!(
        relayed["body"]["refusal"], "TokenScopeUndeclared",
        "{relayed}"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_given_reading_on_one_app_reads_its_schema_and_never_a_waiting_registration()
-> TestResult {
    let (service, _rauthy, ada) = table_on(&shipped_model(), true).await?;
    let waiting = "fixture_waiting";
    identity_contract::apps::register(&service, &ada, waiting).await?;
    // The administrator is told the registration is there, with no version
    // approved yet.
    let (status, own) = service
        .get(&format!("/apps/{waiting}/schema"), Some(&ada))
        .await?;
    assert_eq!(status, 404, "{own}");
    assert_eq!(own["refusal"], "schema_version_unknown", "{own}");

    // Ada gives a new agent reading on the approved app alone, from a root
    // of her own.
    let agent = agent_of(&service, &ada).await?;
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    let app = json!({"kind": "app", "id": APP});
    let (status, root) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({
            "operation": OperationId::generate()?.to_string(), "route": "api",
            "holder": me["person"]["id"], "resource": app, "relation": "reader",
            "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["agent"]},
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
            "resource": app, "relation": "reader",
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}}),
        )
        .await?;
    assert_eq!(status, 200, "{passed}");
    let key = certified(&service, &ada, &agent).await?;
    let reader = (agent.as_str(), key.as_ref());
    let (status, schema) = read_as(&service, reader, &format!("/apps/{APP}/schema")).await?;
    assert_eq!(status, 200, "{schema}");
    assert_eq!(schema["app"], APP, "{schema}");
    let grant = passed["grant"].as_str().ok_or("a grant")?;
    let held = listed(&service, &ada, grant).await?;
    assert_eq!(held["last_use"]["seen"], true, "{held}");

    // The grant reaches that app alone.
    let (status, refused) = read_as(&service, reader, &format!("/apps/{waiting}/schema")).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");

    // A registration still waiting is the install administrator's to know
    // of: even an agent with administrator standing is answered as if it
    // were not there.
    give(&service, &ada, &agent, "administrator").await?;
    let (status, hidden) = read_as(&service, reader, &format!("/apps/{waiting}/schema")).await?;
    assert_eq!(status, 404, "{hidden}");
    assert_eq!(hidden["refusal"], "app_unknown", "{hidden}");
    Ok(())
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

    // An agent that may only admit, holding no grant on Bea's account, is
    // refused her email change by its own signed MCP call, and nothing
    // changes.
    let only_admits = agent_of(&service, &ada).await?;
    give(&service, &ada, &only_admits, "only.organisation.admit").await?;
    let only_admits_key = certified(&service, &ada, &only_admits).await?;
    let key = certified(&service, &ada, &agent).await?;
    let email_path = format!("/directory/people/{bea_person}/account/email");
    let refused = change_as(
        &service,
        (only_admits.as_str(), only_admits_key.as_ref()),
        &email_path,
        &json!({"email": "bea.new@example.test"}),
    )
    .await?;
    assert!(refused.to_string().contains("NotHeld"), "{refused}");
    assert_eq!(
        accounts_for(&rauthy, "bea.new@example.test")?,
        0,
        "{refused}"
    );

    // Ada passes the only_admits the change of Bea's email, from a root of her
    // own, and it changes it through its own signed MCP call.
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
            "source": root["grant"], "recipient": only_admits, "responsible": me["person"]["id"],
            "resource": account, "relation": "only.person.email.set",
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}}),
        )
        .await?;
    assert_eq!(status, 200, "{passed}");
    let reply = change_as(
        &service,
        (only_admits.as_str(), only_admits_key.as_ref()),
        &email_path,
        &json!({"email": "bea.new@example.test"}),
    )
    .await?;
    assert!(reply["error"].is_null(), "{reply}");
    assert_eq!(accounts_for(&rauthy, "bea.new@example.test")?, 1, "{reply}");
    // The administrator agent changes it again by its standing alone.
    let reply = change_as(
        &service,
        (agent.as_str(), key.as_ref()),
        &email_path,
        &json!({"email": "bea.later@example.test"}),
    )
    .await?;
    assert!(reply["error"].is_null(), "{reply}");
    assert_eq!(
        accounts_for(&rauthy, "bea.later@example.test")?,
        1,
        "{reply}"
    );

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
    let line = |kind: &str, by: &str| {
        lines
            .iter()
            .position(|line| line["kind"] == kind && line["acting_agent"] == by)
            .ok_or_else(|| format!("no {kind} line by {by}: {lines:?}"))
    };
    let admission = line("organisation.admit", &agent)?;
    let email = line("person.email.set", &only_admits)?;
    // The change by standing is recorded against the standing it used.
    let standing = line("organisation.administer", &agent)?;
    assert!(
        standing < email && email < admission,
        "newest first: {lines:?}"
    );
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
            email: "bea.later@example.test".to_owned(),
        })
        .await?;
    let (_, theirs) = dashboard(&service, &bea_cookie).await?;
    assert!(
        theirs
            .iter()
            .all(|line| line["acting_agent"] != agent.as_str()
                && line["acting_agent"] != only_admits.as_str()),
        "{theirs:?}"
    );
    Ok(())
}
