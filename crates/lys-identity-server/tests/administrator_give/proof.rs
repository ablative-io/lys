//! The issued pass and independent revocation use the same standing gift.

use super::{agent_pass, batch, fixture, give};
use identity_contract::apps::{Auth, NOTES, TestResult, ok, op, post, refused};
use identity_contract::harness::Service;
use serde_json::{Value, json};

pub(super) async fn pass_rights_and_refusals(
    service: &Service,
    admin: &str,
    person: &str,
    agent: &str,
    grant: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let answer = agent_pass(service, admin, agent, grant).await?;
    let pass = fixture::verified(service, &json!({"access_token": answer["pass"]}), NOTES).await?;
    assert_eq!(pass.claims().holder.id, agent);
    assert_eq!(pass.claims().rights.len(), 2);
    for (kind, id) in [("workspace", "team"), ("channel", "general")] {
        assert_eq!(
            pass.evaluate(&format!("{NOTES}.{kind}"), id, "read", fixture::now()?)?,
            lys_pass::Decision::Allowed { grant }
        );
    }
    let app_pass = fixture::signed_in(service, admin, NOTES).await?;
    let gift = give(person, agent)?;
    for token in [
        app_pass["access_token"].as_str().ok_or("no person pass")?,
        answer["pass"].as_str().ok_or("no agent pass")?,
    ] {
        refused(
            &post(service, "/grants", Auth::Bearer(token), &gift).await?,
            403,
            "RootAuthorityRefused",
        )?;
    }
    Ok(app_pass)
}

pub(super) async fn revoke_independently(
    service: &Service,
    admin: &str,
    person: &str,
    agent: &str,
    grant: &str,
    app_pass: &Value,
) -> TestResult {
    let gift = give(person, agent)?;
    let sibling = ok(post(service, "/grants", Auth::Cookie(admin), &gift).await?)?;
    let revoked = ok(post(
        service,
        &format!("/grants/{grant}/revoke"),
        Auth::Cookie(admin),
        &json!({
            "operation": op()?, "route": "api", "reason": "access ended"
        }),
    )
    .await?)?;
    let allowed = batch(service, admin, agent, &revoked["receipt"]["revision"]).await?;
    assert_eq!(
        allowed["results"][0]["allowed"], true,
        "the independent gift remains"
    );
    let sibling = sibling["grant"].as_str().ok_or("no independent grant")?;
    let revoked = ok(post(
        service,
        &format!("/grants/{sibling}/revoke"),
        Auth::Cookie(admin),
        &json!({
            "operation": op()?, "route": "api", "reason": "access ended"
        }),
    )
    .await?)?;
    let ended = batch(service, admin, agent, &revoked["receipt"]["revision"]).await?;
    assert!(
        ended["results"]
            .as_array()
            .ok_or("no ended checks")?
            .iter()
            .all(|result| result["allowed"] == false)
    );
    let refreshed = fixture::refreshed(service, NOTES, app_pass).await?;
    assert_eq!(refreshed.0, 200);
    let pass = fixture::verified(service, &refreshed.1, NOTES).await?;
    assert!(
        pass.claims()
            .rights
            .iter()
            .any(|right| right.grant == "administrator")
    );
    Ok(())
}
