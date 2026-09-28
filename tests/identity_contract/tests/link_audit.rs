//! R4: one logical event per source operation, observations kept apart, provenance kept on replay.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::fixtures::{administrator, linked, op, shown, source};
use identity_contract::harness::{ADMINISTRATOR, Harness, LINK_AUDIT_SOURCE, Service};
use lys_identity::receipt::{Receipt, verify_receipt};
use lys_identity::{Change, IdentityError, IdentityId, LinkChange, LinkObservation, LoginBinding};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_redelivered_source_operation_is_answered_once_and_survives_a_restart() -> TestResult {
    let harness = Harness::new(7)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let first = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 20)?;
    let again = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 21)?;
    assert_eq!(
        first, again,
        "a duplicate delivery answers the first receipt"
    );
    drop(directory);

    let mut restarted = harness.open()?;
    let replayed = restarted.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 22)?;
    assert_eq!(
        replayed, first,
        "a delivery after a restart is still answered once"
    );
    let record = restarted
        .record(IdentityId::Person(person))?
        .ok_or("person missing")?;
    assert!(
        record.bindings().is_empty(),
        "an observation binds no login"
    );
    let leaf = restarted
        .log()?
        .leaf(first.coordinate().index)?
        .ok_or("leaf missing")?;
    let event = lys_identity::verify_event(&leaf, &restarted.service_key())?;
    assert_eq!(
        event.event().actor(),
        &source()?,
        "the source's provenance survives replay"
    );
    assert!(matches!(event.event().change(), Change::LinkAudit(_)));
    assert_eq!(restarted.log()?.len()?, 2);
    Ok(())
}

#[test]
fn an_observer_longer_than_an_issuer_is_refused_before_it_is_signed() -> TestResult {
    let binding = LoginBinding::new("https://accounts.test", "ada-elsewhere")?;
    let observer = format!("https://{}", "o".repeat(2048));
    let refused = LinkObservation::new("op-1", LinkChange::Linked, binding, &observer, 1);
    assert!(matches!(refused, Err(IdentityError::ChangeMismatch { .. })));
    let harness = Harness::new(7)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let widest = format!("https://{}", "o".repeat(2040));
    let observation = LinkObservation::new(
        "op-1",
        LinkChange::Linked,
        LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
        &widest,
        1,
    )?;
    directory.accept_link_audit(source()?, person, observation, 20)?;
    drop(directory);
    assert_eq!(
        harness.open()?.log()?.len()?,
        2,
        "the widest observer allowed is signed, logged and read back"
    );
    Ok(())
}

/// The source's outbox as row 01 selected it: each source operation keeps
/// its stable id and stays pending until the receiver's answer is
/// acknowledged. The receiver's first answer to each operation is kept so
/// every later answer can be compared with it.
struct Outbox {
    pending: Vec<&'static str>,
    first: BTreeMap<&'static str, Receipt>,
}

impl Outbox {
    /// Record `answer` to `operation`, the same as any earlier one, and
    /// acknowledge it unless the acknowledgement is `lost`.
    fn answered(&mut self, operation: &'static str, answer: Receipt, lost: bool) {
        match self.first.entry(operation) {
            Entry::Occupied(first) => {
                assert_eq!(first.get(), &answer, "{operation}: as first answered");
            }
            Entry::Vacant(slot) => {
                slot.insert(answer);
            }
        }
        if !lost {
            self.pending.retain(|pending| *pending != operation);
        }
    }
}

/// `ID001_RECEIVER`: fixture delivery, duplicate delivery, a lost
/// acknowledgement and a receiver restart each exercise the reviewed
/// link-audit contract against its fixtures, and each leg is counted. Three
/// source operations leave exactly three logical events, each answered by a
/// receipt that verifies against the log.
#[test]
fn every_receiver_leg_leaves_one_logical_event_per_source_operation() -> TestResult {
    let harness = Harness::new(7)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let registered = directory.log()?.len()?;
    let mut outbox = Outbox {
        pending: vec!["rauthy-op-1", "rauthy-op-2", "rauthy-op-3"],
        first: BTreeMap::new(),
    };
    let mut legs = BTreeMap::new();

    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 20)?;
    outbox.answered("rauthy-op-1", answer, false);
    *legs.entry("fixture delivery").or_insert(0) += 1;

    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 21)?;
    outbox.answered("rauthy-op-1", answer, false);
    *legs.entry("duplicate delivery").or_insert(0) += 1;

    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-2")?, 22)?;
    outbox.answered("rauthy-op-2", answer, true);
    assert!(outbox.pending.contains(&"rauthy-op-2"), "still pending");
    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-2")?, 23)?;
    outbox.answered("rauthy-op-2", answer, false);
    *legs.entry("lost acknowledgement").or_insert(0) += 1;

    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-3")?, 24)?;
    outbox.answered("rauthy-op-3", answer, true);
    drop(directory);
    let mut directory = harness.open()?;
    for operation in outbox.pending.clone() {
        let answer = directory.accept_link_audit(source()?, person, linked(operation)?, 25)?;
        outbox.answered(operation, answer, false);
    }
    let answer = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 26)?;
    outbox.answered("rauthy-op-1", answer, false);
    *legs.entry("receiver restart").or_insert(0) += 1;

    assert_eq!(legs.len(), 4, "{legs:?}");
    assert!(legs.values().all(|count| *count == 1), "{legs:?}");
    assert!(outbox.pending.is_empty(), "all acknowledged");
    assert_eq!(outbox.first.len(), 3);
    assert_eq!(
        directory.log()?.len()?,
        registered + 3,
        "one logical event per source operation"
    );
    let key = directory.service_key();
    let checkpoint = directory.log()?.head()?;
    let mut verified = 0;
    for receipt in outbox.first.values() {
        let index = receipt.coordinate().index;
        let leaf = directory.log()?.leaf(index)?.ok_or("leaf missing")?;
        let proof = directory.log()?.inclusion_proof(index)?;
        verify_receipt(receipt, &leaf, &key, checkpoint, &proof)?;
        verified += 1;
    }
    assert_eq!(verified, 3, "each receipt verifies against the log");
    Ok(())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

/// `ID001_RECEIVER`: an unauthorized source is refused by name over HTTP and
/// logs nothing, whether it is no session, the administrator, or another
/// login; the configured source's delivery is logged once however often it
/// is sent.
#[tokio::test]
async fn an_unauthorized_source_is_refused_and_the_configured_one_is_logged_once() -> TestResult {
    let service = Service::start().await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service
        .post(
            "/people",
            Some(&administrator),
            &json!({ "operation": op(1).to_string(), "display_name": "Ada" }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let sent = json!({
        "person": body["person"],
        "source_operation_id": "rauthy-op-1",
        "change": "linked",
        "issuer": "https://accounts.test",
        "subject": "ada-elsewhere",
        "observer": service.issuer.issuer(),
        "observed_at": 1_790_000_050,
    });
    let size = service.log_size().await?;
    let other = service.sign_in(login("someone-else")).await?;
    let callers: [(Option<&str>, u16, &str); 3] = [
        (None, 401, "NotSignedIn"),
        (Some(&administrator), 403, "NotAdmitted"),
        (Some(&other), 403, "NotAdmitted"),
    ];
    let mut refused = 0;
    for (caller, status, name) in callers {
        let (answered, body) = service.post("/link-audit", caller, &sent).await?;
        assert_eq!(answered, status, "{body}");
        assert_eq!(body["refusal"], name, "{body}");
        refused += 1;
    }
    assert_eq!(refused, 3, "one refusal per unauthorized source");
    assert_eq!(service.log_size().await?, size, "nothing logged");

    let source = service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let (status, first) = service.post("/link-audit", Some(&source), &sent).await?;
    assert_eq!(status, 200, "{first}");
    let (status, again) = service.post("/link-audit", Some(&source), &sent).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(first, again, "the first receipt again");
    assert_eq!(service.log_size().await?, size + 1, "one logical event");
    Ok(())
}
