#![cfg(test)]

use lys_identity::grants::GrantError;

#[test]
fn ended_grants_name_eligible_grantors_but_storage_failures_do_not() {
    for error in [
        GrantError::Revoked {
            grant: "grant".to_owned(),
        },
        GrantError::Expired {
            grant: "grant".to_owned(),
            ended_at: 1,
        },
    ] {
        assert!(super::names_grantors(&error), "{error}");
    }
    assert!(!super::names_grantors(&GrantError::LogUnavailable {
        reason: "unavailable".to_owned()
    }));
}
