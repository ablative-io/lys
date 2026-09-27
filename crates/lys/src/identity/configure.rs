//! `lys identity configure`: idempotent reconciliation of the two managed
//! clients, their themes and their secrets.
//!
//! Invariants:
//!
//! - Exactly two clients are managed, platform and Cambium. The built-in
//!   `rauthy` client is never created, changed or deleted; any other client
//!   Rauthy holds refuses the run by name before anything is written, and
//!   nothing is ever deleted.
//! - Every operation has a stable identifier derived from what it reconciles
//!   toward (SHA-256 over a versioned label, the resource and the desired
//!   state), so a repeated run with the same configuration names the same
//!   operations and reports them `unchanged`.
//! - A write whose response is lost is settled by reading the resource back,
//!   never by a blind retry under a new identity: the client id is the
//!   resource's own stable key, so a second client cannot come of it.
//! - A theme is written only after its contrast is measured and passes; the
//!   four mapped dark fields are the only values configure chooses.
//! - The client secret Rauthy issues is kept once, privately; a kept secret is
//!   reused and never fetched again.

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::commands::error::CliResult;
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;
use crate::identity::config::{BUILTIN_CLIENT_ID, ClientConfig, ClientRole, DeploymentConfig};
use crate::identity::credentials::{self, CredentialKind};
use crate::identity::error::{IdentityError, IdentityResult};
use crate::identity::private_files;
use crate::identity::rauthy::{ClientRecord, ClientUpdate, NewClient, RauthyApi, ThemeDocument, WriteResult};
use crate::identity::themes::{self, Measurement, ThemeMapping};

/// The label every operation identifier is derived under.
const OPERATION_LABEL: &str = "lys/identity-configure/v1";

/// What one reconciliation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationOutcome {
    /// The resource did not exist and was created.
    Created,
    /// The resource differed and was brought to the desired state.
    Updated,
    /// The resource already matched; nothing was written.
    Unchanged,
    /// A write's response was lost; reading back showed it applied.
    Recovered,
    /// A secret was fetched and kept for the first time.
    Stored,
    /// A kept secret was reused; nothing was fetched.
    Reused,
}

impl OperationOutcome {
    /// Stable lower-case name for output.
    pub fn as_str(self) -> &'static str {
        match self {
            OperationOutcome::Created => "created",
            OperationOutcome::Updated => "updated",
            OperationOutcome::Unchanged => "unchanged",
            OperationOutcome::Recovered => "recovered",
            OperationOutcome::Stored => "stored",
            OperationOutcome::Reused => "reused",
        }
    }
}

/// One reconciliation.
#[derive(Debug)]
pub struct Operation {
    /// Stable identifier, `op-` and 16 hex digits.
    pub id: String,
    /// `client`, `theme` or `client_secret`.
    pub kind: &'static str,
    /// The client id it concerns.
    pub resource: String,
    /// What happened.
    pub outcome: OperationOutcome,
}

/// What a configure run did.
#[derive(Debug)]
pub struct ConfigureReport {
    /// Every operation, in order.
    pub operations: Vec<Operation>,
    /// Each client's measured theme contrast.
    pub contrast: Vec<(String, Vec<Measurement>)>,
}

/// Run `lys identity configure --config <config> --themes <themes>`.
pub fn run(config_path: &Path, themes_path: &Path, json: bool) -> CliResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let mapping = ThemeMapping::load(themes_path)?;
    let report = configure(&config, &mapping)?;
    let mut out = Emitter::new(json);
    out.field(
        "theme source",
        "theme_source",
        format!(
            "{}@{} {}",
            mapping.source.repository, mapping.source.commit, mapping.source.path
        ),
    );
    if out.is_json() {
        let operations: Vec<serde_json::Value> = report
            .operations
            .iter()
            .map(|operation| {
                serde_json::json!({
                    "id": operation.id,
                    "kind": operation.kind,
                    "resource": operation.resource,
                    "outcome": operation.outcome.as_str(),
                })
            })
            .collect();
        out.field("operations", "operations", serde_json::Value::Array(operations));
    } else {
        for operation in &report.operations {
            out.note(&format!(
                "{:<9} {:<13} {:<12} {}",
                operation.outcome.as_str(),
                operation.kind,
                operation.resource,
                operation.id
            ));
        }
        for (client, measured) in &report.contrast {
            for measurement in measured {
                out.note(&format!(
                    "contrast {client}: {} {:.2}:1 ({}, {:.1}:1)",
                    measurement.pair, measurement.ratio, measurement.tier.name, measurement.tier.minimum
                ));
            }
        }
    }
    out.finish();
    Ok(())
}

/// Reconcile both managed clients, their themes and their secrets.
pub fn configure(config: &DeploymentConfig, mapping: &ThemeMapping) -> IdentityResult<ConfigureReport> {
    let state_dir = config.state_dir();
    let api_key = credentials::load_one(&state_dir, CredentialKind::RauthyApiKey)?;
    let api = RauthyApi::new(&config.rauthy.publish, &api_key);
    let managed = config.managed_clients();
    let unmanaged: Vec<String> = api
        .list_clients()?
        .into_iter()
        .map(|record| record.id)
        .filter(|id| id != BUILTIN_CLIENT_ID && managed.iter().all(|(_, client)| &client.id != id))
        .collect();
    if !unmanaged.is_empty() {
        return Err(IdentityError::UnmanagedClients {
            ids: unmanaged.join(", "),
        });
    }
    let mut report = ConfigureReport {
        operations: Vec::new(),
        contrast: Vec::new(),
    };
    for (role, client) in managed {
        report.operations.push(reconcile_client(&api, client)?);
        let (operation, measured) = reconcile_theme(&api, mapping, role, client)?;
        report.operations.push(operation);
        report.contrast.push((client.id.clone(), measured));
        report
            .operations
            .push(reconcile_secret(&api, &state_dir, client)?);
    }
    Ok(report)
}

/// The full managed state of `client`.
pub fn desired_client(client: &ClientConfig) -> ClientUpdate {
    let strings = |values: &[&str]| -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    };
    ClientUpdate {
        name: client.name.clone(),
        confidential: true,
        redirect_uris: client.redirect_uris.clone(),
        post_logout_redirect_uris: Some(client.post_logout_redirect_uris.clone()),
        enabled: true,
        flows_enabled: strings(&["authorization_code", "refresh_token"]),
        access_token_alg: client.signing_alg.as_str().to_string(),
        id_token_alg: client.signing_alg.as_str().to_string(),
        auth_code_lifetime: 60,
        access_token_lifetime: 1800,
        scopes: strings(&["email", "openid", "profile"]),
        default_scopes: strings(&["openid"]),
        challenges: client.pkce_s256.then(|| strings(&["S256"])),
        force_mfa: false,
    }
}

/// A client record in the shape of an update, for comparison.
pub fn current_client(record: &ClientRecord) -> ClientUpdate {
    ClientUpdate {
        name: record.name.clone().unwrap_or_default(),
        confidential: record.confidential,
        redirect_uris: record.redirect_uris.clone(),
        post_logout_redirect_uris: record.post_logout_redirect_uris.clone(),
        enabled: record.enabled,
        flows_enabled: record.flows_enabled.clone(),
        access_token_alg: record.access_token_alg.clone(),
        id_token_alg: record.id_token_alg.clone(),
        auth_code_lifetime: record.auth_code_lifetime,
        access_token_lifetime: record.access_token_lifetime,
        scopes: record.scopes.clone(),
        default_scopes: record.default_scopes.clone(),
        challenges: record.challenges.clone(),
        force_mfa: record.force_mfa,
    }
}

/// Whether two client states are the same, ignoring the order of the sets
/// Rauthy stores unordered and an empty list against an absent one.
pub fn same_client(first: &ClientUpdate, second: &ClientUpdate) -> bool {
    normalised(first) == normalised(second)
}

fn normalised(update: &ClientUpdate) -> ClientUpdate {
    let sorted = |values: &[String]| {
        let mut values = values.to_vec();
        values.sort();
        values
    };
    let optional = |values: Option<&Vec<String>>| {
        values
            .filter(|values| !values.is_empty())
            .map(|values| sorted(values.as_slice()))
    };
    let mut normal = update.clone();
    normal.redirect_uris = sorted(&update.redirect_uris);
    normal.post_logout_redirect_uris = optional(update.post_logout_redirect_uris.as_ref());
    normal.flows_enabled = sorted(&update.flows_enabled);
    normal.scopes = sorted(&update.scopes);
    normal.default_scopes = sorted(&update.default_scopes);
    normal.challenges = optional(update.challenges.as_ref());
    normal
}

/// The stable identifier of reconciling `resource` of `kind` toward `desired`.
pub fn operation_id(kind: &str, resource: &str, desired: &impl Serialize) -> IdentityResult<String> {
    let body = serde_json::to_string(desired).map_err(|error| IdentityError::RauthyResponseInvalid {
        operation: format!("derive {kind} operation id"),
        resource: resource.to_string(),
        reason: error.to_string(),
    })?;
    let mut hasher = Sha256::new();
    for part in [OPERATION_LABEL, kind, resource, body.as_str()] {
        hasher.update(part.as_bytes());
        hasher.update([0_u8]);
    }
    let digest = hex_lower(&hasher.finalize());
    Ok(format!("op-{}", &digest[..16]))
}

fn reconcile_client(api: &RauthyApi, client: &ClientConfig) -> IdentityResult<Operation> {
    let desired = desired_client(client);
    let id = operation_id("client", &client.id, &desired)?;
    let operation = |outcome| Operation {
        id: id.clone(),
        kind: "client",
        resource: client.id.clone(),
        outcome,
    };
    let (record, mut outcome) = match api.read_client(&client.id)? {
        Some(record) => (record, OperationOutcome::Unchanged),
        None => create_client(api, client)?,
    };
    if !same_client(&current_client(&record), &desired) {
        let recovered = update_client(api, &client.id, &desired)?;
        outcome = match (outcome, recovered) {
            (OperationOutcome::Unchanged, false) => OperationOutcome::Updated,
            (OperationOutcome::Unchanged, true) => OperationOutcome::Recovered,
            (earlier, _) => earlier,
        };
    }
    Ok(operation(outcome))
}

/// Create a client, settling a lost response by reading it back. Returns the
/// client as Rauthy then holds it.
fn create_client(api: &RauthyApi, client: &ClientConfig) -> IdentityResult<(ClientRecord, OperationOutcome)> {
    let request = NewClient {
        id: &client.id,
        name: &client.name,
        confidential: true,
        redirect_uris: &client.redirect_uris,
        post_logout_redirect_uris: &client.post_logout_redirect_uris,
    };
    let mut outcome = OperationOutcome::Created;
    for attempt in 0..2 {
        match api.create_client(&request)? {
            WriteResult::Applied => break,
            WriteResult::Uncertain(reason) => {
                if api.read_client(&client.id)?.is_some() {
                    outcome = OperationOutcome::Recovered;
                    break;
                }
                if attempt == 1 {
                    return Err(uncertain("create client", &client.id, api.address(), reason));
                }
            }
        }
    }
    let record = api.read_client(&client.id)?.ok_or_else(|| {
        uncertain(
            "create client",
            &client.id,
            api.address(),
            "Rauthy acknowledged the client but reading it back found nothing".to_string(),
        )
    })?;
    Ok((record, outcome))
}

/// Update a client. Returns whether a lost response had to be settled by
/// reading the client back.
fn update_client(api: &RauthyApi, id: &str, desired: &ClientUpdate) -> IdentityResult<bool> {
    match api.update_client(id, desired)? {
        WriteResult::Applied => Ok(false),
        WriteResult::Uncertain(reason) => match api.read_client(id)? {
            Some(record) if same_client(&current_client(&record), desired) => Ok(true),
            _ => Err(uncertain("update client", id, api.address(), reason)),
        },
    }
}

fn reconcile_theme(
    api: &RauthyApi,
    mapping: &ThemeMapping,
    role: ClientRole,
    client: &ClientConfig,
) -> IdentityResult<(Operation, Vec<Measurement>)> {
    let current = api.read_theme(&client.id)?;
    let desired = mapping.apply(role, &client.id, &current);
    let measured = themes::check_contrast(&client.id, &desired.dark)?;
    let id = operation_id("theme", &client.id, &desired)?;
    let outcome = if current == desired {
        OperationOutcome::Unchanged
    } else {
        write_theme(api, &desired)?
    };
    let operation = Operation {
        id,
        kind: "theme",
        resource: client.id.clone(),
        outcome,
    };
    Ok((operation, measured))
}

fn write_theme(api: &RauthyApi, desired: &ThemeDocument) -> IdentityResult<OperationOutcome> {
    match api.write_theme(desired)? {
        WriteResult::Applied => Ok(OperationOutcome::Updated),
        WriteResult::Uncertain(reason) => {
            if api.read_theme(&desired.client_id)? == *desired {
                Ok(OperationOutcome::Recovered)
            } else {
                Err(uncertain("write theme", &desired.client_id, api.address(), reason))
            }
        }
    }
}

fn reconcile_secret(api: &RauthyApi, state_dir: &Path, client: &ClientConfig) -> IdentityResult<Operation> {
    let path = credentials::client_secret_path(state_dir, &client.id);
    let resource = format!("client_{}", client.id);
    let outcome = match private_files::read_private(&path, &resource) {
        Ok(_) => OperationOutcome::Reused,
        Err(IdentityError::SecretMissing { .. }) => {
            let secret = api.read_client_secret(&client.id)?;
            match credentials::store_client_secret(state_dir, &client.id, &secret)? {
                private_files::WriteOutcome::Created => OperationOutcome::Stored,
                _ => OperationOutcome::Reused,
            }
        }
        Err(error) => return Err(error),
    };
    Ok(Operation {
        id: operation_id("client_secret", &client.id, &client.id)?,
        kind: "client_secret",
        resource: client.id.clone(),
        outcome,
    })
}

fn uncertain(operation: &str, resource: &str, address: &str, reason: String) -> IdentityError {
    IdentityError::OutcomeUncertain {
        operation: operation.to_string(),
        resource: resource.to_string(),
        address: address.to_string(),
        reason,
    }
}
