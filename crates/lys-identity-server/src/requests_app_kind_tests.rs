//! An ask for access, and a role's template, on an app kind resolve the
//! relation by that kind (`Model::actions_on`), not by Lys's own table: before
//! this, every request on a `liminal.*` or `haematite.*` resource answered
//! `RelationUnknown`, so no agent could ask for access to any app resource, and
//! no role could carry a grant on one (Hermes, 16 Oct 2026).

use serde_json::{Value, json};

use super::{TestResult, client, fixture_with, operation};
use crate::apps_state::{Approved, By, Client, Line, Registered};

const APP: &str = "fixture_product";
const KIND: &str = "fixture_product.agent";

/// Register and approve `app` with one kind, `<app>.agent`, whose `viewer`
/// carries `read`.
pub(super) fn register(apps: &mut crate::apps_store::AppStore, app: &str) -> TestResult {
    apps.keep(Line::Registered(Registered {
        operation: format!("register-{app}"),
        app: app.to_owned(),
        name: app.to_owned(),
        redirects: vec!["https://app.test/callback".to_owned()],
        schema: json!({"kinds": {format!("{app}.agent"): {"actions": ["read"], "relations": {"viewer": ["read"]}, "agents": ["read"]}}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))?;
    apps.keep(Line::Approved(Approved {
        operation: format!("approve-{app}"),
        app: app.to_owned(),
        client: Client {
            client_id: app.to_owned(),
            secret_sha256: crate::apps_binding::sha256_hex("fixture"),
        },
        binding: None,
        by: By::Start,
        at: 2,
    }))?;
    Ok(())
}

async fn ask(base: &str, pass: &str, relation: &str) -> TestResult<(u16, Value)> {
    let body = json!({"operation": operation()?, "resource": {"kind": KIND, "id": "shared"},
        "relation": relation, "ends_at": null, "why": "to read the fixture product"});
    let response = client()
        .post(format!("{base}/requests"))
        .header("lys-agent-pass", pass)
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    Ok((status, response.json().await?))
}

fn role(relation: &str) -> TestResult<Value> {
    Ok(json!({
        "operation": operation()?, "name": format!("Reads the product ({relation})"),
        "responsibilities": "Read the product", "goals": "Accurate reads",
        "practice": "Read, never write", "profile": "", "note": "First version",
        "grant_templates": [{"resource": {"kind": KIND, "id": "shared"}, "relation": relation, "days": null}]
    }))
}

#[tokio::test]
async fn an_agent_asks_for_an_app_kind_and_its_relation_resolves_by_that_kind() -> TestResult {
    let (service, pass, _, _, agent) = fixture_with(&[APP]).await?;
    let (status, asked) = ask(&service.base, &pass, "viewer").await?;
    assert_eq!(status, 200, "{asked}");
    assert_eq!(asked["asked_by"], agent, "{asked}");
    assert_eq!(asked["resource"]["kind"], KIND, "{asked}");
    assert_eq!(asked["relation"], "viewer", "{asked}");
    assert_eq!(asked["actions"], json!(["read"]), "{asked}");
    assert_eq!(asked["state"], "waiting", "{asked}");
    let (status, refused) = ask(&service.base, &pass, "operator").await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "RelationUnknown", "{refused}");
    Ok(())
}

#[tokio::test]
async fn a_role_template_on_an_app_kind_resolves_its_relation_by_that_kind() -> TestResult {
    let (service, _, cookie, _, _) = fixture_with(&[APP]).await?;
    let (status, made) = service
        .post("/roles", Some(&cookie), &role("viewer")?)
        .await?;
    assert_eq!(status, 200, "{made}");
    let (status, refused) = service
        .post("/roles", Some(&cookie), &role("operator")?)
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "RelationUnknown", "{refused}");
    Ok(())
}
