#![cfg(test)]
//! The password policy, the email shape and the issuer's name for a person,
//! each with the values it refuses as well as those it takes.

use std::error::Error;

use super::{PASSWORD_MAX, PASSWORD_MIN, check_email, check_password, issuer_name};
use crate::sign_in_providers::unnamed;

#[test]
fn the_policy_takes_a_long_mixed_password_and_refuses_each_shortfall() -> Result<(), Box<dyn Error>>
{
    check_password("Correct-Horse-7-Battery")?;
    let refused = [
        "Short-1a".to_owned(),
        "all-lower-case-1234".to_owned(),
        "ALL-UPPER-CASE-1234".to_owned(),
        "No-Digits-Anywhere-Here".to_owned(),
        format!("Aa1{}", "x".repeat(PASSWORD_MAX)),
        "Aa1".repeat(PASSWORD_MIN / 3 - 1),
    ];
    for password in &refused {
        let refusal = check_password(password).err().map(|error| error.name());
        assert_eq!(refusal.as_deref(), Some("AccountRefused"), "{password}");
    }
    assert_eq!(refused.len(), 6);
    Ok(())
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
