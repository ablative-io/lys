//! Relationship readings live only for one judgement, and the guard releases them afterward.

use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, ReadStamp, Reading, Resource};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Question = (IdentityId, Resource);
pub(crate) type Permission = (IdentityId, Resource, Action);

#[derive(Clone)]
pub(crate) struct Data {
    pub(crate) revision: u64,
    pub(crate) resources_read: usize,
    pub(crate) reads_elapsed_ms: u128,
    pub(crate) batch: bool,
    pub(crate) truncated: bool,
    pub(crate) stamp: Option<ReadStamp>,
    pub(crate) validation: Result<(), GrantError>,
    pub(crate) readings: BTreeMap<Question, Result<Reading, GrantError>>,
    pub(crate) checks: BTreeMap<Permission, Result<bool, GrantError>>,
}

impl Data {
    pub(crate) fn permits(&self, asked: &ExerciseRequest) -> Result<bool, GrantError> {
        self.validation.clone()?;
        self.checks
            .get(&(asked.caller, asked.resource.clone(), asked.action.clone()))
            .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                reason: "the decision has no prepared permission question".to_owned(),
            })?
            .clone()
    }
}

thread_local! {
    static READING: std::cell::RefCell<Option<Data>> = const { std::cell::RefCell::new(None) };
}

pub(crate) struct Hold(Option<Data>);
impl Hold {
    pub(crate) fn enter(prepared: &Data) -> Self {
        Self(READING.with(|held| held.replace(Some(prepared.clone()))))
    }
}
impl Drop for Hold {
    fn drop(&mut self) {
        READING.with(|held| *held.borrow_mut() = self.0.take());
    }
}

pub(crate) fn about(
    subject: IdentityId,
    resource: &Resource,
    revision: u64,
) -> Option<Result<BTreeSet<lys_identity::grants::Relationship>, GrantError>> {
    READING.with(|held| {
        held.borrow().as_ref().map(|prepared| {
            prepared.validation.clone()?;
            if prepared.revision != revision {
                return Err(GrantError::StaleDecision {
                    required: revision,
                    projected: prepared.revision,
                });
            }
            prepared
                .readings
                .get(&(subject, resource.clone()))
                .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                    reason: "the decision has no prepared relationship reading".to_owned(),
                })?
                .as_ref()
                .map(|reading| reading.relationships().clone())
                .map_err(Clone::clone)
        })
    })
}

pub(crate) fn whole() -> Option<Result<BTreeSet<lys_identity::grants::Relationship>, GrantError>> {
    READING.with(|held| {
        held.borrow().as_ref().map(|prepared| {
            prepared.validation.clone()?;
            let mut rows = BTreeSet::new();
            for reading in prepared.readings.values() {
                rows.extend(
                    reading
                        .as_ref()
                        .map_err(Clone::clone)?
                        .relationships()
                        .iter()
                        .cloned(),
                );
            }
            Ok(rows)
        })
    })
}

pub(crate) fn check(
    subject: IdentityId,
    resource: &Resource,
    action: &Action,
) -> Option<Result<bool, GrantError>> {
    READING.with(|held| {
        held.borrow().as_ref().map(|prepared| {
            prepared.permits(&ExerciseRequest {
                caller: subject,
                route: lys_identity::grants::Route::Api,
                resource: resource.clone(),
                action: action.clone(),
            })
        })
    })
}

pub(crate) fn stamp() -> Option<ReadStamp> {
    READING.with(|held| held.borrow().as_ref().and_then(|data| data.stamp.clone()))
}

#[cfg(test)]
mod tests {
    use super::{
        Action, BTreeMap, BTreeSet, Data, GrantError, Hold, IdentityId, Resource, about, check,
        whole,
    };

    #[test]
    fn a_read_context_ends_with_its_guard_and_refuses_a_different_revision()
    -> Result<(), Box<dyn std::error::Error>> {
        let subject = IdentityId::Person("person-11111111111111111111111111111111".parse()?);
        let resource = Resource::new("project", "asked")?;
        let action = Action::new("read")?;
        assert!(check(subject, &resource, &action).is_none());
        let data = Data {
            batch: false,
            truncated: false,
            revision: 1,
            resources_read: 0,
            reads_elapsed_ms: 0,
            stamp: None,
            validation: Ok(()),
            readings: BTreeMap::new(),
            checks: BTreeMap::from([((subject, resource.clone(), action.clone()), Ok(true))]),
        };
        let guard = Hold::enter(&data);
        assert_eq!(check(subject, &resource, &action), Some(Ok(true)));
        assert!(matches!(
            about(subject, &resource, 2),
            Some(Err(GrantError::StaleDecision {
                required: 2,
                projected: 1
            }))
        ));
        assert_eq!(whole(), Some(Ok(BTreeSet::new())));
        drop(guard);
        assert!(check(subject, &resource, &action).is_none());
        Ok(())
    }
}
