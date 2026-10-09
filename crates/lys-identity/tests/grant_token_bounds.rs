#![cfg(test)]
//! Each grant token carries the permission engine's own bound for what it
//! becomes there: an action and a relation 64 bytes, a resource kind 128,
//! a resource id 1024. One byte more is refused naming its bound. Kinds,
//! actions and relations are lowercase; a resource id is named exactly as
//! its product names it, uppercase included, and never case-folded.

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
fn every_token_keeps_its_character_set_and_is_never_empty() {
    for bad in ["", "Read", "read write", "read:all", "read/all"] {
        assert!(Action::new(bad).is_err(), "{bad:?}");
        assert!(Relation::new(bad).is_err(), "{bad:?}");
        assert!(Resource::new(bad, "one").is_err(), "{bad:?}");
    }
    for bad in ["", "read write", "read:all", "read/all"] {
        assert!(Resource::new("note", bad).is_err(), "{bad:?}");
    }
    assert!(Action::new("only.agent.mcp-request_approve").is_ok());
    assert!(Resource::new("lys.note", "one.two-three_4").is_ok());
}

#[test]
fn a_cambium_stream_id_is_a_resource_id_as_named() -> TestResult {
    let stream = "mHyUs0FppSFqSzfT5kjBzZk23asX6tgScUEKNwJawq0";
    let resource = Resource::new("cambium.stream", stream)?;
    assert_eq!(resource.id().as_bytes(), stream.as_bytes());
    assert_eq!(resource.to_string(), format!("cambium.stream:{stream}"));
    Ok(())
}

#[test]
fn two_ids_that_differ_only_in_case_are_two_resources() -> TestResult {
    let upper = Resource::new("cambium.stream", "Stream-A.1")?;
    let lower = Resource::new("cambium.stream", "stream-a.1")?;
    assert_ne!(upper, lower);
    assert_eq!((upper.id(), lower.id()), ("Stream-A.1", "stream-a.1"));
    Ok(())
}

#[test]
fn a_kind_or_an_action_with_an_uppercase_letter_is_still_refused() {
    assert!(Resource::new("Cambium.stream", "one").is_err());
    assert!(Resource::new("cambium.Stream", "one").is_err());
    assert!(Action::new("Read").is_err());
    assert!(Action::new("reAd").is_err());
    assert!(Relation::new("Editor").is_err());
}

#[test]
fn a_slash_colon_equals_plus_or_space_is_still_refused_in_an_id() {
    for bad in ["a/b", "a:b", "a=b", "a+b", "a b", "/", ":", "=", "+", " "] {
        let refused = Resource::new("cambium.stream", bad);
        assert!(
            matches!(
                &refused,
                Err(GrantError::TokenInvalid {
                    kind: "resource id",
                    ..
                })
            ),
            "{bad:?}: {refused:?}"
        );
    }
}
