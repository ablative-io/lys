//! A scratch scope of the permission engine: a throwaway namespace in the
//! same `SpiceDB` the service runs its grants on, which the test bench loads
//! a draft into so its question goes through the engine the real check uses.
//!
//! Every name a scope holds, each definition and each caveat, is written
//! under the scope's prefix `lys/b{hex}/`. The first segment is `lys`, which
//! is Lys's own app id: Lys's own kinds are written without a prefix, so no
//! standing definition is ever `lys/…`, and every name under it is scratch.
//! A scoped engine sees only its own names, the prefix taken off, and writes
//! its schema beside every name it does not own; the service's engine sees
//! every name but the scratch ones and keeps them as they stand when it
//! writes its schema, so neither can remove what the other holds.
//!
//! `SpiceDB` holds one schema, which each write replaces whole, so a scope's
//! schema is only written while the service's grants are held (the bench
//! asks under the grants' lock), and never while the service writes its own.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use lys_identity::grants::GrantError;
use serde_json::{Value, json};

use super::{SpiceDb, SpiceDbSettings, unavailable};
use crate::spicedb_http::post_json;

/// The prefix every scratch name is held under.
pub(crate) const SCRATCH: &str = "lys/";

/// The longest a scope's prefix may be: a kind name reaches 105 bytes (an
/// app id of forty, a slash and a kind of sixty-four), and the engine takes
/// a name of at most 128.
const SCOPE_MAX: usize = 22;

/// The scope's prefix for `random`: `lys/b` and sixteen of its characters.
pub(crate) fn scope_for(random: &str) -> String {
    let scope = format!("{SCRATCH}b{}", random.chars().take(16).collect::<String>());
    scope.chars().take(SCOPE_MAX).collect()
}

/// One definition or caveat of a schema text: its name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    /// The definition's or caveat's name.
    pub(crate) name: String,
    /// Its whole text, from its first line to the line closing it.
    pub(crate) text: String,
}

fn named(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("definition ")
        .or_else(|| line.strip_prefix("caveat "))?;
    rest.split([' ', '{', '(']).next()
}

/// Every definition and caveat of the schema text `text`, in order.
pub(crate) fn blocks(text: &str) -> Vec<Block> {
    let mut found = Vec::new();
    let mut open: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        if let Some((name, lines)) = &mut open {
            lines.push(line);
            if line.trim_end() == "}" {
                found.push(Block {
                    name: name.clone(),
                    text: lines.join("\n"),
                });
                open = None;
            }
            continue;
        }
        match named(line) {
            Some(name) if line.trim_end().ends_with('}') => found.push(Block {
                name: name.to_owned(),
                text: line.to_owned(),
            }),
            Some(name) => open = Some((name.to_owned(), vec![line])),
            None => {}
        }
    }
    found
}

fn joined<'a>(blocks: impl Iterator<Item = &'a Block>) -> String {
    blocks
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Whether the name `name` belongs to `scope`: the scope's own names for a
/// scope, and every name but the scratch ones for the service.
fn owns(scope: Option<&str>, name: &str) -> bool {
    match scope {
        Some(scope) => name
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/')),
        None => !name.starts_with(SCRATCH),
    }
}

/// The schema text `text` as `scope` sees it: its own names only, the
/// prefix taken off. The service's view of a text holding no scratch name is
/// the text unchanged.
pub(crate) fn seen(scope: Option<&str>, text: &str) -> String {
    let all = blocks(text);
    if scope.is_none() && all.iter().all(|block| owns(None, &block.name)) {
        return text.to_owned();
    }
    let own = joined(all.iter().filter(|block| owns(scope, &block.name)));
    match scope {
        Some(scope) => own.replace(&format!("{scope}/"), ""),
        None => own,
    }
}

/// The schema to write when `scope` writes `schema` over the text `held`:
/// every name `scope` does not own as it stands, and `schema` under the
/// scope's prefix.
pub(crate) fn composed(scope: Option<&str>, held: &str, schema: &str) -> String {
    let all = blocks(held);
    let others = joined(all.iter().filter(|block| !owns(scope, &block.name)));
    let own = match scope {
        Some(scope) => scoped_schema(scope, schema),
        None => schema.to_owned(),
    };
    [own, others]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn scoped_type(scope: &str, reference: &str) -> String {
    let (subject, caveat) = match reference.split_once(" with ") {
        Some((subject, caveat)) => (subject.trim(), Some(caveat.trim())),
        None => (reference.trim(), None),
    };
    match caveat {
        Some(caveat) => format!("{scope}/{subject} with {scope}/{caveat}"),
        None => format!("{scope}/{subject}"),
    }
}

/// The schema text `schema` with every definition, caveat and subject type
/// it names under `scope`'s prefix. Relations, permissions and caveat
/// expressions name no definition and are kept as they are.
pub(crate) fn scoped_schema(scope: &str, schema: &str) -> String {
    schema
        .lines()
        .map(|line| {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            let body = line.trim_start();
            if let Some(rest) = body.strip_prefix("definition ") {
                return format!("{indent}definition {scope}/{rest}");
            }
            if let Some(rest) = body.strip_prefix("caveat ") {
                return format!("{indent}caveat {scope}/{rest}");
            }
            if let Some((relation, types)) = body
                .strip_prefix("relation ")
                .and_then(|rest| rest.split_once(':'))
            {
                let types: Vec<String> = types
                    .split('|')
                    .map(|reference| scoped_type(scope, reference))
                    .collect();
                return format!("{indent}relation {relation}: {}", types.join(" | "));
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The request `body` with every object type, subject type and caveat it
/// names under `scope`'s prefix.
pub(crate) fn scoped_request(scope: &str, body: &mut Value) {
    match body {
        Value::Object(members) => {
            for (key, value) in members.iter_mut() {
                let names = matches!(
                    key.as_str(),
                    "objectType" | "resourceType" | "subjectType" | "caveatName"
                );
                match value {
                    Value::String(name) if names => *name = format!("{scope}/{name}"),
                    other => scoped_request(scope, other),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scoped_request(scope, item);
            }
        }
        _ => {}
    }
}

/// An answer's body with `scope`'s prefix taken off every name it quotes.
pub(crate) fn unscoped_answer(scope: &str, body: &str) -> String {
    body.replace(&format!("\"{scope}/"), "\"")
}

impl SpiceDb {
    fn raw(&self, path: &str, body: &Value) -> Result<(u16, String), GrantError> {
        let answer =
            post_json(&self.endpoint, path, &self.key, &body.to_string()).map_err(unavailable)?;
        Ok((answer.status, answer.body))
    }

    /// The whole schema text the engine holds, every scope's names in it.
    fn held_text(&self) -> Result<String, GrantError> {
        let (status, body) = self.raw("/v1/schema/read", &json!({}))?;
        match status {
            404 => Ok(String::new()),
            200 => serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| value.get("schemaText")?.as_str().map(str::to_owned))
                .ok_or_else(|| {
                    unavailable(format!("the schema answer carries no schemaText: {body}"))
                }),
            _ => Err(unavailable(format!("the schema could not be read: {body}"))),
        }
    }

    /// Send `body` to `path` as this engine's scope: a schema read answers
    /// only its own names, a schema write keeps every name it does not own,
    /// and every other request names its types under the scope's prefix.
    pub(super) fn call(&self, path: &str, body: &Value) -> Result<(u16, String), GrantError> {
        let scope = self.scope.as_deref();
        match path {
            "/v1/schema/read" => {
                let (status, answer) = self.raw(path, body)?;
                if status != 200 {
                    return Ok((status, answer));
                }
                let text = self.held_text()?;
                Ok((200, json!({"schemaText": seen(scope, &text)}).to_string()))
            }
            "/v1/schema/write" => {
                let schema = body
                    .get("schema")
                    .and_then(Value::as_str)
                    .ok_or_else(|| unavailable("a schema write carries no schema"))?;
                let schema = composed(scope, &self.held_text()?, schema);
                self.raw(path, &json!({"schema": schema}))
            }
            _ => match scope {
                None => self.raw(path, body),
                Some(scope) => {
                    let mut scoped = body.clone();
                    scoped_request(scope, &mut scoped);
                    let (status, answer) = self.raw(path, &scoped)?;
                    Ok((status, unscoped_answer(scope, &answer)))
                }
            },
        }
    }

    /// Remove the scratch scope `scope` from the engine `settings` names:
    /// every relationship of every definition it holds, then its names.
    pub(crate) fn remove_scratch(
        settings: &SpiceDbSettings,
        scope: &str,
    ) -> Result<(), GrantError> {
        let engine = Self {
            endpoint: settings.endpoint.clone(),
            key: Self::key(settings)?,
            mirror: settings.mirror.clone(),
            relations: BTreeMap::new(),
            app_kinds: Mutex::new(BTreeMap::new()),
            scope: Some(scope.to_owned()),
        };
        let own = seen(Some(scope), &engine.held_text()?);
        let definitions = blocks(&own)
            .into_iter()
            .filter(|block| block.text.starts_with("definition "));
        for block in definitions {
            let filter = json!({"relationshipFilter": {"resourceType": block.name}});
            let (status, body) = engine.call("/v1/relationships/delete", &filter)?;
            if status != 200 {
                return Err(unavailable(format!(
                    "the scratch relationships of {} could not be removed: {body}",
                    block.name
                )));
            }
        }
        let (status, body) = engine.call("/v1/schema/write", &json!({"schema": ""}))?;
        if status == 200 {
            return Ok(());
        }
        Err(unavailable(format!(
            "the scratch scope {scope} could not be removed: {body}"
        )))
    }

    /// Remove every scratch scope the engine `settings` names holds,
    /// answering how many were removed: none outlives the process that made it.
    pub(crate) fn clear_scratch(settings: &SpiceDbSettings) -> Result<usize, GrantError> {
        let engine = Self {
            endpoint: settings.endpoint.clone(),
            key: Self::key(settings)?,
            mirror: settings.mirror.clone(),
            relations: BTreeMap::new(),
            app_kinds: Mutex::new(BTreeMap::new()),
            scope: None,
        };
        let scopes: BTreeSet<String> = blocks(&engine.held_text()?)
            .into_iter()
            .filter_map(|block| {
                let rest = block.name.strip_prefix(SCRATCH)?;
                let (scope, _) = rest.split_once('/')?;
                Some(format!("{SCRATCH}{scope}"))
            })
            .collect();
        for scope in &scopes {
            Self::remove_scratch(settings, scope)?;
        }
        Ok(scopes.len())
    }
}

#[cfg(test)]
#[path = "spicedb_scope_tests.rs"]
mod tests;
