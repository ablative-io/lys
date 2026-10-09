#![cfg(test)]
//! A machine's pass over HTTP (ACCESS-005 R1, "a grant to the machine on a
//! liminal link kind is issued, read back and in its pass"): the machine a
//! computer's join made asks with the key that join recorded, and is
//! answered a pass lys-pass's own verifier accepts, naming the machine as
//! its holder, the administrator who asked for the code as the person
//! answering for it, and the grant it holds on the audience app's kind.
//! A request its key did not sign, a nonce sent twice, a replaced machine's
//! key and a retired computer's machine are each refused by name.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::apps::{BEA, NOTES, TestResult, login, op, registered};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_pass::{Decision, KeySet, VerifiedPass};
use lys_runner::dial::{
    EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, dial_signed_bytes, pass_route,
};
use lys_runner::protocol::{hex, nonce};
use serde_json::{Value, json};

/// The public address the reachable service is configured at, so a
/// connection code is given.
const PUBLIC: &str = "https://lys.example.test";

/// The seeded service, reachable from another computer, with Lys's provider
/// on and its signing key written.
async fn world() -> Result<(Service, Seeded), Box<dyn Error>> {
    let broker = identity_contract::app_custody::start().await?;
    Box::pin(Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.redirect_url = format!("{PUBLIC}/callback");
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: CODE_SECONDS,
                pass_seconds: PASS_SECONDS,
                rights_bytes: None,
            });
        },
        None,
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        },
    ))
    .await
}

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// Name a computer when `computer` names none, give it a code and join it
/// with `key`, answering the computer's id.
async fn join(
    service: &Service,
    cookie: &str,
    computer: Option<&str>,
    key: &Ed25519Identity,
) -> Result<String, Box<dyn Error>> {
    let computer = if let Some(computer) = computer {
        computer.to_owned()
    } else {
        let operation = op()?;
        let body = json!({"operation": operation, "name": "Liminal node", "kind": "Computer", "runtime": "lys-runner", "slots": 0, "may_run": [], "may_reach": []});
        let (status, answer) = service
            .post("/network/machines", Some(cookie), &body)
            .await?;
        assert_eq!(status, 200, "{answer}");
        operation
    };
    let (status, given) = service
        .post(
            &format!("/network/machines/{computer}/join-code"),
            Some(cookie),
            &json!({ "operation": op()? }),
        )
        .await?;
    assert_eq!(status, 200, "{given}");
    let code = given["code"].as_str().ok_or("no code was given")?;
    let body = json!({"machine": computer, "code": code, "key": hex(&key.public_key_bytes())});
    let (status, joined) = service.post("/runner/join", None, &body).await?;
    assert_eq!(status, 200, "{joined}");
    Ok(computer)
}

/// The identity of the newest machine joined.
async fn newest(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
    let (status, answer) = service
        .get("/network/machine-identities", Some(cookie))
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer["machines"]
        .as_array()
        .and_then(|machines| machines.last())
        .and_then(|machine| machine["identity"].as_str())
        .ok_or("no machine has joined")?
        .to_owned())
}

/// The computer `computer` asks for a pass with `body`, signed by `key`
/// under `once` as its nonce.
async fn ask(
    service: &Service,
    computer: &str,
    key: &Ed25519Identity,
    body: &Value,
    once: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    let epoch = reqwest::get(format!("{}{EPOCH_ROUTE}", service.base))
        .await?
        .text()
        .await?
        .trim()
        .to_owned();
    let route = pass_route(computer);
    let bytes = serde_json::to_vec(body)?;
    let signature = hex(&key.sign(&dial_signed_bytes("POST", &route, &epoch, once, &bytes)));
    let answer = reqwest::Client::new()
        .post(format!("{}{route}", service.base))
        .header(EPOCH_HEADER, &epoch)
        .header(NONCE_HEADER, once)
        .header(SIGNATURE_HEADER, signature)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(bytes)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

/// The answer's pass, verified by lys-pass for `NOTES`.
async fn verified(service: &Service, answer: &Value) -> Result<VerifiedPass, Box<dyn Error>> {
    let keys = reqwest::get(format!("{}/oauth/jwks", service.base))
        .await?
        .text()
        .await?;
    let pass = answer["access_token"].as_str().ok_or("no pass")?;
    Ok(VerifiedPass::verify(
        pass,
        &KeySet::from_json(&keys)?,
        &service.base,
        NOTES,
        now()?,
    )?)
}

/// A link grant passed to the machine `device` on `NOTES`'s document `1`,
/// under a root the administrator `person` holds: the grant's id.
async fn link_grant(
    service: &Service,
    cookie: &str,
    person: &str,
    device: &str,
) -> Result<String, Box<dyn Error>> {
    let resource = json!({"kind": format!("{NOTES}.doc"), "id": "1"});
    let window = json!({"starts_at": 0, "ends_at": null});
    let pass_on = json!({"kind": "to", "actions": ["read", "write"], "recipients": ["machine"]});
    let root = json!({"operation": op()?, "route": "api", "holder": person, "resource": resource, "relation": "editor", "pass_on": pass_on, "window": window});
    let (status, root) = service.post("/grants/roots", Some(cookie), &root).await?;
    assert_eq!(status, 200, "{root}");
    let give = json!({"operation": op()?, "route": "api", "source": root["grant"], "recipient": device, "responsible": person, "resource": resource, "relation": "editor", "pass_on": {"kind": "use_only"}, "window": window});
    let (status, given) = service.post("/grants", Some(cookie), &give).await?;
    assert_eq!(status, 200, "{given}");
    Ok(given["grant"].as_str().ok_or("no grant")?.to_owned())
}

#[tokio::test]
async fn a_machine_is_issued_a_pass_carrying_its_grant_and_its_responsible_person() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let keys = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&keys.path().join("computer.key"))?;
    let computer = join(&service, &admin, None, &key).await?;
    let device = newest(&service, &admin).await?;
    let person = seeded.people[0].id.to_string();
    let held = link_grant(&service, &admin, &person, &device).await?;

    let body = json!({"app": NOTES});
    let (status, answer) = ask(&service, &computer, &key, &body, &nonce()).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["token_type"], "Bearer");
    assert!(
        answer.get("refresh_token").is_none(),
        "a machine asks again"
    );
    assert!(answer.get("grant_binding").is_none(), "none was asked for");
    let pass = verified(&service, &answer).await?;
    let claims = pass.claims();
    assert_eq!(claims.sub, device);
    assert_eq!(claims.holder.id, device);
    assert_eq!(claims.holder.kind, "machine");
    assert_eq!(claims.holder.responsible.as_deref(), Some(person.as_str()));
    assert!(claims.exp > claims.iat && claims.exp - claims.iat <= PASS_SECONDS);
    let [right] = claims.rights.as_slice() else {
        return Err(format!("one right: {:?}", claims.rights).into());
    };
    assert_eq!(right.resource.kind, format!("{NOTES}.doc"));
    assert_eq!(right.resource.id, "1");
    assert_eq!(right.actions, ["read", "write"]);
    assert_eq!(right.grant, held);
    assert_eq!(
        pass.evaluate(&format!("{NOTES}.doc"), "1", "write", now()?)?,
        Decision::Allowed { grant: &held }
    );
    Ok(())
}

#[tokio::test]
async fn a_pass_is_refused_to_another_key_a_spent_nonce_a_replaced_key_and_a_retired_computer()
-> TestResult {
    let (service, _seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let keys = tempfile::tempdir()?;
    let first = Ed25519Identity::load_or_generate(&keys.path().join("first.key"))?;
    let other = Ed25519Identity::load_or_generate(&keys.path().join("other.key"))?;
    let node = join(&service, &admin, None, &first).await?;
    let body = json!({"app": NOTES});

    let (status, refused) = ask(&service, &node, &other, &body, &nonce()).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (401, &json!("runner_dial_refused")),
        "{refused}"
    );
    let once = nonce();
    let (status, answer) = ask(&service, &node, &first, &body, &once).await?;
    assert_eq!(status, 200, "{answer}");
    let (status, refused) = ask(&service, &node, &first, &body, &once).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (401, &json!("runner_dial_refused")),
        "{refused}"
    );
    let unknown = json!({"app": "nobody_registered"});
    let (status, refused) = ask(&service, &node, &first, &unknown, &nonce()).await?;
    assert_eq!(
        refused["refusal"], "credential_refused",
        "{status} {refused}"
    );
    let binding = json!({"app": NOTES, "grant_binding": "2"});
    let (_, refused) = ask(&service, &node, &first, &binding, &nonce()).await?;
    assert_eq!(refused["refusal"], "grant_binding_unsupported", "{refused}");

    let second = Ed25519Identity::load_or_generate(&keys.path().join("second.key"))?;
    join(&service, &admin, Some(&node), &second).await?;
    let (status, refused) = ask(&service, &node, &first, &body, &nonce()).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (401, &json!("runner_dial_refused")),
        "{refused}"
    );
    let (status, answer) = ask(&service, &node, &second, &body, &nonce()).await?;
    assert_eq!(status, 200, "{answer}");
    let pass = verified(&service, &answer).await?;
    assert_eq!(pass.claims().sub, newest(&service, &admin).await?);
    assert!(
        pass.claims().rights.is_empty(),
        "the new machine holds no grant"
    );

    let (status, retired) = service
        .post(
            &format!("/network/machines/{node}/retire"),
            Some(&admin),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    let (status, refused) = ask(&service, &node, &second, &body, &nonce()).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("HolderRetired")),
        "{refused}"
    );
    Ok(())
}
