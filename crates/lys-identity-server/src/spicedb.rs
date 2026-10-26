//! The grants' permission engine in `SpiceDB`: the schema the model gives,
//! the relationships the grant log projects, and the check.
//!
//! Lys's own kinds, written without a prefix, each take one definition with a
//! relation per model relation. An approved app's kind `{app}.{kind}` is the
//! definition `{app}/{kind}`, under the app's prefix and no other: its own
//! relations, a `parent_` relation to each parent kind, and one permission
//! per declared action, carried by its relations and by the same permission
//! on each parent that declares it. Placing a resource in its parent writes
//! the parent relation and moves no revision, since it is no grant event.
//!
//! The relationships are written by the grants' own projection, one grant
//! event at a time in log order, so `SpiceDB` holds a mirror of the grant log.
//! The revision the mirror stands at is itself a relationship, moved in the
//! same write as the relationships it counts and under a precondition that
//! the revision before it is held, so two writers cannot both move it.
//!
//! An engine may be a scratch scope of the same `SpiceDB` (`spicedb_scope`):
//! every name it holds is under the scope's prefix, it sees only its own,
//! and the service's engine never sees a scratch name and keeps every one
//! when it writes its schema. Every request passes through [`SpiceDb`]'s one
//! call, where the scope is applied, so no other part of the engine knows it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lys_identity::IdentityId;
use lys_identity::grants::{
    GrantError, KindModel, Model, Relationship, RelationshipStore, Resource, placement,
};
use serde_json::{Value, json};

use crate::spicedb_apps::definition;

#[path = "spicedb/batch.rs"]
pub(crate) mod batch;
#[path = "spicedb/snapshot.rs"]
pub(crate) mod snapshot;

#[path = "spicedb/context.rs"]
pub(crate) mod context;
#[path = "spicedb/credential.rs"]
mod credential;
#[path = "spicedb/names.rs"]
mod names;
#[path = "spicedb/reads.rs"]
mod reads;
#[path = "spicedb/relationships.rs"]
mod relationships;
#[path = "spicedb/schema.rs"]
mod schema;
#[path = "spicedb/wire.rs"]
mod wire;
pub use credential::{SpiceDbConnection, SpiceDbEngine, SpiceDbSettings};
use names::{FIXED, RESOURCE_SUBJECTS, engine_ident_on, engine_name_for, lys_name_on};
use wire::relationship_json;
#[cfg(test)]
use wire::relationship_of;

#[path = "spicedb_scope.rs"]
pub(crate) mod scope;

/// The line of an environment file that holds the preshared key.
const KEY_LINE: &str = "SPICEDB_GRPC_PRESHARED_KEY=";
/// The definitions the mirror's revision is kept in.
const MIRROR_SCHEMA: &str = "\ndefinition lys_revision {}\n\ndefinition lys_mirror {\n  relation revision: lys_revision\n}\n";

/// A caller-owned HTTP policy for the engine's transport, including each retry.
pub(crate) type PostJson = fn(
    &crate::spicedb_http::Client,
    &str,
    &str,
    &str,
    &str,
) -> Result<crate::spicedb_http::Answer, String>;

pub(crate) fn plain_post_json(
    client: &crate::spicedb_http::Client,
    authority: &str,
    path: &str,
    bearer: &str,
    body: &str,
) -> Result<crate::spicedb_http::Answer, String> {
    client.post_json(authority, path, bearer, body, |_| Ok(|| Ok(())))
}

/// The permission engine, reached over its gateway.
pub struct SpiceDb {
    endpoint: String,
    key: Arc<str>,
    mirror: String,
    relations: BTreeMap<String, BTreeSet<String>>,
    app_kinds: Mutex<BTreeMap<String, KindModel>>,
    scope: Option<String>,
    client: Arc<crate::spicedb_http::Client>,
    post_json: PostJson,
    tokens: Arc<Mutex<BTreeMap<u64, String>>>,
    confirmed: Arc<Mutex<Option<u64>>>,
}

impl std::fmt::Debug for SpiceDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceDb")
            .field("endpoint", &self.endpoint)
            .field("mirror", &self.mirror)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

fn unavailable(reason: impl Into<String>) -> GrantError {
    GrantError::PermissionEngineUnavailable {
        reason: reason.into(),
    }
}

/// A resource kind the engine cannot hold, refused by name before any commit.
fn unheld(kind: &str, reason: impl Into<String>) -> GrantError {
    GrantError::ResourceKindUnheld {
        kind: kind.to_owned(),
        reason: reason.into(),
    }
}

/// Refuse a name the engine's schema does not take.
fn engine_name(what: &str, name: &str) -> Result<(), GrantError> {
    if names::engine_takes(name) {
        return Ok(());
    }
    Err(unavailable(format!(
        "the {what} {name} is not a name the permission engine takes: three to sixty-four lowercase letters, digits and underscores, starting with a letter"
    )))
}

pub(super) static SCHEMA_WRITE: Mutex<()> = Mutex::new(());

impl SpiceDb {
    /// Reach the engine `settings` names and write the schema `model` gives,
    /// keeping every resource kind the engine already holds.
    ///
    /// # Errors
    /// Refuses credential loading, model schema construction or permission
    /// engine connection and schema write failures.
    pub fn open(settings: &SpiceDbSettings, model: &Model) -> Result<Self, GrantError> {
        Self::open_connected(&SpiceDbConnection::load(settings)?, model)
    }

    /// Open the live engine with an already loaded connection.
    ///
    /// # Errors
    /// Refuses model schema construction or permission engine connection,
    /// schema read and schema write failures.
    pub fn open_connected(
        connection: &SpiceDbConnection,
        model: &Model,
    ) -> Result<Self, GrantError> {
        Self::open_in(connection, model, None)
    }

    /// Reach the engine `connection` names as the scratch scope `scope`, and
    /// write the schema `model` gives under the scope's prefix, beside every
    /// name the engine holds outside it.
    pub(crate) fn open_scratch(
        connection: &SpiceDbConnection,
        model: &Model,
        scope: &str,
    ) -> Result<Self, GrantError> {
        Self::open_in(connection, model, Some(scope.to_owned()))
    }

    fn open_in(
        connection: &SpiceDbConnection,
        model: &Model,
        scope: Option<String>,
    ) -> Result<Self, GrantError> {
        let mut relations = BTreeMap::new();
        let mut engine_names = BTreeMap::new();
        for (relation, actions) in model.relations() {
            let relation = relation.to_string();
            engine_names.insert(engine_name_for("relation", &relation)?, relation.clone());
            let actions: BTreeSet<String> = actions.iter().map(ToString::to_string).collect();
            for action in &actions {
                let engine = engine_name_for("action", action)?;
                if let Some(other) = engine_names.insert(engine.clone(), action.clone()) {
                    if other != *action {
                        return Err(unavailable(format!(
                            "{other} and {action} both become {engine} in the permission engine"
                        )));
                    }
                }
            }
            relations.insert(relation, actions);
        }
        let engine = Self {
            endpoint: connection.endpoint.clone(),
            key: Arc::clone(&connection.key),
            mirror: connection.mirror.clone(),
            relations,
            app_kinds: Mutex::new(model.kinds().clone()),
            scope,
            client: Arc::default(),
            post_json: connection.post_json,
            tokens: Arc::default(),
            confirmed: Arc::default(),
        };
        if let Some(both) = engine
            .relations
            .values()
            .flatten()
            .find(|action| engine.relations.contains_key(*action))
        {
            return Err(unavailable(format!(
                "{both} names both a relation and an action, and the permission engine takes one name once"
            )));
        }
        engine.write_schema(&engine.kinds()?)?;
        Ok(engine)
    }

    /// Place `child` in `parent`, so the permissions on the parent flow to
    /// the child. It moves no revision: a placement is no grant event.
    ///
    /// # Errors
    /// Refuses permission engine transport or placement refusal, or a revision
    /// and written-token read that fail.
    pub fn place(&self, child: &Resource, parent: &Resource) -> Result<(), GrantError> {
        let update = json!({
            "operation": "OPERATION_TOUCH",
            "relationship": relationship_json(&placement(child, parent)),
        });
        let (status, body) = self.call("/v1/relationships/write", &json!({"updates": [update]}))?;
        if status == 200 {
            let revision = self.revision()?;
            self.keep_token(revision, &body, "/writtenAt/token")?;
            return Ok(());
        }
        Err(unavailable(format!("the placement was refused: {body}")))
    }

    fn marker(&self, revision: u64) -> Value {
        json!({
            "resource": {"objectType": "lys_mirror", "objectId": self.mirror},
            "relation": "revision",
            "subject": {"object": {"objectType": "lys_revision", "objectId": revision.to_string()}},
        })
    }
}

impl RelationshipStore for SpiceDb {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        let kind = resource.kind();
        if kind.contains('.') {
            if self
                .app_kinds
                .lock()
                .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))?
                .contains_key(kind)
            {
                return Ok(());
            }
            return Err(unheld(
                kind,
                format!(
                    "resource kind {kind} has no held permission model; the app owner must publish its resource schema and the Lys administrator must approve it before a grant can be committed"
                ),
            ));
        }
        if FIXED.contains(&kind) && !RESOURCE_SUBJECTS.contains(&kind) {
            return Err(unheld(
                kind,
                format!(
                    "resource kind {kind} is an internal permission-engine definition without model resource relations; a Lys maintainer must add support for that resource kind before a grant can be committed"
                ),
            ));
        }
        if !names::engine_takes(kind) {
            return Err(unheld(
                kind,
                format!(
                    "resource kind {kind} is not a name the permission engine can hold; the Lys administrator must choose three to sixty-four lowercase letters, digits and underscores, starting with a letter and not ending with an underscore, or a Lys maintainer must add a supported resource-kind mapping"
                ),
            ));
        }
        Ok(())
    }

    fn revision(&self) -> Result<u64, GrantError> {
        if let Some(revision) = *self
            .confirmed
            .lock()
            .map_err(|error| unavailable(format!("permission revision lock poisoned: {error}")))?
        {
            return Ok(revision);
        }
        let (held, token) = self.rows(
            &json!({
                "resourceType": "lys_mirror",
                "optionalResourceId": self.mirror,
            }),
            &json!({"fullyConsistent": true}),
        )?;
        let revisions = held
            .iter()
            .map(|relationship| {
                relationship.subject.id.parse::<u64>().map_err(|error| {
                    unavailable(format!(
                        "the mirror's revision `{}` is not a number: {error}",
                        relationship.subject.id
                    ))
                })
            })
            .collect::<Result<Vec<u64>, GrantError>>()?;
        let revision = revisions.into_iter().max().unwrap_or(0);
        if let Some(token) = token {
            self.tokens
                .lock()
                .map_err(|error| {
                    unavailable(format!(
                        "permission consistency tokens lock poisoned: {error}"
                    ))
                })?
                .insert(revision, token);
        }
        *self.confirmed.lock().map_err(|error| {
            unavailable(format!("permission revision lock poisoned: {error}"))
        })? = Some(revision);
        Ok(revision)
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        let Some(before) = revision.checked_sub(1) else {
            return Err(unavailable("a write at revision 0 follows nothing"));
        };
        let needed: BTreeSet<String> = touch
            .iter()
            .map(|relationship| relationship.resource.kind.clone())
            .filter(|kind| !FIXED.contains(&kind.as_str()) && !kind.contains('.'))
            .collect();
        let held = self.kinds()?;
        if !needed.is_subset(&held) {
            self.write_schema(&held.union(&needed).cloned().collect())?;
        }
        let update = |operation: &str, relationship: Value| json!({"operation": operation, "relationship": relationship});
        let mut updates: Vec<Value> = delete
            .iter()
            .map(|relationship| update("OPERATION_DELETE", relationship_json(relationship)))
            .chain(
                touch
                    .iter()
                    .map(|relationship| update("OPERATION_TOUCH", relationship_json(relationship))),
            )
            .collect();
        let mirror = json!({"resourceType": "lys_mirror", "optionalResourceId": self.mirror});
        let precondition = if before == 0 {
            json!({"operation": "OPERATION_MUST_NOT_MATCH", "filter": mirror})
        } else {
            updates.push(update("OPERATION_DELETE", self.marker(before)));
            json!({"operation": "OPERATION_MUST_MATCH", "filter": {
                "resourceType": "lys_mirror",
                "optionalResourceId": self.mirror,
                "optionalRelation": "revision",
                "optionalSubjectFilter": {"subjectType": "lys_revision", "optionalSubjectId": before.to_string()},
            }})
        };
        updates.push(update("OPERATION_TOUCH", self.marker(revision)));
        let request = json!({"updates": updates, "optionalPreconditions": [precondition]});
        let result = self.call("/v1/relationships/write", &request);
        if !result.as_ref().is_ok_and(|(status, _)| *status == 200) {
            *self.confirmed.lock().map_err(|error| {
                unavailable(format!("permission revision lock poisoned: {error}"))
            })? = None;
        }
        let (status, body) = result?;
        if status == 200 {
            self.keep_token(revision, &body, "/writtenAt/token")?;
            *self.confirmed.lock().map_err(|error| {
                unavailable(format!("permission revision lock poisoned: {error}"))
            })? = Some(revision);
            return Ok(());
        }
        if body.contains("PRECONDITION") {
            return Err(unavailable(format!(
                "a write at revision {revision} does not follow the revision the permission engine holds"
            )));
        }
        Err(unavailable(format!("the write was refused: {body}")))
    }

    fn prepared_stamp(&self) -> Option<lys_identity::grants::ReadStamp> {
        context::stamp()
    }

    fn read_about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        self.read_about_at(subject, resource, self.revision()?)
    }

    fn read_about_at(
        &self,
        subject: IdentityId,
        resource: &Resource,
        revision: u64,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        if let Some(held) = context::about(subject, resource, revision) {
            return held;
        }
        self.about(subject, resource, revision)
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        if let Some(held) = context::whole() {
            return held;
        }
        let kinds = self.kinds()?;
        let mut held = BTreeSet::new();
        for kind in kinds.iter().map(String::as_str).chain(["grant"]) {
            held.extend(self.read_of(&json!({"resourceType": definition(kind)}))?);
        }
        Ok(held)
    }
}

#[cfg(test)]
pub use relationships::FaultRelationships;
#[cfg(test)]
pub(crate) use relationships::ReadHook;
pub use relationships::Relationships;

#[cfg(test)]
#[path = "spicedb_upgrade_tests.rs"]
mod upgrade_tests;

#[cfg(test)]
#[path = "spicedb_calls_tests.rs"]
mod calls_tests;

#[cfg(test)]
#[path = "spicedb_key_tests.rs"]
mod key_tests;

#[cfg(test)]
#[path = "spicedb_poison_tests.rs"]
mod poison_tests;
