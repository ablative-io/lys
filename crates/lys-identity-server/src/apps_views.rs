//! The apps as the routes answer them. Each view derives its schema for the
//! `OpenAPI` document from its own fields.
//!
//! No view carries a secret or a digest of one. The one answer that carries
//! a client secret is an approval's, [`ClientIssued`], made once for the
//! approving administrator; a registrar's credential is answered the same
//! way, once, to the administrator who made it.

use lys_identity::grants::{Named, SchemaDiff};
use serde::Serialize;
use serde_json::Value;

use crate::apps_error::Strand;
use crate::apps_state::{App, By, Proposed, Standing};

/// A schema change waiting for an administrator.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PendingView {
    /// The operation it was proposed with.
    pub operation: String,
    /// The version it replaces.
    pub replaces: u64,
    /// The schema proposed.
    #[schema(value_type = Object)]
    pub schema: Value,
    /// Who proposed it.
    #[schema(value_type = Object)]
    pub by: By,
    /// When.
    pub at: u64,
}

impl From<&Proposed> for PendingView {
    fn from(proposed: &Proposed) -> Self {
        Self {
            operation: proposed.operation.clone(),
            replaces: proposed.replaces,
            schema: proposed.schema.clone(),
            by: proposed.by.clone(),
            at: proposed.at,
        }
    }
}

/// An app as the routes answer it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AppView {
    /// The app's id, its kinds' prefix.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Where it stands.
    pub state: Standing,
    /// The addresses its sign-in client may send a person back to.
    pub redirects: Vec<String>,
    /// The current schema; the registered one while it waits for approval.
    #[schema(value_type = Object)]
    pub schema: Value,
    /// The current schema version; 0 before approval.
    pub version: u64,
    /// Every version it has had, in order.
    pub versions: Vec<u64>,
    /// The schema change waiting for an administrator, if one is.
    pub pending: Option<PendingView>,
    /// The sign-in client's id, once approved.
    pub client_id: Option<String>,
    /// The service account it acts as, once bound.
    pub service_account: Option<String>,
    /// Who registered it.
    #[schema(value_type = Object)]
    pub registered_by: By,
    /// When.
    pub registered_at: u64,
}

impl From<&App> for AppView {
    fn from(app: &App) -> Self {
        let registered = &app.registered;
        let current = app.current();
        let bound = app
            .approved
            .as_ref()
            .and_then(|approved| approved.binding.as_ref())
            .map(|binding| binding.service_account.clone());
        Self {
            id: registered.app.clone(),
            name: registered.name.clone(),
            state: app.standing(),
            redirects: registered.redirects.clone(),
            schema: current.map_or_else(|| registered.schema.clone(), |held| held.schema.clone()),
            version: current.map_or(0, |held| held.version),
            versions: app.versions.iter().map(|held| held.version).collect(),
            pending: app.pending.as_ref().map(PendingView::from),
            client_id: app
                .approved
                .as_ref()
                .map(|approved| approved.client.client_id.clone()),
            service_account: bound.or_else(|| registered.service_account.clone()),
            registered_by: registered.by.clone(),
            registered_at: registered.at,
        }
    }
}

/// The answer of `GET /apps`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AppsView {
    /// Every app the caller may see, in the order registered.
    pub apps: Vec<AppView>,
}

/// A client created at approval, answered once to the approving
/// administrator. Its `Debug` names the client and never the secret or the
/// credential.
#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct ClientIssued {
    /// The client id.
    pub client_id: String,
    /// The client secret. It is shown here once and kept nowhere: Lys holds
    /// only its SHA-256.
    pub client_secret: String,
    /// The bearer credential the app calls Lys's API with.
    pub credential: String,
}

/// The answer of an approval.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Approval {
    /// The app as it now stands.
    pub app: AppView,
    /// The client created, the one time it is answered; absent when the
    /// same approval is sent again, since the secret is shown only once.
    pub client: Option<ClientIssued>,
}

impl std::fmt::Debug for ClientIssued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientIssued")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

/// A registrar's credential, answered once to the administrator who made
/// it. Its `Debug` names the service account and never the credential.
#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct RegistrarIssued {
    /// The service account made a registrar.
    pub service_account: String,
    /// The bearer credential it registers apps with, shown here once.
    pub credential: Option<String>,
}

impl std::fmt::Debug for RegistrarIssued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistrarIssued")
            .field("service_account", &self.service_account)
            .field("credential_issued", &self.credential.is_some())
            .finish_non_exhaustive()
    }
}

/// A name within one kind, as an answer names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct NamedView {
    /// The kind.
    pub kind: String,
    /// The relation, action or parent.
    pub name: String,
}

fn named(list: &[Named]) -> Vec<NamedView> {
    list.iter()
        .map(|named| NamedView {
            kind: named.kind.clone(),
            name: named.name.clone(),
        })
        .collect()
}

/// What a schema change adds and removes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DiffView {
    /// Kinds added.
    pub kinds_added: Vec<String>,
    /// Kinds removed.
    pub kinds_removed: Vec<String>,
    /// Relations added.
    pub relations_added: Vec<NamedView>,
    /// Relations removed.
    pub relations_removed: Vec<NamedView>,
    /// Actions added.
    pub actions_added: Vec<NamedView>,
    /// Actions removed.
    pub actions_removed: Vec<NamedView>,
    /// Parents added.
    pub parents_added: Vec<NamedView>,
    /// Parents removed.
    pub parents_removed: Vec<NamedView>,
}

impl From<&SchemaDiff> for DiffView {
    fn from(diff: &SchemaDiff) -> Self {
        Self {
            kinds_added: diff.kinds_added.clone(),
            kinds_removed: diff.kinds_removed.clone(),
            relations_added: named(&diff.relations_added),
            relations_removed: named(&diff.relations_removed),
            actions_added: named(&diff.actions_added),
            actions_removed: named(&diff.actions_removed),
            parents_added: named(&diff.parents_added),
            parents_removed: named(&diff.parents_removed),
        }
    }
}

/// The dry run of a schema change: what it would do, and whether it would be taken.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SchemaCheck {
    /// The app.
    pub app: String,
    /// The current version the change would replace.
    pub current: u64,
    /// The version it would become.
    pub next: u64,
    /// What it adds and removes.
    pub diff: DiffView,
    /// Each relation it would strand, with the count of standing grants.
    pub stranded: Vec<Strand>,
    /// Whether the change would be taken as it stands.
    pub applies: bool,
}

/// The answer of a schema change.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SchemaChanged {
    /// The app as it now stands.
    pub app: AppView,
    /// Whether the change took effect now; false when it waits for an administrator.
    pub applied: bool,
    /// What it adds and removes.
    pub diff: DiffView,
}

/// One schema version.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SchemaVersionView {
    /// The app.
    pub app: String,
    /// The version.
    pub version: u64,
    /// The schema.
    #[schema(value_type = Object)]
    pub schema: Value,
}

#[cfg(test)]
#[path = "apps_views_tests.rs"]
mod tests;
