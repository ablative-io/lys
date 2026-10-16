#![cfg(test)]
use super::super::Table;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, header};
use lys_identity::draft_event::Target;
use lys_identity::grants::{GrantId, Mode};
use lys_identity::product_draft_event::{
    Created, ProductDraftEvent, draft_operation, words_digest,
};
use lys_identity::{IdentityId, OperationId};
use serde_json::{Value, json};
use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Waker};
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const WORDS: &str = "private fixture draft words";

async fn table() -> Result<Table> {
    let table = Table::new().await?;
    table.register_app("fixture_product", true)?;
    crate::apps_api::refresh(&table.state)?;
    Ok(table)
}
fn draft(table: &Table) -> Result<String> {
    let holder = IdentityId::Agent(table.own.parse()?);
    let operation = draft_operation(holder, "fixture-read");
    let event = ProductDraftEvent::Created(Arc::new(Created {
        operation,
        client_operation: "fixture-read".to_owned(),
        holder,
        responsible: table.person,
        recorded_at: crate::session::now(),
        app: "fixture_product".to_owned(),
        grant: GrantId::from_bytes([9; 16]),
        mode: Mode::ByDraft,
        target: Target {
            kind: "fixture_product.agent".to_owned(),
            id: table.own.clone(),
            action: "read".to_owned(),
        },
        request_digest: words_digest(WORDS),
        words: WORDS.to_owned(),
    }));
    crate::routes::with_directory(&table.state, |directory| {
        directory.record_product_draft(event).map_err(Into::into)
    })?;
    Ok(operation.to_string())
}
fn agent(table: &Table, other: bool) -> Result<HeaderMap> {
    let id = if other { &table.other } else { &table.own };
    let pass = crate::agent_pass::store(&table.state)?.issue(id.parse()?, id, id)?;
    let mut headers = HeaderMap::new();
    headers.insert(crate::agent_pass::HEADER, pass.parse()?);
    Ok(headers)
}
async fn send(
    table: &Table,
    method: &str,
    path: &str,
    headers: HeaderMap,
    body: Option<Value>,
) -> Result<(u16, Value)> {
    let state = Arc::clone(&table.state);
    let router = crate::signed_first::guarded(
        crate::routes_table::router(Arc::clone(&state)),
        Arc::clone(&state),
    )
    .layer(axum::middleware::from_fn_with_state(
        Arc::clone(&state),
        crate::changes::observe,
    ));
    let router = crate::session_admission::guarded(router, state);
    let mut request = Request::builder().method(method).uri(path);
    *request.headers_mut().ok_or("request headers missing")? = headers;
    let bytes = body
        .map(|body| serde_json::to_vec(&body))
        .transpose()?
        .unwrap_or_default();
    let response = router
        .oneshot(
            request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(bytes))?,
        )
        .await?;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    Ok((
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        },
    ))
}

#[tokio::test]
async fn product_drafts_responsible_person_reads_one_held_draft() -> Result {
    let table = table().await?;
    let id = draft(&table)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {}", table.pass("fixture_product", false, true)?).parse()?,
    );
    let (status, answer) = send(
        &table,
        "GET",
        &format!("/product-drafts/{id}"),
        headers,
        None,
    )
    .await?;
    assert_eq!(
        status, 200,
        "the single held draft read answered {status}: {answer}"
    );
    assert_eq!(answer["state"], "held");
    assert_eq!(answer["target"]["kind"], "fixture_product.agent");
    assert_eq!(answer["target"]["id"], table.own);
    assert_eq!(answer["target"]["action"], "read");
    assert!(answer["decided_by"].is_null());
    assert!(answer["decided_at"].is_null());
    Ok(())
}

#[tokio::test]
async fn product_drafts_agent_waits_on_a_decision_signal_then_reads_its_answer() -> Result {
    let table = table().await?;
    let id = draft(&table)?;
    let headers = agent(&table, false)?;
    let (status, initial) = send(&table, "GET", "/changes", headers.clone(), None).await?;
    assert_eq!(
        status, 200,
        "the run pass could not read its change generation: {initial}"
    );
    let path = format!(
        "/changes?after={}",
        initial["generation"].as_str().ok_or("generation missing")?
    );
    let mut waiting = Box::pin(send(&table, "GET", &path, headers.clone(), None));
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(table.state.changes.waiting(), 1);
    let pass = table.pass("fixture_product", false, true)?;
    let mut person = HeaderMap::new();
    person.insert(header::AUTHORIZATION, format!("Bearer {pass}").parse()?);
    let (status, approved) = send(&table, "POST", &format!("/product-drafts/{id}/approve"), person, Some(json!({"operation":OperationId::generate()?.to_string(),"request_digest":crate::routes::hex(&words_digest(WORDS))}))).await?;
    assert_eq!(status, 200, "{approved}");
    let (status, signal) = waiting.await?;
    assert_eq!(status, 200, "{signal}");
    assert_ne!(signal["generation"], initial["generation"]);
    assert_eq!(table.state.changes.waiting(), 0);
    let (status, answer) = send(
        &table,
        "GET",
        &format!("/product-drafts/{id}"),
        headers,
        None,
    )
    .await?;
    assert_eq!(status, 200, "the one-draft read was refused: {answer}");
    assert_eq!(answer["state"], "approved");
    assert_eq!(answer["decided_by"], table.person.to_string());
    assert!(answer["decided_at"].as_u64().is_some());
    assert_eq!(answer["target"]["action"], "read");
    Ok(())
}

#[tokio::test]
async fn product_drafts_single_read_refuses_another_agent_and_an_unentitled_person() -> Result {
    let table = table().await?;
    let id = draft(&table)?;
    // A second person's app pass keeps that person's scope even when its session is the administrator's.
    let mut foreign_table = table;
    foreign_table.administrator()?;
    let mut foreign_person = HeaderMap::new();
    foreign_person.insert(
        header::AUTHORIZATION,
        format!(
            "Bearer {}",
            foreign_table.pass("fixture_product", false, true)?
        )
        .parse()?,
    );
    for headers in [agent(&foreign_table, true)?, foreign_person] {
        let (status, answer) = send(
            &foreign_table,
            "GET",
            &format!("/product-drafts/{id}"),
            headers,
            None,
        )
        .await?;
        assert_eq!(status, 403, "another identity read the draft: {answer}");
        assert_eq!(answer["refusal"], "NotAdmitted");
        assert!(!answer.to_string().contains(WORDS));
    }
    Ok(())
}
