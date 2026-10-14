//! Prepare complete readings outside the authority hold, with fallback only outside coverage.

use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, ReadStamp, Resource};
use std::collections::BTreeMap;

use crate::spicedb::Relationships;
use crate::spicedb::batch::Batch;
use crate::spicedb::context::{Data, Permission};

pub(super) type Asked = BTreeMap<Permission, (ExerciseRequest, Vec<Resource>)>;

#[derive(Clone, Copy)]
pub(super) enum Style {
    Resource,
    Pass { cap: Option<usize>, bind: bool },
}

pub(super) fn prepare(
    engine: &Relationships,
    stamp: &ReadStamp,
    asked: Asked,
    at: u64,
    style: Style,
) -> Result<Data, GrantError> {
    let started = std::time::Instant::now();
    // Questions include revoked grants and denied placements. Their count cannot imply a minimum
    // number of rights; only the empty JSON array's size guarantees truncation before judgement.
    let truncated = matches!(
        style,
        Style::Pass {
            cap: Some(0 | 1),
            bind: false
        }
    );
    let count = asked.len();
    let batch = if matches!(style, Style::Pass { .. }) && count != 0 && !truncated {
        let covered = asked
            .keys()
            .map(|(subject, resource, _)| (*subject, resource.clone()))
            .collect();
        let questions = asked
            .iter()
            .flat_map(|((subject, _, action), (_, reach))| {
                reach
                    .iter()
                    .map(move |resource| (*subject, resource.clone(), action.clone()))
            })
            .collect();
        Some(engine.read_batch(stamp.revision(), covered, &questions, at)?)
    } else {
        None
    };
    let mut data = from_batch(
        engine,
        stamp,
        if truncated { BTreeMap::new() } else { asked },
        at,
        batch.as_ref(),
    )?;
    data.truncated = truncated;
    data.reads_elapsed_ms = started.elapsed().as_millis();
    Ok(data)
}

pub(super) fn from_batch(
    engine: &Relationships,
    stamp: &ReadStamp,
    asked: Asked,
    at: u64,
    batch: Option<&Batch>,
) -> Result<Data, GrantError> {
    let mut data = Data {
        revision: stamp.revision(),
        resources_read: usize::from(batch.is_some()),
        reads_elapsed_ms: 0,
        batch: batch.is_some(),
        truncated: false,
        stamp: Some(stamp.clone()),
        validation: if batch.is_some() {
            Ok(())
        } else {
            engine.certify(stamp.revision())
        },
        readings: BTreeMap::new(),
        checks: BTreeMap::new(),
    };
    for ((subject, resource, action), (_, reach)) in asked {
        if !data.readings.contains_key(&(subject, resource.clone())) {
            let reading = match &batch {
                Some(batch) if batch.snapshot.covers(subject, &resource) => {
                    stamp.read_about(&batch.snapshot, subject, &resource)
                }
                _ => {
                    data.resources_read += 1;
                    stamp.read_about(engine, subject, &resource)
                }
            };
            data.readings.insert((subject, resource.clone()), reading);
        }
        let reading = data
            .readings
            .get(&(subject, resource))
            .cloned()
            .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                reason: "the prepared resource has no reading".to_owned(),
            })?;
        for resource in reach {
            data.readings
                .entry((subject, resource.clone()))
                .or_insert_with(|| reading.clone());
            let key = (subject, resource.clone(), action.clone());
            if data.checks.contains_key(&key) {
                continue;
            }
            let check = batch
                .as_ref()
                .and_then(|batch| batch.checks.get(&key))
                .cloned()
                .unwrap_or_else(|| {
                    check(engine, subject, &resource, &action, at, stamp.revision())
                });
            data.checks.insert(key, check);
        }
    }
    Ok(data)
}

fn check(
    engine: &Relationships,
    subject: IdentityId,
    resource: &Resource,
    action: &Action,
    at: u64,
    revision: u64,
) -> Result<bool, GrantError> {
    match engine {
        Relationships::SpiceDb(engine) => engine.check_at(resource, action, subject, at, revision),
        #[cfg(not(test))]
        Relationships::Memory(_) => Ok(true),
        #[cfg(test)]
        Relationships::Memory(_) | Relationships::Fixture(_) => Ok(true),
    }
}

#[cfg(test)]
#[path = "grants_preparation_tests.rs"]
mod tests;
