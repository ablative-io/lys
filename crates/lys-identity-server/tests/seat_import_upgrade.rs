#![cfg(test)]
//! AGENTS-003 R5 against an install that already holds records in the
//! destination shapes it shipped with: every import write waits behind the
//! shared upgrade-intent fence while the upgrade is reversible, an intent
//! that cannot be read refuses by name, the records already held are kept
//! as they were, and the import journal is a new versioned record kind
//! with no legacy format.

#[path = "support/seat_import_stores.rs"]
mod stores;

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::agents_log::Folded;
use lys_identity_server::budgets_limits::Limits;
use lys_identity_server::budgets_state::{Holder, HolderKind};
use lys_identity_server::provisioning_store::Version;
use lys_identity_server::seat_import_apply::{Unsignalled, apply};
use lys_identity_server::seat_import_state::Journal;
use lys_identity_server::words_state::{Layer, Setting, Slot};
use lys_identity_server::words_store::Save;
use serde_json::json;
use stores::{AGENT, PERSON, Stores, fragment, now, reserved};

type TestResult = Result<(), Box<dyn Error>>;

/// The installed limits' holder: the person, not the seat's agent.
fn person() -> Holder {
    Holder {
        kind: HolderKind::Person,
        id: PERSON.to_owned(),
    }
}

/// What the install held before the import: a reviewed profile version of
/// the seat's agent, a workspace words slot, and another holder's limits.
fn installed(stores: &Stores) -> TestResult {
    let settings = serde_json::from_value(json!({
        "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
        "mcp_servers": [], "instructions": "the installed profile", "note": "",
    }))?;
    stores
        .provisioning
        .lock()
        .map_err(|error| error.to_string())?
        .set(
            AGENT,
            0,
            Version {
                number: 0,
                operation: lys_identity::OperationId::generate()?.to_string(),
                settings,
                set_by: PERSON.to_owned(),
                set_at: 1,
                reviewed: None,
            },
        )?;
    lys_identity_server::words_store::set(
        &stores.words,
        Save {
            layer: Layer::Workspace,
            slot: Slot::WakeUp,
            setting: Setting::Text {
                text: "Wake up.".to_owned(),
            },
            revision: 0,
            by: PERSON.to_owned(),
        },
    )?;
    let limits = serde_json::from_value(json!([
        { "unit": "tokens", "amount": 1000, "period": "day", "act": "notice" }
    ]))?;
    stores
        .budgets
        .lock()
        .map_err(|error| error.to_string())?
        .set_limits(
            Limits {
                holder: person(),
                limits,
                warn_at: None,
                version: 0,
                by: PERSON.to_owned(),
                at: 1,
            },
            0,
        )?;
    Ok(())
}

/// The records the install held, read whole.
fn held(stores: &Stores) -> Result<serde_json::Value, Box<dyn Error>> {
    let profile = stores
        .provisioning
        .lock()
        .map_err(|error| error.to_string())?
        .version(AGENT, 1)
        .cloned()
        .ok_or("no installed profile version")?;
    let slot = lys_identity_server::words_store::held(&stores.words)?
        .layers
        .get("workspace")
        .and_then(|slots| slots.get(&Slot::WakeUp))
        .cloned()
        .ok_or("no installed slot")?;
    let limits = stores
        .budgets
        .lock()
        .map_err(|error| error.to_string())?
        .held()
        .limit_set(&person())
        .cloned()
        .ok_or("no installed limits")?;
    Ok(json!({ "profile": profile, "slot": slot, "limits": limits }))
}

#[test]
fn seat_import_old_install_upgrade() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let stores = Stores::open(dir.path(), &key)?;
    installed(&stores)?;
    let before = held(&stores)?;
    let at = now();
    let plan = stores.plan(at, vec![fragment("r1", at)]);
    assert!(plan.refusals.is_empty(), "{:?}", plan.refusals);
    let profile = plan
        .destinations
        .iter()
        .find(|destination| destination.record_kind == "profile_version")
        .ok_or("no profile destination")?;
    assert_eq!(
        profile.expected_revision,
        Some(1),
        "bound over the installed version"
    );
    let operation = lys_identity::OperationId::generate()?.to_string();
    stores.imports.reserve(reserved(&operation, &plan))?;
    let revisions = stores.revisions()?;

    // While the upgrade is reversible, nothing is written.
    let pending = || -> std::io::Result<bool> { Ok(true) };
    let held_back = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &pending,
        &Unsignalled,
    );
    assert!(
        matches!(&held_back, Err(error) if error.name() == "import_upgrade_pending"),
        "{held_back:?}"
    );
    assert_eq!(stores.revisions()?, revisions);

    // An intent that cannot be read refuses by name, never as permission.
    let unreadable = || -> std::io::Result<bool> {
        Err(std::io::Error::other("fixture: the intent cannot be read"))
    };
    let refused = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &unreadable,
        &Unsignalled,
    );
    assert!(
        matches!(&refused, Err(error) if error.name() == "import_upgrade_intent_unreadable"),
        "{refused:?}"
    );
    assert_eq!(stores.revisions()?, revisions);
    let kept = stores
        .imports
        .operation(&operation)?
        .ok_or("no operation")?;
    assert!(kept.steps.is_empty());
    assert_eq!(kept.state(), "in_progress");

    // Once committed, the same operation applies; what was installed stays.
    let committed = || -> std::io::Result<bool> { Ok(false) };
    let applied = apply(
        &stores.imports,
        &stores.owners(),
        &operation,
        &committed,
        &Unsignalled,
    )?;
    assert_eq!(applied.operation.state(), "completed");
    drop(stores);
    let stores = Stores::open(dir.path(), &key)?;
    assert_eq!(held(&stores)?, before);
    let versions = stores
        .provisioning
        .lock()
        .map_err(|error| error.to_string())?
        .profile(AGENT)
        .map(|profile| profile.versions.len())
        .ok_or("no profile")?;
    assert_eq!(
        versions, 2,
        "the imported version follows the installed one"
    );
    Ok(())
}

#[test]
fn the_import_journal_has_no_legacy_format() -> TestResult {
    let sealed = Journal::default().encode()?;
    let read: serde_json::Value = serde_json::from_slice(&sealed)?;
    assert_eq!(read["format"], Journal::FORMAT);
    assert_eq!(Journal::decode(&sealed)?, Journal::default());
    let older = json!({ "format": "lys-seat-imports-state/v0", "held": {} });
    let refused = Journal::decode(&serde_json::to_vec(&older)?);
    assert!(refused.is_err(), "{refused:?}");
    let unformatted = json!({ "operations": {} });
    assert!(Journal::decode(&serde_json::to_vec(&unformatted)?).is_err());
    Ok(())
}
