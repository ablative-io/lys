//! Everything the apps refuse, each by the name an app codes against, and
//! the fields at fault each refusal names.
//!
//! A refusal's name is the first word of its message, as every refusal of
//! the service is. The fields at fault are JSON pointers into the request
//! body the refusal answers, each with the count it concerns when it
//! concerns one, so a schema fault names where it is and a stranding change
//! names each relation it would strand and how many grants.

use axum::http::StatusCode;
use lys_identity::grants::SchemaError;
use serde::Serialize;

/// One field a refusal names as at fault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Field {
    /// The JSON pointer of the field within the request body.
    pub at: String,
    /// The count the fault concerns, when it concerns one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}

/// A relation a schema change would strand, and how many standing grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Strand {
    /// The kind the grants are on.
    pub kind: String,
    /// The relation they are held under.
    pub relation: String,
    /// How many standing grants the change would strand.
    pub count: u64,
}

fn strands_words(stranded: &[Strand]) -> String {
    stranded
        .iter()
        .map(|strand| {
            format!(
                "{} on {} ({} standing)",
                strand.relation, strand.kind, strand.count
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Everything the apps refuse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// The app id is not one the permission store takes as a prefix.
    #[error("app_id_invalid: `{id}` is not an app id: {reason}")]
    AppIdInvalid {
        /// The id given.
        id: String,
        /// The rule it broke.
        reason: &'static str,
    },
    /// The schema breaks a rule, at the pointer of the fault.
    #[error("schema_invalid: at `{pointer}`: {reason}")]
    SchemaInvalid {
        /// The JSON pointer of the fault within the schema.
        pointer: String,
        /// The rule it broke.
        reason: String,
    },
    /// An app by that id is already registered.
    #[error("app_exists: an app with the id `{app}` is already registered")]
    AppExists {
        /// The id.
        app: String,
    },
    /// No app by that id is visible to the caller.
    #[error("app_unknown: no app with the id `{app}` is visible to the caller")]
    AppUnknown {
        /// The id.
        app: String,
    },
    /// The app is registered and not approved, so its kinds are judged by nothing.
    #[error(
        "app_not_approved: the app `{app}` is not approved, so nothing is checked on its kinds"
    )]
    AppNotApproved {
        /// The app.
        app: String,
    },
    /// The app is retired: its client is disabled and every check on its kinds is refused.
    #[error("app_retired: the app `{app}` is retired, so nothing is checked on its kinds")]
    AppRetired {
        /// The app.
        app: String,
    },
    /// The app `lys` is Lys itself: it is never retired, and only an
    /// administrator changes its schema.
    #[error("app_is_lys: {reason}")]
    AppIsLys {
        /// What was refused.
        reason: &'static str,
    },
    /// The app's registration or its pending change was already decided.
    #[error("app_decided: the app `{app}` has nothing pending to approve or decline")]
    AppDecided {
        /// The app.
        app: String,
    },
    /// The operation id already names an act on the apps in other words.
    #[error("app_operation_reused: operation `{operation}` already names another act")]
    AppOperationReused {
        /// The operation id.
        operation: String,
    },
    /// No approved app declares the kind.
    #[error("kind_not_registered: no approved app declares the kind `{kind}`")]
    KindNotRegistered {
        /// The kind.
        kind: String,
    },
    /// An app kind does not declare the action asked about.
    #[error("action_not_declared: the kind `{kind}` declares no action `{action}`")]
    ActionNotDeclared {
        /// The kind.
        kind: String,
        /// The action.
        action: String,
    },
    /// A caller acting for one app named a kind of another.
    #[error(
        "not_your_app: the kind `{kind}` belongs to the prefix `{owner}`, and the caller acts for the app `{acting_for}`"
    )]
    NotYourApp {
        /// The kind named.
        kind: String,
        /// The prefix that owns it.
        owner: String,
        /// The app the caller acts for.
        acting_for: String,
    },
    /// A schema change names a version that is not the current one.
    #[error(
        "schema_version_moved: the change replaces version {replaces}, and the current version is {current}"
    )]
    SchemaVersionMoved {
        /// The version the change said it replaces.
        replaces: u64,
        /// The current version.
        current: u64,
    },
    /// A schema change would strand standing grants.
    #[error(
        "schema_change_strands_grants: revoke these standing grants first: {}",
        strands_words(.stranded)
    )]
    SchemaChangeStrandsGrants {
        /// Each relation the change would strand, with its count.
        stranded: Vec<Strand>,
    },
    /// The app has no schema version by that number.
    #[error("schema_version_unknown: the app `{app}` has no schema version {version}")]
    SchemaVersionUnknown {
        /// The app.
        app: String,
        /// The version asked for.
        version: u64,
    },
    /// A schema change already waits for approval.
    #[error(
        "schema_change_pending: a change to the app `{app}`'s schema already waits for an administrator"
    )]
    SchemaChangePending {
        /// The app.
        app: String,
    },
    /// A redirect address is not one a sign-in client takes.
    #[error("redirect_invalid: `{address}` {reason}")]
    RedirectInvalid {
        /// The address.
        address: String,
        /// The rule it broke.
        reason: &'static str,
    },
    /// A resource cannot be placed in the parent named.
    #[error("placement_invalid: {reason}")]
    PlacementInvalid {
        /// Why.
        reason: String,
    },
    /// An app or registrar credential was presented and is not one Lys holds.
    #[error("credential_refused: {reason}")]
    CredentialRefused {
        /// Which check refused it, never the credential.
        reason: &'static str,
    },
    /// No bench is open by that id.
    #[error("bench_unknown: no test bench by that id is open")]
    BenchUnknown,
    /// The apps log cannot be read or written.
    #[error("apps_unavailable: {reason}")]
    AppsUnavailable {
        /// What failed.
        reason: String,
    },
}

impl From<SchemaError> for AppError {
    fn from(error: SchemaError) -> Self {
        match error {
            SchemaError::AppIdInvalid { id, reason } => Self::AppIdInvalid { id, reason },
            SchemaError::Invalid { pointer, reason } => Self::SchemaInvalid { pointer, reason },
        }
    }
}

impl AppError {
    /// The HTTP status the refusal is answered with.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::AppIdInvalid { .. }
            | Self::SchemaInvalid { .. }
            | Self::RedirectInvalid { .. }
            | Self::ActionNotDeclared { .. }
            | Self::PlacementInvalid { .. } => StatusCode::BAD_REQUEST,
            Self::AppUnknown { .. } | Self::SchemaVersionUnknown { .. } | Self::BenchUnknown => {
                StatusCode::NOT_FOUND
            }
            Self::AppNotApproved { .. }
            | Self::AppRetired { .. }
            | Self::AppIsLys { .. }
            | Self::KindNotRegistered { .. }
            | Self::NotYourApp { .. } => StatusCode::FORBIDDEN,
            Self::CredentialRefused { .. } => StatusCode::UNAUTHORIZED,
            Self::AppExists { .. }
            | Self::AppDecided { .. }
            | Self::AppOperationReused { .. }
            | Self::SchemaVersionMoved { .. }
            | Self::SchemaChangeStrandsGrants { .. }
            | Self::SchemaChangePending { .. } => StatusCode::CONFLICT,
            Self::AppsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// The fields at fault, as JSON pointers into the request body.
    pub fn fields(&self) -> Vec<Field> {
        let at = |at: String| Field { at, count: None };
        match self {
            Self::AppIdInvalid { .. } => vec![at("/id".to_owned())],
            Self::SchemaInvalid { pointer, .. } => vec![at(format!("/schema{pointer}"))],
            Self::SchemaVersionMoved { .. } => vec![at("/replaces".to_owned())],
            Self::RedirectInvalid { .. } => vec![at("/redirects".to_owned())],
            Self::SchemaChangeStrandsGrants { stranded } => stranded
                .iter()
                .map(|strand| Field {
                    at: if strand.kind.contains('.') {
                        format!(
                            "/schema/kinds/{}/relations/{}",
                            strand.kind, strand.relation
                        )
                    } else {
                        format!("/schema/relations/{}", strand.relation)
                    },
                    count: Some(strand.count),
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}
