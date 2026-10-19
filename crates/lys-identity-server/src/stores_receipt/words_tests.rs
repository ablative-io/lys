#![cfg(test)]

//! The loss sentence each refusal carries (DIRECTORY-107 R4), and none
//! without a loss.

use std::collections::BTreeMap;

use lys_identity::stores_receipt::Loss;

use super::{Held, sentence};

const AT: &str = "2026-10-19T02:00:00.000000Z";

fn lost_at(store: &str) -> Held {
    Held {
        epochs: BTreeMap::from([(store.to_owned(), AT.to_owned())]),
        lost: BTreeMap::new(),
        computer: "ward-mac".to_owned(),
        projection: None,
    }
}

#[test]
fn each_refusal_a_loss_causes_ends_with_when_and_what_to_do_again() {
    for (store, refusal, reason, words) in [
        (
            "grant-log",
            "NotHeld",
            "not held",
            "Lys lost its grant records at 2026-10-19T02:00:00.000000Z; ask for the grant again.",
        ),
        (
            "certificates",
            "AgentSignatureRefused",
            "the signature does not verify",
            "Lys lost its certificate records at 2026-10-19T02:00:00.000000Z; issue the agent's certificate again.",
        ),
        (
            "network",
            "runner_dial_refused",
            "the key names no dialled runner",
            "Lys lost its computer records at 2026-10-19T02:00:00.000000Z; connect this computer again.",
        ),
        (
            "network",
            "MachineUnknown",
            "no such machine",
            "Lys lost its computer records at 2026-10-19T02:00:00.000000Z; connect this computer again.",
        ),
    ] {
        let said = sentence(&lost_at(store), refusal, reason);
        assert_eq!(
            said,
            Some((words.to_owned(), Some(AT.to_owned()))),
            "{refusal}"
        );
    }
}

#[test]
fn no_loss_or_another_cause_carries_no_sentence() {
    let quiet = Held::default();
    for (refusal, reason) in [
        ("NotHeld", "not held"),
        ("runner_dial_refused", "the key names no dialled runner"),
    ] {
        assert_eq!(sentence(&quiet, refusal, reason), None, "{refusal}");
    }
    let certificates = lost_at("certificates");
    assert_eq!(
        sentence(
            &certificates,
            "AgentSignatureRefused",
            "the nonce was already used"
        ),
        None,
        "a refusal the loss did not cause"
    );
    assert_eq!(
        sentence(
            &lost_at("grant-log"),
            "runner_dial_refused",
            "names no dialled runner"
        ),
        None
    );
}

#[test]
fn an_own_store_still_lost_names_both_heads_and_the_command() {
    let held = Held {
        epochs: BTreeMap::new(),
        lost: BTreeMap::from([(
            "network".to_owned(),
            Loss {
                store: "network".to_owned(),
                refusal: "store_lost",
                held: "2 records".to_owned(),
                found: "1 records".to_owned(),
            },
        )]),
        computer: "ward-mac".to_owned(),
        projection: None,
    };
    let said = sentence(
        &held,
        "NetworkUnavailable",
        "the configuration names no network_file",
    );
    assert_eq!(
        said,
        Some((
            "Lys lost its computer records: the receipt holds 2 records; Lys found 1 records. Run `lys identity accept-loss --store network` on ward-mac to serve what was found.".to_owned(),
            None
        ))
    );
}
