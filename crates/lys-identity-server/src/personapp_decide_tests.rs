#![cfg(test)]
use super::super::Table;
use lys_identity::draft_event::{Created, DraftEvent, Target};
use lys_identity::grants::{GrantId, Mode};
use lys_identity::product_draft_event::{self, ProductDraftEvent};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::{Value, json};
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const WORDS: &str = "a fixture permission request";

fn draft(table: &Table, person: PersonId, product: bool) -> Result<(String, Value)> {
    let actor = table
        .state
        .sessions
        .live(|_| true)?
        .into_iter()
        .find(|entry| entry.id == table.session)
        .ok_or("session missing")?
        .actor;
    let target = Target {
        kind: if product {
            "fixture_product.agent"
        } else {
            "agent"
        }
        .to_owned(),
        id: table.own.clone(),
        action: "read".to_owned(),
    };
    let (id, body) = if product {
        let holder = IdentityId::Agent(table.own.parse()?);
        let client_operation = OperationId::generate()?.to_string();
        let operation = product_draft_event::draft_operation(holder, &client_operation);
        let event = ProductDraftEvent::Created(Arc::new(product_draft_event::Created {
            operation,
            client_operation,
            holder,
            responsible: person,
            recorded_at: crate::session::now(),
            app: "fixture_product".to_owned(),
            grant: GrantId::from_bytes([9; 16]),
            mode: Mode::ByDraft,
            target,
            request_digest: product_draft_event::words_digest(WORDS),
            words: WORDS.to_owned(),
        }));
        crate::routes::with_directory(&table.state, |directory| {
            directory.record_product_draft(event).map_err(Into::into)
        })?;
        (
            operation,
            json!({"operation": OperationId::generate()?.to_string(), "request_digest": crate::routes::hex(&product_draft_event::words_digest(WORDS))}),
        )
    } else {
        let operation = OperationId::generate()?;
        let event = DraftEvent::Created(Arc::new(Created {
            operation,
            actor,
            recorded_at: crate::session::now(),
            target,
            method: "POST".to_owned(),
            path: format!("/agents/{}/wake", table.own),
            body: b"{}".to_vec(),
            note: "fixture".to_owned(),
            reviewer: IdentityId::Person(person),
            corrects: None,
            evidence: None,
            request_signature: None,
        }));
        let hash =
            lys_identity::encoding::payload_commitment(&lys_identity::draft_event::encode(&event));
        crate::routes::with_directory(&table.state, |directory| {
            directory.record_draft(event).map_err(Into::into)
        })?;
        (
            operation,
            json!({"operation": OperationId::generate()?.to_string(), "creation_hash": crate::routes::hex(&hash), "application": OperationId::generate()?.to_string()}),
        )
    };
    let prefix = if product { "product-drafts" } else { "drafts" };
    Ok((format!("/{prefix}/{id}/approve"), body))
}

fn decision(path: &str, mut body: Value, approve: bool) -> (String, Value) {
    if approve {
        (path.to_owned(), body)
    } else {
        if let Some(fields) = body.as_object_mut() {
            fields.remove("application");
            fields.insert("reason".to_owned(), json!("the responsible person refuses"));
        }
        (path.replace("/approve", "/refuse"), body)
    }
}

#[tokio::test]
async fn personapp_decide_admits_its_persons_operator_and_product_drafts() -> Result {
    let table = Table::app_fixture().await?;
    let pass = table.pass("fixture_product", false, true)?;
    let mut decisions = 0;
    for product in [false, true] {
        for approve in [false, true] {
            let (path, body) = draft(&table, table.person, product)?;
            let (path, body) = decision(&path, body, approve);
            let (status, answer) = table.send("POST", &path, &pass, &body, &[]).await?;
            assert_eq!(
                status, 200,
                "the own person's draft decision was refused: {answer}"
            );
            decisions += 1;
        }
    }
    assert_eq!(decisions, 4);
    Ok(())
}

async fn refuses_another_person(product: bool) -> Result {
    let mut table = Table::app_fixture().await?;
    let other = PersonId::from_str(&table.other_person()?)?;
    table.administrator()?;
    let pass = table.pass("fixture_product", false, true)?;
    let mut refusals = 0;
    for approve in [false, true] {
        let (path, body) = draft(&table, other, product)?;
        let (path, body) = decision(&path, body, approve);
        let (status, answer) = table.send("POST", &path, &pass, &body, &[]).await?;
        assert_eq!(
            status, 403,
            "a person-app pass decided another person's draft: {answer}"
        );
        assert_eq!(answer["refusal"], "not_permitted");
        refusals += 1;
    }
    assert_eq!(refusals, 2);
    Ok(())
}

#[tokio::test]
async fn personapp_decide_refuses_another_persons_operator_draft() -> Result {
    refuses_another_person(false).await
}

#[tokio::test]
async fn personapp_decide_refuses_another_persons_product_draft() -> Result {
    refuses_another_person(true).await
}
