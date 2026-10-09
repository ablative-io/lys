#![cfg(test)]

//! ACCESS-006 R6: a decision reads only the live grants on the resource it
//! is asked about, from the book's live index; a revoked grant is read only
//! when no live grant answers, to refuse by its exact name, and never allows.

mod support;

use std::error::Error;

use lys_identity::grants::{Frame, GrantError, PassOn, RevokeRequest, Route};
use lys_identity::{IdentityId, OperationId};
use support::World;

type TestResult = Result<(), Box<dyn Error>>;

fn revoke(world: &mut World, grant: lys_identity::grants::GrantId) -> TestResult {
    let request = RevokeRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        grant,
        reason: "withdrawn".to_owned(),
    };
    let now = world.now;
    world.grants.revoke(&request, now)?;
    Ok(())
}

#[test]
fn an_allowed_decision_reads_no_revoked_grant_and_a_refusal_names_it() -> TestResult {
    let mut world = World::new()?;
    let tom = IdentityId::Person(world.tom);
    let mut retired = Vec::new();
    for _ in 0..3 {
        let grant = world.root(world.tom, "heron", PassOn::UseOnly, None)?;
        revoke(&mut world, grant)?;
        retired.push(grant);
    }
    let live = world.root(world.tom, "tern", PassOn::UseOnly, None)?;

    let before = Frame::history_reads();
    let permit = world.exercise(tom, "read", Route::Api)?;
    assert_eq!(permit.grant, live);
    assert_eq!(
        Frame::history_reads() - before,
        0,
        "an allowed decision reads the live index alone"
    );

    // Write is held only by the revoked grants: refused by a revoked
    // grant's exact name, read from the retired history.
    let before = Frame::history_reads();
    let refused = world.exercise(tom, "write", Route::Api);
    assert!(
        matches!(&refused, Err(GrantError::Revoked { grant }) if retired.iter().any(|id| &id.to_string() == grant)),
        "{refused:?}"
    );
    assert!(Frame::history_reads() - before > 0);

    revoke(&mut world, live)?;
    let refused = world.exercise(tom, "read", Route::Api);
    assert!(
        matches!(&refused, Err(GrantError::Revoked { .. })),
        "the last live grant revoked is refused by name: {refused:?}"
    );
    Ok(())
}
