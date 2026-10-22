//! The administrator's setup roots can give an agent access, and no grant,
//! from them or any other, gives an agent an act withheld from agents.

use std::error::Error;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::apps::{Auth, get, login, op, post};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::OperationId;
use lys_identity::grants::{WITHHELD_FROM_AGENTS, shipped_model};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};
use zeroize::Zeroizing;

type TestResult = Result<(), Box<dyn Error>>;

fn ok((status, answer): (u16, Value)) -> Result<Value, Box<dyn Error>> {
    if status != 200 {
        return Err(format!("status {status}: {answer}").into());
    }
    Ok(answer)
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

/// The grant log's revision: what a refusal must leave unchanged.
async fn revision(service: &Service, cookie: &str) -> Result<u64, Box<dyn Error>> {
    let grants = ok(get(service, "/grants", Auth::Cookie(cookie)).await?)?;
    Ok(grants["revision"].as_u64().ok_or("no revision")?)
}

/// A service on `model` whose administrator has finished setup, with their
/// signed-in cookie and person.
async fn set_up_on(model: &str) -> Result<(Service, String, String), Box<dyn Error>> {
    let service = Service::start_adjusted(model, None, None, None, |_| {}, |_| Ok(()))
        .await?
        .0;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = ok(post(
        &service,
        "/setup",
        Auth::Cookie(&cookie),
        &json!({"operation": op()?, "display_name": "Administrator"}),
    )
    .await?)?;
    let person = person["person"].as_str().ok_or("no person")?.to_owned();
    Ok((service, cookie, person))
}

async fn set_up() -> Result<(Service, String, String), Box<dyn Error>> {
    set_up_on(&shipped_model()).await
}

/// The administrator's grants passable to an agent.
async fn agent_roots(service: &Service, cookie: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let grants = ok(get(service, "/grants", Auth::Cookie(cookie)).await?)?;
    Ok(grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter(|grant| {
            grant["pass_on"]["recipients"]
                .as_array()
                .is_some_and(|recipients| recipients.contains(&json!("agent")))
        })
        .cloned()
        .collect())
}

#[tokio::test]
async fn setup_gives_the_administrator_roots_an_agent_can_be_given_access_through() -> TestResult {
    let (service, cookie, person) = set_up().await?;
    let auth = Auth::Cookie(&cookie);
    let roots = agent_roots(&service, &cookie).await?;
    let directory: Vec<&Value> = roots
        .iter()
        .filter(|root| root["resource"]["kind"] == "directory")
        .collect();
    assert_eq!(directory.len(), 2, "{roots:?}");
    for root in &directory {
        let passable = root["pass_on"]["actions"].as_array().ok_or("no actions")?;
        assert!(passable.contains(&json!("agent.start")), "{root}");
    }
    // AGENTS-017 R4: beside them, the root on the administrator's own person
    // the one standing choice gives from, and the one on the computer Lys runs
    // on, named at setup.
    let own = roots
        .iter()
        .find(|root| root["resource"] == json!({"kind": "person", "id": person}))
        .ok_or("no root on the administrator's own person")?;
    assert_eq!(
        own["pass_on"]["actions"],
        json!([
            "agent.create",
            "agent.provisioning.review",
            "agent.provisioning.set"
        ]),
        "{own}"
    );
    let computer = roots
        .iter()
        .find(|root| root["resource"]["kind"] == "machine")
        .ok_or("no root on the computer Lys runs on")?;
    assert_eq!(
        computer["pass_on"]["actions"],
        json!(["machine.agent.set"]),
        "{computer}"
    );
    assert_eq!(roots.len(), 4, "{roots:?}");
    for root in &roots {
        assert_eq!(root["holder"], person.as_str());
        let passable = root["pass_on"]["actions"].as_array().ok_or("no actions")?;
        for withheld in WITHHELD_FROM_AGENTS {
            assert!(!passable.contains(&json!(withheld)), "{withheld} in {root}");
        }
    }
    let recorded = revision(&service, &cookie).await?;
    let again = ok(post(&service, "/grants/agent-roots", auth, &json!({})).await?)?;
    assert_eq!(
        revision(&service, &cookie).await?,
        recorded,
        "asking again writes nothing"
    );
    let mut ids: Vec<&str> = roots
        .iter()
        .filter_map(|root| root["id"].as_str())
        .collect();
    ids.sort_unstable();
    let mut repeated: Vec<&str> = again["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    repeated.sort_unstable();
    assert_eq!(repeated, ids, "asking again records nothing new");

    let agent = ok(post(
        &service,
        "/agents",
        auth,
        &json!({"operation": op()?, "display_name": "Helper"}),
    )
    .await?)?;
    let agent = agent["agent"].as_str().ok_or("no agent")?.to_owned();
    ok(post(
        &service,
        &format!("/identities/{agent}/transitions"),
        auth,
        &json!({"operation": op()?, "transition": "activate", "reason": ""}),
    )
    .await?)?;
    let source = roots
        .iter()
        .find(|root| root["resource"]["id"] == "agents")
        .ok_or("no agents root")?;
    let give = |relation: &str, pass_on: Value| -> Result<Value, Box<dyn Error>> {
        Ok(
            json!({"operation": op()?, "route": "api", "source": source["id"], "recipient": agent, "responsible": person, "resource": source["resource"], "relation": relation, "pass_on": pass_on, "window": {"starts_at": 0, "ends_at": null}}),
        )
    };
    let use_only = json!({"kind": "use_only"});
    ok(post(
        &service,
        "/grants",
        auth,
        &give("only.agent.start", use_only.clone())?,
    )
    .await?)?;

    // The roots pass on nothing withheld, so editor is outside them.
    let (status, refused) = post(&service, "/grants", auth, &give("editor", use_only)?).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "ActionsOutside", "{refused}");

    let before = revision(&service, &cookie).await?;
    // No root may let an agent be passed a person's credential (AGENTS-019 R1).
    let (status, refused) = post(
        &service,
        "/grants/roots",
        auth,
        &json!({"operation": op()?, "route": "api", "holder": person, "resource": source["resource"], "relation": "editor", "pass_on": {"kind": "to", "actions": ["agent.stop", "grant.delegate", "person.password.set", "role.create"], "recipients": ["agent"]}, "window": {"starts_at": 0, "ends_at": null}}),
    )
    .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "WithheldFromAgents", "{refused}");
    // Nor may an agent's grant let it pass one on.
    let onward = json!({"kind": "to", "actions": ["agent.stop", "person.sign-in.set"], "recipients": ["agent"]});
    let (status, refused) =
        post(&service, "/grants", auth, &give("only.agent.stop", onward)?).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "WithheldFromAgents", "{refused}");
    assert_eq!(
        revision(&service, &cookie).await?,
        before,
        "a refusal writes nothing to the grant log"
    );
    Ok(())
}

#[tokio::test]
async fn asking_for_agent_roots_on_a_model_with_no_editor_is_refused_by_name() -> TestResult {
    let model = json!({"version": 1, "relations": {"viewer": ["view"]}}).to_string();
    let (service, cookie, _) = set_up_on(&model).await?;
    let before = revision(&service, &cookie).await?;
    let (status, refused) = post(
        &service,
        "/grants/agent-roots",
        Auth::Cookie(&cookie),
        &json!({}),
    )
    .await?;
    assert_eq!(refused["refusal"], "RelationUnknown", "{status}: {refused}");
    assert!((400..500).contains(&status), "{status}: {refused}");
    assert_eq!(
        revision(&service, &cookie).await?,
        before,
        "a refusal writes nothing to the grant log"
    );
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_may_ask_for_agent_roots() -> TestResult {
    let (service, cookie, _) = set_up().await?;
    let stranger = service.sign_in(login("stranger")).await?;
    let before = revision(&service, &cookie).await?;
    let (status, refused) = post(
        &service,
        "/grants/agent-roots",
        Auth::Cookie(&stranger),
        &json!({}),
    )
    .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert_eq!(
        revision(&service, &cookie).await?,
        before,
        "a refusal writes nothing to the grant log"
    );
    Ok(())
}

/// AGENTS-019 R1, after Tom's "assign everything to AI" (22 October 2026):
/// a person may give an agent every shipped act but a person's credentials,
/// so exactly setting a password, a sign-in and a bound login stay withheld.
/// The acts AGENTS-015 R1 released, and the twenty-nine this brief releases,
/// are each one a person may give.
#[test]
fn the_withheld_list_keeps_exactly_a_persons_credentials() {
    let kept = [
        "person.login.bind",
        "person.password.set",
        "person.sign-in.set",
    ];
    assert_eq!(WITHHELD_FROM_AGENTS, kept);
    for released in [
        "agent.provisioning.set",
        "agent.provisioning.review",
        "machine.agent.set",
        "grant.delegate",
        "grant.revoke",
        "agent.certificate.issue",
        "grant",
        "organisation.admit",
        "person.email.set",
        "request.approve",
        "secret.add",
        "runtime.stop-everything",
    ] {
        assert!(!WITHHELD_FROM_AGENTS.contains(&released), "{released}");
    }
}

/// The one standing choice's four grants, in its order, from the roots on
/// the administrator's own person and on the computer, to `agent`; their ids.
async fn give_four(
    service: &Service,
    auth: Auth<'_>,
    (own, computer): (&Value, &Value),
    (person, agent): (&str, &str),
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut held = Vec::new();
    for (source, action) in [
        (own, "agent.create"),
        (computer, "machine.agent.set"),
        (own, "agent.provisioning.set"),
        (own, "agent.provisioning.review"),
    ] {
        let body = json!({"operation": op()?, "route": "api", "source": source["id"],
            "recipient": agent, "responsible": person, "resource": source["resource"],
            "relation": format!("only.{action}"), "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null}});
        let grant = ok(post(service, "/grants", auth, &body).await?)?;
        held.push(grant["grant"].as_str().ok_or("no grant")?.to_owned());
    }
    Ok(held)
}

/// The computer Lys runs on, as `GET /network` lists it, with its runner.
async fn own_computers(service: &Service, cookie: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let network = ok(get(service, "/network", Auth::Cookie(cookie)).await?)?;
    Ok(network["machines"]
        .as_array()
        .ok_or("no machines")?
        .iter()
        .filter(|machine| machine["runtime"] == "lys-runner" && machine["kind"] == "Computer")
        .cloned()
        .collect())
}

/// AGENTS-017 R4: an administrator set up before their roots holds none an
/// agent can be given from; the one standing choice posts agent-roots first,
/// which names the computer Lys runs on once, then gives four grants, and the
/// agent reads all four back as its own on `GET /agent/grants`.
#[tokio::test]
async fn a_standing_give_after_agent_roots_leaves_the_agent_four_grants() -> TestResult {
    let (service, seeded) = Service::start_asking(&shipped_model(), None, None, |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
    })
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let auth = Auth::Cookie(&cookie);
    let person = seeded.people[0].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    assert!(
        agent_roots(&service, &cookie).await?.is_empty(),
        "seeded without setup, the administrator holds no root to give an agent from"
    );
    assert!(own_computers(&service, &cookie).await?.is_empty());

    let given = ok(post(&service, "/grants/agent-roots", auth, &json!({})).await?)?;
    let computers = own_computers(&service, &cookie).await?;
    assert_eq!(computers.len(), 1, "{computers:?}");
    let machine = computers[0]["id"]
        .as_str()
        .ok_or("no machine id")?
        .to_owned();
    assert_eq!(computers[0]["named_by"], person.as_str(), "{computers:?}");
    let runner = ok(get(
        &service,
        &format!("/network/machines/{machine}/runner"),
        auth,
    )
    .await?)?;
    assert_eq!(runner["runner"]["kind"], "lys", "{runner}");
    // Asked again, nothing new: the same roots, the same one computer.
    let again = ok(post(&service, "/grants/agent-roots", auth, &json!({})).await?)?;
    assert_eq!(again["grants"], given["grants"]);
    assert_eq!(own_computers(&service, &cookie).await?.len(), 1);

    let roots = agent_roots(&service, &cookie).await?;
    let on = |resource: Value| -> Result<Value, Box<dyn Error>> {
        roots
            .iter()
            .find(|root| root["resource"] == resource)
            .cloned()
            .ok_or_else(|| format!("no root on {resource}").into())
    };
    let own = on(json!({"kind": "person", "id": person}))?;
    let computer = on(json!({"kind": "machine", "id": machine}))?;
    let held = give_four(&service, auth, (&own, &computer), (&person, &agent)).await?;

    // The agent, certified, reads its own grants by its own signature.
    let key = Arc::new(Ed25519Identity::from_seed(&Zeroizing::new([4; 32])));
    let request = STANDARD.encode(create_certificate_request(&key, &agent)?);
    ok(post(
        &service,
        &format!("/agents/{agent}/certificates"),
        auth,
        &json!({"operation": op()?, "request": request}),
    )
    .await?)?;
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = hex(OperationId::generate()?.as_bytes());
    let cose =
        sign_attestation(&payload("GET", "/agent/grants", &[], at, &nonce), &key).to_cose_bytes();
    let signature = format!("{agent} {at} {nonce} {}", hex(&cose));
    let read: Value = reqwest::Client::new()
        .get(format!("{}/agent/grants", service.base))
        .header(HEADER, signature)
        .send()
        .await?
        .json()
        .await?;
    let own_grants: Vec<&str> = read["grants"]
        .as_array()
        .ok_or_else(|| format!("no grants: {read}"))?
        .iter()
        .filter_map(|grant| grant["grant"].as_str())
        .collect();
    for grant in &held {
        assert!(
            own_grants.contains(&grant.as_str()),
            "{grant} not held: {read}"
        );
    }
    assert_eq!(held.len(), 4);
    Ok(())
}

#[path = "support/runner_start.rs"]
pub mod support;

#[path = "agent_roots/unnamed_at_setup.rs"]
mod unnamed_at_setup;
