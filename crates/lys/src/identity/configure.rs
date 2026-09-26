//! `lys identity configure`: idempotent reconciliation of the two managed
//! OIDC clients and their themes.
//!
//! # Invariants
//!
//! - Exactly two clients are managed: platform and Cambium. The built-in
//!   `rauthy` client that Rauthy's own migration inserts is never created,
//!   changed or deleted, and configure deletes nothing at all.
//! - Every reconciliation carries a stable operation identifier,
//!   `lys-identity/<kind>/<client id>/<16 hex of SHA-256 over the desired
//!   state>`: the same desired state yields the same identifier on every
//!   run, and a changed one a new identifier.
//! - Desired state is compared with what Rauthy holds before anything is
//!   written, so a second run writes nothing and reports `unchanged`.
//! - Every write is confirmed by reading back; a client or theme that does
//!   not read back as declared is `reconcile_mismatch`. A create whose
//!   answer was lost is resolved by reading back and never sends a second
//!   create ([`RauthyApi::create_client`]).
//! - A theme is written only if its six contrast pairs meet their WCAG tiers,
//!   and they are measured again on what Rauthy returns.

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::config::{ClientSpec, LoadedConfig};
use super::credentials::{API_KEY_SECRET, Credentials};
use super::error::IdentityError;
use super::prepare::SECRETS_DIR;
use super::rauthy::{Delivery, NewClient, RauthyApi, Theme};
use super::themes::{self, DarkMapping, Measure};
use crate::commands::error::CliResult;
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;

/// One reconciliation and what it did.
#[derive(Debug, Serialize)]
pub struct Reconciled {
    /// Stable identifier over the desired state.
    pub operation_id: String,
    /// `client/<id>` or `theme/<id>`.
    pub resource: String,
    /// `created`, `created_resolved_by_read_back`, `updated` or `unchanged`.
    pub outcome: &'static str,
}

/// `lys identity configure --config <file> --themes <mapping>`.
pub fn run(config: &Path, themes_path: &Path, json: bool) -> CliResult<()> {
    let loaded = LoadedConfig::load(config)?;
    let mapping = themes::load(themes_path)?;
    let secrets = loaded.private_dir.join(SECRETS_DIR);
    let credentials = Credentials::load_or_generate(&secrets, false)?;
    let api = RauthyApi::new(
        loaded.admin.clone(),
        credentials.get(API_KEY_SECRET, &secrets)?,
    );
    let before = api.list_clients()?;
    let mut operations = Vec::new();
    let mut exports = Vec::new();
    let mut contrast = Vec::new();
    for (key, spec) in loaded.config.clients.managed() {
        operations.push(reconcile_client(&api, spec)?);
        let entry = mapping
            .for_client(key)
            .ok_or_else(|| IdentityError::ThemeInvalid {
                path: themes_path.to_path_buf(),
                reason: format!("client {key} is not mapped"),
            })?;
        let (operation, theme, measures) = reconcile_theme(&api, &spec.id, &entry.dark)?;
        operations.push(operation);
        for measure in measures {
            contrast.push((spec.id.clone(), measure));
        }
        exports.push(theme);
    }
    let after = api.list_clients()?;

    let mut emit = Emitter::new(json);
    for operation in &operations {
        emit.note(&format!(
            "{}: {} ({})",
            operation.resource, operation.outcome, operation.operation_id
        ));
    }
    for (client, measure) in &contrast {
        emit.note(&format!(
            "contrast {client} {}: {:.2} (tier {})",
            measure.pair,
            measure.ratio,
            measure.tier.label()
        ));
    }
    let managed = loaded
        .config
        .clients
        .managed()
        .map(|(_, spec)| spec.id.clone());
    let unmanaged: Vec<String> = after
        .iter()
        .filter_map(|client| client.get("id").and_then(Value::as_str))
        .filter(|id| !managed.iter().any(|managed_id| managed_id == id))
        .map(str::to_string)
        .collect();
    emit.field(
        "unmanaged clients",
        "unmanaged_clients",
        unmanaged.join(","),
    );
    if emit.is_json() {
        emit.field("operations", "operations", to_json(&operations)?);
        emit.field("clients_before", "clients_before", to_json(&before)?);
        emit.field("clients", "clients", to_json(&after)?);
        emit.field("themes", "themes", to_json(&exports)?);
        let rows: Vec<Value> = contrast
            .iter()
            .map(|(client, measure)| {
                json!({
                    "client": client,
                    "pair": measure.pair,
                    "ratio": format!("{:.2}", measure.ratio),
                    "tier": measure.tier.label(),
                })
            })
            .collect();
        emit.field("contrast", "contrast", Value::Array(rows));
    }
    emit.finish();
    Ok(())
}

/// Brings one managed client to its declared state.
fn reconcile_client(api: &RauthyApi, spec: &ClientSpec) -> Result<Reconciled, IdentityError> {
    let desired = desired_client(spec);
    let operation_id = operation_id("client", &spec.id, &Value::Object(desired.clone()));
    let resource = format!("client/{}", spec.id);
    let mut outcome = "unchanged";
    let current = if let Some(current) = api.find_client(&spec.id)? {
        current
    } else {
        let delivery = api.create_client(&NewClient {
            id: &spec.id,
            name: &spec.name,
            confidential: true,
            redirect_uris: &spec.redirect_uris,
            post_logout_redirect_uris: &spec.post_logout_redirect_uris,
        })?;
        outcome = if delivery == Delivery::ResolvedByReadBack {
            "created_resolved_by_read_back"
        } else {
            "created"
        };
        read_back_client(api, &spec.id, &resource)?
    };
    if difference(&current, &desired).is_some() {
        let mut record = current;
        record.remove("id");
        for (field, value) in &desired {
            record.insert(field.clone(), value.clone());
        }
        // An uncertain answer is settled by the read-back below either way.
        api.update_client(&spec.id, &record)?;
        if outcome == "unchanged" {
            outcome = "updated";
        }
        let back = read_back_client(api, &spec.id, &resource)?;
        if let Some(field) = difference(&back, &desired) {
            return Err(IdentityError::ReconcileMismatch {
                resource,
                reason: format!("{field} does not read back as configured"),
            });
        }
    }
    Ok(Reconciled {
        operation_id,
        resource,
        outcome,
    })
}

fn read_back_client(
    api: &RauthyApi,
    id: &str,
    resource: &str,
) -> Result<Map<String, Value>, IdentityError> {
    api.find_client(id)?
        .ok_or_else(|| IdentityError::ReconcileMismatch {
            resource: resource.to_string(),
            reason: "the client does not read back".to_string(),
        })
}

/// The fields configure manages on a client; a `null` is an absent value.
fn desired_client(spec: &ClientSpec) -> Map<String, Value> {
    let optional = |list: &[String]| {
        if list.is_empty() {
            Value::Null
        } else {
            json!(list)
        }
    };
    let mut desired = Map::new();
    for (field, value) in [
        ("name", json!(spec.name)),
        ("confidential", Value::Bool(true)),
        ("redirect_uris", json!(spec.redirect_uris)),
        (
            "post_logout_redirect_uris",
            optional(&spec.post_logout_redirect_uris),
        ),
        ("access_token_alg", json!(spec.signing_alg)),
        ("id_token_alg", json!(spec.signing_alg)),
        ("challenges", optional(&spec.challenges)),
    ] {
        desired.insert(field.to_string(), value);
    }
    desired
}

/// The first managed field whose value Rauthy holds differently.
fn difference<'d>(
    current: &Map<String, Value>,
    desired: &'d Map<String, Value>,
) -> Option<&'d str> {
    desired
        .iter()
        .find(|(field, value)| current.get(field.as_str()).unwrap_or(&Value::Null) != *value)
        .map(|(field, _)| field.as_str())
}

/// Brings one client's theme to the declared dark mapping, every gap as
/// Rauthy holds it.
fn reconcile_theme(
    api: &RauthyApi,
    client_id: &str,
    mapping: &DarkMapping,
) -> Result<(Reconciled, Theme, Vec<Measure>), IdentityError> {
    let current = api.read_theme(client_id)?;
    // A client with no theme of its own reads back Rauthy's default, which
    // names the rauthy client; the id written is always this client's.
    let desired = Theme {
        client_id: client_id.to_string(),
        light: current.light.clone(),
        dark: mapping.apply(&current.dark),
        border_radius: current.border_radius.clone(),
    };
    themes::check_contrast(client_id, &desired.dark)?;
    let mapped = json!({
        "dark": {
            "text": mapping.text.hsl,
            "bg": mapping.bg.hsl,
            "bg_high": mapping.bg_high.hsl,
            "accent": mapping.accent.hsl,
        }
    });
    let resource = format!("theme/{client_id}");
    let outcome = if current == desired {
        "unchanged"
    } else {
        // An uncertain answer is settled by the read-back below either way.
        api.write_theme(&desired)?;
        "updated"
    };
    let back = api.read_theme(client_id)?;
    if back != desired {
        return Err(IdentityError::ReconcileMismatch {
            resource,
            reason: "the theme does not read back as written".to_string(),
        });
    }
    let measures = themes::check_contrast(client_id, &back.dark)?;
    let operation = Reconciled {
        operation_id: operation_id("theme", client_id, &mapped),
        resource,
        outcome,
    };
    Ok((operation, back, measures))
}

/// `lys-identity/<kind>/<id>/<16 hex of SHA-256 over the desired state>`.
fn operation_id(kind: &str, id: &str, desired: &Value) -> String {
    let digest = hex_lower(&Sha256::digest(desired.to_string().as_bytes()));
    format!(
        "lys-identity/{kind}/{id}/{}",
        digest.get(..16).unwrap_or(digest.as_str())
    )
}

fn to_json<T: Serialize>(value: &T) -> Result<Value, IdentityError> {
    serde_json::to_value(value).map_err(|error| IdentityError::RauthyResponse {
        operation: "render configure output",
        resource: "output".to_string(),
        reason: error.to_string(),
    })
}
