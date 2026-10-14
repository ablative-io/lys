#![cfg(test)]
//! The model endpoint serves each approved app's own agent action declaration.

use identity_contract::apps::{Auth, NOTES, approve, login, ok, post, registration, seeded};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;
use std::error::Error;

#[tokio::test]
async fn the_served_model_names_agent_actions_and_exact_app_carriers() -> Result<(), Box<dyn Error>>
{
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
    let (status, model) = service.get("/grants/model", Some(&cookie)).await?;
    assert_eq!(status, 200, "{model}");
    assert_eq!(model["kinds"][&kind]["agents"], json!(["read"]), "{model}");
    assert_eq!(
        model["kinds"][&kind]["relations"]["reader"],
        json!(["read"])
    );
    assert_eq!(
        model["kinds"][&kind]["relations"]["editor"],
        json!(["read", "write"])
    );
    Ok(())
}
