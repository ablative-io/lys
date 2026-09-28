//! The Claude Code launch template (HOME-002 R1): one JSON object, kept in
//! the home by its SHA-256, that names how a session becomes a running
//! Claude Code session. Its schema is `docs/design/home/launch-template.schema.json`
//! and this parser accepts exactly that shape: `harness`, `flags` and the
//! five `slots` (`transcript`, `mcp`, `env`, `secrets`, `instructions`).
//!
//! Nothing in a template is interpreted, expanded or executed here. A slot
//! outside the five is refused by name, a missing slot is refused by name, a
//! readable secret is refused because no broker reader exists yet, and a
//! variable named twice is refused by name. No error carries the
//! instructions text, an environment value or the MCP contents.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::HomeError;
use crate::harness::claude_code::HARNESS;
use crate::record::blocks::Hash;

/// The one way this harness fills the transcript slot.
pub const FILL_RESUME_BY_PATH: &str = "resume-by-path";
/// The five slots, in the schema's order.
pub const SLOTS: [&str; 5] = ["transcript", "mcp", "env", "secrets", "instructions"];
/// The three members of a template.
pub const MEMBERS: [&str; 3] = ["harness", "flags", "slots"];
/// The most characters of a refused value an error repeats.
const VALUE_CUT: usize = 80;

/// One secret named by the variable it fills and the handle that stands for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretRef {
    /// The environment variable.
    pub env: String,
    /// The handle the door swaps for the credential; never the value.
    pub handle: String,
}

/// A launch template as parsed and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    /// The SHA-256 of the template file's bytes exactly as read.
    pub hash: Hash,
    /// The extra Claude Code arguments, in order.
    pub flags: Vec<String>,
    /// The canon file whose examples render first, when one is named.
    pub canon: Option<PathBuf>,
    /// The MCP configuration, written verbatim.
    pub mcp: Map<String, Value>,
    /// Environment variables, by name.
    pub env: BTreeMap<String, String>,
    /// Secrets written as their handles.
    pub use_only: Vec<SecretRef>,
    /// The text appended to the system prompt.
    pub instructions: String,
}

/// Read a template file and parse it, returning the template and the bytes
/// as read, which are what its hash is of and what the home keeps.
pub fn read_template(path: &Path) -> Result<(Template, Vec<u8>), HomeError> {
    let bytes =
        std::fs::read(path).map_err(|e| HomeError::io("reading the launch template", path, e))?;
    let template = parse_template(&bytes)?;
    Ok((template, bytes))
}

/// Parse a template from its bytes, checking every rule the schema states.
pub fn parse_template(bytes: &[u8]) -> Result<Template, HomeError> {
    let hash = Hash::of(bytes);
    let root: Value = serde_json::from_slice(bytes).map_err(|source| HomeError::Json {
        context: "the launch template is not JSON",
        source,
    })?;
    let Value::Object(root) = root else {
        return Err(shape("", "must be a JSON object"));
    };
    for key in root.keys() {
        if !MEMBERS.contains(&key.as_str()) {
            return Err(shape(key, "is not a member of a launch template"));
        }
    }
    let harness = member(&root, "harness")?;
    let flags = member(&root, "flags")?;
    let slots = member(&root, "slots")?;
    let Value::Object(slots) = slots else {
        return Err(shape("slots", "must be an object"));
    };
    for key in slots.keys() {
        if !SLOTS.contains(&key.as_str()) {
            return Err(HomeError::UnknownSlot { slot: key.clone() });
        }
    }
    for slot in SLOTS {
        if !slots.contains_key(slot) {
            return Err(HomeError::MissingSlot {
                slot: slot.to_owned(),
            });
        }
    }
    expect_value("harness", harness, HARNESS)?;
    let flags = string_list("flags", flags)?;
    let canon = transcript(&slots["transcript"])?;
    let Value::Object(mcp) = &slots["mcp"] else {
        return Err(shape("slots.mcp", "must be an object"));
    };
    let env = env_map(&slots["env"])?;
    let (use_only, readable) = secrets(&slots["secrets"])?;
    if let Some(first) = readable.first() {
        return Err(HomeError::SecretReaderUnbuilt {
            env: first.env.clone(),
        });
    }
    let Value::String(instructions) = &slots["instructions"] else {
        return Err(shape("slots.instructions", "must be a string"));
    };
    let mut named: BTreeSet<&str> = env.keys().map(String::as_str).collect();
    for secret in use_only.iter().chain(readable.iter()) {
        if !named.insert(secret.env.as_str()) {
            return Err(HomeError::DuplicateVariable {
                name: secret.env.clone(),
            });
        }
    }
    Ok(Template {
        hash,
        flags,
        canon,
        mcp: mcp.clone(),
        env,
        use_only,
        instructions: instructions.clone(),
    })
}

fn shape(field: &str, reason: &'static str) -> HomeError {
    HomeError::TemplateShape {
        field: field.to_owned(),
        reason,
    }
}

fn member<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, HomeError> {
    object.get(name).ok_or_else(|| shape(name, "is missing"))
}

fn expect_value(field: &str, value: &Value, expected: &str) -> Result<(), HomeError> {
    match value {
        Value::String(s) if s == expected => Ok(()),
        Value::String(s) => Err(HomeError::TemplateValue {
            field: field.to_owned(),
            value: s.chars().take(VALUE_CUT).collect(),
        }),
        _ => Err(shape(field, "must be a string")),
    }
}

fn string_list(field: &str, value: &Value) -> Result<Vec<String>, HomeError> {
    let Value::Array(items) = value else {
        return Err(shape(field, "must be an array of strings"));
    };
    items
        .iter()
        .map(|item| match item {
            Value::String(s) => Ok(s.clone()),
            _ => Err(shape(field, "must be an array of strings")),
        })
        .collect()
}

/// The transcript slot: `fill` must be `resume-by-path`; `canon` is a path or null.
fn transcript(value: &Value) -> Result<Option<PathBuf>, HomeError> {
    let Value::Object(slot) = value else {
        return Err(shape("slots.transcript", "must be an object"));
    };
    for key in slot.keys() {
        if key != "fill" && key != "canon" {
            return Err(shape(
                "slots.transcript",
                "holds a member other than fill and canon",
            ));
        }
    }
    let fill = slot
        .get("fill")
        .ok_or_else(|| shape("slots.transcript.fill", "is missing"))?;
    expect_value("transcript.fill", fill, FILL_RESUME_BY_PATH)?;
    match slot.get("canon") {
        Some(Value::Null) => Ok(None),
        Some(Value::String(path)) => Ok(Some(PathBuf::from(path))),
        Some(_) => Err(shape("slots.transcript.canon", "must be a path or null")),
        None => Err(shape("slots.transcript.canon", "is missing")),
    }
}

fn env_map(value: &Value) -> Result<BTreeMap<String, String>, HomeError> {
    let Value::Object(env) = value else {
        return Err(shape("slots.env", "must be an object of name to string"));
    };
    let mut out = BTreeMap::new();
    for (name, value) in env {
        let Value::String(value) = value else {
            return Err(shape("slots.env", "must be an object of name to string"));
        };
        out.insert(name.clone(), value.clone());
    }
    Ok(out)
}

/// The secrets slot: two lists of `{env, handle}` and the reader text, which
/// is checked for shape and then left where it is; nothing here keeps it.
fn secrets(value: &Value) -> Result<(Vec<SecretRef>, Vec<SecretRef>), HomeError> {
    let Value::Object(slot) = value else {
        return Err(shape("slots.secrets", "must be an object"));
    };
    for key in slot.keys() {
        if key != "use_only" && key != "readable" && key != "reader" {
            return Err(shape(
                "slots.secrets",
                "holds a member other than use_only, readable and reader",
            ));
        }
    }
    let use_only = secret_list(
        "slots.secrets.use_only",
        slot.get("use_only")
            .ok_or_else(|| shape("slots.secrets.use_only", "is missing"))?,
    )?;
    let readable = secret_list(
        "slots.secrets.readable",
        slot.get("readable")
            .ok_or_else(|| shape("slots.secrets.readable", "is missing"))?,
    )?;
    match slot.get("reader") {
        Some(Value::String(_)) => {}
        Some(_) => return Err(shape("slots.secrets.reader", "must be a string")),
        None => return Err(shape("slots.secrets.reader", "is missing")),
    }
    Ok((use_only, readable))
}

fn secret_list(field: &str, value: &Value) -> Result<Vec<SecretRef>, HomeError> {
    let Value::Array(items) = value else {
        return Err(shape(field, "must be an array of {env, handle}"));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Value::Object(entry) = item else {
            return Err(shape(field, "must be an array of {env, handle}"));
        };
        if entry.len() != 2 {
            return Err(shape(field, "must be an array of {env, handle}"));
        }
        let (Some(Value::String(env)), Some(Value::String(handle))) =
            (entry.get("env"), entry.get("handle"))
        else {
            return Err(shape(field, "must be an array of {env, handle}"));
        };
        out.push(SecretRef {
            env: env.clone(),
            handle: handle.clone(),
        });
    }
    Ok(out)
}
