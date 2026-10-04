//! `GET /drafts`: the drafts the signed-in person may decide, and the
//! dashboard's count of them.

use serde_json::{Value, json};

use super::{Result, Table, operation};

async fn listed(table: &Table, cookie: &str, path: &str) -> Result<Value> {
    let (status, answer) = table.service.get(path, Some(cookie)).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

fn ids(answer: &Value) -> Result<Vec<String>> {
    Ok(answer["drafts"]
        .as_array()
        .ok_or("no drafts")?
        .iter()
        .map(|draft| {
            draft["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or("a draft has no id")
        })
        .collect::<std::result::Result<_, _>>()?)
}

#[tokio::test]
async fn no_draft_lists_none() -> Result {
    let table = Table::start().await?;
    let answer = listed(&table, &table.owner, "/drafts").await?;
    assert_eq!(answer, json!({ "drafts": [] }));
    let answer = listed(&table, &table.owner, "/drafts?limit=5").await?;
    assert_eq!(answer, json!({ "drafts": [], "total": 0, "next": null }));
    Ok(())
}

#[tokio::test]
async fn a_waiting_draft_is_listed_whole_to_its_responsible_person_alone() -> Result {
    let table = Table::start().await?;
    let (draft, body, _, created) = table.create().await?;
    let before = table.service.log_size().await?;
    let answer = listed(&table, &table.owner, "/drafts").await?;
    assert_eq!(
        table.service.log_size().await?,
        before,
        "a list records nothing"
    );
    let prepared: Value = serde_json::from_slice(&body)?;
    let drafts = answer["drafts"].as_array().ok_or("no drafts")?;
    assert_eq!(drafts.len(), 1, "{answer}");
    let row = &drafts[0];
    assert_eq!(row["id"], draft.to_string());
    assert_eq!(
        row["agent"],
        json!({ "id": table.agent, "display_name": "Agent" })
    );
    assert_eq!(row["responsible"]["id"], table.person);
    assert_eq!(row["responsible"]["display_name"], "Owner");
    assert_eq!(row["target"], prepared["target"]);
    assert_eq!(row["method"], prepared["method"]);
    assert_eq!(row["path"], prepared["path"]);
    assert_eq!(row["body"], prepared["body"]);
    assert_eq!(row["note"], prepared["note"]);
    assert!(row["created_at"].as_u64().is_some_and(|at| at > 0), "{row}");
    assert_eq!(row["creation_hash"], created["creation_hash"]);
    assert_eq!(row["state"], "waiting");
    assert!(row.get("decided").is_none(), "{row}");

    let unrelated = listed(&table, &table.other, "/drafts").await?;
    assert_eq!(unrelated, json!({ "drafts": [] }));
    let unrelated = listed(&table, &table.other, "/drafts?state=all").await?;
    assert_eq!(unrelated, json!({ "drafts": [] }));
    Ok(())
}

#[tokio::test]
async fn decided_drafts_leave_the_waiting_list_and_carry_their_decision() -> Result {
    let table = Table::start().await?;
    let mut created = Vec::new();
    for _ in 0..4 {
        created.push(table.create().await?);
    }
    let replacement = lys_identity::OperationId::generate()?;
    let decisions = [
        ("approve", json!({ "application": operation()? })),
        ("refuse", json!({ "reason": "Not this one" })),
        (
            "correct",
            json!({ "reason": "Mine instead", "replacement": table.body(replacement) }),
        ),
    ];
    for ((draft, _, _, answer), (act, extra)) in created.iter().zip(decisions) {
        let mut body = extra;
        body["operation"] = json!(operation()?);
        body["creation_hash"] = answer["creation_hash"].clone();
        let (status, decided) = table
            .service
            .post(&format!("/drafts/{draft}/{act}"), Some(&table.owner), &body)
            .await?;
        assert_eq!(status, 200, "{decided}");
    }
    let waiting = listed(&table, &table.owner, "/drafts").await?;
    assert_eq!(ids(&waiting)?, vec![created[3].0.to_string()]);

    let decided = listed(&table, &table.owner, "/drafts?state=decided").await?;
    let rows = decided["drafts"].as_array().ok_or("no drafts")?;
    assert_eq!(rows.len(), 3, "{decided}");
    let row = |draft: &lys_identity::OperationId| {
        rows.iter()
            .find(|row| row["id"] == draft.to_string())
            .ok_or("decided draft missing")
    };
    let approved = row(&created[0].0)?;
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["decided"]["by"]["id"], table.person);
    assert!(approved["decided"]["application"].as_str().is_some());
    assert!(approved["decided"]["at"].as_u64().is_some());
    assert_eq!(approved["decided"]["by"]["display_name"], "Owner");
    assert!(approved["decided"].get("reason").is_none(), "{approved}");
    let refused = row(&created[1].0)?;
    assert_eq!(refused["state"], "refused");
    assert_eq!(refused["decided"]["reason"], "Not this one");
    let corrected = row(&created[2].0)?;
    assert_eq!(corrected["state"], "corrected");
    assert_eq!(corrected["decided"]["replacement"], replacement.to_string());
    assert!(
        rows.iter().all(|row| row["id"] != replacement.to_string()),
        "a correction's own change is no row of its own"
    );

    let all = listed(&table, &table.owner, "/drafts?state=all").await?;
    let mut expected: Vec<String> = created.iter().map(|made| made.0.to_string()).collect();
    expected.sort();
    assert_eq!(ids(&all)?, expected);
    Ok(())
}

#[tokio::test]
async fn the_waiting_list_pages_in_draft_order() -> Result {
    let table = Table::start().await?;
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.push(table.create().await?.0.to_string());
    }
    expected.sort();
    let first = listed(&table, &table.owner, "/drafts?limit=2").await?;
    assert_eq!(ids(&first)?, expected[..2].to_vec());
    assert_eq!(first["total"], 3);
    let next = first["next"].as_str().ok_or("no next page")?;
    let second = listed(
        &table,
        &table.owner,
        &format!("/drafts?limit=2&after={next}"),
    )
    .await?;
    assert_eq!(ids(&second)?, expected[2..].to_vec());
    assert_eq!(second["total"], 3);
    assert_eq!(second["next"], Value::Null);

    let (status, answer) = table
        .service
        .get(
            &format!("/drafts?state=all&limit=2&after={next}"),
            Some(&table.owner),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed");
    let (status, answer) = table
        .service
        .get("/drafts?state=pending", Some(&table.owner))
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed");
    Ok(())
}

#[tokio::test]
async fn the_dashboard_counts_the_waiting_drafts_and_names_each_part_it_cannot_read() -> Result {
    let table = Table::start().await?;
    table.create().await?;
    table.create().await?;
    let drafts = listed(&table, &table.owner, "/drafts").await?;
    let dashboard = listed(&table, &table.owner, "/dashboard").await?;
    assert_eq!(
        dashboard["waiting"]["drafts"],
        json!(drafts["drafts"].as_array().ok_or("no drafts")?.len())
    );
    assert_eq!(dashboard["waiting"]["drafts"], 2);
    // This service keeps no requests, reviews, budgets, goals, sessions or
    // teams: each part is its own route's refusal, never a zero or an empty list.
    assert_eq!(
        dashboard["waiting"]["requests"]["refusal"],
        "RequestsUnavailable"
    );
    let agents = dashboard["agents"].as_array().ok_or("no agents")?;
    assert_eq!(agents.len(), 1, "{dashboard}");
    assert_eq!(agents[0]["agent"]["id"], table.agent);
    for part in ["teams", "sessions", "usage", "goals"] {
        assert!(
            agents[0][part]["refusal"].as_str().is_some(),
            "{part} is not a refusal: {dashboard}"
        );
    }
    let (status, own) = table
        .service
        .get(
            &format!("/agents/{}/goals", table.agent),
            Some(&table.owner),
        )
        .await?;
    assert_ne!(status, 200, "{own}");
    assert_eq!(agents[0]["goals"]["refusal"], own["refusal"]);
    assert_eq!(agents[0]["goals"]["reason"], own["reason"]);

    let other = listed(&table, &table.other, "/dashboard").await?;
    assert_eq!(other["agents"], json!([]));
    assert_eq!(other["waiting"]["drafts"], 0);
    Ok(())
}
