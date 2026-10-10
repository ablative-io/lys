#![cfg(test)]
//! A seat is a Lys record (AGENTS-002 R1, R2): added by name for an agent,
//! a reviewed profile version and a machine with a runner; seen with its
//! runner's own knowledge; started only managed; and an install holds one
//! machine naming its own runner.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;

use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

fn machine_body(table: &Table, name: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": operation()?, "name": name, "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [table.agent()], "may_reach": [],
    }))
}

fn seat_body(table: &Table, name: &str, machine: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": operation()?, "name": name, "agent": table.agent(),
        "profile_version": 1, "machine": machine,
    }))
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_seat_is_added_once_by_name_and_listed_not_seen() -> TestResult {
    let table = Table::set().await?;
    let machine = table
        .machine(
            &machine_body(&table, "Box")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    let body = seat_body(&table, "waffles", &machine)?;
    let added = table.ok("/seats", &body).await?;
    assert_eq!(added["name"], "waffles", "{added}");
    assert_eq!(added["agent"], table.agent());
    assert_eq!(added["harness"], "Claude Code");
    assert_eq!(added["profile_version"], 1);
    assert_eq!(added["machine"], machine);
    assert_eq!(added["state"], "not-seen", "{added}");
    assert_eq!(added["responsible"], table.person(0));
    assert_eq!(added["revision"], 1);

    // The same seat sent again answers what was kept and adds nothing.
    let again = table.ok("/seats", &body).await?;
    assert_eq!(again["name"], "waffles");
    assert_eq!(again["revision"], 1);

    let mut reused = body.clone();
    reused["name"] = json!("other");
    let answer = table
        .service
        .post("/seats", Some(&table.ada), &reused)
        .await?;
    refused(&answer, 409, "seat_operation_reused");

    let taken = seat_body(&table, "waffles", &machine)?;
    let answer = table
        .service
        .post("/seats", Some(&table.ada), &taken)
        .await?;
    refused(&answer, 409, "seat_name_taken");

    for invalid in ["Waffles", "", "a b", &"x".repeat(65)] {
        let body = seat_body(&table, invalid, &machine)?;
        let answer = table
            .service
            .post("/seats", Some(&table.ada), &body)
            .await?;
        refused(&answer, 400, "seat_name_invalid");
    }

    let mut unknown_version = seat_body(&table, "gaia", &machine)?;
    unknown_version["profile_version"] = json!(9);
    let answer = table
        .service
        .post("/seats", Some(&table.ada), &unknown_version)
        .await?;
    refused(&answer, 404, "ProfileVersionUnknown");

    let (status, listed) = table.service.get("/seats", Some(&table.ada)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["runner"], "read", "{listed}");
    let seats = listed["seats"].as_array().ok_or("no seats")?;
    assert_eq!(seats.len(), 1, "{listed}");
    assert_eq!(seats[0]["state"], "not-seen");

    let (status, one) = table
        .service
        .get("/seats/waffles", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{one}");
    assert_eq!(one["name"], "waffles");
    let answer = table.service.get("/seats/absent", Some(&table.ada)).await?;
    refused(&answer, 404, "seat_unknown");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_seat_needs_a_machine_with_a_runner() -> TestResult {
    let table = Table::set().await?;
    let bare = table.machine(&machine_body(&table, "Bare")?, None).await?;
    let body = seat_body(&table, "waffles", &bare)?;
    let answer = table
        .service
        .post("/seats", Some(&table.ada), &body)
        .await?;
    refused(&answer, 409, "MachineWithoutRunner");
    let body = seat_body(&table, "waffles", "op-00000000000000000000000000000009")?;
    let answer = table
        .service
        .post("/seats", Some(&table.ada), &body)
        .await?;
    refused(&answer, 404, "MachineUnknown");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_machine_naming_this_installs_runner_is_refused_naming_the_first() -> TestResult {
    let table = Table::set().await?;
    let first = table
        .machine(
            &machine_body(&table, "First")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    let second = table
        .machine(&machine_body(&table, "Second")?, None)
        .await?;
    let answer = table
        .service
        .post(
            &format!("/network/machines/{second}/runner"),
            Some(&table.ada),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
    refused(&answer, 409, "runner_lys_taken");
    assert!(
        answer.1["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains(&first)),
        "{}",
        answer.1
    );
    // Naming it again for the machine that holds it is not a second one.
    table
        .ok(
            &format!("/network/machines/{first}/runner"),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_seat_whose_profile_does_not_require_controls_is_not_started() -> TestResult {
    let table = Table::set().await?;
    let machine = table
        .machine(
            &machine_body(&table, "Box")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    table
        .ok("/seats", &seat_body(&table, "waffles", &machine)?)
        .await?;
    let answer = table
        .service
        .post(
            "/seats/waffles/start",
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 409, "seat_not_managed");
    let (_, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(
        live["sessions"].as_array().map(Vec::len),
        Some(0),
        "a refused start starts nothing: {live}"
    );
    let answer = table
        .service
        .post(
            "/seats/absent/start",
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 404, "seat_unknown");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_the_runner_holds_for_no_seat_is_seen_but_unregistered() -> TestResult {
    let table = Table::set().await?;
    let machine = table
        .machine(
            &machine_body(&table, "Box")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    let (status, started) = table
        .start(
            &table.agent(),
            &json!({ "machine": machine, "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 200, "{started}");
    let session = started["session"].as_str().ok_or("no session")?;
    let (status, unregistered) = table
        .service
        .get("/seats/unregistered", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{unregistered}");
    let listed = unregistered["sessions"]
        .as_array()
        .and_then(|all| all.iter().find(|held| held["session"] == session))
        .ok_or("the session is not listed unregistered")?;
    assert_eq!(listed["machine"], machine);
    assert_eq!(listed["agent"], table.agent());
    assert!(listed["pid"].as_u64().is_some(), "{listed}");
    assert_eq!(unregistered["unanswered"], json!([]));
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_that_cannot_be_read_leaves_its_seats_unknown_and_named() -> TestResult {
    let mut table = Table::set().await?;
    let machine = table
        .machine(
            &machine_body(&table, "Box")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    table
        .ok("/seats", &seat_body(&table, "waffles", &machine)?)
        .await?;
    if let Some(serving) = table.serving.take() {
        serving.stop()?;
    }
    let (status, unregistered) = table
        .service
        .get("/seats/unregistered", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{unregistered}");
    let unanswered = unregistered["unanswered"]
        .as_array()
        .ok_or("no unanswered list")?;
    assert_eq!(unanswered.len(), 1, "{unregistered}");
    assert_eq!(unanswered[0]["machine"], machine);
    // A seat never started is not seen, whatever its runner says.
    let (_, listed) = table.service.get("/seats", Some(&table.ada)).await?;
    assert_eq!(listed["seats"][0]["state"], "not-seen", "{listed}");
    table.close()
}

/// Every refusal the seat routes answer with is named in the document.
#[tokio::test(flavor = "multi_thread")]
async fn the_document_names_every_seat_refusal() -> TestResult {
    let table = Table::set().await?;
    let (status, document) = table.service.get("/openapi.json", None).await?;
    assert_eq!(status, 200);
    let named = |path: &str| -> Vec<String> {
        document["paths"][path]["post"]["x-refusals"]
            .as_array()
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    for (path, refusals) in [
        (
            "/seats",
            &[
                "seat_name_invalid",
                "seat_name_taken",
                "seat_operation_reused",
                "seats_unavailable",
            ][..],
        ),
        (
            "/seats/{name}/start",
            &[
                "seat_unknown",
                "seat_no_responsible",
                "seat_running",
                "seat_not_managed",
                "seat_online_in_monitor",
                "import_incomplete",
                "not_permitted",
            ][..],
        ),
        (
            "/seats/{name}/stop",
            &["seat_not_running", "seat_turn_in_progress", "not_permitted"][..],
        ),
        (
            "/seats/{name}/send",
            &["seat_not_running", "seat_text_empty", "seat_text_control"][..],
        ),
        (
            "/seats/{name}/attach",
            &["seat_attach_refused", "attach_pty_use_read_bytes"][..],
        ),
        ("/network/machines/{id}/runner", &["runner_lys_taken"][..]),
    ] {
        let named = named(path);
        for refusal in refusals {
            assert!(
                named.iter().any(|name| name == refusal),
                "{path} does not name {refusal}: {named:?}"
            );
        }
    }
    table.close()
}

/// A seat whose latest import is not complete is refused `import_incomplete`
/// before anything starts. The import here is reserved and then held by a
/// pending upgrade's fence, so it stays in progress.
#[tokio::test(flavor = "multi_thread")]
async fn a_seat_whose_latest_import_is_incomplete_is_not_started() -> TestResult {
    let mut table = Table::set().await?;
    let machine = table
        .machine(
            &machine_body(&table, "Box")?,
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    table
        .ok("/seats", &seat_body(&table, "waffles", &machine)?)
        .await?;
    let folder = table.dir.path().join("seat-resources").join("waffles");
    std::fs::create_dir_all(&folder)?;
    let settings = json!({ "model": "claude-fable-5-1", "permissions": { "defaultMode": "plan" } });
    std::fs::write(folder.join("settings.json"), settings.to_string())?;
    std::fs::write(folder.join("system-prompt.md"), "You are Waffles.\n")?;
    std::fs::write(
        folder.join("mcp.json"),
        json!({ "mcpServers": {} }).to_string(),
    )?;
    let intent = table.dir.path().join("upgrade-intent.json");
    std::fs::write(&intent, b"pending")?;
    table
        .service
        .restart_adjusted(|config| config.operator_upgrade_file = Some(intent))
        .await?;

    let manifest = json!({ "seat": "waffles", "harness": "claude", "claude_folder": folder });
    let plan = table
        .ok(
            "/seats/waffles/import/dry-run",
            &json!({ "manifest": manifest }),
        )
        .await?;
    assert_eq!(plan["refusals"], json!([]), "{plan}");
    let confirm = json!({
        "plan_id": plan["plan_id"], "plan_revision": plan["plan_revision"],
        "operation": operation()?,
    });
    let answer = table
        .service
        .post("/seats/waffles/import/confirm", Some(&table.ada), &confirm)
        .await?;
    refused(&answer, 503, "import_upgrade_pending");

    let answer = table
        .service
        .post(
            "/seats/waffles/start",
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&answer, 409, "import_incomplete");
    let (_, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(
        live["sessions"].as_array().map(Vec::len),
        Some(0),
        "a refused start starts nothing: {live}"
    );
    table.close()
}
