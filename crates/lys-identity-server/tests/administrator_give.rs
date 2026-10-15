#![cfg(test)]
//! Standing gives a recorded, bounded and revocable grant without an intermediate root.

use identity_contract::apps::{
    Auth, BEA, NOTES, TestResult, check, get, login, ok, op, post, put, refused, registered,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

#[path = "provider_rights/fixture.rs"]
mod fixture;
#[path = "administrator_give/proof.rs"]
mod proof;
#[path = "administrator_give/storage.rs"]
mod storage;
use proof::{pass_rights_and_refusals, revoke_independently};

fn give(person: &str, agent: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "operation": op()?, "route": "api", "source": "administrator",
        "recipient": agent, "responsible": person,
        "resource": {"kind": format!("{NOTES}.workspace"), "id": "team"},
        "relation": "member", "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null}
    }))
}

async fn approve_agent_actions(service: &Service, admin: &str) -> TestResult {
    let path = format!("/apps/{NOTES}/schema");
    let current = ok(get(service, &path, Auth::Cookie(admin)).await?)?;
    let mut schema = current["schema"].clone();
    for kind in ["workspace", "channel"] {
        schema["kinds"][format!("{NOTES}.{kind}")]["agents"] = json!(["read"]);
    }
    let changed = ok(put(
        service,
        &path,
        Auth::Cookie(admin),
        &json!({
            "operation": op()?, "replaces": current["version"], "schema": schema
        }),
    )
    .await?)?;
    assert_eq!(changed["applied"], true, "{changed}");
    Ok(())
}

async fn batch(
    service: &Service,
    admin: &str,
    agent: &str,
    revision: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    ok(post(
        service,
        "/grants/check/batch",
        Auth::Cookie(admin),
        &json!({
            "checks": [check(agent, &format!("{NOTES}.workspace"), "team", "read"),
                check(agent, &format!("{NOTES}.channel"), "general", "read")],
            "at_least": revision
        }),
    )
    .await?)
}

async fn agent_pass(
    service: &Service,
    admin: &str,
    agent: &str,
    grant: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let (status, credential) = service
        .post(
            &format!("/grants/{grant}/tokens"),
            Some(admin),
            &json!({"expires_at": fixture::now()? + 300}),
        )
        .await?;
    assert_eq!(status, 200, "credential issue status");
    let credential = credential["token"].as_str().ok_or("no grant credential")?;
    let (status, answer) = service
        .post_carrying(
            &format!("/agents/{agent}/pass"),
            &[(lys_identity_server::grant_tokens::HEADER, credential)],
            serde_json::to_vec(&json!({"audience": NOTES}))?,
        )
        .await?;
    assert_eq!(status, 200, "agent pass issue status");
    Ok(answer)
}

#[tokio::test]
async fn standing_gives_own_agent_membership_reach_which_pass_rights_and_revocation() -> TestResult
{
    let (service, seeded) = fixture::world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    approve_agent_actions(&service, &admin).await?;
    let before = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    assert_eq!(before["grants"], json!([]));
    ok(post(
        &service,
        &format!("/apps/{NOTES}/placements"),
        Auth::Bearer(&credential),
        &json!({
            "operation": op()?, "child": {"kind": format!("{NOTES}.channel"), "id": "general"},
            "parent": {"kind": format!("{NOTES}.workspace"), "id": "team"}, "restricted": false
        }),
    )
    .await?)?;
    let body = give(&person, &agent)?;
    let recorded = ok(post(&service, "/grants", Auth::Cookie(&admin), &body).await?)?;
    let grant = recorded["grant"].as_str().ok_or("no recorded grant")?;
    let revision = &recorded["receipt"]["revision"];
    assert!(revision.as_u64().is_some_and(|revision| revision > 0));
    assert_eq!(recorded["operation"], body["operation"]);
    assert_eq!(
        ok(post(&service, "/grants", Auth::Cookie(&admin), &body).await?)?,
        recorded
    );
    let listed = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    assert_eq!(listed["grants"].as_array().ok_or("no grants")?.len(), 1);
    assert_eq!(listed["grants"][0]["source"], "administrator");
    assert_eq!(listed["grants"][0]["holder"], agent);
    assert_eq!(listed["grants"][0]["standing"]["stands"], true);
    let allowed = batch(&service, &admin, &agent, revision).await?;
    for result in allowed["results"].as_array().ok_or("no check results")? {
        assert_eq!(result["allowed"], true, "{result}");
        assert_eq!(result["grant"], grant);
        assert_eq!(result["path"], json!([grant, "administrator"]));
    }
    for (kind, id) in [("workspace", "team"), ("channel", "general")] {
        let which = ok(post(
            &service,
            "/grants/which",
            Auth::Bearer(&credential),
            &json!({
                "subject": agent, "kind": format!("{NOTES}.{kind}"), "action": "read",
                "page_size": 10, "after": null, "at_least": revision
            }),
        )
        .await?)?;
        assert_eq!(which["ids"], json!([id]), "{which}");
    }
    let app_pass = pass_rights_and_refusals(&service, &admin, &person, &agent, grant).await?;
    revoke_independently(&service, &admin, &person, &agent, grant, &app_pass).await?;
    Ok(())
}

#[tokio::test]
async fn standing_refuses_other_people_withheld_owner_undeclared_kinds_mismatched_responsibility_and_retired_agents()
-> TestResult {
    let (service, seeded) = fixture::world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    registered(&service, &admin, NOTES).await?;
    approve_agent_actions(&service, &admin).await?;
    let other = service.sign_in(login(BEA)).await?;
    refused(
        &post(
            &service,
            "/grants",
            Auth::Cookie(&other),
            &give(&person, &agent)?,
        )
        .await?,
        403,
        "RootAuthorityRefused",
    )?;
    for (member, value, name) in [
        ("relation", json!("owner"), "WithheldFromAgents"),
        (
            "resource",
            json!({"kind": format!("{NOTES}.undeclared"), "id": "team"}),
            "kind_not_registered",
        ),
        (
            "responsible",
            json!(seeded.people[1].id.to_string()),
            "ResponsibleMismatch",
        ),
    ] {
        let mut body = give(&person, &agent)?;
        body[member] = value;
        refused(
            &post(&service, "/grants", Auth::Cookie(&admin), &body).await?,
            403,
            name,
        )?;
    }
    ok(post(
        &service,
        &format!("/identities/{agent}/transitions"),
        Auth::Cookie(&admin),
        &json!({
            "operation": op()?, "transition": "retire", "reason": "agent ended"
        }),
    )
    .await?)?;
    refused(
        &post(
            &service,
            "/grants",
            Auth::Cookie(&admin),
            &give(&person, &agent)?,
        )
        .await?,
        403,
        "IdentityNotActive",
    )?;
    let list = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    assert_eq!(list["grants"], json!([]), "refusals record no grants");
    Ok(())
}

#[tokio::test]
async fn standing_preserves_window_mode_and_further_pass_on_limits() -> TestResult {
    let (service, seeded) = fixture::world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id.to_string();
    let other = seeded.people[1].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    registered(&service, &admin, NOTES).await?;
    approve_agent_actions(&service, &admin).await?;
    let mut body = give(&person, &other)?;
    body["responsible"] = json!(other);
    body["mode"] = json!("by_draft");
    body["pass_on"] = json!({"kind": "to", "actions": ["read"], "recipients": ["agent"]});
    body["window"]["ends_at"] = json!(fixture::now()? + 600);
    let parent = ok(post(&service, "/grants", Auth::Cookie(&admin), &body).await?)?;
    let mut onward = give(&other, &agent)?;
    onward["source"] = parent["grant"].clone();
    onward["responsible"] = json!(person);
    onward["window"] = body["window"].clone();
    let other_cookie = service.sign_in(login(BEA)).await?;
    let child = ok(post(&service, "/grants", Auth::Cookie(&other_cookie), &onward).await?)?;
    let allowed = batch(&service, &admin, &agent, &child["receipt"]["revision"]).await?;
    assert_eq!(allowed["results"][0]["allowed"], false);
    assert_eq!(allowed["results"][0]["mode"], "by_draft");
    assert_eq!(
        allowed["results"][0]["path"],
        json!([child["grant"], parent["grant"], "administrator"])
    );
    let mut refused_person = onward.clone();
    refused_person["operation"] = json!(op()?);
    refused_person["recipient"] = json!(person);
    refused_person["responsible"] = json!(person);
    refused(
        &post(
            &service,
            "/grants",
            Auth::Cookie(&other_cookie),
            &refused_person,
        )
        .await?,
        403,
        "RecipientRefused",
    )?;
    onward["operation"] = json!(op()?);
    onward["relation"] = json!("owner");
    refused(
        &post(&service, "/grants", Auth::Cookie(&other_cookie), &onward).await?,
        403,
        "ActionsOutside",
    )?;
    onward["operation"] = json!(op()?);
    onward["relation"] = json!("member");
    onward["window"]["ends_at"] = Value::Null;
    refused(
        &post(&service, "/grants", Auth::Cookie(&other_cookie), &onward).await?,
        403,
        "ExpiryBeyondSource",
    )?;
    let mut future = give(&person, &agent)?;
    future["resource"]["id"] = json!("later");
    future["window"] = json!({"starts_at": fixture::now()? + 3600, "ends_at": null});
    let future = ok(post(&service, "/grants", Auth::Cookie(&admin), &future).await?)?;
    let checked = ok(post(&service, "/grants/check/batch", Auth::Cookie(&admin), &json!({
        "checks": [check(&agent, &format!("{NOTES}.workspace"), "later", "read")], "at_least": future["receipt"]["revision"]
    })).await?)?;
    assert_eq!(checked["results"][0]["allowed"], false);
    assert_eq!(checked["results"][0]["refusal"], "NotStarted");
    Ok(())
}
