//! Relationship export and permission checks share one certified exact snapshot.

use lys_identity::grants::{GrantError, ObjectRef, Relationship};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use super::context::{Permission, Question};
use super::snapshot::Snapshot;
use super::wire::{object_json, relationship_of};
use super::{SpiceDb, engine_ident_on, unavailable};

pub(crate) struct Batch {
    pub(crate) snapshot: Snapshot,
    pub(crate) checks: BTreeMap<Permission, Result<bool, GrantError>>,
}

impl SpiceDb {
    pub(crate) fn read_batch(
        &self,
        revision: u64,
        covered: BTreeSet<Question>,
        questions: &BTreeSet<Permission>,
        at: u64,
    ) -> Result<Batch, GrantError> {
        let certified = self.certify(revision)?;
        let token = certified
            .pointer("/atLeastAsFresh/token")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                unavailable("the relationship batch has no certified consistency token")
            })?;
        // Export has no read token of its own; freeze the certified marker's snapshot for both calls.
        let consistency = json!({"atExactSnapshot":{"token":token}});
        let rows = self.export_rows(&consistency)?;
        if revision != 0
            && !rows.iter().any(|row| {
                row.resource.kind == "lys_mirror"
                    && row.resource.id == self.mirror
                    && row.relation == "revision"
                    && row.subject.kind == "lys_revision"
                    && row.subject.id == revision.to_string()
            })
        {
            return Err(unavailable(
                "the relationship batch does not hold its settled revision",
            ));
        }
        let checks = self.bulk_checks(questions, at, &consistency)?;
        Ok(Batch {
            snapshot: Snapshot::new(revision, covered, rows),
            checks,
        })
    }

    fn export_rows(&self, consistency: &Value) -> Result<BTreeSet<Relationship>, GrantError> {
        // A subject filter omits resource edges and grant sources. Export reads the complete graph once.
        let (status, body) = self.raw(
            "/v1/relationships/exportbulk",
            &json!({"consistency":consistency}),
        )?;
        if status != 200 {
            return Err(unavailable(format!(
                "the relationship batch was refused with HTTP {status}"
            )));
        }
        let mut held = BTreeSet::new();
        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let page: Value = serde_json::from_str(line).map_err(|error| {
                unavailable(format!("the relationship batch is unreadable: {error}"))
            })?;
            let rows = page
                .pointer("/result/relationships")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    unavailable("the relationship batch answered without a complete page")
                })?;
            for row in rows {
                let kind = row
                    .pointer("/resource/objectType")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        unavailable("the relationship batch carries an unnamed resource")
                    })?;
                if !super::scope::owns(self.scope.as_deref(), kind) {
                    continue;
                }
                let unscoped = self.scope.as_ref().map_or_else(
                    || Ok(row.clone()),
                    |scope| {
                        serde_json::from_str::<Value>(&super::scope::unscoped_answer(
                            scope,
                            &row.to_string(),
                        ))
                        .map_err(|error| {
                            unavailable(format!("the scoped batch is unreadable: {error}"))
                        })
                    },
                )?;
                held.insert(relationship_of(&unscoped).ok_or_else(|| {
                    unavailable("the relationship batch carries an unreadable relationship")
                })?);
            }
        }
        Ok(held)
    }

    fn bulk_checks(
        &self,
        questions: &BTreeSet<Permission>,
        at: u64,
        consistency: &Value,
    ) -> Result<BTreeMap<Permission, Result<bool, GrantError>>, GrantError> {
        let kinds = self.kinds()?;
        let mut checks = BTreeMap::new();
        let mut sent = Vec::new();
        for (subject, resource, action) in questions {
            let carried = resource.kind().contains('.')
                || self
                    .relations
                    .values()
                    .any(|actions| actions.contains(action.as_str()));
            if !carried || !kinds.contains(resource.kind()) {
                checks.insert((*subject, resource.clone(), action.clone()), Ok(false));
                continue;
            }
            let item = json!({"resource":object_json(&ObjectRef::resource(resource)),
                "permission":engine_ident_on(resource.kind(), action.as_str()),
                "subject":{"object":object_json(&ObjectRef::identity(*subject))}, "context":{"now":at}});
            sent.push(((*subject, resource.clone(), action.clone()), item));
        }
        if sent.is_empty() {
            return Ok(checks);
        }
        let items: Vec<&Value> = sent.iter().map(|(_, item)| item).collect();
        let (status, body) = self.call(
            "/v1/permissions/checkbulk",
            &json!({"consistency":consistency,"items":items}),
        )?;
        if status != 200 {
            return Err(unavailable(format!(
                "the permission batch was refused with HTTP {status}"
            )));
        }
        let answer: Value = serde_json::from_str(&body)
            .map_err(|error| unavailable(format!("the permission batch is unreadable: {error}")))?;
        let pairs = answer
            .get("pairs")
            .and_then(Value::as_array)
            .filter(|pairs| pairs.len() == sent.len())
            .ok_or_else(|| unavailable("the permission batch did not answer every question"))?;
        for ((key, item), pair) in sent.into_iter().zip(pairs) {
            if pair.get("request") != Some(&item) {
                return Err(unavailable(
                    "the permission batch answered a different question",
                ));
            }
            let verdict = match pair.pointer("/item/permissionship").and_then(Value::as_str) {
                Some("PERMISSIONSHIP_HAS_PERMISSION") => Ok(true),
                Some("PERMISSIONSHIP_NO_PERMISSION" | "PERMISSIONSHIP_CONDITIONAL_PERMISSION") => {
                    Ok(false)
                }
                _ => Err(unavailable(
                    "the permission batch answered without a permission verdict",
                )),
            };
            checks.insert(key, verdict);
        }
        Ok(checks)
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
