//! An agent's access request is decided by whoever may give that agent the
//! access with no source, exactly as Give access gives it from the
//! administrator's standing, and a relation no agent may ever be given is
//! refused when it is asked for, in Give access's own words, with nothing
//! kept (seen live 28 Oct 2026: an agent asked for `administrator` on a
//! Cambium workspace, and the administrator was told no one could decide it).

use serde_json::{Value, json};

use super::{TestResult, client, fixture_with, operation};
use identity_contract::apps::{Auth, put};
use identity_contract::harness::Service;

const APP: &str = "fixture_product";
const KIND: &str = "fixture_product.agent";

/// Replace the app's schema so `viewer` carries `read`, which an agent may
/// hold, and `administrator` also carries `configure` and `invite`, which
/// no agent may.
async fn administered(service: &Service, cookie: &str) -> TestResult {
    let (status, current) = service
        .get(&format!("/apps/{APP}/schema"), Some(cookie))
        .await?;
    let replaces = if status == 200 {
        current["version"].as_u64().ok_or("no version")?
    } else {
        0
    };
    let body = json!({"operation": operation()?, "replaces": replaces,
        "schema": {"kinds": {KIND: {"actions": ["read", "configure", "invite"],
            "relations": {"viewer": ["read"], "administrator": ["read", "configure", "invite"]},
            "agents": ["read"]}}}});
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

async fn ask(base: &str, pass: &str, relation: &str) -> TestResult<(u16, Value)> {
    let body = json!({"operation": operation()?, "resource": {"kind": KIND, "id": "shared"},
        "relation": relation, "ends_at": null, "why": "to run the workspace"});
    let response = client()
        .post(format!("{base}/requests"))
        .header("lys-agent-pass", pass)
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    Ok((status, response.json().await?))
}

#[tokio::test]
async fn the_administrator_decides_an_agents_request_nobody_could_lend_and_the_agent_holds_it()
-> TestResult {
    let (service, pass, cookie, person, agent) = fixture_with(&[APP]).await?;
    administered(&service, &cookie).await?;
    let (status, asked) = ask(&service.base, &pass, "viewer").await?;
    assert_eq!(status, 200, "{asked}");
    assert_eq!(asked["asked_by"], agent, "{asked}");
    assert_eq!(asked["sources"], json!([]), "{asked}");
    let id = asked["id"].as_str().ok_or("no id")?;

    let (status, seen) = service.get("/requests", Some(&cookie)).await?;
    assert_eq!(status, 200, "{seen}");
    let seen = &seen["requests"][0];
    assert_eq!(seen["id"], id, "{seen}");
    assert_eq!(
        seen["can_decide"], true,
        "the administrator gives this agent the access on Give access, so decides its request: {seen}"
    );
    assert_eq!(seen["can_issue_root"], true, "{seen}");
    assert_eq!(seen["approvers"][0]["id"], person, "{seen}");
    assert_eq!(
        seen["approvers"].as_array().map(Vec::len),
        Some(1),
        "{seen}"
    );

    let approval =
        json!({"operation": operation()?, "route": "api", "source": null, "note": "yes"});
    let (status, approved) = service
        .post(&format!("/requests/{id}/approve"), Some(&cookie), &approval)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["state"], "approved", "{approved}");
    assert_eq!(approved["decision"]["by"], person, "{approved}");
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let (status, held) = service
        .get(&format!("/grants/{grant}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["holder"], agent, "{held}");
    assert_eq!(
        held["resource"],
        json!({"kind": KIND, "id": "shared"}),
        "{held}"
    );
    assert_eq!(held["relation"], "viewer", "{held}");
    assert_eq!(held["actions"], json!(["read"]), "{held}");
    assert_eq!(held["pass_on"]["kind"], "use_only", "{held}");

    let (status, again) = service
        .post(&format!("/requests/{id}/approve"), Some(&cookie), &approval)
        .await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again["decision"]["grant"], grant,
        "approved again, no second grant"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_asking_for_a_relation_no_agent_may_hold_is_refused_as_give_access_refuses_it()
-> TestResult {
    let (service, pass, cookie, person, agent) = fixture_with(&[APP]).await?;
    administered(&service, &cookie).await?;
    let (status, refused) = ask(&service.base, &pass, "administrator").await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "WithheldFromAgents", "{refused}");
    let reason = refused["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.contains("`administrator` carries configure, invite, which no agent may be given"),
        "{refused}"
    );

    let (status, given) = service
        .post(
            "/grants",
            Some(&cookie),
            &json!({
                "operation": operation()?, "route": "browser", "source": "administrator",
                "recipient": agent, "responsible": person, "mode": "outright",
                "resource": {"kind": KIND, "id": "shared"}, "relation": "administrator",
                "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}
            }),
        )
        .await?;
    assert_eq!(status, 403, "{given}");
    assert_eq!(
        (&refused["refusal"], &refused["reason"], &refused["fields"]),
        (&given["refusal"], &given["reason"], &given["fields"]),
        "the ask is refused exactly as Give access refuses the same access"
    );

    let (status, seen) = service.get("/requests", Some(&cookie)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(
        seen["requests"],
        json!([]),
        "nothing is kept, and the root authority is shown every request: {seen}"
    );
    Ok(())
}
