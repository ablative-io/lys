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
    Action, GrantError, KindModel, MemoryRelationships, Model, ObjectRef, Relationship,
    RelationshipStore, Resource, SCHEMA, placement,
};
use serde_json::{Value, json};

use crate::spicedb_apps::{app_definition, app_definitions, definition};

mod credential;
mod names;
mod wire;
pub use credential::{SpiceDbConnection, SpiceDbEngine, SpiceDbSettings};
use names::{engine_ident, engine_ident_on, engine_name_for, lys_name_on};
use wire::{object_json, object_of, relationship_json, relationship_of};

#[path = "spicedb_scope.rs"]
pub(crate) mod scope;

/// The line of an environment file that holds the preshared key.
const KEY_LINE: &str = "SPICEDB_GRPC_PRESHARED_KEY=";
/// The definitions every schema holds beside the resource kinds.
const FIXED: [&str; 6] = [
    "person",
    "agent",
    "service_account",
    "grant",
    "lys_revision",
    "lys_mirror",
];
/// Principals that also carry ordinary resource relations.
const RESOURCE_SUBJECTS: [&str; 3] = ["person", "agent", "service_account"];
/// The definitions the mirror's revision is kept in.
const MIRROR_SCHEMA: &str = "\ndefinition lys_revision {}\n\ndefinition lys_mirror {\n  relation revision: lys_revision\n}\n";

/// The permission engine, reached over its gateway.
pub struct SpiceDb {
    endpoint: String,
    key: Arc<str>,
    mirror: String,
    relations: BTreeMap<String, BTreeSet<String>>,
    app_kinds: Mutex<BTreeMap<String, KindModel>>,
    scope: Option<String>,
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
    pub fn open(settings: &SpiceDbSettings, model: &Model) -> Result<Self, GrantError> {
        Self::open_connected(&SpiceDbConnection::load(settings)?, model)
    }

    /// Open the live engine with an already loaded connection.
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

    /// The schema for the resource kinds `kinds`.
    pub fn schema(&self, kinds: &BTreeSet<String>) -> Result<String, GrantError> {
        let mut carried: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (relation, actions) in &self.relations {
            for action in actions {
                carried.entry(action).or_default().push(relation);
            }
        }
        let relations = self
            .relations
            .keys()
            .map(|relation| format!("  relation {}: grant#holder\n", engine_ident(relation)));
        let permissions = carried.iter().map(|(action, relations)| {
            let terms: Vec<String> = relations
                .iter()
                .map(|relation| engine_ident(relation))
                .collect();
            format!(
                "  permission {} = {}\n",
                engine_ident(action),
                terms.join(" + ")
            )
        });
        let body: String = relations.chain(permissions).collect();
        let kinds = kinds
            .iter()
            .filter(|kind| !kind.contains('.') && !RESOURCE_SUBJECTS.contains(&kind.as_str()))
            .map(|kind| format!("\ndefinition {kind} {{\n{body}}}\n"));
        let app_kinds = self
            .app_kinds
            .lock()
            .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))?
            .clone();
        let apps = app_kinds
            .iter()
            .map(|(kind, model)| app_definition(kind, model, &app_kinds));
        // Principals are both subjects and resources, so their one definition
        // must carry ordinary resource relations.
        let mut subjects = SCHEMA.to_owned();
        for kind in RESOURCE_SUBJECTS {
            subjects = subjects.replace(
                &format!("definition {kind} {{}}"),
                &format!("definition {kind} {{\n{body}}}"),
            );
        }
        Ok([subjects, MIRROR_SCHEMA.to_owned()]
            .into_iter()
            .chain(kinds)
            .chain(apps)
            .collect())
    }

    pub(crate) fn schema_writer(&self) -> Result<Self, GrantError> {
        Ok(Self {
            endpoint: self.endpoint.clone(),
            key: Arc::clone(&self.key),
            mirror: self.mirror.clone(),
            relations: self.relations.clone(),
            scope: self.scope.clone(),
            app_kinds: Mutex::new(
                self.app_kinds
                    .lock()
                    .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))?
                    .clone(),
            ),
        })
    }

    pub(crate) fn hold_app_kinds(
        &self,
        kinds: &BTreeMap<String, KindModel>,
    ) -> Result<(), GrantError> {
        *self
            .app_kinds
            .lock()
            .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))? =
            kinds.clone();
        Ok(())
    }

    /// Hold `kinds` as the approved apps' kinds from now on and write the
    /// schema they give, keeping every kind of Lys's own the engine holds.
    pub fn set_app_kinds(&self, kinds: &BTreeMap<String, KindModel>) -> Result<(), GrantError> {
        *self
            .app_kinds
            .lock()
            .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))? =
            kinds.clone();
        self.write_schema(&self.kinds()?)
    }

    /// Place `child` in `parent`, so the permissions on the parent flow to
    /// the child. It moves no revision: a placement is no grant event.
    pub fn place(&self, child: &Resource, parent: &Resource) -> Result<(), GrantError> {
        let update = json!({
            "operation": "OPERATION_TOUCH",
            "relationship": relationship_json(&placement(child, parent)),
        });
        let (status, body) = self.call("/v1/relationships/write", &json!({"updates": [update]}))?;
        if status == 200 {
            return Ok(());
        }
        Err(unavailable(format!("the placement was refused: {body}")))
    }

    /// The engine's schema text; none when it holds no schema.
    fn schema_text(&self) -> Result<Option<String>, GrantError> {
        let (status, body) = self.call("/v1/schema/read", &json!({}))?;
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            return Err(unavailable(format!("the schema could not be read: {body}")));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| unavailable(format!("the schema answer is not JSON: {error}")))?;
        let text = value
            .get("schemaText")
            .and_then(Value::as_str)
            .ok_or_else(|| unavailable("the schema answer carries no schemaText"))?;
        Ok(Some(text.to_owned()))
    }

    /// The resource kinds the engine's schema holds.
    pub fn kinds(&self) -> Result<BTreeSet<String>, GrantError> {
        let text = self.schema_text()?.unwrap_or_default();
        Ok(text
            .lines()
            .filter_map(|line| line.strip_prefix("definition "))
            .filter_map(|rest| rest.split([' ', '{']).next())
            .filter(|name| RESOURCE_SUBJECTS.contains(name) || !FIXED.contains(name))
            .map(|name| name.replacen('/', ".", 1))
            .collect())
    }

    /// Write the schema for `kinds`, keeping as they stand the definitions
    /// of every app kind the engine holds and this engine was not given, so
    /// a writer that knows only Lys's own model never removes an app's kinds.
    fn write_schema(&self, kinds: &BTreeSet<String>) -> Result<(), GrantError> {
        let schema_guard = SCHEMA_WRITE
            .lock()
            .map_err(|error| unavailable(format!("schema writer unavailable: {error}")))?;
        for kind in kinds.iter().filter(|kind| !kind.contains('.')) {
            engine_name("resource kind", kind)?;
        }
        let known = self
            .app_kinds
            .lock()
            .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))?
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let text = self.schema_text()?.unwrap_or_default();
        let mut schema = self.schema(kinds)?;
        for (kind, definition) in app_definitions(&text) {
            if !known.contains(&kind) {
                schema.push('\n');
                schema.push_str(&definition);
                schema.push('\n');
            }
        }
        let (status, body) = self.call("/v1/schema/write", &json!({"schema": schema}))?;
        if status == 200 {
            drop(schema_guard);
            return Ok(());
        }
        Err(unavailable(format!("the schema was refused: {body}")))
    }

    fn read_of(&self, filter: &Value) -> Result<Vec<Relationship>, GrantError> {
        let request =
            json!({"consistency": {"fullyConsistent": true}, "relationshipFilter": filter});
        let (status, body) = self.call("/v1/relationships/read", &request)?;
        if status != 200 {
            return Err(unavailable(format!(
                "the relationships could not be read: {body}"
            )));
        }
        body.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<Value>(line)
                    .ok()
                    .as_ref()
                    .and_then(|row| row.pointer("/result/relationship"))
                    .and_then(relationship_of)
                    .ok_or_else(|| {
                        unavailable(format!("a relationship could not be read from {line}"))
                    })
            })
            .collect()
    }

    fn marker(&self, revision: u64) -> Value {
        json!({
            "resource": {"objectType": "lys_mirror", "objectId": self.mirror},
            "relation": "revision",
            "subject": {"object": {"objectType": "lys_revision", "objectId": revision.to_string()}},
        })
    }

    /// Whether the engine gives `subject` the permission `action` on
    /// `resource` at `now`, read at full consistency.
    pub fn check(
        &self,
        resource: &Resource,
        action: &Action,
        subject: IdentityId,
        now: u64,
    ) -> Result<bool, GrantError> {
        let carried = resource.kind().contains('.')
            || self
                .relations
                .values()
                .any(|actions| actions.contains(action.as_str()));
        if !carried || !self.kinds()?.contains(resource.kind()) {
            return Ok(false);
        }
        let request = json!({
            "consistency": {"fullyConsistent": true},
            "resource": object_json(&ObjectRef::resource(resource)),
            "permission": engine_ident_on(resource.kind(), action.as_str()),
            "subject": {"object": object_json(&ObjectRef::identity(subject))},
            "context": {"now": now},
        });
        let (status, body) = self.call("/v1/permissions/check", &request)?;
        if status != 200 {
            return Err(unavailable(format!("the check was refused: {body}")));
        }
        Ok(body.contains("PERMISSIONSHIP_HAS_PERMISSION"))
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
        let held = self.read_of(&json!({
            "resourceType": "lys_mirror",
            "optionalResourceId": self.mirror,
        }))?;
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
        Ok(revisions.into_iter().max().unwrap_or(0))
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
        let (status, body) = self.call("/v1/relationships/write", &request)?;
        if status == 200 {
            return Ok(());
        }
        if body.contains("PRECONDITION") {
            return Err(unavailable(format!(
                "a write at revision {revision} does not follow the revision the permission engine holds"
            )));
        }
        Err(unavailable(format!("the write was refused: {body}")))
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        let kinds = self.kinds()?;
        let mut held = BTreeSet::new();
        for kind in kinds.iter().map(String::as_str).chain(["grant"]) {
            held.extend(self.read_of(&json!({"resourceType": definition(kind)}))?);
        }
        Ok(held)
    }
}

/// The permission engine the service runs the grants on.
#[derive(Debug)]
pub enum Relationships {
    /// Relationships held in this process, gone when it stops.
    Memory(MemoryRelationships),
    /// Relationships held in `SpiceDB`.
    SpiceDb(SpiceDb),
}

impl RelationshipStore for Relationships {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        match self {
            Self::Memory(held) => held.admit_resource(resource),
            Self::SpiceDb(engine) => engine.admit_resource(resource),
        }
    }

    fn revision(&self) -> Result<u64, GrantError> {
        match self {
            Self::Memory(held) => held.revision(),
            Self::SpiceDb(engine) => engine.revision(),
        }
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        match self {
            Self::Memory(held) => held.write(revision, touch, delete),
            Self::SpiceDb(engine) => engine.write(revision, touch, delete),
        }
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        match self {
            Self::Memory(held) => held.read(),
            Self::SpiceDb(engine) => engine.read(),
        }
    }
}

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
