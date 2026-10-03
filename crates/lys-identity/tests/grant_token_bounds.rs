#![cfg(test)]
//! Each grant token carries the permission engine's own bound for what it
//! becomes there: an action and a relation 64 bytes, a resource kind 128,
//! a resource id 1024. One byte more is refused naming its bound; the
//! character set is the same for all four.

use std::error::Error;

use lys_identity::grants::types::{
    ACTION_MAX_BYTES, RELATION_MAX_BYTES, RESOURCE_ID_MAX_BYTES, RESOURCE_KIND_MAX_BYTES,
};
use lys_identity::grants::{Action, GrantError, Relation, Resource};

type TestResult = Result<(), Box<dyn Error>>;

fn refused_at(result: &Result<(), GrantError>, bound: usize) -> bool {
    matches!(result, Err(GrantError::TokenInvalid { max, .. }) if *max == bound)
}

#[test]
fn each_token_takes_its_engine_bound_and_refuses_one_byte_more() -> TestResult {
    let at = |len: usize| "a".repeat(len);
    assert_eq!(Action::new(&at(64))?.as_str().len(), 64);
    assert_eq!(Relation::new(&at(64))?.as_str().len(), 64);
    let resource = Resource::new(&at(128), &at(1024))?;
    assert_eq!((resource.kind().len(), resource.id().len()), (128, 1024));
    assert_eq!((ACTION_MAX_BYTES, RELATION_MAX_BYTES), (64, 64));
    assert_eq!(
        (RESOURCE_KIND_MAX_BYTES, RESOURCE_ID_MAX_BYTES),
        (128, 1024)
    );
    assert!(refused_at(&Action::new(&at(65)).map(drop), 64));
    assert!(refused_at(&Relation::new(&at(65)).map(drop), 64));
    assert!(refused_at(&Resource::new(&at(129), "one").map(drop), 128));
    assert!(refused_at(
        &Resource::new("note", &at(1025)).map(drop),
        1024
    ));
    Ok(())
}

#[test]
fn every_token_keeps_one_character_set_and_is_never_empty() {
    for bad in ["", "Read", "read write", "read:all", "read/all"] {
        assert!(Action::new(bad).is_err(), "{bad:?}");
        assert!(Relation::new(bad).is_err(), "{bad:?}");
        assert!(Resource::new(bad, "one").is_err(), "{bad:?}");
        assert!(Resource::new("note", bad).is_err(), "{bad:?}");
    }
    assert!(Action::new("only.agent.mcp-request_approve").is_ok());
    assert!(Resource::new("lys.note", "one.two-three_4").is_ok());
}
