#![cfg(test)]
//! AGENTS-003 R4: one seat's plan, assembled from the readers' fragments at
//! one captured instant, names everything it reads and writes; its revision
//! binds its sources, destinations, references and replacements; a plan
//! holding a refusal is shown and cannot be confirmed; and a confirmation is
//! one signed person's, of that one seat's exact dry run.
//!
//! Fragments are written here directly: the readers are met only through
//! the plan's public entry points.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;

use identity_contract::fake_issuer::Login;
use lys_identity_server::ServerError;
use lys_identity_server::error_seat_import::SeatImportError;
use lys_identity_server::seat_import_plan::{
    Completeness, CredentialReference, DestinationEntry, Fragment, Harness, PLAN_VERSION, Plan,
    Refusal, ScheduleCounts, SeatFacts, SourceEntry, assemble, confirmable, vector_key,
};
use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

const AGENT: &str = "agent-waffles";
const CANARY: &str = "-----BEGIN PRIVATE KEY----- canary-7f3e";

fn facts() -> SeatFacts {
    SeatFacts {
        seat: "waffles".to_owned(),
        agent: AGENT.to_owned(),
        responsible: Some("person-tom".to_owned()),
    }
}

fn source(id: &str, revision: &str, completeness: Completeness) -> SourceEntry {
    SourceEntry {
        id: id.to_owned(),
        kind: "claude_settings".to_owned(),
        locator: format!("/seats/ablative/waffles/{id}"),
        scope: "waffles".to_owned(),
        revision_kind: "content_sha256".to_owned(),
        source_revision: revision.to_owned(),
        completeness,
    }
}

fn destination(kind: &str, record: &str, change: Value) -> DestinationEntry {
    DestinationEntry {
        record_kind: kind.to_owned(),
        record_id: record.to_owned(),
        expected_revision: None,
        change,
        source_entry_ids: vec!["settings.json".to_owned()],
    }
}

fn variable(name: &str) -> DestinationEntry {
    destination(
        "variable",
        &format!("agent:{AGENT}/{name}"),
        json!({
            "scope": { "kind": "agent", "id": AGENT },
            "name": name, "value": 1, "author": "person-tom",
        }),
    )
}

/// The readers' findings for one seat at `revision`, with `replacement`.
fn fragments(revision: &str, replacement: &str) -> Vec<Fragment> {
    let files = Fragment {
        sources: vec![source("settings.json", revision, Completeness::Complete)],
        destinations: vec![
            destination(
                "words_template",
                "waffles-inform",
                json!({ "name": "waffles-inform", "text": "Context is high." }),
            ),
            variable("goal"),
            variable("focus"),
        ],
        references: vec![CredentialReference {
            kind: "seat_identity_file".to_owned(),
            locator: "/Users/tom/.cambium/waffles.seat.json".to_owned(),
            owner: "cambium".to_owned(),
            public_identity: Some("seat-public-waffles".to_owned()),
            usable: true,
            reason: None,
        }],
        replacements: vec![replacement.to_owned()],
        ..Fragment::default()
    };
    let schedules = Fragment {
        schedule_counts: Some(ScheduleCounts {
            total: 7,
            live: 3,
            expired: 2,
            finished: 2,
            not_imported: 4,
        }),
        prerequisites: vec!["the seat moves under AGENTS-002 R5".to_owned()],
        ..Fragment::default()
    };
    vec![files, schedules]
}

fn plan(captured_at: u64, fragments: Vec<Fragment>) -> Plan {
    let bind = |_: &DestinationEntry| -> Result<u64, ServerError> { Ok(4) };
    assemble(facts(), Harness::Claude, captured_at, fragments, &bind)
}

fn names(plan: &Plan) -> Vec<&str> {
    plan.refusals
        .iter()
        .map(|refusal| refusal.name.as_str())
        .collect()
}

#[test]
fn seat_import_dry_run_names_every_field_at_one_instant() -> TestResult {
    let plan = plan(
        1_700_000_000,
        fragments("r1", "Argus hooks become Lys hooks"),
    );
    assert!(plan.refusals.is_empty(), "{:?}", plan.refusals);
    confirmable(&plan)?;
    assert_eq!(plan.version, PLAN_VERSION);
    assert!(
        plan.plan_id.starts_with("plan-") && plan.plan_id.len() == 37,
        "{}",
        plan.plan_id
    );
    assert_eq!(plan.plan_revision.len(), 64);
    assert_eq!(plan.captured_at, 1_700_000_000);
    assert_eq!(plan.seat, "waffles");
    assert_eq!(plan.agent, AGENT);
    assert_eq!(plan.responsible.as_deref(), Some("person-tom"));
    assert_eq!(
        plan.schedule_counts,
        ScheduleCounts {
            total: 7,
            live: 3,
            expired: 2,
            finished: 2,
            not_imported: 4,
        }
    );
    let order: Vec<(&str, &str, Option<u64>)> = plan
        .destinations
        .iter()
        .map(|destination| {
            (
                destination.record_kind.as_str(),
                destination.record_id.as_str(),
                destination.expected_revision,
            )
        })
        .collect();
    // A template before the variables; two variables of one scope bound
    // one after the other.
    assert_eq!(
        order,
        vec![
            ("words_template", "waffles-inform", Some(4)),
            ("variable", "agent:agent-waffles/focus", Some(4)),
            ("variable", "agent:agent-waffles/goal", Some(5)),
        ]
    );
    assert_eq!(plan.references.len(), 1);
    assert_eq!(
        plan.replacements,
        vec!["Argus hooks become Lys hooks".to_owned()]
    );
    assert_eq!(plan.prerequisites.len(), 1);
    assert_eq!(plan.bounds.plan_bytes, 4 * 1024 * 1024);
    Ok(())
}

#[test]
fn seat_import_confirmation_binds_replacements() {
    let shown = plan(10, fragments("r1", "Argus hooks become Lys hooks"));
    let later = plan(20, fragments("r1", "Argus hooks become Lys hooks"));
    assert_eq!(
        shown.plan_revision, later.plan_revision,
        "the instant is not what is confirmed"
    );
    assert_ne!(shown.plan_id, later.plan_id, "each dry run is its own plan");
    let swapped = plan(10, fragments("r1", "Argus hooks become an unrelated hook"));
    assert_ne!(shown.plan_revision, swapped.plan_revision);
    let moved = plan(10, fragments("r2", "Argus hooks become Lys hooks"));
    assert_ne!(shown.plan_revision, moved.plan_revision);
    assert_ne!(vector_key(&shown), vector_key(&moved));
}

#[test]
fn a_refusal_or_an_incomplete_source_is_shown_and_refuses_confirmation() {
    let mut found = fragments("r1", "hooks");
    found[0].refusals.push(Refusal {
        name: "import_member_unsupported".to_owned(),
        member: "settings.json.hooks.PreCompact".to_owned(),
        detail: "no Lys owner".to_owned(),
    });
    found[0].sources.push(source(
        "argus-budgets",
        "",
        Completeness::Incomplete {
            reason: "skipped 2 lines".to_owned(),
        },
    ));
    let refused = plan(1, found);
    assert!(names(&refused).contains(&"import_member_unsupported"));
    assert!(names(&refused).contains(&"import_source_incomplete"));
    let error = confirmable(&refused).err().map(|error| error.name());
    assert_eq!(error.as_deref(), Some("import_plan_refused"));
}

#[test]
fn every_destination_is_checked_before_it_is_bound() {
    let mut found = fragments("r1", "hooks");
    found[0].destinations.extend([
        destination(
            "context_window",
            "agent.waffles",
            json!({ "window_tokens": 200_000 }),
        ),
        destination(
            "words_slot",
            "agent:someone-else/wake_up",
            json!({ "layer": { "kind": "agent", "id": "someone-else" }, "slot": "wake_up",
                    "setting": { "kind": "inherit" } }),
        ),
        destination("words_template", "broken", json!({ "name": "broken" })),
        destination(
            "budget_limits",
            "agent.waffles",
            json!({ "holder": { "kind": "agent", "id": AGENT }, "limits": [], "warn_at": null,
                    "context_policy": { "compact_at": 90 } }),
        ),
        destination(
            "variable",
            "agent:agent-waffles/gone",
            json!({
                "scope": { "kind": "agent", "id": AGENT },
                "name": "gone", "value": null, "author": "a",
            }),
        ),
        variable("goal"),
        DestinationEntry {
            source_entry_ids: vec!["never-read".to_owned()],
            ..variable("orphan")
        },
    ]);
    let refused = plan(1, found);
    for name in [
        "import_destination_unsupported",
        "import_destination_foreign",
        "import_destination_malformed",
        "import_member_unsupported",
        "import_destination_ambiguous",
        "import_destination_unsourced",
    ] {
        assert!(
            names(&refused).contains(&name),
            "{name}: {:?}",
            refused.refusals
        );
    }
}

#[test]
fn no_credential_byte_reaches_a_plan() -> TestResult {
    let dir = tempfile::tempdir()?;
    let seat_file = dir.path().join("waffles.seat.json");
    std::fs::write(&seat_file, json!({ "privateKeyPem": CANARY }).to_string())?;
    let mut found = fragments("r1", "hooks");
    found[0].references[0].locator = seat_file.display().to_string();
    let plan = plan(1, found);
    let written = serde_json::to_string(&plan)?;
    assert!(!written.contains(CANARY));
    assert!(written.contains(&seat_file.display().to_string()));
    Ok(())
}

#[test]
fn every_import_refusal_keeps_its_name_through_the_server_error() {
    let text = || "x".to_owned();
    let cases = [
        (
            SeatImportError::Unavailable { reason: text() },
            "seat_imports_unavailable",
        ),
        (
            SeatImportError::PlanRefused {
                seat: text(),
                refusals: Vec::new(),
            },
            "import_plan_refused",
        ),
        (
            SeatImportError::PlanUnknown {
                seat: text(),
                plan_id: text(),
            },
            "import_plan_unknown",
        ),
        (
            SeatImportError::PlanStale {
                seat: text(),
                plan_id: text(),
                held: text(),
                given: text(),
            },
            "import_plan_stale",
        ),
        (
            SeatImportError::SourceChanged {
                locator: text(),
                shown: text(),
                now: text(),
            },
            "import_source_changed",
        ),
        (
            SeatImportError::DestinationMoved {
                record: text(),
                expected: 1,
                held: 2,
            },
            "import_destination_moved",
        ),
        (
            SeatImportError::SeatMismatch {
                seat: text(),
                named: text(),
            },
            "import_seat_mismatch",
        ),
        (
            SeatImportError::BulkRefused { name: text() },
            "import_bulk_refused",
        ),
        (
            SeatImportError::ConfirmerNotPerson { caller: text() },
            "import_confirmer_not_person",
        ),
        (
            SeatImportError::NoResponsible {
                seat: text(),
                agent: text(),
            },
            "import_no_responsible",
        ),
        (
            SeatImportError::SeatRunning {
                seat: text(),
                agent: text(),
            },
            "import_seat_running",
        ),
        (
            SeatImportError::OperationReused { operation: text() },
            "import_operation_reused",
        ),
        (
            SeatImportError::InProgress {
                seat: text(),
                operation: text(),
            },
            "import_in_progress",
        ),
        (
            SeatImportError::Stopped {
                operation: text(),
                record: text(),
                reason: text(),
            },
            "import_stopped",
        ),
        (
            SeatImportError::Incomplete {
                seat: text(),
                operation: text(),
                state: text(),
            },
            "import_incomplete",
        ),
        (
            SeatImportError::DestinationUnconfigured { kind: text() },
            "import_destination_unconfigured",
        ),
        (
            SeatImportError::DestinationUnsupported {
                record: text(),
                kind: text(),
            },
            "import_destination_unsupported",
        ),
        (
            SeatImportError::DestinationMalformed {
                record: text(),
                reason: text(),
            },
            "import_destination_malformed",
        ),
        (
            SeatImportError::ScheduleActive { record: text() },
            "import_schedule_active",
        ),
        (SeatImportError::UpgradePending, "import_upgrade_pending"),
        (
            SeatImportError::UpgradeIntentUnreadable { reason: text() },
            "import_upgrade_intent_unreadable",
        ),
    ];
    for (error, expected) in cases {
        let error = ServerError::from(error);
        assert_eq!(error.name(), expected, "{error}");
        assert!(error.to_string().starts_with(expected), "{error}");
    }
}

/// The fixture's model, with one relation carrying the import.
const MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"],"operator":["seat.import"]}}"#;

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

async fn seated(table: &Table) -> TestResult {
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

#[tokio::test(flavor = "multi_thread")]
async fn seat_import_confirmation_is_per_seat() -> TestResult {
    let table = Table::set_judging(MODEL).await?;
    seated(&table).await?;
    let confirm = |plan: &str| {
        Ok::<Value, Box<dyn Error>>(json!({
            "operation": operation()?, "plan_id": plan, "plan_revision": "0".repeat(64),
        }))
    };

    // Someone who neither holds the grant nor answers for the agent.
    let bea = table
        .service
        .sign_in(Login {
            subject: "bea-subject".to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let path = "/seats/waffles/import/confirm";
    let answer = table
        .service
        .post(path, Some(&bea), &confirm("plan-x")?)
        .await?;
    refused(&answer, 403, "not_permitted");
    let reason = answer.1["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.contains("seat.import") && reason.contains(path),
        "{reason}"
    );

    // The person responsible, confirming a plan no dry run showed.
    let answer = table
        .service
        .post(path, Some(&table.ada), &confirm("plan-x")?)
        .await?;
    refused(&answer, 404, "import_plan_unknown");

    // More than one seat at once.
    for many in ["waffles,gaia", "all"] {
        let path = format!("/seats/{many}/import/confirm");
        let answer = table
            .service
            .post(&path, Some(&table.ada), &confirm("plan-x")?)
            .await?;
        refused(&answer, 400, "import_bulk_refused");
    }
    let mut malformed = confirm("plan-x")?;
    malformed["operation"] = json!("not-an-operation");
    let answer = table
        .service
        .post(path, Some(&table.ada), &malformed)
        .await?;
    assert_eq!(answer.0, 400, "{}", answer.1);

    // A manifest naming another seat than the route.
    let manifest = json!({ "manifest": { "seat": "gaia", "harness": "claude" } });
    let answer = table
        .service
        .post("/seats/waffles/import/dry-run", Some(&table.ada), &manifest)
        .await?;
    refused(&answer, 400, "import_seat_mismatch");

    let (status, held) = table
        .service
        .get("/seats/waffles/import", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["seat"], "waffles");
    assert_eq!(held["selected"], Value::Null);
    assert_eq!(held["imports"], json!([]));
    table.close()
}
