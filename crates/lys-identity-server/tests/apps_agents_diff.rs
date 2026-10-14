#![cfg(test)]
//! A schema dry run names each change to the acts an agent may hold.

use identity_contract::apps::{Auth, NOTES, approve, login, ok, post, registration, seeded};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;
use std::error::Error;

#[tokio::test]
async fn a_schema_check_names_agent_actions_added_and_removed_per_kind()
-> Result<(), Box<dyn Error>> {
    let (service, _) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let kind = format!("{NOTES}.doc");
    let schema = json!({"kinds": {&kind: {
        "actions": ["read", "write"], "relations": {"reader": ["read"], "editor": ["read", "write"]},
        "parents": [], "agents": ["read"]
    }}});
    ok(post(
        &service,
        "/apps",
        Auth::Cookie(&cookie),
        &registration(NOTES, &schema)?,
    )
    .await?)?;
    approve(&service, &cookie, NOTES).await?;
    let mut changed = schema;
    changed["kinds"][&kind]["agents"] = json!(["write"]);
    let answer = ok(post(
        &service,
        &format!("/apps/{NOTES}/schema/check"),
        Auth::Cookie(&cookie),
        &json!({"schema": changed}),
    )
    .await?)?;
    assert_eq!(
        answer["diff"]["agents_added"],
        json!([{"kind": &kind, "name": "write"}]),
        "{answer}"
    );
    assert_eq!(
        answer["diff"]["agents_removed"],
        json!([{"kind": &kind, "name": "read"}]),
        "{answer}"
    );
    assert_eq!(answer["diff"]["actions_added"], json!([]));
    assert_eq!(answer["diff"]["actions_removed"], json!([]));
    Ok(())
}
