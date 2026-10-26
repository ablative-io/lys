//! Permission schema construction and publication preserve unowned app kinds.
use super::names::{FIXED, RESOURCE_SUBJECTS, engine_ident};
use super::{MIRROR_SCHEMA, SCHEMA_WRITE, SpiceDb, engine_name, unavailable};
use crate::spicedb_apps::{app_definition, app_definitions};
use lys_identity::grants::{GrantError, KindModel, SCHEMA};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

impl SpiceDb {
    /// The schema for the resource kinds `kinds`.
    ///
    /// # Errors
    /// Refuses a poisoned approved app kinds lock.
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
            client: Arc::clone(&self.client),
            post_json: self.post_json,
            tokens: Arc::clone(&self.tokens),
            confirmed: Arc::clone(&self.confirmed),
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
    ///
    /// # Errors
    /// Refuses poisoned app kinds or schema writer locks, invalid resource
    /// kind names, or permission engine schema reads and writes that fail.
    pub fn set_app_kinds(&self, kinds: &BTreeMap<String, KindModel>) -> Result<(), GrantError> {
        *self
            .app_kinds
            .lock()
            .map_err(|error| unavailable(format!("app kinds unavailable: {error}")))? =
            kinds.clone();
        self.write_schema(&self.kinds()?)
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
    ///
    /// # Errors
    /// Refuses permission engine schema transport failures or a refused schema read.
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
    pub(super) fn write_schema(&self, kinds: &BTreeSet<String>) -> Result<(), GrantError> {
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
}
