#![cfg(test)]
//! Each action of an app's schema is hot or deliberate (ACCESS-001 R2, D2,
//! D4): a kind names its hot actions, decided from the pass alone, and every
//! other action is deliberate, so every schema written before the class
//! existed keeps its meaning and its bytes. A grant held by draft or by two
//! never applies to a hot action: Lys refuses to issue one, by name, writing
//! nothing, and refuses a schema change that would make a held grant's action
//! hot, as it refuses any change that strands standing grants.

use identity_contract::apps::{
    Auth, NOTES, TestResult, get, login, ok, op, post, put, refused, registered, seeded,
    workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

/// The fixture schema with `actions` of the doc kind marked hot.
fn with_hot(actions: &[&str]) -> Value {
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][format!("{NOTES}.doc")]["hot"] = json!(actions);
    schema
}

/// A root grant on doc `id` to `holder` as `relation`, in `mode`.
async fn issue(
    service: &Service,
    admin: &str,
    holder: &str,
    id: &str,
    relation: &str,
    mode: &str,
) -> Result<identity_contract::apps::Answer, Box<dyn std::error::Error>> {
    let body = json!({
        "operation": op()?,
        "route": "api",
        "holder": holder,
        "resource": {"kind": format!("{NOTES}.doc"), "id": id},
        "relation": relation,
        "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
        "mode": mode,
    });
    post(service, "/grants/roots", Auth::Cookie(admin), &body).await
}

#[tokio::test]
async fn an_action_class_round_trips_at_every_version() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_hot(&["read"])});
    ok(put(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 2, "{current}");
    assert_eq!(current["schema"], with_hot(&["read"]), "{current}");
    let first = ok(get(&service, &format!("{path}?version=1"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(
        first["schema"],
        workspace_schema(NOTES),
        "version 1 has no class"
    );
    Ok(())
}

#[tokio::test]
async fn a_hot_action_must_be_one_its_kind_declares() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_hot(&["delete"])});
    let answer = put(&service, &path, Auth::Cookie(&admin), &change).await?;
    refused(&answer, 400, "schema_invalid")?;
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 1, "nothing was written");
    Ok(())
}

#[tokio::test]
async fn a_held_grant_on_a_hot_action_is_refused_by_name_and_nothing_is_written() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_hot(&["read"])});
    ok(put(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    let before = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    for mode in ["by_draft", "by_two"] {
        let answer = issue(&service, &admin, &bea, "1", "reader", mode).await?;
        refused(&answer, 409, "grant_mode_on_hot_action")?;
        let words = answer.1["reason"].as_str().unwrap_or_default();
        for named in [NOTES, "read", "hot"] {
            assert!(words.contains(named), "{mode}: {words}");
        }
    }
    let after = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    assert_eq!(before, after, "nothing was written");
    ok(issue(&service, &admin, &bea, "1", "reader", "outright").await?)?;
    Ok(())
}

#[tokio::test]
async fn making_a_held_grants_action_hot_is_refused_naming_the_count() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    for id in ["1", "2"] {
        ok(issue(&service, &admin, &bea, id, "editor", "by_draft").await?)?;
    }
    let path = format!("/apps/{NOTES}/schema");
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_hot(&["write"])});
    let answer = put(&service, &path, Auth::Cookie(&admin), &change).await?;
    refused(&answer, 409, "schema_change_strands_grants")?;
    assert_eq!(answer.1["fields"][0]["count"], 2, "{}", answer.1);
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 1, "nothing was written");
    Ok(())
}
