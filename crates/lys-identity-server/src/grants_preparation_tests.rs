use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, RelationshipStore, Resource, Route};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use super::{Asked, from_batch};
use crate::spicedb::batch::Batch;
use crate::spicedb::snapshot::Snapshot;

#[tokio::test]
async fn rights_batch_an_uncovered_resource_falls_back_and_is_still_decided()
-> Result<(), Box<dyn Error>> {
    let (dir, state, _, person, agent, target) =
        crate::grant_settlement_tests::person_fixture().await?;
    let subject = IdentityId::Person(person);
    let outside = Resource::new("agent", &target.to_string())?;
    let covered = Resource::new("agent", &agent.to_string())?;
    let action = Action::new("read")?;
    let (stamp, engine) = crate::grants::with_grants(&state, |judged| {
        Ok((
            judged.grants.read_stamp(None)?,
            judged.grants.relationships().reader()?,
        ))
    })?;
    let reference = stamp.read_about(&engine, subject, &outside)?;
    let rows = engine.read()?;
    let snapshot = Snapshot::new(
        stamp.revision(),
        BTreeSet::from([(subject, covered.clone())]),
        rows,
    );
    let mut asked = Asked::new();
    for resource in [outside.clone(), covered] {
        let question = ExerciseRequest {
            caller: subject,
            route: Route::Api,
            resource: resource.clone(),
            action: action.clone(),
        };
        asked.insert(
            (subject, resource.clone(), action.clone()),
            (question, vec![resource]),
        );
    }
    let batch = Batch {
        snapshot,
        checks: BTreeMap::new(),
    };
    let data = from_batch(&engine, &stamp, asked, crate::session::now(), Some(&batch))?;
    assert_eq!(
        data.resources_read, 2,
        "one batch and one uncovered-resource read"
    );
    assert_eq!(
        data.readings
            .get(&(subject, outside.clone()))
            .ok_or("fallback missing")?
            .as_ref()
            .map_err(Clone::clone)?
            .relationships(),
        reference.relationships()
    );
    crate::grants::with_prepared_directory_grants(
        &state,
        &data,
        |judged| {
            let question = ExerciseRequest {
                caller: subject,
                route: Route::Api,
                resource: outside,
                action,
            };
            let frame = data.frame(&judged, &question)?;
            crate::grants::decide_in(&judged, &frame, &question, crate::session::now())?;
            Ok(())
        },
        |_, answer| Ok(answer),
    )?;
    drop(state);
    dir.close()?;
    Ok(())
}
