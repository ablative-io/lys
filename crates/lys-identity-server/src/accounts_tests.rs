#![cfg(test)]
//! The configured password policy, the email shape and the issuer's name for a person,
//! each with the values it refuses as well as those it takes.

use std::error::Error;

use super::{PasswordPolicy, check_email, check_password, issuer_name};
use crate::sign_in_providers::unnamed;

/// A policy no default holds: 11 to 40 characters, two digits, a character
/// that is neither letter nor digit, and not one of the last two.
fn configured() -> PasswordPolicy {
    PasswordPolicy {
        length_min: 11,
        length_max: 40,
        lower_case: None,
        upper_case: None,
        digits: Some(2),
        special: Some(1),
        not_recently_used: Some(2),
    }
}

#[test]
fn the_configured_policy_takes_what_it_asks_for_and_refuses_each_shortfall()
-> Result<(), Box<dyn Error>> {
    let policy = configured();
    check_password(Some(&policy), "horse-battery-77")?;
    let refused = [
        "horse-b-77".to_owned(),
        format!("horse-battery-77{}", "x".repeat(25)),
        "horse-battery-7".to_owned(),
        "horsebattery77".to_owned(),
    ];
    for password in &refused {
        let refusal = check_password(Some(&policy), password)
            .err()
            .map(|error| error.name());
        assert_eq!(refusal.as_deref(), Some("AccountRefused"), "{password}");
    }
    assert_eq!(refused.len(), 4);
    Ok(())
}

#[test]
fn the_policy_is_shown_in_the_configured_values_and_nothing_else() {
    let policy = configured();
    let view = policy.view();
    assert_eq!(view["length_min"], 11);
    assert_eq!(view["length_max"], 40);
    assert_eq!(view["digits"], 2);
    assert_eq!(
        view["words"],
        "At least 11 and at most 40 characters, with 2 of the kind: digit, a character that is not a letter or a digit. A new password is not one of your last 2."
    );
    assert!(!view.to_string().contains("14"), "{view}");
    assert!(!view.to_string().contains("128"), "{view}");
}

/// The configured policy with one field changed by `change`.
fn changed(change: impl FnOnce(&mut PasswordPolicy)) -> PasswordPolicy {
    let mut policy = configured();
    change(&mut policy);
    policy
}

#[test]
fn a_policy_outside_what_the_issuer_holds_is_refused_by_field() {
    let cases = [
        (changed(|policy| policy.length_min = 7), "length_min"),
        (changed(|policy| policy.length_max = 10), "length_max"),
        (changed(|policy| policy.length_max = 129), "length_max"),
        (changed(|policy| policy.digits = Some(0)), "digits"),
        (
            changed(|policy| policy.not_recently_used = Some(11)),
            "not_recently_used",
        ),
    ];
    for (policy, field) in &cases {
        let refusal = policy.validate().err().unwrap_or_default();
        assert!(refusal.starts_with(field), "{field}: {refusal}");
    }
    assert_eq!(cases.len(), 5);
    assert_eq!(configured().validate(), Ok(()));
}

#[test]
fn with_no_policy_configured_only_an_empty_password_is_refused_here() {
    assert!(check_password(None, "").is_err());
    assert!(check_password(None, "x").is_ok());
}

#[test]
fn an_email_is_taken_trimmed_and_every_other_shape_is_refused() -> Result<(), Box<dyn Error>> {
    assert_eq!(check_email("  ada@example.test ")?, "ada@example.test");
    let refused = [
        "",
        "ada",
        "@example.test",
        "ada@example",
        "ada@.test",
        "ada@example.test.",
        "ada@@example.test",
        "ada lovelace@example.test",
        "ada@example.test/admin",
        "ada@example.test?x",
    ];
    for email in refused {
        assert!(check_email(email).is_err(), "{email}");
    }
    assert_eq!(refused.len(), 10);
    Ok(())
}

#[test]
fn the_issuer_keeps_a_name_only_of_the_characters_it_takes() {
    assert_eq!(
        issuer_name(" Ada Lovelace "),
        Some("Ada Lovelace".to_owned())
    );
    assert_eq!(
        issuer_name("Zoë O'Brien-Smith"),
        Some("Zoë O'Brien-Smith".to_owned())
    );
    assert_eq!(issuer_name("Ada (admin)"), None);
    assert_eq!(issuer_name(&"a".repeat(33)), None);
    assert_eq!(issuer_name("   "), None);
}

#[test]
fn the_issuers_name_is_never_carried_to_a_person() {
    assert_eq!(
        unnamed("Rauthy could not find it; see rauthy logs, RAUTHY"),
        "the sign-in service could not find it; see the sign-in service logs, the sign-in service"
    );
    assert_eq!(unnamed("nothing to change"), "nothing to change");
}
