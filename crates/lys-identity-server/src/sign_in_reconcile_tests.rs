//! Each judgement of DIRECTORY-124 R1, on the facts as the repair reads them.
use std::str::FromStr;

use lys_identity::PersonId;
use serde_json::json;

use super::{Read, Seen, SignInHealth, judge};

fn person(n: u8) -> PersonId {
    PersonId::from_str(&format!("person-{}", format!("{n:02x}").repeat(16))).expect("a person id")
}

fn seen(n: u8, held: Option<&str>, subjects: &[&str]) -> Seen {
    Seen {
        person: person(n),
        active: true,
        held: held.map(str::to_owned),
        several: Vec::new(),
        subjects: subjects.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn live(subject: &str, email: &str, enabled: bool) -> Read {
    Read {
        live: vec![(subject.to_owned(), email.to_owned(), enabled)],
        dead: Vec::new(),
        under_address: None,
    }
}

fn none(under: Option<serde_json::Value>) -> Read {
    Read {
        live: Vec::new(),
        dead: vec!["gone".to_owned()],
        under_address: under,
    }
}

fn name(person: PersonId) -> String {
    format!("name of {person}")
}

#[test]
fn an_account_under_the_held_address_is_ready_whatever_its_case() {
    let ada = seen(1, Some("Ada@example.test"), &["a"]);
    let everyone = [seen(1, Some("Ada@example.test"), &["a"])];
    assert_eq!(
        judge(&ada, &live("a", "ada@example.test", true), &everyone, &name),
        SignInHealth::Ready
    );
}

#[test]
fn a_turned_off_account_is_disabled() {
    let ada = seen(1, Some("ada@example.test"), &["a"]);
    assert_eq!(
        judge(&ada, &live("a", "ada@example.test", false), &[], &name),
        SignInHealth::Disabled
    );
}

#[test]
fn an_account_carrying_another_persons_address_is_not_theirs_and_names_whose_it_is() {
    // 22 October: Dean's record, its account rewritten to Annabel's address.
    let dean = seen(1, Some("dean@example.test"), &["d"]);
    let everyone = [
        seen(1, Some("dean@example.test"), &["d"]),
        seen(2, Some("annabel@example.test"), &["gone"]),
    ];
    assert_eq!(
        judge(
            &dean,
            &live("d", "annabel@example.test", true),
            &everyone,
            &name
        ),
        SignInHealth::NotTheirs {
            account_email: "annabel@example.test".to_owned(),
            belongs_to: Some(person(2).to_string()),
        }
    );
}

#[test]
fn a_second_login_under_an_address_nobody_else_holds_is_ready() {
    // A person may hold a work and a personal login; only another person's
    // address makes an account not theirs.
    let annabel = seen(2, Some("annabel.personal@example.test"), &["d"]);
    let everyone = [
        seen(1, Some("dean@example.test"), &[]),
        seen(2, Some("annabel.personal@example.test"), &["d"]),
    ];
    assert_eq!(
        judge(
            &annabel,
            &live("d", "annabel@example.test", true),
            &everyone,
            &name
        ),
        SignInHealth::Ready
    );
}

#[test]
fn two_addresses_held_for_one_person_wait_for_an_administrator_even_with_a_live_account() {
    let mut mixed = seen(1, None, &["d"]);
    mixed.several = vec![
        "annabel@example.test".to_owned(),
        "dean@example.test".to_owned(),
    ];
    assert_eq!(
        judge(&mixed, &live("d", "annabel@example.test", true), &[], &name),
        SignInHealth::SeveralAddresses {
            addresses: vec![
                "annabel@example.test".to_owned(),
                "dean@example.test".to_owned()
            ],
        }
    );
}

#[test]
fn a_missing_account_under_a_free_address_is_repairable() {
    let josh = seen(1, Some("josh@example.test"), &["gone"]);
    let everyone = [seen(1, Some("josh@example.test"), &["gone"])];
    assert_eq!(
        judge(&josh, &none(None), &everyone, &name),
        SignInHealth::Repairable
    );
}

#[test]
fn an_unbound_account_already_under_the_address_is_repairable_by_finding_it() {
    let josh = seen(1, Some("josh@example.test"), &[]);
    assert_eq!(
        judge(&josh, &none(Some(json!({"id": "orphan"}))), &[], &name),
        SignInHealth::Repairable
    );
}

#[test]
fn an_account_under_the_address_bound_to_someone_else_is_never_taken() {
    let annabel = seen(2, Some("annabel@example.test"), &["gone"]);
    let everyone = [
        seen(1, Some("dean@example.test"), &["d"]),
        seen(2, Some("annabel@example.test"), &["gone"]),
    ];
    assert_eq!(
        judge(&annabel, &none(Some(json!({"id": "d"}))), &everyone, &name),
        SignInHealth::AddressTaken {
            holder: name(person(1)),
        }
    );
}

#[test]
fn an_address_two_people_hold_is_shared_and_never_repaired() {
    let david = seen(1, Some("david@example.test"), &["gone"]);
    let everyone = [
        seen(1, Some("david@example.test"), &["gone"]),
        seen(2, Some("DAVID@example.test"), &["gone-too"]),
    ];
    assert_eq!(
        judge(&david, &none(None), &everyone, &name),
        SignInHealth::SharedAddress {
            with: vec![name(person(2))],
        }
    );
}

#[test]
fn no_address_and_no_live_account_is_no_address() {
    let early = seen(1, None, &["gone"]);
    assert_eq!(
        judge(&early, &none(None), &[], &name),
        SignInHealth::NoAddress
    );
}
