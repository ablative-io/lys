#![cfg(test)]
//! Many questions at once: a batch answers each check in the order sent at
//! one named revision, refuses more than it takes by name, `which` lists
//! exactly the ids the batch allows across its pages, and a question naming
//! the revision a caller last wrote sees that write while one naming a later
//! revision is refused rather than answered early.

use identity_contract::apps::{
    Auth, NOTES, TestResult, check, login, ok, post, refused, registered, root, seeded,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};

#[tokio::test]
async fn a_mixed_batch_is_answered_in_order_at_one_named_revision() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "1"), "reader").await?)?;
    ok(root(&service, &admin, &bea, (&doc, "2"), "editor").await?)?;
    let checks = json!({"checks": [
        check(&bea, &doc, "1", "read"),
        check(&bea, &doc, "1", "write"),
        check(&bea, &doc, "2", "write"),
        check(&bea, &doc, "3", "read"),
        check(&bea, &format!("{NOTES}.nothing"), "1", "read"),
    ]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&credential),
        &checks,
    )
    .await?)?;
    let allowed: Vec<Value> = answer["results"]
        .as_array()
        .ok_or("no results")?
        .iter()
        .map(|result| result["allowed"].clone())
        .collect();
    assert_eq!(
        allowed,
        vec![
            json!(true),
            json!(false),
            json!(true),
            json!(false),
            json!(false)
        ]
    );
    assert_eq!(answer["results"][1]["refusal"], "NotHeld", "{answer}");
    assert_eq!(
        answer["results"][4]["refusal"], "kind_not_registered",
        "{answer}"
    );
    assert!(
        answer["revision"]
            .as_u64()
            .is_some_and(|revision| revision >= 2),
        "{answer}"
    );
    Ok(())
}

#[tokio::test]
async fn five_hundred_and_one_checks_are_refused_batch_too_large() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let one = check(&bea, "doc", "1", "read");
    let most = json!({"checks": vec![one.clone(); 500]});
    let answer = ok(post(&service, "/grants/check/batch", Auth::Cookie(&admin), &most).await?)?;
    assert_eq!(answer["results"].as_array().map(Vec::len), Some(500));
    let over = json!({"checks": vec![one; 501]});
    let refusal = post(&service, "/grants/check/batch", Auth::Cookie(&admin), &over).await?;
    refused(&refusal, 400, "batch_too_large")?;
    assert_eq!(
        refusal.1["fields"][0],
        json!({"at": "/checks", "count": 501})
    );
    let bea_cookie = service.sign_in(login(identity_contract::apps::BEA)).await?;
    let small = json!({"checks": [check(&bea, "doc", "1", "read")]});
    refused(
        &post(
            &service,
            "/grants/check/batch",
            Auth::Cookie(&bea_cookie),
            &small,
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    Ok(())
}

#[tokio::test]
async fn which_lists_exactly_the_ids_the_batch_allows_across_pages() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let (doc, channel, workspace) = (
        format!("{NOTES}.doc"),
        format!("{NOTES}.channel"),
        format!("{NOTES}.workspace"),
    );
    for id in ["a", "b", "c", "d", "e"] {
        ok(root(&service, &admin, &bea, (&doc, id), "reader").await?)?;
    }
    ok(root(&service, &admin, &bea, (&doc, "f"), "editor").await?)?;
    for id in ["general", "random"] {
        let place = json!({
            "operation": identity_contract::apps::op()?,
            "child": {"kind": channel, "id": id},
            "parent": {"kind": workspace, "id": "team"},
        });
        ok(post(
            &service,
            &format!("/apps/{NOTES}/placements"),
            Auth::Bearer(&credential),
            &place,
        )
        .await?)?;
    }
    ok(root(&service, &admin, &bea, (&workspace, "team"), "member").await?)?;

    for (kind, action, ids) in [
        (doc.as_str(), "read", vec!["a", "b", "c", "d", "e", "f"]),
        (doc.as_str(), "write", vec!["f"]),
        (channel.as_str(), "read", vec!["general", "random"]),
    ] {
        let mut listed = Vec::new();
        let mut after = Value::Null;
        let mut pages = 0;
        loop {
            let body = json!({"subject": bea, "kind": kind, "action": action, "page_size": 2, "after": after});
            let page =
                ok(post(&service, "/grants/which", Auth::Bearer(&credential), &body).await?)?;
            pages += 1;
            listed.extend(
                page["ids"]
                    .as_array()
                    .ok_or("no ids")?
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            match page.get("next") {
                Some(next) if !next.is_null() => after = next.clone(),
                _ => break,
            }
        }
        assert_eq!(listed, ids, "{kind} {action}");
        assert_eq!(
            pages,
            ids.len().div_ceil(2).max(1),
            "{kind} {action} took {pages} pages"
        );
        let checks: Vec<Value> = ["a", "b", "c", "d", "e", "f", "general", "random", "none"]
            .iter()
            .map(|id| check(&bea, kind, id, action))
            .collect();
        let answer = ok(post(
            &service,
            "/grants/check/batch",
            Auth::Bearer(&credential),
            &json!({"checks": checks}),
        )
        .await?)?;
        let allowed: Vec<String> = answer["results"]
            .as_array()
            .ok_or("no results")?
            .iter()
            .zip(["a", "b", "c", "d", "e", "f", "general", "random", "none"])
            .filter(|(result, _)| result["allowed"] == true)
            .map(|(_, id)| id.to_owned())
            .collect();
        assert_eq!(
            allowed, listed,
            "which and the batch agree on {kind} {action}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_check_naming_the_revision_of_a_just_made_grant_sees_it() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let issued = ok(root(&service, &admin, &bea, ("doc", "fresh"), "alpha").await?)?;
    let written = issued["receipt"]["revision"]
        .as_u64()
        .ok_or("no revision")?;
    let body = json!({"checks": [check(&bea, "doc", "fresh", "write")], "at_least": written});
    let answer = ok(post(&service, "/grants/check/batch", Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(answer["results"][0]["allowed"], true, "{answer}");
    assert!(
        answer["revision"]
            .as_u64()
            .is_some_and(|revision| revision >= written)
    );
    let ahead =
        json!({"checks": [check(&bea, "doc", "fresh", "write")], "at_least": written + 100});
    let stale = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Cookie(&admin),
        &ahead,
    )
    .await?)?;
    assert_eq!(stale["results"][0]["refusal"], "StaleDecision", "{stale}");
    Ok(())
}
