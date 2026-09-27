//! The grants' permission engine in `SpiceDB`: the schema the model gives,
//! the relationships the grant log projects, and the check.
//!
//! The relationships are written by the grants' own projection, one grant
//! event at a time in log order, so `SpiceDB` holds a mirror of the grant log.
//! The revision the mirror stands at is itself a relationship, moved in the
//! same write as the relationships it counts and under a precondition that
//! the revision before it is held, so two writers cannot both move it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use lys_identity::IdentityId;
use lys_identity::grants::{
    Action, GrantError, MemoryRelationships, Model, ObjectRef, Relationship, RelationshipStore,
    Resource, SCHEMA,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::spicedb_http::post_json;

/// The line of an environment file that holds the preshared key.
const KEY_LINE: &str = "SPICEDB_GRPC_PRESHARED_KEY=";
/// The definitions every schema holds beside the resource kinds.
const FIXED: [&str; 5] = ["person", "agent", "grant", "lys_revision", "lys_mirror"];
/// The definitions the mirror's revision is kept in.
const MIRROR_SCHEMA: &str = "\ndefinition lys_revision {}\n\ndefinition lys_mirror {\n  relation revision: lys_revision\n}\n";

/// Where the permission engine is and what it is reached with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpiceDbSettings {
    /// The gateway's host and port.
    pub endpoint: String,
    /// The file holding the preshared key, alone or as the
    /// `SPICEDB_GRPC_PRESHARED_KEY=` line of an environment file.
    pub key_file: PathBuf,
    /// The name the mirror's revision is kept under.
    #[serde(default = "default_mirror")]
    pub mirror: String,
}

fn default_mirror() -> String {
    "grants".to_owned()
}

/// The permission engine, reached over its gateway.
pub struct SpiceDb {
    endpoint: String,
    key: String,
    mirror: String,
    relations: BTreeMap<String, BTreeSet<String>>,
}

impl std::fmt::Debug for SpiceDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceDb")
            .field("endpoint", &self.endpoint)
            .field("mirror", &self.mirror)
            .finish_non_exhaustive()
    }
}

fn unavailable(reason: impl Into<String>) -> GrantError {
    GrantError::PermissionEngineUnavailable {
        reason: reason.into(),
    }
}

/// Refuse a name the engine's schema does not take.
fn engine_name(what: &str, name: &str) -> Result<(), GrantError> {
    let bytes = name.as_bytes();
    let inner = |byte: &u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_';
    let fits = (3..=64).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(|byte| *byte != b'_')
        && bytes.iter().all(inner);
    if fits {
        return Ok(());
    }
    Err(unavailable(format!(
        "the {what} {name} is not a name the permission engine takes: three to sixty-four lowercase letters, digits and underscores, starting with a letter"
    )))
}

fn object_json(object: &ObjectRef) -> Value {
    json!({"objectType": object.kind, "objectId": object.id.replace('.', "|")})
}

fn relationship_json(relationship: &Relationship) -> Value {
    let mut subject = json!({"object": object_json(&relationship.subject)});
    if let Some(relation) = &relationship.subject_relation {
        subject["optionalRelation"] = json!(relation);
    }
    let mut value = json!({
        "resource": object_json(&relationship.resource),
        "relation": relationship.relation,
        "subject": subject,
    });
    if let Some(ends_at) = relationship.ends_at {
        value["optionalCaveat"] =
            json!({"caveatName": "unexpired", "context": {"ends_at": ends_at}});
    }
    value
}

fn object_of(value: &Value) -> Option<ObjectRef> {
    Some(ObjectRef {
        kind: value.get("objectType")?.as_str()?.to_owned(),
        id: value.get("objectId")?.as_str()?.replace('|', "."),
    })
}

fn relationship_of(value: &Value) -> Option<Relationship> {
    let subject = value.get("subject")?;
    let ends_at = match value.pointer("/optionalCaveat/context/ends_at") {
        Some(end) => Some(end.as_u64()?),
        None => None,
    };
    Some(Relationship {
        resource: object_of(value.get("resource")?)?,
        relation: value.get("relation")?.as_str()?.to_owned(),
        subject: object_of(subject.get("object")?)?,
        subject_relation: subject
            .get("optionalRelation")
            .and_then(Value::as_str)
            .filter(|relation| !relation.is_empty())
            .map(str::to_owned),
        ends_at,
    })
}

impl SpiceDb {
    /// Reach the engine `settings` names and write the schema `model` gives,
    /// keeping every resource kind the engine already holds.
    pub fn open(settings: &SpiceDbSettings, model: &Model) -> Result<Self, GrantError> {
        let text = std::fs::read_to_string(&settings.key_file).map_err(|error| {
            unavailable(format!(
                "the permission engine's key file {} could not be read: {}",
                settings.key_file.display(),
                error.kind()
            ))
        })?;
        let key = text
            .lines()
            .find_map(|line| line.strip_prefix(KEY_LINE))
            .unwrap_or(&text)
            .trim()
            .to_owned();
        if key.is_empty() {
            return Err(unavailable("the permission engine's key file is empty"));
        }
        engine_name("mirror name", &settings.mirror)?;
        let mut relations = BTreeMap::new();
        for (relation, actions) in model.relations() {
            let relation = relation.to_string();
            engine_name("relation", &relation)?;
            let actions: BTreeSet<String> = actions.iter().map(ToString::to_string).collect();
            for action in &actions {
                engine_name("action", action)?;
            }
            relations.insert(relation, actions);
        }
        let engine = Self {
            endpoint: settings.endpoint.clone(),
            key,
            mirror: settings.mirror.clone(),
            relations,
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

    fn call(&self, path: &str, body: &Value) -> Result<(u16, String), GrantError> {
        let answer =
            post_json(&self.endpoint, path, &self.key, &body.to_string()).map_err(unavailable)?;
        Ok((answer.status, answer.body))
    }

    /// The schema for the resource kinds `kinds`.
    pub fn schema(&self, kinds: &BTreeSet<String>) -> String {
        let mut carried: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (relation, actions) in &self.relations {
            for action in actions {
                carried.entry(action).or_default().push(relation);
            }
        }
        let relations = self
            .relations
            .keys()
            .map(|relation| format!("  relation {relation}: grant#holder\n"));
        let permissions = carried.iter().map(|(action, relations)| {
            format!("  permission {action} = {}\n", relations.join(" + "))
        });
        let body: String = relations.chain(permissions).collect();
        let kinds = kinds
            .iter()
            .map(|kind| format!("\ndefinition {kind} {{\n{body}}}\n"));
        [SCHEMA.to_owned(), MIRROR_SCHEMA.to_owned()]
            .into_iter()
            .chain(kinds)
            .collect()
    }

    /// The resource kinds the engine's schema holds.
    pub fn kinds(&self) -> Result<BTreeSet<String>, GrantError> {
        let (status, body) = self.call("/v1/schema/read", &json!({}))?;
        if status == 404 {
            return Ok(BTreeSet::new());
        }
        if status != 200 {
            return Err(unavailable(format!("the schema could not be read: {body}")));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| unavailable(format!("the schema answer is not JSON: {error}")))?;
        let text = value
            .get("schemaText")
            .and_then(Value::as_str)
            .unwrap_or("");
        Ok(text
            .lines()
            .filter_map(|line| line.strip_prefix("definition "))
            .filter_map(|rest| rest.split([' ', '{']).next())
            .filter(|name| !FIXED.contains(name))
            .map(str::to_owned)
            .collect())
    }

    fn write_schema(&self, kinds: &BTreeSet<String>) -> Result<(), GrantError> {
        for kind in kinds {
            engine_name("resource kind", kind)?;
        }
        let (status, body) =
            self.call("/v1/schema/write", &json!({"schema": self.schema(kinds)}))?;
        if status == 200 {
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
        let carried = self
            .relations
            .values()
            .any(|actions| actions.contains(action.as_str()));
        if !carried || !self.kinds()?.contains(resource.kind()) {
            return Ok(false);
        }
        let request = json!({
            "consistency": {"fullyConsistent": true},
            "resource": object_json(&ObjectRef::resource(resource)),
            "permission": action.as_str(),
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
    fn revision(&self) -> Result<u64, GrantError> {
        let held = self.read_of(&json!({
            "resourceType": "lys_mirror",
            "optionalResourceId": self.mirror,
        }))?;
        held.iter()
            .map(|relationship| relationship.subject.id.parse::<u64>())
            .max_by_key(|revision| revision.clone().unwrap_or(0))
            .unwrap_or(Ok(0))
            .map_err(|error| unavailable(format!("the mirror's revision is not a number: {error}")))
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
            .filter(|kind| !FIXED.contains(&kind.as_str()))
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
            held.extend(self.read_of(&json!({"resourceType": kind}))?);
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
