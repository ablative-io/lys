//! All-or-nothing review acts, read back through the durable store.
use super::*;
use identity_contract::apps::{Auth, NOTES, get, ok, op, post, registered};

#[tokio::test]
async fn one_batch_keeps_120_decisions_with_one_write_and_retries_without_writing() -> TestResult {
    let table = Table::set().await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    let mut grants = Vec::new();
    for number in 0..120 {
        grants.push(
            table
                .lend(0, &table.ada, &format!("batch-{number}"))
                .await?
                .1,
        );
    }
    let shown = due_ids(&table.reviews(&table.ada).await?)?;
    assert_eq!(shown, grants.iter().cloned().collect::<BTreeSet<_>>());
    let body = json!({"operation": operation()?, "grants": grants});
    let (_, unseen) = table.lend(0, &table.ada, "arrived-after-read").await?;
    let before = table.store()?.write_count();
    let (status, answer) = table.keep_all(&table.ada, &agent, &body).await?;
    assert_eq!(status, 200, "{answer}");
    let decisions = answer["kept"].as_array().ok_or("kept decisions")?;
    assert_eq!(decisions.len(), 120);
    for decision in decisions {
        assert_eq!(decision["kept_by"], table.seeded.people[0].id.to_string());
        assert_eq!(decision["operation"], body["operation"]);
        assert!(
            body["grants"]
                .as_array()
                .ok_or("ids")?
                .contains(&decision["grant"])
        );
    }
    let store = table.store()?;
    assert_eq!(store.decisions().len(), 120);
    assert_eq!(store.write_count() - before, 1);
    let (status, retry) = table.keep_all(&table.ada, &agent, &body).await?;
    assert_eq!(status, 200, "{retry}");
    assert_eq!(retry, answer);
    assert_eq!(table.store()?.write_count(), before + 1);
    assert_eq!(
        due_ids(&table.reviews(&table.ada).await?)?,
        BTreeSet::from([unseen])
    );
    let changed = json!({"operation": body["operation"], "grants": [grants[0]]});
    let (status, refused) = table.keep_all(&table.ada, &agent, &changed).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "ReviewReused");
    assert_eq!(table.store()?.write_count(), before + 1);
    Ok(())
}

#[tokio::test]
async fn each_named_refusal_keeps_none_of_the_batch() -> TestResult {
    let table = Table::set().await?;
    let ada_agent = table.seeded.people[0].agents[0].id.to_string();
    let bea_agent = table.seeded.people[1].agents[0].id.to_string();
    let (_, ada) = table.lend(0, &table.ada, "ada").await?;
    let (_, bea) = table.lend(1, &table.bea, "bea").await?;
    let (_, issued_to_ada) = table
        .lend_to(1, 0, &table.bea, "visible-but-not-mine")
        .await?;
    let (_, fresh) = table
        .lend_at(0, 0, &table.ada, "fresh", 1_800_000_000)
        .await?;
    for (cookie, agent, ids, status, name) in [
        (
            &table.ada,
            &ada_agent,
            vec![ada.clone(), fresh.clone()],
            409,
            "GrantNotDue",
        ),
        (
            &table.bea,
            &bea_agent,
            vec![bea.clone(), ada.clone()],
            404,
            "GrantNotVisible",
        ),
        (
            &table.ada,
            &ada_agent,
            vec![ada.clone(), bea.clone()],
            409,
            "AgentMismatch",
        ),
        (
            &table.bea,
            &ada_agent,
            vec![issued_to_ada.clone()],
            403,
            "ReviewerOnly",
        ),
    ] {
        let before = table.store()?.write_count();
        let body = json!({"operation": operation()?, "grants": ids});
        let (actual, refused) = table.keep_all(cookie, agent, &body).await?;
        assert_eq!(actual, status, "{refused}");
        assert_eq!(refused["refusal"], name);
        if name == "GrantNotDue" {
            assert!(refused["reason"].as_str().ok_or("reason")?.contains(&fresh));
        }
        assert_eq!(
            table.store()?.write_count(),
            before,
            "{name} writes nothing"
        );
        assert!(table.store()?.decisions().is_empty(), "{name} keeps none");
    }
    Ok(())
}

#[tokio::test]
async fn the_root_authority_keeps_all_of_another_persons_agent() -> TestResult {
    let table = Table::set().await?;
    let bea_agent = table.seeded.people[1].agents[0].id.to_string();
    let (_, first) = table.lend(1, &table.bea, "root-1").await?;
    let (_, second) = table.lend(1, &table.bea, "root-2").await?;
    let body = json!({"operation": operation()?, "grants": [first, second]});
    let before = table.store()?.write_count();
    let (status, answer) = table.keep_all(&table.ada, &bea_agent, &body).await?;
    assert_eq!(status, 200, "{answer}");
    for decision in answer["kept"].as_array().ok_or("kept decisions")? {
        assert_eq!(decision["kept_by"], table.seeded.people[0].id.to_string());
    }
    assert_eq!(table.store()?.write_count(), before + 1);
    assert!(due_ids(&table.reviews(&table.ada).await?)?.is_empty());
    Ok(())
}

pub(super) async fn request_and_app_action_decided_from_waiting(table: &Table) -> TestResult {
    registered(&table.service, &table.ada, NOTES).await?;
    let grant = ok(post(&table.service, "/grants/roots", Auth::Cookie(&table.ada), &json!({
        "operation": op()?, "route": "api", "holder": table.seeded.people[1].id.to_string(),
        "resource": {"kind": "fixture_notes.doc", "id": "1"}, "relation": "editor",
        "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}, "mode": "by_draft",
    })).await?)?;
    let words = "{\"stop\":\"seat-7\"}";
    ok(post(
        &table.service,
        "/product-drafts",
        Auth::Cookie(&table.bea),
        &json!({
            "operation": op()?, "grant": grant["grant"],
            "target": {"kind": "fixture_notes.doc", "id": "1", "action": "write"},
            "request_digest": lys_pass::drafts::request_digest(words), "words": words,
        }),
    )
    .await?)?;
    ok(post(
        &table.service,
        "/requests",
        Auth::Cookie(&table.bea),
        &json!({
            "operation": op()?, "resource": {"kind": "doc", "id": "real-request"},
            "relation": "beta", "ends_at": null, "why": "a real request after the checks",
        }),
    )
    .await?)?;
    let rows = table.waiting().await?;
    assert_eq!(rows.len(), 2, "one request and one app action: {rows:?}");
    let mut remaining = 2;
    for row in rows {
        let id = row["id"].as_str().ok_or("row id")?;
        let (path, body) = if row["kind"] == "request" {
            assert!(row["request"].is_object());
            (
                format!("/requests/{id}/decline"),
                json!({"note": "not needed"}),
            )
        } else {
            assert_eq!(row["kind"], "product_draft");
            assert!(row["product_draft"].is_object());
            (
                format!("/product-drafts/{id}/approve"),
                json!({
                    "operation": op()?, "request_digest": row["product_draft"]["request_digest"],
                }),
            )
        };
        ok(post(&table.service, &path, Auth::Cookie(&table.ada), &body).await?)?;
        remaining -= 1;
        assert_eq!(table.waiting().await?.len(), remaining);
    }
    assert!(table.waiting().await?.is_empty());
    let reviews = ok(get(&table.service, "/reviews", Auth::Cookie(&table.ada)).await?)?;
    assert!(due_ids(&reviews)?.is_empty());
    Ok(())
}
