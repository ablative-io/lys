//! `SpiceDB` over its HTTP gateway, as DIRECTORY-006 built it, kept only for
//! the secrets broker, which reads its permission checks through it until
//! its own card moves it. The identity server never writes a grant's
//! relationship, writes its schema or decides a grant through this module:
//! road step 2 answers every grant question through the `client`, `schema`,
//! `projector` and `check` modules beside it, and a configuration without a
//! gRPC address has no grant engine at all.
//!
//! The settings are the identity server's own `SpiceDB` settings, which name
//! the gRPC address step 2 speaks to beside the gateway's.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use lys_identity::IdentityId;
use lys_identity::grants::{Action, GrantError, Model, ObjectRef, Resource, SCHEMA};
use serde::Deserialize;
use serde_json::{Value, json};

use super::gateway_http::post_json;

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
    /// The name DIRECTORY-006's gateway mirror kept its revision under,
    /// still refused unless the engine takes it as a name; nothing is
    /// mirrored under it now.
    #[serde(default = "default_mirror")]
    pub mirror: String,
    /// The gRPC address the identity server's client speaks to, as
    /// `http://host:port` or `https://host:port`. Every grant and permission
    /// check the server makes asks `SpiceDB` through that client (road step
    /// 2); without it no grant is decided and every grant route is refused
    /// `spicedb_grpc_absent`.
    #[serde(default)]
    pub grpc: Option<String>,
    /// The most updates `SpiceDB` takes in one `WriteRelationships` call, as
    /// the deployment configures it (`SpiceDB`'s
    /// `--write-relationships-max-updates-per-call`, 1000 unless changed).
    #[serde(default = "default_max_updates_per_write")]
    pub max_updates_per_write: u32,
}

fn default_mirror() -> String {
    "grants".to_owned()
}

fn default_max_updates_per_write() -> u32 {
    1000
}

/// The permission engine, reached over its gateway.
pub struct SpiceDb {
    endpoint: String,
    key: String,
    relations: BTreeMap<String, BTreeSet<String>>,
}

impl std::fmt::Debug for SpiceDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceDb")
            .field("endpoint", &self.endpoint)
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
            .ok_or_else(|| unavailable("the schema answer carries no schemaText"))?;
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
