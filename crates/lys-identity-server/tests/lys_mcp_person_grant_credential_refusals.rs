//! Grant routes preserve the credential guard's documented refusal.

use identity_contract::apps::{Auth, get, login, post};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use serde_json::{Value, json};
use std::error::Error;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn root_body(holder: &str) -> Result<Value> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "api",
        "holder": holder,
        "resource": {"kind": "doc", "id": "credential-refusal"},
        "relation": "alpha",
        "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["person"]},
        "window": {"starts_at": 0, "ends_at": null}
    }))
}

async fn assert_documented_credential_refusal(route: &str) -> Result {
    let (service, seeded) = Service::start_with(|config| {
        Ok(lys_identity_server::dev_seed::seed_configured(
            config,
            [ADMINISTRATOR, "recipient"],
        )?)
    })
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id.to_string();
    let recipient = seeded.people[1].id.to_string();
    let source_body = root_body(&person)?;
    let (status, source) = service
        .post("/grants/roots", Some(&cookie), &source_body)
        .await?;
    assert_eq!(status, 200, "fixture root: {source}");
    let body = if route == "/grants/roots" {
        root_body(&recipient)?
    } else {
        json!({
            "operation": OperationId::generate()?.to_string(),
            "route": "api",
            "source": source["grant"],
            "recipient": recipient,
            "responsible": recipient,
            "resource": {"kind": "doc", "id": "credential-refusal"},
            "relation": "beta",
            "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null}
        })
    };
    let credential = "not-a-held-credential";
    let expected = get(&service, "/apps/me", Auth::Bearer(credential)).await?;
    assert_eq!(expected.0, 401, "ordinary credential guard status");
    assert_eq!(expected.1["refusal"], "credential_refused");
    let before = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(before.0, 200, "grant book before refused request");
    let refused = post(&service, route, Auth::Bearer(credential), &body).await?;
    assert_eq!(
        refused.0, expected.0,
        "{route} must preserve credential_refused, never RefusalUndocumented: {}",
        refused.1
    );
    assert_eq!(refused.1, expected.1, "{route} credential guard body");
    let after = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(after, before, "refused credential must not change grants");
    let accepted = service.post(route, Some(&cookie), &body).await?;
    assert_eq!(
        accepted.0, 200,
        "browser control for {route}: {}",
        accepted.1
    );
    Ok(())
}

#[tokio::test]
async fn passing_on_a_grant_preserves_the_documented_credential_refusal() -> Result {
    assert_documented_credential_refusal("/grants").await
}

#[tokio::test]
async fn issuing_a_root_grant_preserves_the_documented_credential_refusal() -> Result {
    assert_documented_credential_refusal("/grants/roots").await
}
