#![cfg(test)]
//! Seat edits preserve reviewed profile selection, retirement history and refusal names.

#[path = "support/runner_start.rs"]
pub mod support;

use identity_contract::fake_issuer::Login;
use serde_json::{Value, json};
use std::error::Error;
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

async fn profile(table: &Table, from: u32, harness: Option<Value>) -> TestResult {
    table
        .ok(
            &format!("/agents/{}/provisioning", table.agent()),
            &json!({
                "operation": operation()?, "from_version": from,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
                "mcp_servers": [], "instructions": "", "note": "", "harness": harness,
                "permissions": {"default_mode": "plan"}, "working_folder": table.dir.path(),
            }),
        )
        .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn stopped_seat_edits_are_checked_idempotent_and_retained_across_restart() -> TestResult {
    let mut table = Table::set().await?;
    let machine = table.lys_machine("Fixture").await?;
    let add = json!({"operation":operation()?,"name":"fixture-seat","agent":table.agent(),"profile_version":1,"machine":machine});
    table.ok("/seats", &add).await?;
    version_controls(&table).await?;
    retirement_controls(&mut table, &add).await?;
    table.close()
}

async fn version_controls(table: &Table) -> TestResult {
    let path = "/seats/fixture-seat/version";
    let edit = |version| -> Result<Value, Box<dyn Error>> {
        Ok(json!({"operation":operation()?,"profile_version":version}))
    };
    let unknown = edit(99)?;
    refused(
        &table.service.post(path, Some(&table.ada), &unknown).await?,
        404,
        "ProfileVersionUnknown",
    );
    profile(table, 1, Some(table.harness())).await?;
    let unreviewed = edit(2)?;
    refused(
        &table
            .service
            .post(path, Some(&table.ada), &unreviewed)
            .await?,
        409,
        "ProfileNotReviewed",
    );
    table
        .ok(
            &format!("/agents/{}/provisioning/2/review", table.agent()),
            &json!({"operation":operation()?}),
        )
        .await?;
    let moved = edit(2)?;
    assert_eq!(table.ok(path, &moved).await?["profile_version"], 2);
    assert_eq!(table.ok(path, &moved).await?["revision"], 2);
    let mut reused = moved.clone();
    reused["profile_version"] = json!(1);
    refused(
        &table.service.post(path, Some(&table.ada), &reused).await?,
        409,
        "seat_operation_reused",
    );
    let stranger = table
        .service
        .sign_in(Login {
            subject: "bea-subject".to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let version_request = edit(2)?;
    let retirement_request = json!({"operation":operation()?});
    for (route, body) in [
        (path, version_request),
        ("/seats/fixture-seat/retire", retirement_request),
    ] {
        refused(
            &table.service.post(route, Some(&stranger), &body).await?,
            403,
            "NotAdmitted",
        );
        refused(
            &table.service.post(route, None, &body).await?,
            401,
            "NotSignedIn",
        );
    }
    profile(table, 2, None).await?;
    table
        .ok(
            &format!("/agents/{}/provisioning/3/review", table.agent()),
            &json!({"operation":operation()?}),
        )
        .await?;
    let undeclared = edit(3)?;
    refused(
        &table
            .service
            .post(path, Some(&table.ada), &undeclared)
            .await?,
        409,
        "HarnessUndeclared",
    );
    Ok(())
}

async fn retirement_controls(table: &mut Table, add: &Value) -> TestResult {
    let retire = json!({"operation":operation()?});
    assert_eq!(
        table.ok("/seats/fixture-seat/retire", &retire).await?["name"],
        "fixture-seat"
    );
    table.ok("/seats/fixture-seat/retire", &retire).await?;
    refused(
        &table
            .service
            .get("/seats/fixture-seat", Some(&table.ada))
            .await?,
        404,
        "seat_unknown",
    );
    table.service.restart().await?;
    let listed = table.service.get("/seats", Some(&table.ada)).await?;
    assert!(listed.1["seats"].as_array().ok_or("no seats")?.is_empty());
    let mut again = add.clone();
    again["operation"] = json!(operation()?);
    assert_eq!(table.ok("/seats", &again).await?["revision"], 1);
    for (route, body) in [
        (
            "/seats/absent/version",
            json!({"operation":operation()?,"profile_version":1}),
        ),
        ("/seats/absent/retire", json!({"operation":operation()?})),
    ] {
        refused(
            &table.service.post(route, Some(&table.ada), &body).await?,
            404,
            "seat_unknown",
        );
    }
    Ok(())
}
