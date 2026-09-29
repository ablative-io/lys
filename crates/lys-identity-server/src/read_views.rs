//! The typed answers of the read routes, as they go on the wire.
//!
//! The routes in `read_api` answer these types and nothing else, so the wire
//! shape is the type: a client deserializes into the same structs the server
//! serializes from.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A sign-in identity: the issuer that authenticates it and the subject it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Login {
    /// The issuer URL, exactly as recorded.
    pub provider: String,
    /// The subject at that issuer, exactly as recorded.
    pub subject: String,
}

/// A person as a view names them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PersonSummary {
    /// The person's enduring id.
    pub id: String,
    /// Their display name.
    pub display_name: String,
    /// Their lifecycle state: registered, active, suspended or retired.
    pub state: String,
}

/// An agent as a list of agents names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentSummary {
    /// The agent's enduring id.
    pub id: String,
    /// Its display name.
    pub display_name: String,
    /// Its lifecycle state: registered, active, suspended or retired.
    pub state: String,
}

/// The answer of `GET /me`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MeView {
    /// The signed-in person.
    pub person: PersonSummary,
    /// The login this session signed in through.
    pub signed_in: Login,
    /// Every login bound to the person, in the order they were bound.
    pub sign_in_identities: Vec<Login>,
    /// The service accounts the person owns and may use, apart from their
    /// sign-in identities: every one of theirs not retired, in the order
    /// created. Empty when the configuration names no service accounts.
    pub service_accounts: Vec<ServiceAccountView>,
}

/// A service account as the views show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServiceAccountView {
    /// The operation id it was created with, which names it.
    pub id: String,
    /// The person who owns it.
    pub owner: String,
    /// Its name.
    pub name: String,
    /// What it is for; empty when its creator said nothing.
    pub description: String,
    /// `active`, or `retired` once it is retired.
    pub state: String,
    /// The login that created it.
    pub created_by: Login,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: u64,
    /// The login that retired it, null while it is active.
    pub retired_by: Option<Login>,
    /// When it was retired, in seconds since the Unix epoch, null while it is active.
    pub retired_at: Option<u64>,
    /// Refused imports, with original names and content-derived operations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub import_refusals: Vec<crate::service_accounts_state::ImportRefused>,
}

/// The answer of `GET /service-accounts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServiceAccountsView {
    /// `personal` for the signed-in person's own, `directory` for the administrator's.
    pub scope: String,
    /// The service accounts the scope shows, in the order created.
    pub service_accounts: Vec<ServiceAccountView>,
}

/// A person with their agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PersonView {
    /// The person's enduring id.
    pub id: String,
    /// Their display name.
    pub display_name: String,
    /// Their lifecycle state.
    pub state: String,
    /// Every agent that answers to them, in identifier order.
    pub agents: Vec<AgentSummary>,
}

/// The answer of `GET /people` and `GET /directory/people`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PeopleView {
    /// `personal` for the signed-in person's own view, `directory` for the administrator's.
    pub scope: String,
    /// The people the scope shows, each with their agents.
    pub people: Vec<PersonView>,
}

/// Where an agent's record came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Provenance {
    /// The login of the person who registered the agent.
    pub registered_by: Login,
    /// The log indices of every event about the agent, oldest first.
    pub events: Vec<u64>,
    /// The receipt of the registering event, as `GET /receipts/{index}`
    /// answers it: a [`crate::directory_views::DirectoryReceiptView`] as a
    /// value, so a client that keeps a receipt whole keeps it whole here.
    #[schema(value_type = Object)]
    pub registration: Option<Value>,
}

/// The answer of `GET /agents/{id}` and `GET /directory/agents/{id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentView {
    /// The agent's enduring id.
    pub id: String,
    /// Its display name.
    pub display_name: String,
    /// The person it answers to.
    pub person: PersonSummary,
    /// Whether that person is retired, so the agent needs a new one.
    pub needs_new_person: bool,
    /// The agent's role, where recorded. The directory records none yet.
    pub role: Option<String>,
    /// The role's version, where recorded. The directory records none yet.
    pub version: Option<String>,
    /// Its lifecycle state.
    pub state: String,
    /// Where its record came from.
    pub provenance: Provenance,
}
