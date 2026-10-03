//! A secret, owner or account name has no length bound: a long name is
//! accepted whole, while an empty name or one holding a control character is
//! still refused by name.

use super::check_name;
use crate::error::SecretsError;

#[test]
fn a_ten_thousand_byte_name_is_accepted() {
    let long = "n".repeat(10_000);
    for what in ["secret name", "owner", "account"] {
        assert!(check_name(what, &long).is_ok(), "{what}");
    }
}

#[test]
fn an_empty_name_and_a_control_character_are_still_refused() {
    for (given, expected) in [("", "is empty"), ("a\nb", "holds a control character")] {
        let refused = check_name("secret name", given);
        let Err(SecretsError::InvalidName { what, reason, .. }) = &refused else {
            panic!("{given:?} was not refused by name: {refused:?}");
        };
        assert_eq!((*what, *reason), ("secret name", expected));
    }
}
