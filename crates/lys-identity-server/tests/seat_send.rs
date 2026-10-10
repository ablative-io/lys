#![cfg(test)]
//! A message to a seat (AGENTS-002 R6) is text delivered to its running
//! session as a user turn: empty text and control characters are refused by
//! name, and a seat that is not running is refused `seat_not_running`; a
//! line typed in an attach is the same message by another route.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;

use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

async fn seated(table: &Table) -> Result<(), Box<dyn Error>> {
    let machine = operation()?;
    table
        .machine(
            &json!({
                "operation": machine, "name": "Box", "kind": "laptop", "runtime": "sh",
                "slots": 1, "may_run": [table.agent()], "may_reach": [],
            }),
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    table
        .ok(
            "/seats",
            &json!({
                "operation": operation()?, "name": "waffles", "agent": table.agent(),
                "profile_version": 1, "machine": machine,
            }),
        )
        .await?;
    Ok(())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_is_refused_by_name_before_it_reaches_any_session() -> TestResult {
    let table = Table::set().await?;
    seated(&table).await?;
    for route in ["send", "type"] {
        let path = format!("/seats/waffles/{route}");
        let send = |text: &str| -> Result<Value, Box<dyn Error>> {
            Ok(json!({ "operation": operation()?, "text": text }))
        };
        let answer = table
            .service
            .post(&path, Some(&table.ada), &send("hello\u{1b}[2J")?)
            .await?;
        refused(&answer, 400, "seat_text_control");
        let answer = table
            .service
            .post(&path, Some(&table.ada), &send("bell\u{7}")?)
            .await?;
        refused(&answer, 400, "seat_text_control");
        let answer = table
            .service
            .post(&path, Some(&table.ada), &send(" \n\t ")?)
            .await?;
        refused(&answer, 400, "seat_text_empty");
        // Line breaks and tabs are text: refused only because nothing runs.
        let answer = table
            .service
            .post(&path, Some(&table.ada), &send("two\nlines\tand a tab")?)
            .await?;
        refused(&answer, 409, "seat_not_running");
        let answer = table
            .service
            .post(&path, Some(&table.ada), &json!({ "text": "no operation" }))
            .await?;
        refused(&answer, 400, "RequestMalformed");
    }
    let answer = table
        .service
        .post(
            "/seats/absent/send",
            Some(&table.ada),
            &json!({ "operation": operation()?, "text": "hello" }),
        )
        .await?;
    refused(&answer, 404, "seat_unknown");
    let (_, acts) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(
        acts["sessions"].as_array().map(Vec::len),
        Some(0),
        "no refused message started or reached a session: {acts}"
    );
    table.close()
}
