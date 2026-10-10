#![cfg(test)]
//! A seat's start, stop, restart and message are deliberate acts under
//! rights (AGENTS-002 R3): asked of the grant engine with the caller's
//! identity, and refused naming the action and the request URL; an attach
//! is for the seat's responsible person or an administrator alone (R4).

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;

use identity_contract::fake_issuer::Login;
use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

/// The fixture's model, with one relation carrying every seat act.
const MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"],"operator":["seat.start","seat.stop","seat.restart","seat.send"]}}"#;

const BEA: &str = "bea-subject";

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

async fn bea(table: &Table) -> Result<String, Box<dyn Error>> {
    table
        .service
        .sign_in(Login {
            subject: BEA.to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn without_the_right_each_act_is_refused_naming_the_action_and_the_url() -> TestResult {
    let table = Table::set_judging(MODEL).await?;
    seated(&table).await?;
    let bea = bea(&table).await?;
    for (act, action, body) in [
        ("stop", "seat.stop", json!({ "operation": operation()? })),
        ("start", "seat.start", json!({ "operation": operation()? })),
        ("restart", "seat.restart", json!({ "operation": operation()?, "force": true })),
        (
            "send",
            "seat.send",
            json!({ "operation": operation()?, "text": "hello" }),
        ),
    ] {
        let path = format!("/seats/waffles/{act}");
        let answer = table.service.post(&path, Some(&bea), &body).await?;
        refused(&answer, 403, "not_permitted");
        let reason = answer.1["reason"].as_str().ok_or("no reason")?;
        assert!(reason.contains(action), "{reason}");
        assert!(reason.contains(&path), "{reason}");
    }
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_right_the_act_is_admitted_and_judged_on_the_seat() -> TestResult {
    let table = Table::set_judging(MODEL).await?;
    seated(&table).await?;
    let bea = bea(&table).await?;
    let granted = table
        .ok(
            "/grants/roots",
            &json!({
                "operation": operation()?,
                "route": "api",
                "holder": table.person(1),
                "resource": { "kind": "seat", "id": "waffles" },
                "relation": "operator",
                "pass_on": { "kind": "use_only" },
                "window": { "starts_at": 0, "ends_at": null },
            }),
        )
        .await?;
    assert!(granted["grant"].as_str().is_some(), "{granted}");
    // Admitted by the grant, the stop is judged on the seat: it runs nothing.
    let answer = table
        .service
        .post(
            "/seats/waffles/stop",
            Some(&bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 409, "seat_not_running");
    // Admitted by the grant, the start is judged on the profile: it is not managed.
    let answer = table
        .service
        .post(
            "/seats/waffles/start",
            Some(&bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 409, "seat_not_managed");
    // The grant names one seat; another is still refused.
    let answer = table
        .service
        .post(
            "/seats/absent/stop",
            Some(&bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 404, "seat_unknown");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_responsible_person_stands_without_a_grant_and_a_stranger_cannot_attach() -> TestResult
{
    let table = Table::set_judging(MODEL).await?;
    seated(&table).await?;
    let answer = table
        .service
        .post(
            "/seats/waffles/stop",
            Some(&table.ada),
            &json!({ "operation": operation()?, "force": true }),
        )
        .await?;
    refused(&answer, 409, "seat_not_running");
    let answer = table
        .service
        .post(
            "/seats/waffles/restart",
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 409, "seat_not_managed");
    let answer = table
        .service
        .post("/seats/waffles/attach", Some(&table.ada), &json!({}))
        .await?;
    refused(&answer, 409, "seat_not_running");
    let bea = bea(&table).await?;
    let answer = table
        .service
        .post("/seats/waffles/attach", Some(&bea), &json!({}))
        .await?;
    refused(&answer, 403, "seat_attach_refused");
    table.close()
}
