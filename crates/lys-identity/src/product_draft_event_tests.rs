#![cfg(test)]
//! Product draft events (ACCESS-001 R3): each kind reads back as written,
//! a digest that is not the words' is refused, a retry is the same act, and
//! the drafts a snapshot holds are the drafts the log made.

use super::{
    Approved, Created, Executed, ProductDraftEvent, Refused, RefusedOnExecution, close_operation,
    decode, draft_operation, encode, words_digest,
};
use crate::draft_event::Target;
use crate::grants::{GrantId, Mode};
use crate::{Actor, AuthMethod, IdentityError, IdentityId, LoginBinding, PersonId, Provenance};
use lys_core::Ed25519Identity;
use std::sync::Arc;

const WORDS: &str = "{\"stop\":\"seat-7\"}";

fn person(byte: u8) -> PersonId {
    PersonId::from_bytes([byte; 16])
}

fn actor(subject: &str) -> Actor {
    Actor::new(
        LoginBinding::new("https://issuer.example.test", subject).unwrap(),
        Provenance::new(AuthMethod::Oidc, 5),
    )
}

fn created(mode: Mode, at: u64) -> Arc<Created> {
    let holder = IdentityId::Person(person(1));
    Arc::new(Created {
        operation: draft_operation(holder, "product-op-1"),
        client_operation: "product-op-1".to_owned(),
        holder,
        responsible: person(1),
        recorded_at: at,
        app: "fixture_notes".to_owned(),
        grant: GrantId::from_bytes([9; 16]),
        mode,
        target: Target {
            kind: "fixture_notes.doc".to_owned(),
            id: "1".to_owned(),
            action: "write".to_owned(),
        },
        request_digest: words_digest(WORDS),
        words: WORDS.to_owned(),
    })
}

fn hash(event: &ProductDraftEvent) -> [u8; 32] {
    crate::encoding::payload_commitment(&encode(event))
}

fn approval(draft: &ProductDraftEvent, byte: u8) -> ProductDraftEvent {
    ProductDraftEvent::Approved(Arc::new(Approved {
        operation: crate::OperationId::from_bytes([byte; 16]),
        actor: actor(&format!("approver-{byte}")),
        approver: person(byte),
        recorded_at: 6,
        draft: draft.operation(),
        draft_hash: hash(draft),
    }))
}

fn executed(draft: &ProductDraftEvent, receipt: u8) -> ProductDraftEvent {
    ProductDraftEvent::Executed(Arc::new(Executed {
        operation: close_operation(draft.operation()),
        recorded_at: 7,
        draft: draft.operation(),
        draft_hash: hash(draft),
        app: "fixture_notes".to_owned(),
        receipt_digest: [receipt; 32],
    }))
}

fn home() -> Arc<tempfile::TempDir> {
    let home = Arc::new(tempfile::tempdir().unwrap());
    Ed25519Identity::load_or_generate(&home.path().join("key")).unwrap();
    lys_log_store::FileLeafStore::create(
        &home.path().join("log"),
        "example.com/lys/product-draft-test",
    )
    .unwrap();
    home
}

fn open(home: &Arc<tempfile::TempDir>) -> crate::Directory<lys_log_store::FileLeafStore> {
    let home = Arc::clone(home);
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    crate::Directory::open_with(
        Box::new(move || lys_log_store::FileLeafStore::open(&home.path().join("log"))),
        key,
        std::num::NonZeroU64::new(1).unwrap(),
    )
    .unwrap()
}

#[test]
fn every_kind_reads_back_as_written() {
    let draft = ProductDraftEvent::Created(created(Mode::ByTwo, 4));
    let refused = ProductDraftEvent::Refused(Arc::new(Refused {
        operation: crate::OperationId::from_bytes([3; 16]),
        actor: actor("refuser"),
        approver: person(3),
        recorded_at: 6,
        draft: draft.operation(),
        draft_hash: hash(&draft),
        reason: "not this week".to_owned(),
    }));
    let refused_on_execution =
        ProductDraftEvent::RefusedOnExecution(Arc::new(RefusedOnExecution {
            operation: close_operation(draft.operation()),
            recorded_at: 7,
            draft: draft.operation(),
            draft_hash: hash(&draft),
            app: "fixture_notes".to_owned(),
            refusal: "seat_gone".to_owned(),
            reason: "the seat had already stopped".to_owned(),
        }));
    for event in [
        draft.clone(),
        approval(&draft, 2),
        refused,
        executed(&draft, 4),
        refused_on_execution,
    ] {
        let bytes = encode(&event);
        assert_eq!(decode(&bytes).unwrap(), event);
        let signed = crate::sign_product_draft_event(
            event.clone(),
            &Ed25519Identity::load_or_generate(&home().path().join("key")).unwrap(),
        )
        .unwrap();
        assert!(matches!(signed.entry(), crate::Entry::ProductDraft(read) if **read == event));
    }
}

#[test]
fn a_digest_that_is_not_the_words_is_refused_by_name() {
    let mut wrong = created(Mode::ByDraft, 4).as_ref().clone();
    wrong.request_digest = [0; 32];
    let refused = ProductDraftEvent::Created(Arc::new(wrong)).validate();
    assert!(
        matches!(refused, Err(IdentityError::DraftChangeInvalid { reason }) if reason.contains("SHA-256")),
        "{refused:?}"
    );
}

#[test]
fn an_outright_grant_makes_no_draft() {
    let refused = ProductDraftEvent::Created(created(Mode::Outright, 4)).validate();
    assert!(matches!(
        refused,
        Err(IdentityError::DraftChangeInvalid { .. })
    ));
}

#[test]
fn a_retry_at_another_time_is_the_same_act_and_other_words_are_not() {
    let first = ProductDraftEvent::Created(created(Mode::ByDraft, 4));
    let again = ProductDraftEvent::Created(created(Mode::ByDraft, 90));
    assert!(first.same_act(&again));
    let mut other = created(Mode::ByDraft, 4).as_ref().clone();
    other.words = "{\"stop\":\"seat-8\"}".to_owned();
    other.request_digest = words_digest(&other.words);
    assert!(!first.same_act(&ProductDraftEvent::Created(Arc::new(other))));
    assert_eq!(
        close_operation(first.operation()),
        close_operation(again.operation())
    );
}

#[test]
fn by_two_needs_two_distinct_approvers_and_one_close_and_survives_a_reopen() {
    let home = home();
    let mut directory = open(&home);
    let draft = ProductDraftEvent::Created(created(Mode::ByTwo, 4));
    let id = draft.operation();
    let made = directory.record_product_draft(draft.clone()).unwrap();
    assert_eq!(
        directory
            .record_product_draft(ProductDraftEvent::Created(created(Mode::ByTwo, 50)))
            .unwrap(),
        made,
        "the same act asked again is answered as first recorded"
    );
    directory.record_product_draft(approval(&draft, 2)).unwrap();
    let held = directory.projection().unwrap().product_draft(id).unwrap();
    assert!(!held.is_approved(), "one approval of a by_two draft");
    let mut twice = approval(&draft, 2);
    if let ProductDraftEvent::Approved(event) = &mut twice {
        Arc::make_mut(event).operation = crate::OperationId::from_bytes([12; 16]);
    }
    assert!(matches!(
        directory.record_product_draft(twice),
        Err(IdentityError::DraftChangeInvalid { .. })
    ));
    assert!(matches!(
        directory.record_product_draft(executed(&draft, 4)),
        Err(IdentityError::DraftNotPending { .. })
    ));
    directory.record_product_draft(approval(&draft, 3)).unwrap();
    assert!(
        directory
            .projection()
            .unwrap()
            .product_draft(id)
            .unwrap()
            .is_approved()
    );
    let closed = directory.record_product_draft(executed(&draft, 4)).unwrap();
    assert_eq!(
        directory.record_product_draft(executed(&draft, 4)).unwrap(),
        closed
    );
    assert!(matches!(
        directory.record_product_draft(executed(&draft, 5)),
        Err(IdentityError::OperationReused { .. })
    ));
    let before = directory.projection().unwrap().product_draft(id).cloned();
    drop(directory);
    let mut reopened = open(&home);
    let after = reopened.projection().unwrap().product_draft(id).cloned();
    assert_eq!(after, before);
    assert!(!after.unwrap().is_open());
}
