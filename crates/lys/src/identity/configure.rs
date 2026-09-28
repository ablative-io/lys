//! `lys identity configure`: reconcile exactly the platform and Cambium
//! clients and their themes in Rauthy, idempotently.
//!
//! Every change has a stable operation identifier derived from what it
//! makes true, so a second run over an unchanged configuration reports the
//! same identifiers and changes nothing. Rauthy's own built-in `rauthy`
//! client is never created, changed or deleted, and no other client is
//! touched. A client secret Rauthy generates goes to an owner-only file in
//! the state directory and is never printed.

use std::path::Path;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::config::{Client, ClientRole, DeploymentConfig};
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::prepare::{API_KEY_SECRET, read_secret};
use super::private_files;
use super::rauthy::{NewClient, RauthyApi};
use super::themes::{ThemeMapping, contrast_pairs};
use crate::commands::output::Emitter;

/// The client fields configure owns; every other field keeps Rauthy's value.
pub const MANAGED_FIELDS: [&str; 9] = [
    "name",
    "enabled",
    "confidential",
    "redirect_uris",
    "post_logout_redirect_uris",
    "flows_enabled",
    "access_token_alg",
    "id_token_alg",
    "challenges",
];

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The file configure records its last operations in.
pub const OPERATIONS_FILE: &str = "configure-operations.json";

/// One reconciled resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// `client`, `client_secret` or `theme`.
    pub kind: &'static str,
    /// The client id.
    pub resource: String,
    /// The stable operation identifier.
    pub id: String,
    /// What the run did.
    pub outcome: &'static str,
}

fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// The stable identifier of making `desired` true of `resource`.
pub fn operation_id(kind: &str, resource: &str, desired: &Value) -> String {
    let mut text = String::new();
    canonical(desired, &mut text);
    let digest = Sha256::new()
        .chain_update(kind.as_bytes())
        .chain_update([0])
        .chain_update(resource.as_bytes())
        .chain_update([0])
        .chain_update(text.as_bytes())
        .finalize();
    let mut hex = String::with_capacity(32);
    for byte in digest.iter().take(16) {
        hex.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    hex
}

/// The managed fields `client` should hold, with empty lists as absent the
/// way Rauthy stores them.
pub fn desired_fields(client: &Client) -> Value {
    let optional = |items: &[String]| {
        if items.is_empty() {
            Value::Null
        } else {
            json!(items)
        }
    };
    json!({
        "name": client.name,
        "enabled": true,
        "confidential": true,
        "redirect_uris": client.redirect_uris,
        "post_logout_redirect_uris": optional(&client.post_logout_redirect_uris),
        "flows_enabled": ["authorization_code", "refresh_token"],
        "access_token_alg": client.token_alg,
        "id_token_alg": client.token_alg,
        "challenges": optional(&client.challenges),
    })
}

/// The managed fields as `current` holds them.
pub fn current_fields(current: &Value) -> Value {
    let mut fields = Map::new();
    for key in MANAGED_FIELDS {
        let value = match current.get(key) {
            None => Value::Null,
            Some(Value::Array(items)) if items.is_empty() => Value::Null,
            Some(value) => value.clone(),
        };
        fields.insert(key.to_string(), value);
    }
    Value::Object(fields)
}

fn reconcile_client(api: &RauthyApi, client: &Client) -> IdentityResult<Operation> {
    let desired = desired_fields(client);
    let id = operation_id("client", &client.id, &desired);
    let (current, mut outcome) = if let Some(current) = api.get_client(&client.id)? {
        (current, "unchanged")
    } else {
        let (created, read_back) = api.create_client(&NewClient {
            id: client.id.clone(),
            name: client.name.clone(),
            confidential: true,
            redirect_uris: client.redirect_uris.clone(),
            post_logout_redirect_uris: client.post_logout_redirect_uris.clone(),
        })?;
        (
            created,
            if read_back {
                "created (read back)"
            } else {
                "created"
            },
        )
    };
    if current_fields(&current) != desired {
        let mut body = current.as_object().cloned().unwrap_or_default();
        body.remove("id");
        if let Value::Object(fields) = &desired {
            for (key, value) in fields {
                body.insert(key.clone(), value.clone());
            }
        }
        let stored = api.update_client(&client.id, &Value::Object(body))?;
        if current_fields(&stored) != desired {
            return Err(IdentityError::new(
                ErrorKind::ReadBackMismatch,
                "update client",
                client.id.clone(),
                "the client read back differs from the configured values",
            ));
        }
        if outcome == "unchanged" {
            outcome = "updated";
        }
    }
    Ok(Operation {
        kind: "client",
        resource: client.id.clone(),
        id,
        outcome,
    })
}

fn reconcile_secret(
    api: &RauthyApi,
    state_dir: &Path,
    client: &Client,
) -> IdentityResult<Operation> {
    let secret = api.client_secret(&client.id)?;
    let path = state_dir.join(secret.name());
    let outcome = private_files::write(&path, secret.expose().as_bytes())?;
    Ok(Operation {
        kind: "client_secret",
        resource: client.id.clone(),
        id: operation_id("client_secret", &client.id, &json!(secret.name())),
        outcome: outcome.word(),
    })
}

fn reconcile_theme(
    api: &RauthyApi,
    mapping: &ThemeMapping,
    role: ClientRole,
    client: &Client,
) -> IdentityResult<Operation> {
    let declared = mapping.client(role)?;
    let fields = json!({
        "text": declared.dark.text.hsl,
        "bg": declared.dark.bg.hsl,
        "bg_high": declared.dark.bg_high.hsl,
        "accent": declared.dark.accent.hsl,
    });
    let id = operation_id("theme", &client.id, &fields);
    // A client with no theme of its own is answered with Rauthy's default,
    // which names the built-in client; it is relabelled so the write can only
    // ever reach this client's theme.
    let mut current = api.get_theme(&client.id)?;
    current.client_id.clone_from(&client.id);
    let desired = mapping.apply(role, &current);
    // Contrast is measured on the theme about to be written, so a theme that
    // fails it never reaches Rauthy.
    let pairs = contrast_pairs(&desired.dark)?;
    if let Some(failing) = pairs.iter().find(|pair| !pair.passes()) {
        return Err(IdentityError::new(
            ErrorKind::ThemeInvalid,
            "check contrast",
            client.id.clone(),
            format!(
                "{} measures {:.2} against {:.1}",
                failing.name, failing.ratio, failing.tier
            ),
        ));
    }
    let outcome = if desired == current {
        "unchanged"
    } else {
        api.put_theme(&desired)?;
        "applied"
    };
    Ok(Operation {
        kind: "theme",
        resource: client.id.clone(),
        id,
        outcome,
    })
}

/// Runs `lys identity configure` against the configuration at `config_path`.
pub fn run(config_path: &Path, json: bool) -> IdentityResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let state_dir = config.state_dir();
    let credential = read_secret(&state_dir, API_KEY_SECRET)?;
    let api = RauthyApi::new(&config.issuer.admin_url, Some(credential))?;
    let mapping = ThemeMapping::declared()?;
    let mut operations = Vec::new();
    for (role, client) in config.managed_clients() {
        operations.push(reconcile_client(&api, client)?);
        operations.push(reconcile_secret(&api, &state_dir, client)?);
        operations.push(reconcile_theme(&api, &mapping, role, client)?);
    }
    let record: Vec<Value> = operations
        .iter()
        .map(|op| json!({"kind": op.kind, "resource": op.resource, "operation": op.id, "outcome": op.outcome}))
        .collect();
    let encoded = serde_json::to_vec_pretty(&record).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "record operations",
            OPERATIONS_FILE,
            error.to_string(),
        )
    })?;
    private_files::write(&state_dir.join(OPERATIONS_FILE), &encoded)?;
    let managed = [
        config.clients.platform.id.as_str(),
        config.clients.cambium.id.as_str(),
        "rauthy",
    ];
    let mut emitter = Emitter::new(json);
    let source = &mapping.source;
    emitter.field(
        "theme source",
        "theme_source",
        format!(
            "{}:{}@{} (owner {})",
            source.repository, source.path, source.commit, source.owner
        ),
    );
    for other in api.list_clients()? {
        if let Some(id) = other.get("id").and_then(Value::as_str)
            && !managed.contains(&id)
        {
            emitter.note(&format!("unmanaged client {id}: present, left untouched"));
        }
    }
    for op in &operations {
        emitter.note(&format!(
            "{} {}: {} (operation {})",
            op.kind, op.resource, op.outcome, op.id
        ));
    }
    if emitter.is_json() {
        emitter.field("operations", "operations", Value::Array(record));
    }
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "configure_tests.rs"]
mod tests;
