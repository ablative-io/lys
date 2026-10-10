#![cfg(test)]
//! AGENTS-003 R5: a confirmed import is reserved once, written step by step
//! under stable step ids and the revisions its plan bound, and selected by
//! one completed manifest. A rerun of the same source vector writes
//! nothing; an import ended at any step, even after a durable write whose
//! reply was lost, resumes the same operation after a restart and publishes
//! exactly one manifest; a person's edit made while it was interrupted
//! stops it by name and is left as the person left it.

#[path = "support/seat_import_stores.rs"]
mod stores;

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::seat_import_apply::{Unsignalled, apply};
use lys_identity_server::seat_import_state::Outcome;
use lys_identity_server::seat_import_store::Reservation;
use lys_identity_server::variables_state::Scope;
use lys_identity_server::variables_store::{Patch, patch};
use serde_json::json;
use sha2::{Digest, Sha256};
use stores::{AGENT, Exit, PERSON, SEAT, Stores, exits, fragment, now, reserved};

type TestResult = Result<(), Box<dyn Error>>;

/// Every write an import makes is held behind a fence that is open here.
fn open() -> std::io::Result<bool> {
    Ok(false)
}

fn key(dir: &std::path::Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("key"),
    )?))
}

/// The owner records the fixture's import leaves, read after a reopen:
/// one template save, one slot save, two variable patches, one limits
/// version, the schedule and its pause, one profile version.
fn assert_imported_once(stores: &Stores) -> TestResult {
    assert_eq!(stores.revisions()?, [1, 1, 2, 1, 2, 1]);
    let profile = stores
        .provisioning
        .lock()
        .map_err(|error| error.to_string())?
        .profile(AGENT)
        .map(|profile| profile.versions.clone())
        .ok_or("no profile")?;
    assert_eq!(profile.len(), 1);
    assert!(profile[0].reviewed.is_none(), "an import never reviews");
    let item = lys_identity_server::schedules_store::item(&stores.schedules, "schedule-standup")?;
    assert!(item.paused(), "an import activates no delivery");
    assert!(item.fired.is_empty(), "an import sends nothing");
    Ok(())
}

#[test]
fn seat_import_rerun_is_noop() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let stores = Stores::open(dir.path(), &key)?;
    let plan = stores.plan(now(), vec![fragment("r1", now())]);
    assert!(plan.refusals.is_empty(), "{:?}", plan.refusals);
    let operation = lys_identity::OperationId::generate()?.to_string();
    assert_eq!(
        stores.imports.reserve(reserved(&operation, &plan))?,
        Reservation::Fresh(operation.clone())
    );
    let applied = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &open,
        &Unsignalled,
    )?;
    assert_eq!(applied.writes, 7, "{:?}", applied.operation.steps);
    assert_eq!(applied.operation.state(), "completed");
    let transfer = applied
        .operation
        .steps
        .iter()
        .find(|step| step.record_kind == "rule_transfer")
        .ok_or("no transfer step")?;
    assert_eq!(transfer.outcome, Outcome::ReceiptOnly);
    let leaves = stores.imports.leaves()?;
    drop(stores);

    let stores = Stores::open(dir.path(), &key)?;
    assert_imported_once(&stores)?;
    // The same source vector confirmed again, under a new operation id,
    // answers the completed import and writes nothing anywhere.
    let again = lys_identity::OperationId::generate()?.to_string();
    let Reservation::Completed(kept) = stores.imports.reserve(reserved(&again, &plan))? else {
        return Err("a completed rerun reserved a new import".into());
    };
    assert_eq!(kept.reserved.operation, operation);
    let rerun = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &open,
        &Unsignalled,
    )?;
    assert_eq!(rerun.writes, 0);
    assert_eq!(stores.imports.leaves()?, leaves);
    assert_imported_once(&stores)?;
    assert_eq!(
        stores
            .imports
            .selected(SEAT)?
            .map(|selected| selected.reserved.operation),
        Some(operation)
    );
    stores.imports.require_selectable(SEAT)?;
    Ok(())
}

#[test]
fn a_changed_source_revision_is_another_import() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let stores = Stores::open(dir.path(), &key)?;
    let at = now();
    let first = stores.plan(at, vec![fragment("r1", at)]);
    let second = stores.plan(at, vec![fragment("r2", at)]);
    assert_ne!(
        lys_identity_server::seat_import_plan::vector_key(&first),
        lys_identity_server::seat_import_plan::vector_key(&second)
    );
    let operation = lys_identity::OperationId::generate()?.to_string();
    stores.imports.reserve(reserved(&operation, &first))?;
    // The same operation id cannot name the other vector.
    let reused = stores.imports.reserve(reserved(&operation, &second));
    assert!(
        matches!(&reused, Err(error) if error.name() == "import_operation_reused"),
        "{reused:?}"
    );
    // Nor can a second import of the seat begin while the first is open.
    let other = lys_identity::OperationId::generate()?.to_string();
    let open_one = stores.imports.reserve(reserved(&other, &second));
    assert!(
        matches!(&open_one, Err(error) if error.name() == "import_in_progress"),
        "{open_one:?}"
    );
    Ok(())
}

#[test]
fn seat_import_resumes_after_each_step() -> TestResult {
    let at = now();
    let probe = tempfile::tempdir()?;
    let shape = Stores::open(probe.path(), &key(probe.path())?)?.plan(at, vec![fragment("r1", at)]);
    let steps = shape.destinations.len();
    let operation = lys_identity::OperationId::generate()?.to_string();
    for exit in exits(&operation, &shape) {
        let dir = tempfile::tempdir()?;
        let key = key(dir.path())?;
        let stores = Stores::open(dir.path(), &key)?;
        let plan = stores.plan(at, vec![fragment("r1", at)]);
        stores.imports.reserve(reserved(&operation, &plan))?;
        let ended = apply(&stores.imports, &stores.owners(), &operation, &open, &exit);
        assert!(ended.is_err(), "{exit:?} did not end the apply");
        // No incomplete import is selected.
        let selectable = stores.imports.require_selectable(SEAT);
        assert!(
            matches!(&selectable, Err(error) if error.name() == "import_incomplete"),
            "{exit:?}: {selectable:?}"
        );
        drop(stores);

        // The restart resumes the same operation, with no second confirmation.
        let stores = Stores::open(dir.path(), &key)?;
        assert_eq!(stores.imports.open_operations()?, vec![operation.clone()]);
        let applied = apply(
            &stores.imports,
            &stores.owners(),
            &operation,
            &open,
            &Unsignalled,
        )?;
        assert_eq!(applied.operation.state(), "completed", "{exit:?}");
        assert_eq!(applied.operation.steps.len(), steps, "{exit:?}");
        if let Exit::Written(step) = &exit {
            let kept = applied
                .operation
                .step(step)
                .ok_or("the step was not kept")?;
            assert_eq!(
                kept.outcome,
                Outcome::Reconciled,
                "{exit:?}: a lost reply is read back"
            );
        }
        drop(stores);
        let stores = Stores::open(dir.path(), &key)?;
        assert_imported_once(&stores)?;
        assert!(stores.imports.open_operations()?.is_empty());
        stores.imports.require_selectable(SEAT)?;
    }
    Ok(())
}

#[test]
fn an_interrupted_import_resumes_from_its_reservation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let at = now();
    let operation = lys_identity::OperationId::generate()?.to_string();
    {
        let stores = Stores::open(dir.path(), &key)?;
        let plan = stores.plan(at, vec![fragment("r1", at)]);
        stores.imports.reserve(reserved(&operation, &plan))?;
        assert_eq!(
            stores.revisions()?,
            [0; 6],
            "a reservation writes no destination"
        );
    }
    let stores = Stores::open(dir.path(), &key)?;
    let applied = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &open,
        &Unsignalled,
    )?;
    assert_eq!(applied.writes, 7);
    assert_imported_once(&stores)?;
    Ok(())
}

fn digest(path: &std::path::Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

#[test]
fn seat_import_preserves_edits() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = key(dir.path())?;
    let sources = dir.path().join("seat-resources");
    std::fs::create_dir_all(&sources)?;
    let files = ["settings.json", "system-prompt.md", "mcp.json"].map(|name| sources.join(name));
    for file in &files {
        std::fs::write(file, format!("fixture source {}", file.display()))?;
    }
    let before = files
        .iter()
        .map(|file| digest(file))
        .collect::<Result<Vec<_>, _>>()?;

    let at = now();
    let stores = Stores::open(dir.path(), &key)?;
    let plan = stores.plan(at, vec![fragment("r1", at)]);
    let focus = plan
        .destinations
        .iter()
        .position(|destination| destination.record_id.ends_with("/focus"))
        .ok_or("no focus variable")?;
    let operation = lys_identity::OperationId::generate()?.to_string();
    stores.imports.reserve(reserved(&operation, &plan))?;
    let exit = Exit::Kept(lys_identity_server::seat_import_apply::step_id(
        &operation, focus,
    ));
    assert!(apply(&stores.imports, &stores.owners(), &operation, &open, &exit).is_err());

    // While the import is interrupted, the person changes the goal.
    let scope = Scope::Agent {
        id: AGENT.to_owned(),
    };
    let mut values = std::collections::BTreeMap::new();
    values.insert("goal".to_owned(), json!("the person's own goal"));
    patch(
        &stores.variables,
        Patch {
            scope: scope.clone(),
            revision: 1,
            author: PERSON.to_owned(),
            values,
            expires_at: None,
        },
    )?;
    let stopped = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &open,
        &Unsignalled,
    );
    let Err(stopped) = stopped else {
        return Err("the import overwrote a person's edit".into());
    };
    assert_eq!(stopped.name(), "import_stopped", "{stopped}");
    assert!(
        stopped.to_string().contains("import_destination_moved"),
        "{stopped}"
    );
    assert!(stopped.to_string().contains("/goal"), "{stopped}");
    let kept = stores
        .imports
        .operation(&operation)?
        .ok_or("no operation")?;
    assert_eq!(kept.state(), "stopped");
    let halted = kept.halted.ok_or("not halted")?;
    assert_eq!(halted.refusal, "import_destination_moved");
    let read = lys_identity_server::variables_store::read(&stores.variables, &scope)?;
    assert_eq!(
        read.values.get("goal").map(|held| &held.value),
        Some(&json!("the person's own goal"))
    );
    // A stopped import selects nothing and stays stopped.
    let selectable = stores.imports.require_selectable(SEAT);
    assert!(matches!(&selectable, Err(error) if error.name() == "import_incomplete"));
    let again = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &open,
        &Unsignalled,
    );
    assert!(matches!(&again, Err(error) if error.name() == "import_stopped"));

    let after = files
        .iter()
        .map(|file| digest(file))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(before, after, "no source is changed by an import");
    Ok(())
}
