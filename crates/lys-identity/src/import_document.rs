//! A bulk import is an ordered set of named, content-addressed operations.
//!
//! Parsing grants no authority. The service must authenticate the service
//! account and check its current grants for every entry, including retries.
//! An operation depends on the account, entry kind, name and complete body,
//! never its array position or JSON object member order. Names are references
//! within this document; they are not directory identity identifiers.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::OperationId;

/// Which existing request contract an entry uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An app registration, including its schema.
    App,
    /// An agent registered by display name.
    Agent,
    /// A root grant.
    Root,
    /// A delegation from a named source grant.
    Delegation,
}

impl Kind {
    /// The document section holding this entry kind.
    pub fn section(self) -> &'static str {
        match self {
            Self::App => "apps",
            Self::Agent => "agents",
            Self::Root => "root_grants",
            Self::Delegation => "delegations",
        }
    }
}

/// One planned operation; its body excludes the document-only entry name.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The request contract.
    pub kind: Kind,
    /// A human-readable name, unique within its section.
    pub name: String,
    /// The stable operation id, before reference resolution.
    pub operation: OperationId,
    /// The original request fields, with the derived operation id.
    pub body: Value,
}

/// A refusal names the entry without copying its payload into a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("import_invalid: {entry}: {reason}")]
pub struct Invalid {
    /// Section and entry name, or document when no entry was readable.
    pub entry: String,
    /// Why the shape was refused; never payload bytes.
    pub reason: &'static str,
}

fn invalid(entry: impl Into<String>, reason: &'static str) -> Invalid {
    Invalid {
        entry: entry.into(),
        reason,
    }
}

/// The entire document is parsed before the first mutation is attempted.
pub fn parse(bytes: &[u8], account: &str) -> Result<Vec<Entry>, Invalid> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("document", "expected a JSON object"))?;
    let object = document
        .as_object()
        .ok_or_else(|| invalid("document", "expected a JSON object"))?;
    if object
        .keys()
        .any(|key| !["apps", "agents", "root_grants", "delegations"].contains(&key.as_str()))
    {
        return Err(invalid("document", "unknown section"));
    }
    let mut entries = Vec::new();
    let mut grant_names = BTreeSet::new();
    for kind in [Kind::App, Kind::Agent, Kind::Root, Kind::Delegation] {
        let Some(value) = object.get(kind.section()) else {
            continue;
        };
        let values = value
            .as_array()
            .ok_or_else(|| invalid(kind.section(), "expected an array"))?;
        let mut names = BTreeSet::new();
        for (index, value) in values.iter().enumerate() {
            let label = format!("{}[{index}]", kind.section());
            let mut body = value
                .as_object()
                .cloned()
                .ok_or_else(|| invalid(&label, "expected an object"))?;
            let name_key = match kind {
                Kind::App => "id",
                Kind::Agent => "display_name",
                Kind::Root | Kind::Delegation => "name",
            };
            let name = body
                .get(name_key)
                .and_then(Value::as_str)
                .filter(|name| {
                    !name.is_empty() && name.trim() == *name && !name.chars().any(char::is_control)
                })
                .ok_or_else(|| invalid(&label, "entry name is missing, empty or padded"))?
                .to_owned();
            let label = format!("{}/{}", kind.section(), name);
            if !names.insert(name.clone()) {
                return Err(invalid(label, "entry name is repeated"));
            }
            if matches!(kind, Kind::Root | Kind::Delegation) {
                if !grant_names.insert(name.clone()) {
                    return Err(invalid(label, "grant name is repeated across sections"));
                }
                body.remove("name");
            }
            if body.contains_key("operation") {
                return Err(invalid(label, "operation ids are derived, not supplied"));
            }
            shape(kind, &body).map_err(|reason| invalid(&label, reason))?;
            let operation = operation(account, kind, &name, &Value::Object(body.clone()))?;
            body.insert("operation".into(), Value::String(operation.to_string()));
            entries.push(Entry {
                kind,
                name,
                operation,
                body: Value::Object(body),
            });
        }
    }
    let mut available = BTreeSet::from(["@self".to_owned(), "@owner".to_owned()]);
    for entry in &entries {
        for field in ["holder", "source", "recipient", "responsible"] {
            if let Some(reference) = entry.body.get(field).and_then(Value::as_str)
                && reference.starts_with('@')
                && !available.contains(reference)
            {
                return Err(invalid(
                    format!("{}/{}", entry.kind.section(), entry.name),
                    "reference is unknown or names a later entry",
                ));
            }
        }
        match entry.kind {
            Kind::Agent => {
                available.insert(format!("@agent/{}", entry.name));
            }
            Kind::Root | Kind::Delegation => {
                available.insert(format!("@grant/{}", entry.name));
            }
            Kind::App => {}
        }
    }
    Ok(entries)
}

fn shape(kind: Kind, body: &Map<String, Value>) -> Result<(), &'static str> {
    let required: &[&str] = match kind {
        Kind::App => &["id", "name", "redirects", "schema"],
        Kind::Agent => &["display_name"],
        Kind::Root => &[
            "route", "holder", "resource", "relation", "pass_on", "window",
        ],
        Kind::Delegation => &[
            "route",
            "source",
            "recipient",
            "responsible",
            "resource",
            "relation",
            "pass_on",
            "window",
        ],
    };
    if required.iter().any(|key| !body.contains_key(*key)) {
        return Err("required request member is absent");
    }
    if body.keys().any(|key| {
        !required.contains(&key.as_str()) && !(kind == Kind::App && key == "service_account")
    }) {
        return Err("unknown request member");
    }
    Ok(())
}

fn operation(account: &str, kind: Kind, name: &str, body: &Value) -> Result<OperationId, Invalid> {
    let mut digest = Sha256::new();
    for part in [
        "lys/identity-import/operation/v1",
        account,
        kind.section(),
        name,
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    digest.update(canonical(body)?);
    let bytes = digest.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&bytes[..16]);
    Ok(OperationId::from_bytes(id))
}

// Sort recursively even if another consumer enables serde_json/preserve_order.
fn canonical(value: &Value) -> Result<Vec<u8>, Invalid> {
    match value {
        Value::Object(object) => {
            let ordered = object
                .iter()
                .map(|(key, value)| Ok((key, canonical(value)?)))
                .collect::<Result<BTreeMap<_, _>, Invalid>>()?;
            let mut out = vec![b'{'];
            for (index, (key, value)) in ordered.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend(
                    serde_json::to_vec(key)
                        .map_err(|_| invalid("document", "cannot encode member name"))?,
                );
                out.push(b':');
                out.extend(value);
            }
            out.push(b'}');
            Ok(out)
        }
        Value::Array(array) => {
            let mut out = vec![b'['];
            for (index, value) in array.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend(canonical(value)?);
            }
            out.push(b']');
            Ok(out)
        }
        _ => serde_json::to_vec(value).map_err(|_| invalid("document", "cannot encode value")),
    }
}

#[cfg(test)]
#[path = "import_document_tests.rs"]
mod tests;
