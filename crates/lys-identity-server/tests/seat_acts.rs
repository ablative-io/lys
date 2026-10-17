#![cfg(test)]
//! A seat's start, stop, restart and message are deliberate acts under
//! rights (AGENTS-002 R3): asked of the grant engine with the caller's
//! identity, and refused naming the action and the request URL; an attach
//! is for the seat's responsible person or an administrator alone (R4).

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;

use identity_contract::fake_issuer::Login;
use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

/// The fixture's model, with one relation carrying every seat act.
const MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"],"operator":["seat.start","seat.stop","seat.restart","seat.send"]}}"#;

const BEA: &str = "bea-subject";

async fn seated(table: &Table) -> Result<(), Box<dyn Error>> {
    let program = table.dir.path().join("declared-harness");
    let fixture = std::fs::read_to_string(&program)?;
    std::fs::write(
        &program,
        fixture.replacen(
            "#!/bin/sh\n",
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 2.1.285; exit 0; fi\n",
            1,
        ),
    )?;
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

fn live_pty(table: &Table, answer: &(u16, Value)) -> Result<String, Box<dyn Error>> {
    assert_eq!(answer.0, 200, "{}", answer.1);
    assert_eq!(answer.1["seat"]["profile_version"], 1);
    let session = answer.1["session"].as_str().ok_or("no session")?;
    let runner = table.serving.as_ref().ok_or("no runner")?.sessions();
    let live = runner.liveness(Some(session))?;
    assert_eq!(live.len(), 1, "{live:?}");
    assert!(live[0].alive, "{live:?}");
    assert!(!live[0].harness.managed, "{live:?}");
    assert!(!live[0].ended, "{live:?}");
    let client = lys_runner::Client::new(
        table.dir.path().join("runner.sock"),
        std::sync::Arc::clone(&table.server_key),
    );
    let ready = client.ask(&lys_runner::Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: "profile-read".to_owned(),
        regex: false,
    })?;
    assert!(
        matches!(ready, lys_runner::Answer::Matched { .. }),
        "{ready:?}"
    );
    Ok(session.to_owned())
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
        (
            "restart",
            "seat.restart",
            json!({ "operation": operation()?, "force": true }),
        ),
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
    // The grant admits the reviewed profile's terminal launch.
    let answer = table
        .service
        .post(
            "/seats/waffles/start",
            Some(&bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    live_pty(&table, &answer)?;
    assert_eq!(answer.1["admission"]["by"], "grant");
    assert_eq!(answer.1["admission"]["action"], "seat.start");
    assert_eq!(answer.1["admission"]["grant"], granted["grant"]);
    assert!(answer.1["admission"]["use_event"].as_u64().is_some());
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

mod seat_attach {
    use super::{MODEL, Table, TestResult, bea, live_pty, operation, refused, seated};
    use serde_json::json;

    #[tokio::test(flavor = "multi_thread")]
    async fn the_responsible_person_stands_without_a_grant_and_a_stranger_cannot_attach()
    -> TestResult {
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
        let session = live_pty(&table, &answer)?;
        assert_eq!(answer.1["admission"]["by"], "administrator");
        assert_eq!(answer.1["admission"]["action"], "seat.restart");
        let stopped = table
            .ok(
                "/seats/waffles/stop",
                &json!({ "operation": operation()?, "force": true }),
            )
            .await?;
        assert_eq!(stopped["ended"], true, "{stopped}");
        let runner = table.serving.as_ref().ok_or("no runner")?.sessions();
        assert!(!runner.liveness(Some(&session))?[0].alive);
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
}
