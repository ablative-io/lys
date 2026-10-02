#![cfg(test)]

//! The access requests' store: what is kept is read back as it was kept, an
//! append that fails is settled by what the leaves hold, and while the
//! outcome cannot be read nothing is answered.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use identity_contract::harness::{Fault, FaultStore, Harness};
use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::requests_store::{Asked, Decided, Intended, Reopen, RequestStore};
use lys_log_store::StoreError;

type TestResult = Result<(), Box<dyn Error>>;

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("requests.key"),
    )?))
}

fn harness_key(harness: &Harness) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load(
        &harness.dir.path().join("service.key"),
    )?))
}

fn asked(id: &str) -> Asked {
    Asked {
        id: id.to_owned(),
        asked_by: "person-a".to_owned(),
        responsible: "person-a".to_owned(),
        resource_kind: "doc".to_owned(),
        resource_id: "1".to_owned(),
        relation: "beta".to_owned(),
        ends_at: None,
        why: "to read".to_owned(),
        asked_at: 5,
    }
}

fn declined(id: &str) -> Decided {
    Decided {
        id: id.to_owned(),
        by: "person-b".to_owned(),
        approved: false,
        note: "no".to_owned(),
        grant: None,
        decided_at: 6,
    }
}

#[test]
fn the_requests_are_read_back_as_they_were_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let (first, decision) = (asked("op-1"), declined("op-1"));
    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    store.ask(first.clone())?;
    store.decide(decision.clone())?;
    drop(store);
    let store = RequestStore::open(&path, key(dir.path())?)?;
    let kept: Vec<_> = store.requests().collect();
    assert_eq!(kept, [(&first, Some(&decision))]);
    Ok(())
}

#[test]
fn an_append_that_fails_is_settled_by_what_the_leaves_hold() -> TestResult {
    let mut legs = 0;
    for (fault, stored) in [
        (Fault::None, true),
        (Fault::BeforeLeaf, false),
        (Fault::LeafStoredWriteFailed, true),
    ] {
        let harness = Harness::new(7)?;
        let mut store = RequestStore::over(harness.leaves(), harness_key(&harness)?)?;
        store.ask(asked("op-1"))?;
        harness.fail(fault)?;
        let answer = store.ask(asked("op-2"));
        assert_eq!(answer.is_ok(), stored, "{fault:?}: {answer:?}");
        assert_eq!(store.request("op-2").is_some(), stored, "{fault:?}: memory");
        store.decide(declined("op-1"))?;
        let again = store.ask(asked("op-2"));
        assert!(
            again.is_ok(),
            "{fault:?}: asked again it is kept once: {again:?}"
        );
        drop(store);

        let reopened = RequestStore::over(harness.leaves(), harness_key(&harness)?)?;
        let ids: Vec<&str> = reopened
            .requests()
            .map(|(asked, _)| asked.id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["op-1", "op-2"],
            "{fault:?}: no leaf is lost or doubled"
        );
        assert_eq!(
            reopened.request("op-1").and_then(|(_, decided)| decided),
            Some(&declined("op-1")),
            "{fault:?}"
        );
        legs += 1;
    }
    assert_eq!(legs, 3);
    Ok(())
}

#[test]
fn a_store_that_cannot_be_read_back_answers_nothing_until_it_can() -> TestResult {
    let harness = Harness::new(7)?;
    let blocked = Arc::new(AtomicBool::new(false));
    let (leaves, gate) = (harness.leaves(), Arc::clone(&blocked));
    let reopen: Reopen<FaultStore> = Box::new(move || {
        if gate.load(Ordering::SeqCst) {
            return Err(StoreError::Io {
                context: "reopen".to_owned(),
                source: std::io::Error::other("injected"),
            });
        }
        leaves()
    });
    let mut store = RequestStore::over(reopen, harness_key(&harness)?)?;
    harness.fail(Fault::LeafStoredWriteFailed)?;
    blocked.store(true, Ordering::SeqCst);
    let failed = store
        .ask(asked("op-1"))
        .err()
        .ok_or("the append was answered")?;
    assert_eq!(failed.name(), "RequestsUnavailable");
    for held in [
        store.ask(asked("op-2")),
        store.decide(declined("op-1")),
        store.settle(),
    ] {
        let held = held.err().ok_or("answered while the outcome is unknown")?;
        assert_eq!(held.name(), "RequestsUnavailable");
    }
    blocked.store(false, Ordering::SeqCst);
    store.ask(asked("op-1"))?;
    store.ask(asked("op-2"))?;
    let ids: Vec<&str> = store
        .requests()
        .map(|(asked, _)| asked.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["op-1", "op-2"],
        "the stored leaf is read back once, not written twice"
    );
    Ok(())
}

fn intended(id: &str, by: &str, operation: &str) -> Intended {
    Intended {
        id: id.to_owned(),
        by: by.to_owned(),
        operation: operation.to_owned(),
        source: None,
        note: "for the quarter".to_owned(),
        intended_at: 7,
        answer: lys_identity_server::requests_store::Answer::AsAsked,
    }
}

fn approved(id: &str, by: &str) -> Decided {
    Decided {
        id: id.to_owned(),
        by: by.to_owned(),
        approved: true,
        note: "for the quarter".to_owned(),
        grant: Some("grant-1".to_owned()),
        decided_at: 8,
    }
}

#[test]
fn while_an_approval_is_being_settled_only_that_approval_is_taken() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    store.ask(asked("op-1"))?;
    store.intend(intended("op-1", "person-b", "approve-1"))?;
    drop(store);

    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.intent("op-1").map(|intent| intent.by.as_str()),
        Some("person-b"),
        "the intent is read back from the leaves"
    );
    let later = Intended {
        intended_at: 90,
        ..intended("op-1", "person-b", "approve-1")
    };
    store.intend(later)?;
    for refused in [
        store.intend(intended("op-1", "person-c", "approve-2")),
        store.intend(intended("op-1", "person-b", "approve-2")),
        store.decide(declined("op-1")),
        store.decide(approved("op-1", "person-c")),
    ] {
        assert!(
            matches!(&refused, Err(ServerError::RequestHeld { by, .. }) if by == "person-b"),
            "{refused:?}"
        );
    }
    assert_eq!(
        store.request("op-1").map(|(_, decided)| decided),
        Some(None)
    );

    let decision = approved("op-1", "person-b");
    store.decide(decision.clone())?;
    assert!(store.intent("op-1").is_none());
    drop(store);
    let store = RequestStore::open(&path, key(dir.path())?)?;
    assert!(store.intent("op-1").is_none());
    assert_eq!(
        store.request("op-1").map(|(_, decided)| decided),
        Some(Some(&decision))
    );
    Ok(())
}

#[test]
fn an_intent_withdrawn_frees_the_request_for_any_decision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    store.ask(asked("op-1"))?;
    store.intend(intended("op-1", "person-b", "approve-1"))?;
    store.withdraw("op-1", "approve-other")?;
    assert!(
        store.intent("op-1").is_some(),
        "an intent is withdrawn by its own operation only"
    );
    store.withdraw("op-1", "approve-1")?;
    assert!(store.intent("op-1").is_none());
    drop(store);

    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    assert!(store.intent("op-1").is_none());
    store.decide(declined("op-1"))?;
    let refused = store.intend(intended("op-1", "person-b", "approve-1"));
    assert!(
        matches!(refused, Err(ServerError::RequestDecided { .. })),
        "{refused:?}"
    );
    Ok(())
}
