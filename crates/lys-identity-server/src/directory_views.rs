//! The directory routes' answers, typed so the `OpenAPI` document says what
//! the handlers write rather than describing an open object.
//!
//! Every answer here was once built by hand with `json!`, which no schema
//! could be derived from. The types carry exactly the members those literals
//! carried, in the same names, so the wire is unchanged and the document is
//! generated from the same source the answer is.

use lys_identity::receipt::Receipt;
use lys_identity::{IdentityId, projection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::hex;
use crate::sessions_api::SessionLogin;

/// The actor a receipt was signed for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReceiptActorView {
    /// The issuer its login was authenticated at.
    pub issuer: String,
    /// The subject that login names.
    pub subject: String,
    /// When it was authenticated, in seconds since the Unix epoch.
    pub authenticated_at: u64,
}

/// Where a receipt's event stands in the directory log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReceiptLogView {
    /// The leaf's index.
    pub index: u64,
    /// The log's size once it was appended.
    pub tree_size: u64,
    /// The log's root at that size, as lowercase hex.
    pub root: String,
    /// The leaf's hash, as lowercase hex.
    pub leaf_hash: String,
}

/// A directory receipt: what a change was recorded as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DirectoryReceiptView {
    /// The event version it was signed under.
    pub version: u64,
    /// The caller's operation id.
    pub operation: String,
    /// The actor that made the change.
    pub actor: ReceiptActorView,
    /// The identity changed.
    pub identity: String,
    /// The change kind's wire code.
    pub change_kind: u64,
    /// SHA-256 over the event body, as lowercase hex.
    pub payload_commitment: String,
    /// The hash the commitment is made with.
    pub payload_commitment_hash: String,
    /// Where it stands in the directory log.
    pub log: ReceiptLogView,
}

/// The receipt of `receipt`, as every directory answer writes it.
pub(crate) fn receipt_view(receipt: &Receipt) -> DirectoryReceiptView {
    let coordinate = receipt.coordinate();
    let actor = receipt.actor();
    DirectoryReceiptView {
        version: receipt.version(),
        operation: receipt.operation().to_string(),
        actor: ReceiptActorView {
            issuer: actor.binding().issuer().to_owned(),
            subject: actor.binding().subject().to_owned(),
            authenticated_at: actor.provenance().authenticated_at(),
        },
        identity: receipt.identity().to_string(),
        change_kind: receipt.change_kind(),
        payload_commitment: hex(&receipt.payload_commitment()),
        payload_commitment_hash: "sha-256".to_owned(),
        log: ReceiptLogView {
            index: coordinate.index,
            tree_size: coordinate.tree_size,
            root: hex(&coordinate.root),
            leaf_hash: hex(&coordinate.leaf_hash),
        },
    }
}

/// The receipt of `receipt` as a value, for the one answer that carries a
/// receipt within a field rather than as its own member: an agent's
/// provenance. It is the same [`DirectoryReceiptView`] every other answer
/// writes, so the two cannot drift.
pub(crate) fn receipt_json(receipt: &Receipt) -> Result<Value, ServerError> {
    serde_json::to_value(receipt_view(receipt)).map_err(|error| ServerError::DirectoryUnavailable {
        reason: format!("the registration receipt could not be written: {error}"),
    })
}

/// A change answered by its receipt alone.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReceiptAnswer {
    /// The receipt the change was recorded under.
    pub receipt: DirectoryReceiptView,
}

/// A person registered.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PersonRegistered {
    /// The person's id.
    pub person: String,
    /// The receipt the registration was recorded under.
    pub receipt: DirectoryReceiptView,
}

/// An agent registered, under the person answering for it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentRegistered {
    /// The agent's id.
    pub agent: String,
    /// The person responsible for it.
    pub responsible: String,
    /// The receipt the registration was recorded under.
    pub receipt: DirectoryReceiptView,
}

/// One identity as the administrator's read answers it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct IdentityRecordView {
    /// Its id.
    pub id: String,
    /// Its display name.
    pub display_name: String,
    /// Its lifecycle state.
    pub state: String,
    /// The person answering for it, null for a person.
    pub responsible: Option<String>,
    /// Every login bound to it.
    pub logins: Vec<SessionLogin>,
    /// The indices of the directory events that made it what it is.
    pub events: Vec<u64>,
}

/// Every identity the directory holds.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct IdentitiesView {
    /// The identities, in the projection's order.
    pub identities: Vec<IdentityRecordView>,
}

/// The record of `id`, as the identity reads answer it.
pub(crate) fn record_view(id: IdentityId, record: &projection::Record) -> IdentityRecordView {
    IdentityRecordView {
        id: id.to_string(),
        display_name: record.profile().display_name().to_owned(),
        state: record.state().to_string(),
        responsible: record.responsible().map(|person| person.to_string()),
        logins: record
            .bindings()
            .iter()
            .map(|binding| SessionLogin {
                issuer: binding.issuer().to_owned(),
                subject: binding.subject().to_owned(),
            })
            .collect(),
        events: record.events().to_vec(),
    }
}

/// The signed-in answer a finished sign-in writes, beside its cookie.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SignedInView {
    /// The login the session was begun for.
    pub signed_in: SessionLogin,
    /// The authority this service speaks for.
    pub authority: String,
}

/// The person a link-audit lookup names.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LinkAuditPerson {
    /// The person the login is bound to.
    pub person: String,
}

/// The service's public event signing key.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ServiceKeyView {
    /// The Ed25519 public key, as lowercase hex.
    pub ed25519: String,
}

/// The directory log's head a receipt was proved against.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CheckpointView {
    /// The log's size.
    pub tree_size: u64,
    /// Its root at that size, as lowercase hex.
    pub root: String,
}

/// One directory receipt with everything that verifies it offline.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReceiptPage {
    /// The receipt.
    pub receipt: DirectoryReceiptView,
    /// The leaf's bytes, as lowercase hex.
    pub message: String,
    /// The log's head the proof is over.
    pub checkpoint: CheckpointView,
    /// The inclusion proof, as lowercase hex.
    pub inclusion_proof: String,
}
