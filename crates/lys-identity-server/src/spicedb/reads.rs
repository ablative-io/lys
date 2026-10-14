//! Scoped reads retain the private consistency token and reassert the public revision marker.

use super::wire::{object_json, relationship_of};
use super::{SpiceDb, definition, engine_ident_on, unavailable};
use lys_identity::IdentityId;
use lys_identity::grants::{
    Action, GrantError, ObjectRef, Relationship, RelationshipStore, Resource,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

impl SpiceDb {
    pub(super) fn keep_token(
        &self,
        revision: u64,
        body: &str,
        pointer: &str,
    ) -> Result<(), GrantError> {
        let answer: Value = serde_json::from_str(body).map_err(|error| {
            unavailable(format!(
                "the permission engine answered an unreadable consistency token: {error}"
            ))
        })?;
        let token = answer
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| {
                unavailable("the permission engine answered without its consistency token")
            })?;
        self.tokens
            .lock()
            .map_err(|error| {
                unavailable(format!(
                    "permission consistency tokens lock poisoned: {error}"
                ))
            })?
            .insert(revision, token.to_owned());
        Ok(())
    }

    fn consistency(&self, revision: u64) -> Result<Value, GrantError> {
        Ok(
            match self
                .tokens
                .lock()
                .map_err(|error| {
                    unavailable(format!(
                        "permission consistency tokens lock poisoned: {error}"
                    ))
                })?
                .get(&revision)
            {
                Some(token) => json!({"atLeastAsFresh": {"token": token}}),
                None => json!({"fullyConsistent": true}),
            },
        )
    }

    pub(super) fn rows(
        &self,
        filter: &Value,
        consistency: &Value,
    ) -> Result<(BTreeSet<Relationship>, Option<String>), GrantError> {
        let (status, body) = self.call(
            "/v1/relationships/read",
            &json!({"consistency": consistency, "relationshipFilter": filter}),
        )?;
        if status != 200 {
            if reads_nothing_of_a_kind_never_written(filter, status, &body) {
                return Ok((BTreeSet::new(), None));
            }
            return Err(unavailable(format!(
                "the relationships could not be read: {body}"
            )));
        }
        let mut held = BTreeSet::new();
        let mut token = None;
        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = serde_json::from_str(line).map_err(|error| {
                unavailable(format!(
                    "the permission engine answered an unreadable relationship: {error}"
                ))
            })?;
            held.insert(
                row.pointer("/result/relationship")
                    .and_then(relationship_of)
                    .ok_or_else(|| {
                        unavailable("the permission engine answered without a relationship")
                    })?,
            );
            if let Some(read) = row
                .pointer("/result/readAt/token")
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
            {
                token = Some(read.to_owned());
            }
        }
        Ok((held, token))
    }

    pub(super) fn read_of(&self, filter: &Value) -> Result<Vec<Relationship>, GrantError> {
        Ok(self
            .rows(filter, &json!({"fullyConsistent": true}))?
            .0
            .into_iter()
            .collect())
    }

    pub(super) fn certify(&self, revision: u64) -> Result<Value, GrantError> {
        let consistency = self.consistency(revision)?;
        let (held, token) = self.rows(&json!({"resourceType": "lys_mirror", "optionalResourceId": self.mirror,
            "optionalRelation": "revision", "optionalSubjectFilter": {"subjectType": "lys_revision", "optionalSubjectId": revision.to_string()}}), &consistency)?;
        if revision != 0
            && !held.iter().any(|row| {
                row.resource.kind == "lys_mirror"
                    && row.resource.id == self.mirror
                    && row.relation == "revision"
                    && row.subject.kind == "lys_revision"
                    && row.subject.id == revision.to_string()
            })
        {
            return Err(unavailable(format!(
                "the permission engine's mirror does not hold settled revision {revision}"
            )));
        }
        if let Some(token) = token {
            self.tokens
                .lock()
                .map_err(|error| {
                    unavailable(format!(
                        "permission consistency tokens lock poisoned: {error}"
                    ))
                })?
                .insert(revision, token);
        } else if revision != 0 && consistency.get("fullyConsistent").is_some() {
            return Err(unavailable(
                "the permission engine answered without its read consistency token",
            ));
        }
        self.consistency(revision)
    }

    pub(super) fn about(
        &self,
        subject: IdentityId,
        resource: &Resource,
        revision: u64,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        let consistency = self.certify(revision)?;
        let mut ancestors = BTreeSet::new();
        let mut pending = vec![ObjectRef::resource(resource)];
        let mut placements = BTreeSet::new();
        while let Some(object) = pending.pop() {
            if !ancestors.insert(object.clone()) {
                continue;
            }
            let parents = self
                .app_kinds
                .lock()
                .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))?
                .get(&object.kind)
                .map(|model| model.parents.clone())
                .unwrap_or_default();
            for parent in parents {
                let (rows, _) = self.rows(&json!({"resourceType": definition(&object.kind), "optionalResourceId": object.id.replace('.', "|"),
                    "optionalRelation": lys_identity::grants::parent_relation(&parent)}), &consistency)?;
                pending.extend(rows.iter().map(|row| row.subject.clone()));
                placements.extend(rows);
            }
        }
        let mut held = lys_identity::grants::about::select(
            subject,
            resource,
            |object| {
                if object.kind != "grant" {
                    return self.rows(&json!({"resourceType": "grant", "optionalRelation": "holder",
                        "optionalSubjectFilter": {"subjectType": definition(&object.kind), "optionalSubjectId": object.id.replace('.', "|")}}), &consistency).map(|(held, _)| held);
                }
                let mut held = BTreeSet::new();
                for ancestor in &ancestors {
                    held.extend(self.rows(&json!({"resourceType": definition(&ancestor.kind), "optionalResourceId": ancestor.id.replace('.', "|"),
                        "optionalSubjectFilter": {"subjectType": "grant", "optionalSubjectId": object.id}}), &consistency)?.0);
                }
                Ok(held)
            },
            |object| {
                if object.kind != "grant" {
                    return Ok(placements
                        .iter()
                        .filter(|row| row.resource == *object)
                        .cloned()
                        .collect());
                }
                self.rows(
                    &json!({"resourceType": "grant", "optionalResourceId": object.id}),
                    &consistency,
                )
                .map(|(held, _)| held)
            },
        )?;
        held.extend(placements);
        Ok(held)
    }

    /// Ask one permission at the engine's confirmed public revision and private token.
    pub fn check(
        &self,
        resource: &Resource,
        action: &Action,
        subject: IdentityId,
        now: u64,
    ) -> Result<bool, GrantError> {
        if let Some(allowed) = super::context::check(subject, resource, action) {
            return allowed;
        }
        let revision = self.revision()?;
        self.check_at(resource, action, subject, now, revision)
    }

    pub(crate) fn check_at(
        &self,
        resource: &Resource,
        action: &Action,
        subject: IdentityId,
        now: u64,
        revision: u64,
    ) -> Result<bool, GrantError> {
        let carried = resource.kind().contains('.')
            || self
                .relations
                .values()
                .any(|actions| actions.contains(action.as_str()));
        if !carried || !self.kinds()?.contains(resource.kind()) {
            return Ok(false);
        }
        let consistency = self.certify(revision)?;
        let request = json!({"consistency": consistency, "resource": object_json(&ObjectRef::resource(resource)),
            "permission": engine_ident_on(resource.kind(), action.as_str()), "subject": {"object": object_json(&ObjectRef::identity(subject))}, "context": {"now": now}});
        let (status, body) = self.call("/v1/permissions/check", &request)?;
        if status != 200 {
            return Err(unavailable(format!("the check was refused: {body}")));
        }
        let answer: Value = serde_json::from_str(&body).map_err(|error| {
            unavailable(format!(
                "the permission engine answered an unreadable check: {error}"
            ))
        })?;
        match answer.get("permissionship").and_then(Value::as_str) {
            Some("PERMISSIONSHIP_HAS_PERMISSION") => Ok(true),
            Some("PERMISSIONSHIP_NO_PERMISSION" | "PERMISSIONSHIP_CONDITIONAL_PERMISSION") => {
                Ok(false)
            }
            _ => Err(unavailable(
                "the permission engine answered without a permission verdict",
            )),
        }
    }
}

/// The kind the engine says it holds no definition for, when `status` and
/// `body` are that refusal: `SpiceDB` answers 400 with the reason
/// `ERROR_REASON_UNKNOWN_DEFINITION` and the kind's name in its metadata,
/// under `error` on its HTTP door and at the top on its gRPC one.
pub(super) fn undefined_kind(status: u16, body: &str) -> Option<String> {
    if status != 400 {
        return None;
    }
    let refusal: Value = serde_json::from_str(body).ok()?;
    let details = refusal["error"]["details"]
        .as_array()
        .or_else(|| refusal["details"].as_array())?;
    details
        .iter()
        .find(|detail| detail["reason"] == "ERROR_REASON_UNKNOWN_DEFINITION")
        .and_then(|detail| detail["metadata"]["definition_name"].as_str())
        .map(str::to_owned)
}

/// Whether a read refused with `status` and `body` read a kind the engine
/// holds no definition for because nothing of that kind was ever written.
/// A kind is declared to the engine with the first relationship written of
/// it (`SpiceDb::write`), so the first read of a kind, before any grant on
/// one, meets no definition: that is an empty read, not an outage. Only the
/// kind the filter asks for is taken so; a scoped engine names it under
/// its prefix, so the name is matched whole or after its `/`.
pub(super) fn reads_nothing_of_a_kind_never_written(
    filter: &Value,
    status: u16,
    body: &str,
) -> bool {
    let Some(asked) = filter["resourceType"].as_str() else {
        return false;
    };
    undefined_kind(status, body).is_some_and(|named| {
        named == asked
            || named
                .strip_suffix(asked)
                .is_some_and(|prefix| prefix.ends_with('/'))
    })
}

#[cfg(test)]
#[path = "reads_tests.rs"]
mod tests;
