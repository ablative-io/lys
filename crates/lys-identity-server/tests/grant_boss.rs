//! Boss delegation names both authorities and waits for the rollback boundary.

#[path = "support/agent_policy.rs"]
mod support;

use identity_contract::apps::{Auth, get, login, op, post};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn ok((status, answer): (u16, Value)) -> Result<Value, Box<dyn Error>> {
    if status != 200 {
        return Err(format!("status {status}: {answer}").into());
    }
    Ok(answer)
}

#[tokio::test]
async fn boss_delegation_waits_for_upgrade_commit_and_names_person_and_boss_on_retry() -> TestResult
{
    let (mut service, [boss, _]) = support::table(false).await?;
    let intent = tempfile::tempdir()?;
    let intent_path = intent.path().join("upgrade");
    service
        .restart_adjusted(|config| config.operator_upgrade_file = Some(intent_path.clone()))
        .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let auth = Auth::Cookie(&cookie);
    ok(post(
        &service,
        &format!("/identities/{boss}/transitions"),
        auth,
        &json!({"operation": op()?, "transition": "activate", "reason": ""}),
    )
    .await?)?;
    let child = ok(post(
        &service,
        "/agents",
        auth,
        &json!({"operation": op()?, "display_name": "Child", "answers_to": boss}),
    )
    .await?)?;
    let person = child["responsible"]
        .as_str()
        .ok_or("no responsible person")?;
    let child = child["agent"].as_str().ok_or("no child")?.to_owned();
    ok(post(
        &service,
        &format!("/identities/{child}/transitions"),
        auth,
        &json!({"operation": op()?, "transition": "activate", "reason": ""}),
    )
    .await?)?;
    let pass = json!({"kind": "to", "actions": ["read"], "recipients": ["agent"]});
    let resource = json!({"kind": "doc", "id": "one"});
    let window = json!({"starts_at": 0, "ends_at": null});
    let root = ok(post(&service, "/grants/roots", auth, &json!({"operation": op()?, "route": "api", "holder": person, "resource": resource, "relation": "beta", "pass_on": pass, "window": window})).await?)?;
    let source = ok(post(&service, "/grants", auth, &json!({"operation": op()?, "route": "api", "source": root["grant"], "recipient": boss, "responsible": person, "resource": resource, "relation": "beta", "pass_on": pass, "window": window})).await?)?;
    let body = json!({"operation": op()?, "route": "api", "source": source["grant"], "recipient": child, "responsible": person, "resource": resource, "relation": "beta", "pass_on": {"kind": "use_only"}, "window": window});
    std::fs::write(&intent_path, b"pending")?;
    let (status, refused) = post(&service, "/grants", auth, &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("upgrade_pending")),
        "{refused}"
    );
    std::fs::remove_file(&intent_path)?;
    let kept = ok(post(&service, "/grants", auth, &body).await?)?;
    assert_eq!(kept["receipt"]["caller"], person);
    assert_eq!(kept["receipt"]["source_holder"], boss);
    let retry = ok(post(&service, "/grants", auth, &body).await?)?;
    assert_eq!(retry, kept);
    let source = source["grant"].as_str().ok_or("no source")?;
    let question = format!("/grants/cannot-give?route=api&source={source}&recipient={child}");
    ok(get(&service, &question, auth).await?)?;
    Ok(())
}
