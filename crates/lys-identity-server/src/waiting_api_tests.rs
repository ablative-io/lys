#![cfg(test)]
//! The one waiting read, grouped checks and records for decisions in place.
use identity_contract::apps::{Auth, get, ok, op};
use serde_json::{Value, json};
#[path = "waiting_test_support.rs"]
mod support;
use support::{Result, Table};

fn kinds(items: &[Value]) -> Vec<&str> {
    let mut kinds: Vec<&str> = items
        .iter()
        .filter_map(|item| item["kind"].as_str())
        .collect();
    kinds.sort_unstable();
    kinds
}

#[tokio::test]
async fn one_list_of_what_waits_oldest_first_and_a_refused_part_in_place() -> Result {
    let table = Table::set().await?;
    let request = table.bea_asks().await?;
    let draft = table.agent_drafts().await?;
    let product = table.bea_product_drafts().await?;
    let review = table.agent_holds().await?;

    let ada = table.waiting(&table.ada).await?;
    assert_eq!(
        kinds(&ada),
        ["agent_draft", "product_draft", "request", "review"],
        "{ada:?}"
    );
    let entry = |kind: &str| {
        ada.iter()
            .find(|item| item["kind"] == kind)
            .cloned()
            .unwrap_or_default()
    };
    assert_eq!(entry("request")["id"], request.as_str());
    assert_eq!(entry("request")["asked_by"]["id"], table.bea_id.as_str());
    assert_eq!(entry("agent_draft")["id"], draft.as_str());
    assert_eq!(entry("agent_draft")["asked_by"]["id"], table.agent.as_str());
    assert_eq!(entry("product_draft")["id"], product.as_str());
    assert_eq!(
        entry("product_draft")["asked_by"]["id"],
        table.bea_id.as_str()
    );
    assert_eq!(entry("review")["id"], table.agent.as_str());
    assert_eq!(entry("review")["asked_by"]["id"], table.agent.as_str());
    assert_eq!(entry("review")["review"]["grants"][0]["id"], review);
    let mut seconds = Vec::new();
    for item in &ada {
        assert_eq!(item["can_decide"], true, "{item}");
        assert!(item["refusal"].is_null(), "{item}");
        assert!(
            !item["about"].as_str().unwrap_or_default().is_empty(),
            "{item}"
        );
        let at: jiff::Timestamp = item["asked_at"].as_str().ok_or("no asked_at")?.parse()?;
        seconds.push(at.as_second());
    }
    assert!(seconds.is_sorted(), "oldest first: {seconds:?}");
    assert_eq!(
        entry("request")["about"],
        "Bea (test person) asks for beta access to doc quarter"
    );

    let bea = table.waiting(&table.bea).await?;
    assert_eq!(
        bea,
        Vec::<Value>::new(),
        "the asker and holder decides none of it"
    );

    let nobody = table.waiting(&table.nobody).await?;
    assert_eq!(
        kinds(&nobody),
        ["agent_draft", "product_draft", "request", "review"]
    );
    for part in &nobody {
        assert_eq!(part["refusal"], "NoPerson", "{part}");
        assert!(part["reason"].is_string(), "{part}");
        assert!(
            part["id"].is_null(),
            "a refused part carries no entry: {part}"
        );
    }
    Ok(())
}

/// The entry of `kind` in a waiting list, or null.
fn entry_of(items: &[Value], kind: &str) -> Value {
    items
        .iter()
        .find(|item| item["kind"] == kind)
        .cloned()
        .unwrap_or_default()
}

/// The member of `list` whose id is `id`, as that list answers it, or null.
fn listed(answer: &Value, list: &str, id: &str) -> Value {
    answer[list]
        .as_array()
        .and_then(|members| members.iter().find(|member| member["id"] == id))
        .cloned()
        .unwrap_or_default()
}

/// Whether `entry` carries `member` at all, null or not.
fn carries(entry: &Value, member: &str) -> bool {
    entry
        .as_object()
        .is_some_and(|members| members.contains_key(member))
}

#[tokio::test]
async fn a_request_and_a_draft_carry_the_record_their_own_lists_answer() -> Result {
    let table = Table::set().await?;
    let request = table.bea_asks().await?;
    let draft = table.agent_drafts().await?;
    let product = table.bea_product_drafts().await?;
    let review = table.agent_holds().await?;

    let ada = table.waiting(&table.ada).await?;
    let requests = ok(get(&table.service, "/requests", Auth::Cookie(&table.ada)).await?)?;
    let drafts = ok(get(&table.service, "/drafts", Auth::Cookie(&table.ada)).await?)?;

    // What each list answers is there to compare with: the comparison is never null against null.
    let asked = listed(&requests, "requests", &request);
    let prepared = listed(&drafts, "drafts", &draft);
    assert!(
        asked.is_object(),
        "GET /requests lists {request}: {requests}"
    );
    assert!(prepared.is_object(), "GET /drafts lists {draft}: {drafts}");

    let entry = entry_of(&ada, "request");
    assert_eq!(entry["id"], request.as_str(), "{ada:?}");
    assert_eq!(
        entry["request"], asked,
        "the request entry carries the request as GET /requests answers it"
    );
    let entry = entry_of(&ada, "agent_draft");
    assert_eq!(entry["id"], draft.as_str(), "{ada:?}");
    assert_eq!(
        entry["draft"], prepared,
        "the agent_draft entry carries the draft as GET /drafts answers it"
    );

    // Decided on their own pages: neither member, not even a null one.
    for (kind, id) in [("product_draft", &product), ("review", &table.agent)] {
        let entry = entry_of(&ada, kind);
        assert_eq!(entry["id"], id.as_str(), "{ada:?}");
        assert!(!carries(&entry, "request"), "{entry}");
        assert!(!carries(&entry, "draft"), "{entry}");
    }
    // A request carries no draft and a draft no request.
    assert!(!carries(&entry_of(&ada, "request"), "draft"));
    assert!(!carries(&entry_of(&ada, "agent_draft"), "request"));
    assert_eq!(
        entry_of(&ada, "review")["review"]["grants"][0]["id"],
        review
    );
    Ok(())
}

#[tokio::test]
async fn waiting_is_for_a_signed_in_caller_only() -> Result {
    let table = Table::set().await?;
    let (status, answer) = get(&table.service, "/waiting", Auth::Nobody).await?;
    assert_eq!(status, 401, "{answer}");
    Ok(())
}

#[test]
fn entries_are_ordered_oldest_first_after_the_refused_parts() {
    let entry = |kind: &'static str, at: u64| crate::waiting_api::Dated {
        at,
        entry: crate::waiting_api::WaitingEntry {
            kind,
            id: format!("{kind}-{at}"),
            asked_by: crate::waiting_api::Asker {
                id: "a".to_owned(),
                name: None,
            },
            about: String::new(),
            asked_at: String::new(),
            can_decide: true,
            request: None,
            draft: None,
            product_draft: None,
            review: None,
        },
    };
    let refused = crate::waiting_api::WaitingRefused {
        kind: "review",
        refusal: "ReviewsUnavailable".to_owned(),
        reason: "closed".to_owned(),
    };
    let list = crate::waiting_api::ordered(
        vec![
            entry("request", 30),
            entry("agent_draft", 10),
            entry("product_draft", 20),
        ],
        vec![refused],
    );
    let ids: Vec<String> = list
        .iter()
        .map(|item| match item {
            crate::waiting_api::WaitingItem::Entry(entry) => entry.id.clone(),
            crate::waiting_api::WaitingItem::Refused(part) => part.refusal.clone(),
        })
        .collect();
    assert_eq!(
        ids,
        [
            "ReviewsUnavailable",
            "agent_draft-10",
            "product_draft-20",
            "request-30"
        ]
    );
}

#[tokio::test]
async fn three_due_grants_are_one_agent_row_and_two_agents_are_two_rows() -> Result {
    let table = Table::set().await?;
    let mut grants = Vec::new();
    for _ in 0..3 {
        grants.push(table.agent_holds().await?);
    }
    let rows = table.waiting(&table.ada).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], table.agent);
    assert_eq!(rows[0]["review"]["agent"]["id"], table.agent);
    assert_eq!(rows[0]["review"]["count"], 3);
    for grant in rows[0]["review"]["grants"].as_array().ok_or("grants")? {
        assert!(grants.iter().any(|id| grant["id"] == *id));
        assert_eq!(grant["relation"], "beta");
        assert_eq!(grant["resource"], "doc quarter");
        assert_eq!(grant["last_check"], 0);
    }
    let (status, active) = table
        .service
        .post(
            &format!("/identities/{}/transitions", table.other_agent),
            Some(&table.ada),
            &json!({
                "operation": op()?, "transition": "activate", "reason": "second agent",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{active}");
    table.agent_holds_at(&table.other_agent, 0).await?;
    let rows = table.waiting(&table.ada).await?;
    assert_eq!(rows.len(), 2, "{rows:?}");
    Ok(())
}

#[tokio::test]
async fn giving_is_the_first_check_and_an_app_entry_carries_its_listed_draft() -> Result {
    let table = Table::set().await?;
    table.agent_holds_at(&table.agent, 1_800_000_000).await?;
    assert!(table.waiting(&table.ada).await?.is_empty());
    let old = table
        .agent_holds_at(&table.agent, 1_800_000_000 - 31 * 86_400)
        .await?;
    let rows = table.waiting(&table.ada).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["review"]["grants"][0]["id"], old);
    let (status, kept) = table
        .service
        .post(
            &format!("/reviews/{old}/keep"),
            Some(&table.ada),
            &json!({
                "operation": op()?, "note": "still needed",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{kept}");
    assert!(table.waiting(&table.ada).await?.is_empty());
    let product = table.bea_product_drafts().await?;
    let rows = table.waiting(&table.ada).await?;
    let drafts = ok(get(&table.service, "/product-drafts", Auth::Cookie(&table.ada)).await?)?;
    let draft = listed(&drafts, "drafts", &product);
    assert!(draft.is_object(), "{drafts}");
    assert_eq!(entry_of(&rows, "product_draft")["product_draft"], draft);
    table.clock.set(1_800_000_000 + 30 * 86_400 - 1);
    assert!(entry_of(&table.waiting(&table.ada).await?, "review").is_null());
    table.clock.set(1_800_000_000 + 30 * 86_400);
    assert_eq!(
        entry_of(&table.waiting(&table.ada).await?, "review")["review"]["count"],
        2
    );
    Ok(())
}
