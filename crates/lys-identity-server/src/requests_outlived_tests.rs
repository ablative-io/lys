//! A request whose relation the model no longer defines (ACCESS-011): it is
//! listed and read with `standing` `relation_unknown` and no actions, it
//! never hides the rest of the queue, its approval is refused by name before
//! any intent is kept, and its decline stands.

use serde_json::{Value, json};

use super::{TestResult, client, fixture_with, operation};
use identity_contract::apps::{Auth, put};

const APP: &str = "fixture_product";
const KIND: &str = "fixture_product.agent";

async fn ask(base: &str, pass: &str, relation: &str) -> TestResult<String> {
    let body = json!({"operation": operation()?, "resource": {"kind": KIND, "id": "shared"},
        "relation": relation, "ends_at": null, "why": "to read the fixture product"});
    let response = client()
        .post(format!("{base}/requests"))
        .header("lys-agent-pass", pass)
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let asked: Value = response.json().await?;
    assert_eq!(status, 200, "{asked}");
    Ok(asked["id"].as_str().ok_or("no id")?.to_owned())
}

/// Replace the app's schema so `viewer` is gone and `reader` carries `read`.
async fn drop_viewer(service: &identity_contract::harness::Service, cookie: &str) -> TestResult {
    // An app registered by the fixture store holds no version yet; the
    // change route counts that as version 0.
    let (status, current) = service
        .get(&format!("/apps/{APP}/schema"), Some(cookie))
        .await?;
    let replaces = if status == 200 {
        current["version"].as_u64().ok_or("no version")?
    } else {
        0
    };
    let body = json!({"operation": operation()?, "replaces": replaces,
        "schema": {"kinds": {KIND: {"actions": ["read"], "relations": {"reader": ["read"]}, "agents": ["read"]}}}});
    let (status, changed) = put(
        service,
        &format!("/apps/{APP}/schema"),
        Auth::Cookie(cookie),
        &body,
    )
    .await?;
    assert_eq!(status, 200, "{changed}");
    Ok(())
}

fn by_id<'a>(listed: &'a Value, id: &str) -> TestResult<&'a Value> {
    listed["requests"]
        .as_array()
        .ok_or("no requests")?
        .iter()
        .find(|each| each["id"] == id)
        .ok_or_else(|| format!("{id} missing from {listed}").into())
}

#[tokio::test]
async fn an_outlived_request_is_listed_as_not_grantable_beside_the_rest() -> TestResult {
    let (service, pass, cookie, _, _) = fixture_with(&[APP]).await?;
    let outlived = ask(&service.base, &pass, "viewer").await?;
    drop_viewer(&service, &cookie).await?;
    let grantable = ask(&service.base, &pass, "reader").await?;
    let (status, listed) = service.get("/requests", Some(&cookie)).await?;
    assert_eq!(status, 200, "{listed}");
    let old = by_id(&listed, &outlived)?;
    assert_eq!(old["standing"], "relation_unknown", "{old}");
    assert_eq!(old["actions"], json!([]), "{old}");
    assert_eq!(old["state"], "waiting", "{old}");
    let new = by_id(&listed, &grantable)?;
    assert_eq!(new["standing"], "grantable", "{new}");
    assert_eq!(new["actions"], json!(["read"]), "{new}");
    Ok(())
}

#[tokio::test]
async fn an_outlived_request_is_refused_on_approve_by_name_and_declines() -> TestResult {
    let (service, pass, cookie, _, _) = fixture_with(&[APP]).await?;
    let outlived = ask(&service.base, &pass, "viewer").await?;
    drop_viewer(&service, &cookie).await?;
    let approval =
        json!({"operation": operation()?, "route": "api", "source": null, "note": "yes"});
    let (status, refused) = service
        .post(
            &format!("/requests/{outlived}/approve"),
            Some(&cookie),
            &approval,
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "RelationUnknown", "{refused}");
    let (status, listed) = service.get("/requests", Some(&cookie)).await?;
    assert_eq!(status, 200, "{listed}");
    let still = by_id(&listed, &outlived)?;
    assert_eq!(still["state"], "waiting", "{still}");
    assert!(still["held_by"].is_null(), "{still}");
    let (status, declined) = service
        .post(
            &format!("/requests/{outlived}/decline"),
            Some(&cookie),
            &json!({"note": "the model no longer defines it"}),
        )
        .await?;
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["state"], "declined", "{declined}");
    assert_eq!(declined["standing"], "relation_unknown", "{declined}");
    Ok(())
}
