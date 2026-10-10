#![cfg(test)]
//! An agent's pass to an app over HTTP (AGENTS-006 R1, R2, R4): an agent
//! that proves itself with a grant credential its responsible person issued
//! for one of its grants is answered a pass lys-pass's own verifier accepts
//! for the audience app, naming the agent as its holder and its responsible
//! person as the one answering for it, and carrying exactly the agent's own
//! grants on that app's kinds, never the person's. No proof, a credential
//! that does not admit, another agent's credential, an audience that is not
//! an approved app, a retired agent and a revoked credential are each
//! refused by name.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::apps::{
    BEA, FILES, NOTES, TestResult, login, ok, op, refused, register, registered, root,
};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_pass::{Decision, KeySet, VerifiedPass};
use serde_json::{Value, json};

/// The header a grant credential is carried in.
const CREDENTIAL: &str = lys_identity_server::grant_tokens::HEADER;

/// The seeded service with Lys's provider on, its signing key written.
async fn world() -> Result<(Service, Seeded), Box<dyn Error>> {
    let broker = identity_contract::app_custody::start().await?;
    Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
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
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        },
    )
    .await
}

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// What a test needs: the service, the administrator's cookie, the
/// administrator (the person the seeded agents answer to), the active agent
/// Scribe and the registered agent Courier, both hers.
struct World {
    service: Service,
    admin: String,
    person: String,
    scribe: String,
    courier: String,
}

async fn opened() -> Result<World, Box<dyn Error>> {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let ada = &seeded.people[0];
    let [scribe, courier, ..] = ada.agents.as_slice() else {
        return Err("the seed gives the administrator three agents".into());
    };
    Ok(World {
        service,
        admin,
        person: ada.id.to_string(),
        scribe: scribe.id.to_string(),
        courier: courier.id.to_string(),
    })
}

/// A grant of `NOTES`'s document `doc` passed to `agent` under a root the
/// person holds, the person answering for it: the grant's id.
async fn given(world: &World, agent: &str, doc: &str) -> Result<String, Box<dyn Error>> {
    let resource = json!({"kind": format!("{NOTES}.doc"), "id": doc});
    let window = json!({"starts_at": 0, "ends_at": null});
    let pass_on = json!({"kind": "to", "actions": ["read", "write"], "recipients": ["agent"]});
    let root = json!({"operation": op()?, "route": "api", "holder": world.person, "resource": resource, "relation": "editor", "pass_on": pass_on, "window": window});
    let (status, root) = world
        .service
        .post("/grants/roots", Some(&world.admin), &root)
        .await?;
    assert_eq!(status, 200, "{root}");
    let give = json!({"operation": op()?, "route": "api", "source": root["grant"], "recipient": agent, "responsible": world.person, "resource": resource, "relation": "editor", "pass_on": {"kind": "use_only"}, "window": window});
    let (status, given) = world
        .service
        .post("/grants", Some(&world.admin), &give)
        .await?;
    assert_eq!(status, 200, "{given}");
    Ok(given["grant"].as_str().ok_or("no grant")?.to_owned())
}

/// A grant credential for `grant`, issued by the person responsible for it:
/// its id and its value.
async fn credential(world: &World, grant: &str) -> Result<(String, String), Box<dyn Error>> {
    let expires_at = now()? + 300;
    let (status, issued) = world
        .service
        .post(
            &format!("/grants/{grant}/tokens"),
            Some(&world.admin),
            &json!({ "expires_at": expires_at }),
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let id = issued["id"].as_str().ok_or("no credential id")?.to_owned();
    let token = issued["token"].as_str().ok_or("no credential")?.to_owned();
    Ok((id, token))
}

/// `agent` asks for its pass with `body`, carrying `headers`.
async fn ask(
    world: &World,
    agent: &str,
    headers: &[(&str, &str)],
    body: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let path = format!("/agents/{agent}/pass");
    let bytes = serde_json::to_vec(body)?;
    world.service.post_carrying(&path, headers, bytes).await
}

/// The answer's pass, verified by lys-pass for the audience `app`.
async fn verified(
    world: &World,
    answer: &Value,
    app: &str,
) -> Result<VerifiedPass, Box<dyn Error>> {
    let keys = reqwest::get(format!("{}/oauth/jwks", world.service.base))
        .await?
        .text()
        .await?;
    let pass = answer["pass"].as_str().ok_or("no pass")?;
    let keys = KeySet::from_json(&keys)?;
    let base = &world.service.base;
    Ok(VerifiedPass::verify(pass, &keys, base, app, now()?)?)
}

#[tokio::test]
async fn an_agent_is_issued_a_pass_carrying_exactly_its_own_grants() -> TestResult {
    let world = opened().await?;
    let scribe = world.scribe.clone();
    let held = given(&world, &scribe, "1").await?;
    let notes_doc = format!("{NOTES}.doc");
    // The person's own right, which the agent's pass must never carry.
    let own = root(
        &world.service,
        &world.admin,
        &world.person,
        (&notes_doc, "2"),
        "editor",
    )
    .await?;
    ok(own)?;
    let (_, token) = credential(&world, &held).await?;

    let body = json!({ "audience": NOTES });
    let with = [(CREDENTIAL, token.as_str())];
    let (status, answer) = ask(&world, &scribe, &with, &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["audience"], NOTES);
    assert!(
        answer.get("refresh_token").is_none(),
        "an agent asks again"
    );
    assert!(
        !answer.to_string().contains(&token),
        "the credential is never answered"
    );
    let pass = verified(&world, &answer, NOTES).await?;
    let claims = pass.claims();
    assert_eq!(answer["expires_at"], claims.exp);
    assert_eq!(claims.sub, world.scribe);
    assert_eq!(claims.holder.id, world.scribe);
    assert_eq!(claims.holder.kind, "agent");
    assert_eq!(
        claims.holder.responsible.as_deref(),
        Some(world.person.as_str())
    );
    assert!(claims.exp > claims.iat && claims.exp - claims.iat <= PASS_SECONDS);
    let [right] = claims.rights.as_slice() else {
        return Err(format!("one right: {:?}", claims.rights).into());
    };
    assert_eq!(right.resource.kind, notes_doc);
    assert_eq!(right.resource.id, "1");
    assert_eq!(right.actions, ["read", "write"]);
    assert_eq!(right.grant, held);
    assert_eq!(
        pass.evaluate(&notes_doc, "1", "write", now()?)?,
        Decision::Allowed { grant: &held }
    );
    assert_eq!(
        pass.evaluate(&notes_doc, "2", "read", now()?)?,
        Decision::Refused,
        "the person's own right does not leak into the agent's pass"
    );
    let token = answer["pass"].as_str().ok_or("no pass")?;
    let keys = reqwest::get(format!("{}/oauth/jwks", world.service.base))
        .await?
        .text()
        .await?;
    let elsewhere = VerifiedPass::verify(
        token,
        &KeySet::from_json(&keys)?,
        &world.service.base,
        FILES,
        now()?,
    );
    assert!(
        matches!(elsewhere, Err(lys_pass::Error::WrongAudience)),
        "a pass for one app is refused as another's"
    );
    Ok(())
}

#[tokio::test]
async fn each_refusal_is_by_name() -> TestResult {
    let world = opened().await?;
    let scribe = world.scribe.clone();
    let held = given(&world, &scribe, "1").await?;
    let (_, token) = credential(&world, &held).await?;
    let body = json!({ "audience": NOTES });
    let with = [(CREDENTIAL, token.as_str())];

    let answer = ask(&world, &scribe, &[], &body).await?;
    refused(&answer, 401, "agent_pass_unproven")?;
    let unknown = "a".repeat(43);
    let not_issued = [(CREDENTIAL, unknown.as_str())];
    let answer = ask(&world, &scribe, &not_issued, &body).await?;
    refused(&answer, 401, "agent_pass_unproven")?;
    let with_cookie = [
        (CREDENTIAL, token.as_str()),
        ("cookie", world.admin.as_str()),
    ];
    let answer = ask(&world, &scribe, &with_cookie, &body).await?;
    refused(&answer, 401, "agent_pass_unproven")?;
    assert!(!answer.1.to_string().contains(&token), "{}", answer.1);

    let courier = world.courier.clone();
    let answer = ask(&world, &courier, &with, &body).await?;
    refused(&answer, 403, "agent_not_holder")?;

    let unknown_app = json!({ "audience": "nobody_registered" });
    let answer = ask(&world, &scribe, &with, &unknown_app).await?;
    refused(&answer, 400, "audience_not_approved")?;
    register(&world.service, &world.admin, FILES).await?;
    let pending = json!({ "audience": FILES });
    let answer = ask(&world, &scribe, &with, &pending).await?;
    refused(&answer, 400, "audience_not_approved")?;
    let extra = json!({ "audience": NOTES, "refresh": true });
    let answer = ask(&world, &scribe, &with, &extra).await?;
    refused(&answer, 400, "RequestMalformed")?;

    let run_pass = "b".repeat(43);
    let pass_only = [("lys-agent-pass", run_pass.as_str())];
    let answer = ask(&world, &scribe, &pass_only, &body).await?;
    refused(&answer, 401, "AgentPassRefused")?;
    let both = [
        ("lys-agent-pass", run_pass.as_str()),
        (CREDENTIAL, token.as_str()),
    ];
    let answer = ask(&world, &scribe, &both, &body).await?;
    refused(&answer, 401, "AgentPassRefused")?;

    let retire = json!({"operation": op()?, "transition": "retire", "reason": "done"});
    let (status, retired) = world
        .service
        .post(
            &format!("/identities/{scribe}/transitions"),
            Some(&world.admin),
            &retire,
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    let answer = ask(&world, &scribe, &with, &body).await?;
    refused(&answer, 403, "agent_retired")?;
    Ok(())
}

#[tokio::test]
async fn a_revoked_credential_is_refused_at_the_next_ask() -> TestResult {
    let world = opened().await?;
    let scribe = world.scribe.clone();
    let held = given(&world, &scribe, "1").await?;
    let (id, token) = credential(&world, &held).await?;
    let body = json!({ "audience": NOTES });
    let with = [(CREDENTIAL, token.as_str())];
    let (status, answer) = ask(&world, &scribe, &with, &body).await?;
    assert_eq!(status, 200, "{answer}");

    let (status, revoked) = world
        .service
        .post(
            &format!("/grants/{held}/tokens/{id}/revoke"),
            Some(&world.admin),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let answer = ask(&world, &scribe, &with, &body).await?;
    refused(&answer, 401, "agent_pass_unproven")?;
    let reason = answer.1["reason"].as_str().ok_or("no reason")?;
    assert!(reason.contains("GrantTokenRevoked"), "{reason}");
    assert!(!reason.contains(&token), "{reason}");
    Ok(())
}
