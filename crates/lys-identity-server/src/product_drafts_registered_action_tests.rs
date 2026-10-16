#![cfg(test)]
use identity_contract::apps::{Auth, approve, login, ok, op, post, registration, seeded};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;
use std::error::Error;

#[tokio::test]
async fn registered_agent_permission_actions_are_admitted_and_undeclared_actions_are_refused()
-> Result<(), Box<dyn Error>> {
    let (service, people) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let holder = service.sign_in(login(identity_contract::apps::BEA)).await?;
    let mut checked = 0;
    for (app, declared) in [
        ("fixture_permission", true),
        ("fixture_without_permission", false),
    ] {
        let kind = format!("{app}.database");
        let actions = if declared {
            vec!["read", "agent_permission"]
        } else {
            vec!["read"]
        };
        let schema = json!({"kinds": {&kind: {"actions": actions, "relations": {"controller": actions}, "parents": []}}});
        ok(post(
            &service,
            "/apps",
            Auth::Cookie(&admin),
            &registration(app, &schema)?,
        )
        .await?)?;
        approve(&service, &admin, app).await?;
        let grant = ok(post(&service, "/grants/roots", Auth::Cookie(&admin), &json!({
            "operation": op()?, "route": "api", "holder": people.people[1].id.to_string(),
            "resource": {"kind":kind, "id":"one"}, "relation":"controller", "pass_on":{"kind":"use_only"},
            "window":{"starts_at":0,"ends_at":null}, "mode":"by_draft"
        })).await?)?;
        let (status, answer) = post(&service, "/product-drafts", Auth::Cookie(&holder), &json!({
            "operation":op()?, "grant":grant["grant"], "target":{"kind":kind,"id":"one","action":"agent_permission"},
            "words":"fixture request", "request_digest":lys_pass::drafts::request_digest("fixture request")
        })).await?;
        if declared {
            assert_eq!(
                status, 200,
                "a registered agent_permission action was refused: {answer}"
            );
            assert!(answer["draft"].as_str().is_some());
        } else {
            assert!(
                (400..500).contains(&status),
                "an undeclared agent_permission action did not receive a named refusal: {status} {answer}"
            );
            assert_eq!(answer["refusal"], "action_not_declared");
        }
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}
