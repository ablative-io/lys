//! Existing single-grant and visibility contracts.
use super::*;
pub(super) fn due_ids(view: &Value) -> Result<BTreeSet<String>, Box<dyn Error>> {
    view["due"]
        .as_array()
        .ok_or("due is not a list")?
        .iter()
        .map(|due| Ok(due["grant"]["id"].as_str().ok_or("no grant id")?.to_owned()))
        .collect()
}

#[tokio::test]
async fn a_person_reviews_their_own_agents_and_the_root_authority_every_agent() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let (_, of_ada) = table.lend(0, &table.ada, "1").await?;
    let (_, of_bea) = table.lend(1, &table.bea, "2").await?;

    let personal = table.reviews(&table.bea).await?;
    assert_eq!(personal["scope"], "personal");
    assert_eq!(due_ids(&personal)?, BTreeSet::from([of_bea.clone()]));
    let due = &personal["due"][0];
    assert_eq!(due["agent"]["id"], bea.agents[0].id.to_string());
    assert_eq!(due["agent"]["state"], "active");
    assert_eq!(due["reviewer"]["id"], bea.id.to_string());
    assert_eq!(due["grant"]["holder"], bea.agents[0].id.to_string());
    assert_eq!(
        due["grant"]["resource"],
        json!({ "kind": "doc", "id": "2" })
    );
    assert_eq!(personal["unanswered"], json!([]));
    let (_, listed) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(personal["revision"], listed["revision"]);

    let whole = table.reviews(&table.ada).await?;
    assert_eq!(whole["scope"], "directory");
    assert_eq!(due_ids(&whole)?, BTreeSet::from([of_ada, of_bea]));
    let reviewers: BTreeSet<&str> = whole["due"]
        .as_array()
        .ok_or("due is not a list")?
        .iter()
        .filter_map(|due| due["reviewer"]["id"].as_str())
        .collect();
    let (ada_id, bea_id) = (ada.id.to_string(), bea.id.to_string());
    assert_eq!(
        reviewers,
        BTreeSet::from([ada_id.as_str(), bea_id.as_str()])
    );
    Ok(())
}

#[tokio::test]
async fn a_caller_with_no_person_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    table.lend(1, &table.bea, "1").await?;
    let (status, body) = table.service.get("/reviews", None).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let stranger = table.service.sign_in(login(STRANGER)).await?;
    let (status, body) = table.service.get("/reviews", Some(&stranger)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NoPerson");
    assert_eq!(body.get("due"), None, "{body}");
    Ok(())
}

#[tokio::test]
async fn a_grant_whose_source_is_revoked_is_no_longer_due() -> TestResult {
    let table = Table::set().await?;
    let (source, lent) = table.lend(1, &table.bea, "1").await?;
    assert_eq!(
        due_ids(&table.reviews(&table.bea).await?)?,
        BTreeSet::from([lent])
    );
    let revoke = json!({ "operation": operation()?, "route": "api", "reason": "review test" });
    let (status, revoked) = table
        .service
        .post(
            &format!("/grants/{source}/revoke"),
            Some(&table.ada),
            &revoke,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(due_ids(&table.reviews(&table.bea).await?)?, BTreeSet::new());
    assert_eq!(due_ids(&table.reviews(&table.ada).await?)?, BTreeSet::new());
    Ok(())
}

#[tokio::test]
async fn an_agent_whose_person_is_suspended_has_no_one_answering() -> TestResult {
    let table = Table::set().await?;
    let bea = &table.seeded.people[1];
    assert_eq!(table.reviews(&table.ada).await?["unanswered"], json!([]));
    let moved =
        json!({ "operation": operation()?, "transition": "suspend", "reason": "review test" });
    let (status, body) = table
        .service
        .post(
            &format!("/identities/{}/transitions", bea.id),
            Some(&table.ada),
            &moved,
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let whole = table.reviews(&table.ada).await?;
    let unanswered = whole["unanswered"].as_array().ok_or("not a list")?;
    assert_eq!(
        unanswered.len(),
        1,
        "the retired agent is not listed: {whole}"
    );
    assert_eq!(unanswered[0]["agent"]["id"], bea.agents[0].id.to_string());
    assert_eq!(unanswered[0]["person"]["id"], bea.id.to_string());
    assert_eq!(unanswered[0]["person"]["state"], "suspended");
    Ok(())
}

fn due_for<'a>(view: &'a Value, grant: &str) -> Result<&'a Value, Box<dyn Error>> {
    Ok(view["due"]
        .as_array()
        .ok_or("due is not a list")?
        .iter()
        .find(|due| due["grant"]["id"] == grant)
        .ok_or("the grant is not due")?)
}

#[tokio::test]
async fn a_kept_grant_is_read_back_with_who_kept_it_and_no_due_date() -> TestResult {
    let table = Table::set().await?;
    let bea = &table.seeded.people[1];
    let (_, lent) = table.lend(1, &table.bea, "1").await?;
    let before = table.reviews(&table.bea).await?;
    assert_eq!(before["decisions_recorded"], true);
    assert_eq!(due_for(&before, &lent)?["last_kept"], Value::Null);

    let body = json!({ "operation": operation()?, "note": "  still reads the docs  " });
    let (status, kept) = table.keep(&table.bea, &lent, &body).await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["grant"], lent);
    assert_eq!(kept["kept_by"], bea.id.to_string());
    assert_eq!(kept["note"], "still reads the docs");
    assert_eq!(kept["operation"], body["operation"]);
    assert_eq!(kept["revision"], before["revision"]);
    assert!(kept["at"].as_u64().is_some(), "{kept}");

    assert!(due_ids(&table.reviews(&table.bea).await?)?.is_empty());
    table.clock.set(1_800_000_000 + 30 * 86_400);
    for cookie in [&table.bea, &table.ada] {
        let view = table.reviews(cookie).await?;
        let last = &due_for(&view, &lent)?["last_kept"];
        assert_eq!(last["by"], bea.id.to_string());
        assert_eq!(last["note"], "still reads the docs");
        assert_eq!(last["at"], kept["at"]);
        assert_eq!(
            due_for(&view, &lent)?.get("next_review"),
            None,
            "no due date is invented"
        );
    }
    Ok(())
}

#[tokio::test]
async fn the_root_authority_keeps_any_agents_grant() -> TestResult {
    let table = Table::set().await?;
    let ada = &table.seeded.people[0];
    let (_, lent) = table.lend(1, &table.bea, "1").await?;
    let body = json!({ "operation": operation()?, "note": "kept by the root" });
    let (status, kept) = table.keep(&table.ada, &lent, &body).await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["kept_by"], ada.id.to_string());
    Ok(())
}

#[tokio::test]
async fn the_same_keep_is_recorded_once_and_other_words_are_refused() -> TestResult {
    let table = Table::set().await?;
    let (_, lent) = table.lend(1, &table.bea, "1").await?;
    let body = json!({ "operation": operation()?, "note": "still needed" });
    let (status, first) = table.keep(&table.bea, &lent, &body).await?;
    assert_eq!(status, 200, "{first}");
    let (status, again) = table.keep(&table.bea, &lent, &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, first, "asked again it answers what was recorded");

    let reworded = json!({ "operation": body["operation"], "note": "other words" });
    let (status, refused) = table.keep(&table.bea, &lent, &reworded).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "ReviewReused");
    table.clock.set(1_800_000_000 + 30 * 86_400);
    let view = table.reviews(&table.bea).await?;
    let last = &due_for(&view, &lent)?["last_kept"];
    assert_eq!(last["note"], "still needed", "nothing refused is recorded");
    Ok(())
}

#[tokio::test]
async fn a_person_who_is_not_the_reviewer_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let (_, of_ada) = table.lend(0, &table.ada, "1").await?;
    let body = json!({ "operation": operation()?, "note": "not mine" });
    let (status, refused) = table.keep(&table.bea, &of_ada, &body).await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "GrantNotVisible");

    let (_, to_ada) = table.lend_to(1, 0, &table.bea, "2").await?;
    let body = json!({ "operation": operation()?, "note": "issued, not reviewed" });
    let (status, refused) = table.keep(&table.bea, &to_ada, &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "ReviewerOnly");
    let view = table.reviews(&table.ada).await?;
    assert_eq!(due_for(&view, &to_ada)?["last_kept"], Value::Null);

    let stranger = table.service.sign_in(login(STRANGER)).await?;
    let (status, refused) = table.keep(&stranger, &to_ada, &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NoPerson");
    Ok(())
}

#[tokio::test]
async fn a_grant_that_no_longer_stands_is_not_kept() -> TestResult {
    let table = Table::set().await?;
    let (source, lent) = table.lend(1, &table.bea, "1").await?;
    let kept = json!({ "operation": operation()?, "note": "kept before" });
    let (status, first) = table.keep(&table.bea, &lent, &kept).await?;
    assert_eq!(status, 200, "{first}");

    let revoke = json!({ "operation": operation()?, "route": "api", "reason": "review test" });
    let (status, revoked) = table
        .service
        .post(
            &format!("/grants/{source}/revoke"),
            Some(&table.ada),
            &revoke,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");

    let body = json!({ "operation": operation()?, "note": "too late" });
    let (status, refused) = table.keep(&table.bea, &lent, &body).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "GrantNotDue");
    let (status, again) = table.keep(&table.bea, &lent, &kept).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, first, "a recorded decision is answered as recorded");
    Ok(())
}
