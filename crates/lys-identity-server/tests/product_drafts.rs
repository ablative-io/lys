#![cfg(test)]
//! Product drafts (ACCESS-001 R3, ACCESS-003 R2): a product holds an act a
//! held grant names as a draft; by draft is approved by one person who may
//! grant the action, by two by two distinct ones, and never by the holder;
//! only the app's own connector reads its approved drafts and closes each
//! once; and an approved draft stays counted until it is closed. The last
//! test drives lys-pass's own client against these routes.

#[path = "support/product_drafts.rs"]
mod support;

use identity_contract::apps::{Auth, FILES, NOTES, TestResult, get, ok, op, post, registered};
use lys_pass::drafts::{
    ApprovedDraft, DraftRequest, Execution, Executor, RunError, request_digest,
};
use serde_json::{Value, json};
use support::{DOC, Table, decision, refused_as, request};

const WORDS: &str = "{\"stop\":\"seat-7\"}";

async fn create(
    table: &Table,
    cookie: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    post(
        &table.service,
        "/product-drafts",
        Auth::Cookie(cookie),
        body,
    )
    .await
}

async fn decide(
    table: &Table,
    cookie: &str,
    draft: &str,
    act: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    post(
        &table.service,
        &format!("/product-drafts/{draft}/{act}"),
        Auth::Cookie(cookie),
        body,
    )
    .await
}

async fn approved(
    table: &Table,
    credential: &str,
    app: &str,
) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    get(
        &table.service,
        &format!("/product-drafts?app={app}&state=approved"),
        Auth::Bearer(credential),
    )
    .await
}

async fn close(
    table: &Table,
    credential: &str,
    draft: &str,
    route: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    post(
        &table.service,
        &format!("/product-drafts/{draft}/{route}"),
        Auth::Bearer(credential),
        body,
    )
    .await
}

/// Bea's draft of `words` on `fixture_notes.doc:1` under a grant in `mode`.
async fn drafted(
    table: &Table,
    mode: &str,
    words: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let grant = table.bea_holds("1", mode).await?;
    let operation = op()?;
    let body = request(&operation, &grant, "1", words);
    let answered = ok(create(table, &table.bea, &body).await?)?;
    Ok(answered["draft"].as_str().ok_or("a draft id")?.to_owned())
}

#[tokio::test]
async fn by_two_waits_for_two_distinct_approvers_and_the_holder_is_refused() -> TestResult {
    let table = Table::set().await?;
    let grant = table.bea_holds("1", "by_two").await?;
    let body = request("bea-act-1", &grant, "1", WORDS);
    let made = ok(create(&table, &table.bea, &body).await?)?;
    let draft = made["draft"].as_str().ok_or("a draft id")?;
    assert_eq!(
        ok(create(&table, &table.bea, &body).await?)?,
        made,
        "the same operation and request answer the same draft"
    );
    let other = request("bea-act-1", &grant, "1", "{\"stop\":\"seat-8\"}");
    refused_as(
        &create(&table, &table.bea, &other).await?,
        "OperationReused",
    )?;

    let own = decide(&table, &table.bea, draft, "approve", &decision(WORDS)?).await?;
    refused_as(&own, "product_draft_approver_refused")?;
    let first = ok(decide(&table, &table.admin, draft, "approve", &decision(WORDS)?).await?)?;
    assert_eq!(first["draft"], draft);
    assert_eq!(
        first["state"], "waiting",
        "one approval of a by_two draft: {first}"
    );
    let again = decide(&table, &table.admin, draft, "approve", &decision(WORDS)?).await?;
    refused_as(&again, "product_draft_approver_refused")?;
    let page = ok(approved(&table, &table.notes, NOTES).await?)?;
    assert_eq!(
        page["drafts"],
        json!([]),
        "not answered as approved: {page}"
    );

    let second = ok(decide(&table, &table.cy, draft, "approve", &decision(WORDS)?).await?)?;
    assert_eq!(second["state"], "approved", "{second}");
    let page = ok(approved(&table, &table.notes, NOTES).await?)?;
    assert_eq!(page["total"], 1, "{page}");
    assert_eq!(page["drafts"][0]["id"], draft);
    assert_eq!(page["drafts"][0]["words"], WORDS);
    assert_eq!(page["drafts"][0]["grant"], grant.as_str());
    Ok(())
}

#[tokio::test]
async fn a_decision_names_the_exact_words_it_decides() -> TestResult {
    let table = Table::set().await?;
    let draft = drafted(&table, "by_draft", WORDS).await?;
    let other = decide(
        &table,
        &table.admin,
        &draft,
        "approve",
        &decision("other words")?,
    )
    .await?;
    refused_as(&other, "DraftHashMismatch")?;
    let mut refusal = decision(WORDS)?;
    refusal["reason"] = json!("not this seat");
    let refused = ok(decide(&table, &table.admin, &draft, "refuse", &refusal).await?)?;
    assert_eq!(refused["state"], "refused");
    let late = decide(&table, &table.cy, &draft, "approve", &decision(WORDS)?).await?;
    refused_as(&late, "DraftNotPending")?;
    let closed = close(
        &table,
        &table.notes,
        &draft,
        "executed",
        &json!({"receipt_digest": request_digest("receipt")}),
    )
    .await?;
    refused_as(&closed, "product_draft_not_approved")?;
    Ok(())
}

#[tokio::test]
async fn only_a_held_grant_of_the_callers_reaching_the_target_with_the_words_digest_makes_a_draft()
-> TestResult {
    let table = Table::set().await?;
    let outright = table.bea_holds("2", "outright").await?;
    let refused = create(&table, &table.bea, &request(&op()?, &outright, "2", WORDS)).await?;
    refused_as(&refused, "product_draft_grant_refused")?;
    let held = table.bea_holds("1", "by_draft").await?;
    let not_holder = create(&table, &table.admin, &request(&op()?, &held, "1", WORDS)).await?;
    refused_as(&not_holder, "product_draft_grant_refused")?;
    let elsewhere = create(&table, &table.bea, &request(&op()?, &held, "9", WORDS)).await?;
    refused_as(&elsewhere, "product_draft_grant_refused")?;
    let mut wrong = request(&op()?, &held, "1", WORDS);
    wrong["request_digest"] = json!(request_digest("other words"));
    refused_as(
        &create(&table, &table.bea, &wrong).await?,
        "product_draft_digest_mismatch",
    )?;
    Ok(())
}

#[tokio::test]
async fn another_apps_connector_reads_and_closes_none_and_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let files = registered(&table.service, &table.admin, FILES).await?;
    let draft = drafted(&table, "by_draft", WORDS).await?;
    ok(decide(&table, &table.admin, &draft, "approve", &decision(WORDS)?).await?)?;
    refused_as(
        &approved(&table, &files, NOTES).await?,
        "product_draft_not_your_app",
    )?;
    let own = ok(approved(&table, &files, FILES).await?)?;
    assert_eq!(own["drafts"], json!([]), "{own}");
    assert_eq!(own["total"], 0);
    let closed = close(
        &table,
        &files,
        &draft,
        "executed",
        &json!({"receipt_digest": request_digest("receipt")}),
    )
    .await?;
    refused_as(&closed, "product_draft_not_your_app")?;
    let person = get(
        &table.service,
        &format!("/product-drafts?app={NOTES}&state=approved"),
        Auth::Cookie(&table.bea),
    )
    .await?;
    refused_as(&person, "NotAdmitted")?;
    Ok(())
}

async fn waiting(table: &Table) -> Result<Value, Box<dyn std::error::Error>> {
    let dashboard = ok(get(&table.service, "/dashboard", Auth::Cookie(&table.admin)).await?)?;
    Ok(dashboard["waiting"]["product_drafts"].clone())
}

#[tokio::test]
async fn executed_and_refused_on_execution_close_the_draft_and_the_dashboard_count_falls()
-> TestResult {
    let table = Table::set().await?;
    let done = drafted(&table, "by_draft", WORDS).await?;
    let declined = drafted(&table, "by_draft", "{\"stop\":\"seat-9\"}").await?;
    assert_eq!(waiting(&table).await?, 2);
    ok(decide(&table, &table.admin, &done, "approve", &decision(WORDS)?).await?)?;
    ok(decide(
        &table,
        &table.admin,
        &declined,
        "approve",
        &decision("{\"stop\":\"seat-9\"}")?,
    )
    .await?)?;
    assert_eq!(
        waiting(&table).await?,
        2,
        "approved and unexecuted stay counted"
    );

    let receipt = json!({"receipt_digest": request_digest("receipt")});
    let first = ok(close(&table, &table.notes, &done, "executed", &receipt).await?)?;
    assert_eq!(first["state"], "executed");
    assert_eq!(
        ok(close(&table, &table.notes, &done, "executed", &receipt).await?)?,
        first
    );
    let other = json!({"receipt_digest": request_digest("another receipt")});
    refused_as(
        &close(&table, &table.notes, &done, "executed", &other).await?,
        "product_draft_closed",
    )?;
    assert_eq!(waiting(&table).await?, 1);

    let refusal = json!({"refusal": "seat_gone", "reason": "the seat had already stopped"});
    let closed = ok(close(
        &table,
        &table.notes,
        &declined,
        "refused-on-execution",
        &refusal,
    )
    .await?)?;
    assert_eq!(closed["state"], "refused_on_execution");
    assert_eq!(waiting(&table).await?, 0);
    let page = ok(approved(&table, &table.notes, NOTES).await?)?;
    assert_eq!(page["total"], 0, "{page}");

    let seen = ok(get(&table.service, "/product-drafts", Auth::Cookie(&table.bea)).await?)?;
    let states: Vec<&Value> = seen["drafts"]
        .as_array()
        .ok_or("a list")?
        .iter()
        .map(|draft| &draft["state"])
        .collect();
    assert_eq!(states.len(), 2, "the holder sees her own drafts: {seen}");
    for draft in seen["drafts"].as_array().ok_or("a list")? {
        assert_eq!(draft["execution"]["state"], draft["state"], "{draft}");
        assert_eq!(draft["mode"], "by_draft");
        assert_eq!(draft["approvals"].as_array().map(Vec::len), Some(1));
    }
    Ok(())
}

/// The ids of a page's drafts, and its `next` and `total`.
/// A page of drafts: their ids in order, the cursor to the next, and the total.
type Paged = (Vec<String>, Option<String>, u64);

fn paged(page: &Value) -> Result<Paged, Box<dyn std::error::Error>> {
    let ids = page["drafts"].as_array().ok_or("a list")?.iter();
    let ids = ids.filter_map(|draft| draft["id"].as_str().map(str::to_owned));
    let next = page["next"].as_str().map(str::to_owned);
    Ok((
        ids.collect(),
        next,
        page["total"].as_u64().ok_or("a total")?,
    ))
}

/// Both views of `GET /product-drafts` are paged as every list is: the
/// caller's `limit` rows in draft id order, a `next` cursor while more
/// remain, and zero refused. lys-pass follows the connector's cursor.
#[tokio::test]
async fn both_views_answer_the_callers_page_and_a_next_cursor() -> TestResult {
    let table = Table::set().await?;
    let mut made = Vec::new();
    for words in [WORDS, "{\"stop\":\"seat-8\"}", "{\"stop\":\"seat-9\"}"] {
        let draft = drafted(&table, "by_draft", words).await?;
        ok(decide(&table, &table.admin, &draft, "approve", &decision(words)?).await?)?;
        made.push(draft);
    }
    made.sort();
    let bea = Auth::Cookie(&table.bea);
    let page = get(&table.service, "/product-drafts?limit=2", bea).await?;
    let (ids, next, total) = paged(&ok(page)?)?;
    assert_eq!((ids.len(), total), (2, 3), "a first page of two of three");
    let next = next.ok_or("a cursor while more remain")?;
    let path = format!("/product-drafts?limit=2&after={next}");
    let last = paged(&ok(
        get(&table.service, &path, Auth::Cookie(&table.bea)).await?
    )?)?;
    assert_eq!((last.0.len(), &last.1, last.2), (1, &None, 3), "{last:?}");
    assert_eq!([ids, last.0].concat(), made, "in draft id order, each once");
    let zero = get(
        &table.service,
        "/product-drafts?limit=0",
        Auth::Cookie(&table.bea),
    );
    refused_as(&zero.await?, "RequestMalformed")?;

    let connector = format!("/product-drafts?app={NOTES}&state=approved&limit=1");
    let one = get(&table.service, &connector, Auth::Bearer(&table.notes)).await?;
    let (ids, next, total) = paged(&ok(one)?)?;
    assert_eq!((ids.as_slice(), total), (&made[..1], 3), "one of three");
    let next = next.ok_or("a cursor while more remain")?;
    let client = lys_pass::Client::new(
        reqwest::Client::new(),
        reqwest::Url::parse(&table.service.base)?,
    )?;
    let rest = client
        .approved_drafts(&table.notes, NOTES, Some(&next))
        .await?;
    let rest: Vec<String> = rest.drafts.into_iter().map(|draft| draft.id).collect();
    assert_eq!(rest, &made[1..], "lys-pass follows the connector's cursor");
    Ok(())
}

/// A product's durable ledger: it executes each draft once.
#[derive(Default)]
struct Ledger {
    receipts: std::collections::BTreeMap<String, Execution>,
    writes: usize,
}

impl Executor for Ledger {
    type Error = std::io::Error;

    fn receipt(&self, draft: &str) -> Result<Option<Execution>, Self::Error> {
        Ok(self.receipts.get(draft).cloned())
    }

    fn execute_and_record(&mut self, draft: &ApprovedDraft) -> Result<Execution, Self::Error> {
        self.writes += 1;
        let receipt = Execution::Executed {
            request_digest: draft.request_digest.clone(),
            receipt_digest: request_digest(&format!("applied {}", draft.id)),
        };
        self.receipts.insert(draft.id.clone(), receipt.clone());
        Ok(receipt)
    }
}

/// ACCESS-003 R2 against the real routes: lys-pass's draft body is what the
/// server records, its approved page parses, its runner executes each draft
/// once and records it, and a second run finds nothing left. `create_draft`
/// with the pass Lys issued Bea for the app records her held act (the same
/// operation and request answering the same draft); with the connector's
/// credential, which does not hold her grant, or a forged pass, it is refused.
#[tokio::test]
async fn lys_pass_hands_off_and_runs_approved_drafts_against_the_real_routes() -> TestResult {
    let table = Table::set().await?;
    let grant = table.bea_holds("1", "by_draft").await?;
    let held = DraftRequest {
        operation: "product-op-7".to_owned(),
        grant: grant.clone(),
        target: lys_pass::Target::new(DOC, "1", "write")?,
        request_digest: request_digest(WORDS),
        words: WORDS.to_owned(),
    };
    let made = ok(create(&table, &table.bea, &serde_json::to_value(&held)?).await?)?;
    let draft = made["draft"].as_str().ok_or("a draft id")?.to_owned();
    let client = lys_pass::Client::new(
        reqwest::Client::new(),
        reqwest::Url::parse(&table.service.base)?,
    )?;
    let judgment = lys_pass::deliberate::CheckAnswer {
        allowed: false,
        grant: Some(grant.clone()),
        mode: Some(lys_pass::Mode::ByDraft),
        path: Vec::new(),
        via: None,
        refusal: None,
        reason: None,
        degraded: None,
    };
    let pass = table.bea_pass().await?;
    let created = client.create_draft(&pass, &held, &judgment).await?;
    assert_eq!(
        created.draft, draft,
        "Bea's pass records her held act; the same operation and request answer the same draft"
    );
    assert!(
        client
            .create_draft(&table.notes, &held, &judgment)
            .await
            .is_err(),
        "the connector does not hold Bea's grant"
    );
    let mut forged = pass.clone();
    let last = forged.pop().ok_or("an empty pass")?;
    forged.push(if last == 'A' { 'B' } else { 'A' });
    refused_as(
        &post(
            &table.service,
            "/product-drafts",
            Auth::Bearer(&forged),
            &serde_json::to_value(&held)?,
        )
        .await?,
        "PassRefused",
    )?;

    let empty = client.approved_drafts(&table.notes, NOTES, None).await?;
    assert_eq!((empty.drafts.len(), empty.total), (0, 0));
    ok(decide(&table, &table.admin, &draft, "approve", &decision(WORDS)?).await?)?;
    let page = client.approved_drafts(&table.notes, NOTES, None).await?;
    assert_eq!(page.drafts.len(), 1);
    assert_eq!(page.drafts[0].id, draft);
    assert_eq!(page.drafts[0].words, WORDS);

    let mut ledger = Ledger::default();
    let report = client
        .run_approved(&table.notes, NOTES, None, &mut ledger)
        .await;
    let report = report.map_err(|error: RunError<std::io::Error>| error.to_string())?;
    assert_eq!((report.acknowledged, ledger.writes), (1, 1));
    let again = client
        .run_approved(&table.notes, NOTES, None, &mut ledger)
        .await;
    assert_eq!(again.map_err(|error| error.to_string())?.acknowledged, 0);
    assert_eq!(ledger.writes, 1, "a draft is executed once");
    let receipt = ledger.receipts.get(&draft).ok_or("a receipt")?;
    client
        .record_execution(&table.notes, &page.drafts[0], receipt)
        .await?;
    Ok(())
}
