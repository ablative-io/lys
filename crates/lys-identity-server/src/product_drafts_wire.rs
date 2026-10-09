//! The product drafts' wire, lys-pass's own shapes, and their entries in
//! the one `OpenAPI` document with every refusal each route answers.

use lys_openapi::Api;
use serde::{Deserialize, Serialize};

use crate::openapi::route;
use crate::openapi_refusals::{GRANT_ASKED, SIGNED_BODY};
use crate::openapi_table::{A, B, C, GET, POST};

/// The resource and action a held act takes: a kind of the product's own app.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductTarget {
    /// The app's qualified kind, `app.kind`.
    pub kind: String,
    /// The resource's id.
    pub id: String,
    /// The action.
    pub action: String,
}

/// lys-pass's `DraftRequest`: the held act's exact words and their digest.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductDraftBody {
    /// The product's own operation id, kept across retries of the same act.
    pub operation: String,
    /// The held grant the act rests on.
    pub grant: String,
    /// The resource and action.
    pub target: ProductTarget,
    /// SHA-256 of the words' exact UTF-8 bytes, in hex.
    pub request_digest: String,
    /// The exact words of the prepared act.
    pub words: String,
}

/// lys-pass's `DraftCreated`: the draft recorded, never an execution.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProductDraftCreated {
    /// The draft's id.
    pub draft: String,
}

/// The question of `GET /product-drafts`. A connector names its own app
/// and `state=approved`; a signed-in person names neither and reads every
/// draft they may see.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApprovedQuery {
    /// The connector's own app; absent for a person.
    #[serde(default)]
    pub app: Option<String>,
    /// `approved` for a connector; absent for a person.
    #[serde(default)]
    pub state: Option<String>,
    /// The last draft id already read; absent for the first page.
    #[serde(default)]
    pub after: Option<String>,
}

/// lys-pass's `ApprovedDraft`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApprovedDraftView {
    /// The draft's id.
    pub id: String,
    /// The app.
    pub app: String,
    /// The held grant the act rests on.
    pub grant: String,
    /// The resource and action.
    pub target: ProductTarget,
    /// SHA-256 of the words, in lowercase hex.
    pub request_digest: String,
    /// The exact words.
    pub words: String,
}

/// lys-pass's `ApprovedPage`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApprovedPage {
    /// The approved drafts not yet closed, in draft id order.
    pub drafts: Vec<ApprovedDraftView>,
    /// The next page's cursor; absent at the end.
    pub next: Option<String>,
    /// How many approved drafts of the app wait to be closed.
    pub total: usize,
}

/// One approval, as a person reads it: who and when, in seconds.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApprovalView {
    /// The approving person.
    pub by: String,
    /// When, in seconds since the epoch.
    pub at: u64,
}

/// lys-pass's `Execution`: how the product closed the draft.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ExecutionView {
    /// The product applied the act and committed this receipt.
    Executed {
        /// The digest of the words it applied.
        request_digest: String,
        /// The product's committed receipt digest.
        receipt_digest: String,
    },
    /// The product committed a refusal of its own instead.
    RefusedOnExecution {
        /// The digest of the words it refused.
        request_digest: String,
        /// The product's own name for the refusal.
        refusal: String,
        /// The product's words for why.
        reason: String,
    },
}

/// A product draft as a signed-in person reads it: lys-pass's
/// `ApprovedDraft` with where it stands and who decided it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProductDraftView {
    /// The draft's id.
    pub id: String,
    /// The app.
    pub app: String,
    /// The held grant the act rests on.
    pub grant: String,
    /// The resource and action.
    pub target: ProductTarget,
    /// SHA-256 of the words, in lowercase hex.
    pub request_digest: String,
    /// The exact words.
    pub words: String,
    /// `waiting`, `approved`, `refused`, `executed` or `refused_on_execution`.
    pub state: String,
    /// The grant's mode when the draft was recorded: `by_draft` or `by_two`.
    pub mode: String,
    /// The grant's holder.
    pub holder: String,
    /// The person responsible for the holder.
    pub responsible: String,
    /// Each approval, in the order recorded.
    pub approvals: Vec<ApprovalView>,
    /// How the product closed it, once it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionView>,
}

/// Every product draft the signed-in person may see.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProductDraftList {
    /// The drafts, in draft id order.
    pub drafts: Vec<ProductDraftView>,
    /// The next page's cursor; absent at the end.
    pub next: Option<String>,
    /// How many the person may see.
    pub total: usize,
}

/// One person's approval of a product draft, naming the exact words decided.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductApproveBody {
    /// The approval's operation id.
    pub operation: String,
    /// SHA-256 of the words the person saw; another digest is refused.
    pub request_digest: String,
}

/// One person's refusal of a product draft, with the reason.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductRefuseBody {
    /// The refusal's operation id.
    pub operation: String,
    /// SHA-256 of the words the person saw; another digest is refused.
    pub request_digest: String,
    /// Why it is refused.
    pub reason: String,
}

/// A recorded decision, and where the draft stands after it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProductDecision {
    /// The draft.
    pub draft: String,
    /// The decision's operation.
    pub operation: String,
    /// `waiting`, `approved` or `refused`.
    pub state: String,
    /// How many distinct people have approved it.
    pub approvals: usize,
    /// How many its mode needs: one by draft, two by two.
    pub needed: usize,
    /// The decision's log index.
    pub index: u64,
    /// The log's size once it was recorded.
    pub tree_size: u64,
    /// The decision's leaf hash, in hex.
    pub leaf_hash: String,
}

/// The product's receipt that it executed the draft.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutedBody {
    /// The product's committed receipt digest, SHA-256 in hex.
    pub receipt_digest: String,
}

/// The product's refusal of the draft when it came to execute it.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RefusedOnExecutionBody {
    /// The product's own name for the refusal.
    pub refusal: String,
    /// The product's words for why.
    pub reason: String,
}

/// A recorded close.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProductClosed {
    /// The draft.
    pub draft: String,
    /// `executed` or `refused_on_execution`.
    pub state: String,
    /// The close's log index.
    pub index: u64,
    /// The log's size once it was recorded.
    pub tree_size: u64,
    /// The close's leaf hash, in hex.
    pub leaf_hash: String,
}

const CREATE: &[&str] = &[
    "NotAdmitted",
    "NoPerson",
    "credential_refused",
    "GrantIdMalformed",
    "OperationReused",
    "product_draft_grant_refused",
    "product_draft_digest_mismatch",
];
const READ: &[&str] = &[
    "NotAdmitted",
    "NoPerson",
    "RequestMalformed",
    "credential_refused",
    "product_draft_not_your_app",
];
const DECIDE: &[&str] = &[
    "AgentSignatureRefused",
    "NotAdmitted",
    "NoPerson",
    "DraftNotFound",
    "DraftNotPending",
    "DraftHashMismatch",
    "OperationReused",
    "IdentifierMalformed",
    "product_draft_approver_refused",
];
const CLOSE: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "credential_refused",
    "DraftNotFound",
    "IdentifierMalformed",
    "product_draft_not_your_app",
    "product_draft_closed",
    "product_draft_not_approved",
];

/// The product draft routes, each with the types it takes and answers.
pub(crate) fn typed(api: &mut Api) {
    let decision = api.schema::<ProductDecision>();
    let closed = api.schema::<ProductClosed>();
    let routes = [
        route(
            (
                POST,
                "/product-drafts",
                "Record a product's held act as a draft",
            ),
            A,
            Some(api.schema::<ProductDraftBody>()),
            Some(api.schema::<ProductDraftCreated>()),
            &[GRANT_ASKED, CREATE],
        ),
        route(
            (
                GET,
                "/product-drafts",
                "The drafts a person may see; an app's approved drafts, for its connector",
            ),
            A,
            Some(api.schema::<ApprovedQuery>()),
            Some(api.schema::<ProductDraftList>()),
            &[READ],
        ),
        route(
            (
                POST,
                "/product-drafts/{id}/approve",
                "Approve a product draft",
            ),
            C,
            Some(api.schema::<ProductApproveBody>()),
            Some(decision.clone()),
            &[SIGNED_BODY, DECIDE],
        ),
        route(
            (
                POST,
                "/product-drafts/{id}/refuse",
                "Refuse a product draft",
            ),
            C,
            Some(api.schema::<ProductRefuseBody>()),
            Some(decision),
            &[SIGNED_BODY, DECIDE],
        ),
        route(
            (
                POST,
                "/product-drafts/{id}/executed",
                "Close an approved draft as executed",
            ),
            B,
            Some(api.schema::<ExecutedBody>()),
            Some(closed.clone()),
            &[CLOSE],
        ),
        route(
            (
                POST,
                "/product-drafts/{id}/refused-on-execution",
                "Close an approved draft as refused on execution",
            ),
            B,
            Some(api.schema::<RefusedOnExecutionBody>()),
            Some(closed),
            &[CLOSE],
        ),
    ];
    for route in routes {
        api.route(route);
    }
}
