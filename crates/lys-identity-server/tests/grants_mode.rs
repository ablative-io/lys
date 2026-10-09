#![cfg(test)]
//! A grant's mode is answered beside each right (ACCESS-001 R1): check/batch
//! on a `by_draft` right answers allowed false, mode `by_draft` and the grant's
//! id, so a product holds the act for a draft rather than refusing it; an
//! outright right answers allowed true, mode outright; `which` names each
//! id's mode beside it. A grant issued without a mode is outright, and a
//! grant read back names its mode.

use identity_contract::apps::{
    Auth, NOTES, TestResult, check, get, login, ok, op, post, registered, root, seeded,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

/// A root grant to `holder` on `id`, issued in `mode`.
async fn held_root(
    service: &Service,
    admin: &str,
    holder: &str,
    (kind, id): (&str, &str),
    mode: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let body = json!({
        "operation": op()?,
        "route": "api",
        "holder": holder,
        "resource": {"kind": kind, "id": id},
        "relation": "editor",
        "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
        "mode": mode,
    });
    ok(post(service, "/grants/roots", Auth::Cookie(admin), &body).await?)
}

#[tokio::test]
async fn a_by_draft_right_is_answered_held_with_its_mode_and_grant() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let issued = held_root(&service, &admin, &bea, (&doc, "1"), "by_draft").await?;
    let grant = issued["grant"]
        .as_str()
        .ok_or("the issue names its grant")?;
    ok(root(&service, &admin, &bea, (&doc, "2"), "editor").await?)?;
    let checks = json!({"checks": [
        check(&bea, &doc, "1", "write"),
        check(&bea, &doc, "2", "write"),
    ]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&credential),
        &checks,
    )
    .await?)?;
    let held = &answer["results"][0];
    assert_eq!(held["allowed"], json!(false), "{answer}");
    assert_eq!(held["mode"], "by_draft", "{answer}");
    assert_eq!(held["grant"], grant, "{answer}");
    assert!(
        held["refusal"].is_null(),
        "a held right is not refused: {answer}"
    );
    let outright = &answer["results"][1];
    assert_eq!(outright["allowed"], json!(true), "{answer}");
    assert_eq!(outright["mode"], "outright", "{answer}");
    Ok(())
}

#[tokio::test]
async fn which_names_each_ids_mode_beside_it() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    held_root(&service, &admin, &bea, (&doc, "1"), "by_two").await?;
    ok(root(&service, &admin, &bea, (&doc, "2"), "editor").await?)?;
    let asked = json!({"subject": bea, "kind": doc, "action": "write", "page_size": 10});
    let page = ok(post(&service, "/grants/which", Auth::Bearer(&credential), &asked).await?)?;
    assert_eq!(page["ids"], json!(["1", "2"]), "{page}");
    assert_eq!(page["modes"], json!(["by_two", "outright"]), "{page}");
    Ok(())
}

#[tokio::test]
async fn a_mode_outside_the_three_is_refused_by_name() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let refused = held_root(&service, &admin, &bea, (&doc, "1"), "by_three").await;
    assert!(refused.is_err(), "by_three was issued: {refused:?}");
    Ok(())
}

#[tokio::test]
async fn a_grant_read_back_names_its_mode() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let mut issued = Vec::new();
    for (id, mode) in [("1", "by_draft"), ("2", "by_two")] {
        let answer = held_root(&service, &admin, &bea, (&doc, id), mode).await?;
        issued.push((answer["grant"].clone(), mode));
    }
    let plain = ok(root(&service, &admin, &bea, (&doc, "3"), "editor").await?)?;
    issued.push((plain["grant"].clone(), "outright"));
    for (grant, mode) in issued {
        let grant = grant.as_str().ok_or("the issue names its grant")?;
        let read = ok(get(&service, &format!("/grants/{grant}"), Auth::Cookie(&admin)).await?)?;
        assert_eq!(read["mode"], mode, "{read}");
    }
    let listed = ok(get(&service, "/grants", Auth::Cookie(&admin)).await?)?;
    let modes: Vec<&Value> = listed["grants"]
        .as_array()
        .ok_or("the list holds grants")?
        .iter()
        .map(|grant| &grant["mode"])
        .collect();
    assert!(
        modes.iter().all(|mode| mode.is_string()),
        "every listed grant names its mode: {listed}"
    );
    Ok(())
}
