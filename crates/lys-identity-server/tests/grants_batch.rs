#![cfg(test)]
//! Many questions at once: a batch answers each check in the order sent at
//! one named revision, answers a batch of any size whole, `which` lists
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
async fn a_thousand_checks_are_answered_whole_and_in_order() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "777"), "reader").await?)?;
    let checks: Vec<Value> = (0..1000)
        .map(|n| check(&bea, &doc, &n.to_string(), "read"))
        .collect();
    let many = json!({"checks": checks});
    let answer = ok(post(&service, "/grants/check/batch", Auth::Cookie(&admin), &many).await?)?;
    let allowed: Vec<usize> = answer["results"]
        .as_array()
        .ok_or("no results")?
        .iter()
        .enumerate()
        .filter(|(_, result)| result["allowed"] == json!(true))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        answer["results"].as_array().map(Vec::len),
        Some(1000),
        "{answer}"
    );
    assert_eq!(allowed, vec![777], "{answer}");
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

#[tokio::test]
async fn the_administrator_is_allowed_on_every_resource_of_an_app_with_no_grant_made_first()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let administrator = seeded.people[0].id.to_string();
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    // A fresh install: nobody holds anything on this resource, and the app asks about the person who runs Lys.
    let checks = json!({"checks": [
        check(&administrator, &doc, "never-granted", "read"),
        check(&administrator, &doc, "never-granted", "write"),
        check(&bea, &doc, "never-granted", "read"),
        check(&administrator, &format!("{NOTES}.nothing"), "1", "read"),
    ]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&credential),
        &checks,
    )
    .await?)?;
    let results = answer["results"].as_array().ok_or("no results")?;
    assert_eq!(results[0]["allowed"], true, "{answer}");
    assert_eq!(results[0]["via"], "administrator", "{answer}");
    assert_eq!(results[0]["mode"], "outright", "{answer}");
    // The shape every app's client accepts for an outright yes: a non-empty receipt and no refusal.
    assert_eq!(results[0]["grant"], "administrator", "{answer}");
    assert!(results[0]["refusal"].is_null(), "{answer}");
    assert_eq!(results[1]["grant"], "administrator", "{answer}");
    assert_eq!(results[1]["allowed"], true, "{answer}");
    // Everyone else still needs a grant, and a kind the app never declared is still refused, administrator or not.
    assert_eq!(results[2]["allowed"], false, "{answer}");
    assert_eq!(results[2]["refusal"], "NotHeld", "{answer}");
    assert_eq!(results[3]["allowed"], false, "{answer}");
    assert_eq!(results[3]["refusal"], "kind_not_registered", "{answer}");
    Ok(())
}

#[tokio::test]
async fn which_lists_every_known_administrator_resource_without_grants_with_the_same_paging()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let administrator = seeded.people[0].id.to_string();
    let bea = seeded.people[1].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let foreign = registered(&service, &admin, identity_contract::apps::FILES).await?;
    for (app, credential, id, restricted) in [
        (NOTES, &credential, "a", false),
        (NOTES, &credential, "b", true),
        (NOTES, &credential, "c", false),
        (identity_contract::apps::FILES, &foreign, "foreign", false),
    ] {
        ok(post(
            &service,
            &format!("/apps/{app}/placements"),
            Auth::Bearer(credential),
            &json!({
                "operation": identity_contract::apps::op()?,
                "child": {"kind": format!("{app}.channel"), "id": id},
                "parent": {"kind": format!("{app}.workspace"), "id": "team"},
                "restricted": restricted,
            }),
        )
        .await?)?;
    }
    for (subject, suffix, expected) in [
        (&administrator, "channel", vec!["a", "b", "c"]),
        (&administrator, "workspace", vec!["team"]),
        (&bea, "channel", vec![]),
        (&bea, "workspace", vec![]),
        (&agent, "channel", vec![]),
        (&agent, "workspace", vec![]),
    ] {
        for action in ["read", "write"] {
            let mut ids = Vec::new();
            let mut after = Value::Null;
            let mut pages = 0;
            loop {
                let page = ok(post(
                    &service,
                    "/grants/which",
                    Auth::Bearer(&credential),
                    &json!({"subject": subject, "kind": format!("{NOTES}.{suffix}"),
                        "action": action, "page_size": 2, "after": after}),
                )
                .await?)?;
                let listed = page["ids"].as_array().ok_or("no ids")?;
                let modes = page["modes"].as_array().ok_or("no modes")?;
                assert_eq!(modes.len(), listed.len());
                assert!(modes.iter().all(|mode| mode == "outright"));
                ids.extend(listed.iter().filter_map(Value::as_str).map(str::to_owned));
                pages += 1;
                after = page["next"].clone();
                if after.is_null() {
                    break;
                }
            }
            assert_eq!(ids, expected, "{subject} {suffix} {action}");
            assert_eq!(pages, expected.len().div_ceil(2).max(1));
        }
    }
    refused(
        &post(
            &service,
            "/grants/which",
            Auth::Bearer(&credential),
            &json!({"subject": administrator, "kind": format!("{NOTES}.undeclared"),
                "action": "read", "page_size": 2}),
        )
        .await?,
        403,
        "kind_not_registered",
    )?;
    // A resource recorded only by another holder's grant is known to the administrator too.
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "recorded"), "reader").await?)?;
    let page = ok(post(
        &service,
        "/grants/which",
        Auth::Bearer(&credential),
        &json!({"subject": administrator, "kind": doc, "action": "write", "page_size": 2}),
    )
    .await?)?;
    assert_eq!(page["ids"], json!(["recorded"]));
    assert_eq!(page["modes"], json!(["outright"]));
    Ok(())
}
