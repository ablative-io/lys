//! The grant routes' answers, typed so the screens' types cannot drift from
//! what the server writes.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::grants::{
    Action, GrantRecord, LastUse, Model, PassOn, Permit, Recorded, Route, Source,
};
use serde::Serialize;

use crate::routes::hex;

/// What a grant lets its holder pass on, as the answer writes it.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PassOnView {
    /// Exercise only.
    UseOnly,
    /// These actions, to these kinds of recipient.
    To {
        /// The actions that may be passed on.
        actions: Vec<String>,
        /// The kinds of recipient, `person` or `agent`.
        recipients: Vec<String>,
    },
}

impl From<&PassOn> for PassOnView {
    fn from(pass_on: &PassOn) -> Self {
        match pass_on {
            PassOn::UseOnly => Self::UseOnly,
            PassOn::To {
                actions,
                recipients,
            } => Self::To {
                actions: names(actions),
                recipients: recipients.iter().map(ToString::to_string).collect(),
            },
        }
    }
}

fn names(actions: &BTreeSet<Action>) -> Vec<String> {
    actions
        .iter()
        .map(|action| action.as_str().to_owned())
        .collect()
}

fn route_name(route: Route) -> &'static str {
    match route {
        Route::Browser => "browser",
        Route::Api => "api",
        Route::Tool => "tool",
    }
}

/// A resource, by kind and id.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceView {
    /// Its kind.
    pub kind: String,
    /// Its id.
    pub id: String,
}

/// When a grant may be exercised.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WindowView {
    /// When it starts, in seconds since the Unix epoch.
    pub starts_at: u64,
    /// When it ends, or null for no end of its own.
    pub ends_at: Option<u64>,
}

/// When a grant was last seen exercised. Not seen says only that no exercise
/// was observed at an enforcement point, never that it was never used.
#[derive(Debug, Clone, Serialize)]
pub struct LastUseView {
    /// Whether an exercise was observed.
    pub seen: bool,
    /// When, for a seen use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<u64>,
    /// How it arrived, for a seen use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<&'static str>,
    /// The use event's index in the grant log, for a seen use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_event: Option<u64>,
}

impl From<LastUse> for LastUseView {
    fn from(last_use: LastUse) -> Self {
        match last_use {
            LastUse::NotSeen => Self {
                seen: false,
                at: None,
                route: None,
                use_event: None,
            },
            LastUse::Seen { at, route, index } => Self {
                seen: true,
                at: Some(at),
                route: Some(route_name(route)),
                use_event: Some(index),
            },
        }
    }
}

/// Why a grant does not stand, as the caller may read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefusedView {
    /// The refusal's name, the same for a refusal withheld from the caller.
    pub refusal: String,
    /// The grant on the chain the refusal names, or null when it names none
    /// or is withheld from the caller.
    pub grant: Option<String>,
    /// The refusal's text, as the caller may read it.
    pub reason: String,
}

/// Whether a grant stands at the service's clock, judged over its whole
/// chain exactly as a check judges it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StandingView {
    /// Whether it stands.
    pub stands: bool,
    /// Why not, present only when it does not stand.
    #[serde(flatten)]
    pub refused: Option<RefusedView>,
}

/// A grant, as a caller who may inspect it reads it.
#[derive(Debug, Clone, Serialize)]
pub struct GrantView {
    /// The grant's id.
    pub id: String,
    /// The identity that issued it.
    pub issuer: String,
    /// The identity that holds it.
    pub holder: String,
    /// The person responsible for the holder.
    pub responsible: String,
    /// What it is on.
    pub resource: ResourceView,
    /// The relation it was requested as.
    pub relation: String,
    /// The actions it lets the holder exercise.
    pub actions: Vec<String>,
    /// What it lets the holder pass on.
    pub pass_on: PassOnView,
    /// The grant it derives from, or null for a root.
    pub source: Option<String>,
    /// When it may be exercised.
    pub window: WindowView,
    /// The model version it was judged under.
    pub model_version: u64,
    /// The operation that authorised it.
    pub operation: String,
    /// Whether it was revoked directly.
    pub revoked: bool,
    /// When it was revoked directly, in seconds since the epoch, null while it stands.
    pub revoked_at: Option<u64>,
    /// The grants' revision once its revocation was committed, null while it stands.
    pub revoked_revision: Option<u64>,
    /// When it was last seen exercised.
    pub last_use: LastUseView,
    /// Whether it stands, judged over its whole chain.
    pub standing: StandingView,
    /// The earliest end on its chain, null when nothing on the chain ends or
    /// the chain does not resolve. Admission refuses a grant ending after
    /// its source, so this is the grant's own end.
    pub effective_ends_at: Option<u64>,
}

impl GrantView {
    /// The view of `record`, with the standing and effective end the service
    /// judged for it.
    #[must_use]
    pub fn new(
        record: &GrantRecord,
        standing: StandingView,
        effective_ends_at: Option<u64>,
    ) -> Self {
        let grant = record.grant();
        let parts = grant.parts();
        Self {
            id: grant.id().to_string(),
            issuer: parts.issuer.to_string(),
            holder: parts.holder.to_string(),
            responsible: parts.responsible.to_string(),
            resource: ResourceView {
                kind: parts.resource.kind().to_owned(),
                id: parts.resource.id().to_owned(),
            },
            relation: parts.relation.as_str().to_owned(),
            actions: names(&parts.actions),
            pass_on: PassOnView::from(&parts.pass_on),
            source: match parts.source {
                Source::Root => None,
                Source::Grant(id) => Some(id.to_string()),
            },
            window: WindowView {
                starts_at: parts.window.starts_at(),
                ends_at: parts.window.ends_at(),
            },
            model_version: parts.model_version,
            operation: parts.operation.to_string(),
            revoked: record.revoked().is_some(),
            revoked_at: record.revoked().map(|revocation| revocation.at),
            revoked_revision: record.revoked().map(|revocation| revocation.index + 1),
            last_use: record.last_use().into(),
            standing,
            effective_ends_at,
        }
    }
}

/// The grants a caller may see, at one revision.
#[derive(Debug, Clone, Serialize)]
pub struct GrantList {
    /// The grants.
    pub grants: Vec<GrantView>,
    /// The grant log's revision they were read at.
    pub revision: u64,
}

/// Where a recorded change stands in the grant log.
#[derive(Debug, Clone, Serialize)]
pub struct LogView {
    /// The leaf's index.
    pub index: u64,
    /// The log's size once it was appended.
    pub tree_size: u64,
    /// The log's root at that size, as lowercase hex.
    pub root: String,
    /// The leaf's hash, as lowercase hex.
    pub leaf_hash: String,
}

/// A recorded change's receipt.
#[derive(Debug, Clone, Serialize)]
pub struct ReceiptView {
    /// The event version it was signed under.
    pub version: u64,
    /// The identity that made the request.
    pub caller: String,
    /// The change kind's wire code.
    pub change_kind: u64,
    /// SHA-256 over the event body, as lowercase hex.
    pub payload_commitment: String,
    /// The hash the commitment is made with.
    pub payload_commitment_hash: &'static str,
    /// The revision a decision must stand at to reflect this change.
    pub revision: u64,
    /// Where it stands in the grant log.
    pub log: LogView,
}

/// A recorded change and its receipt.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedView {
    /// The caller's operation id.
    pub operation: String,
    /// The grant changed.
    pub grant: String,
    /// Its index among the grant events.
    pub index: u64,
    /// Its receipt.
    pub receipt: ReceiptView,
}

impl From<&Recorded> for RecordedView {
    fn from(recorded: &Recorded) -> Self {
        let receipt = &recorded.receipt;
        let coordinate = receipt.coordinate;
        Self {
            operation: receipt.operation.to_string(),
            grant: receipt.grant.to_string(),
            index: recorded.index,
            receipt: ReceiptView {
                version: receipt.version,
                caller: receipt.caller.to_string(),
                change_kind: receipt.change_kind,
                payload_commitment: hex(&receipt.payload_commitment),
                payload_commitment_hash: "sha-256",
                revision: receipt.revision(),
                log: LogView {
                    index: coordinate.index,
                    tree_size: coordinate.tree_size,
                    root: hex(&coordinate.root),
                    leaf_hash: hex(&coordinate.leaf_hash),
                },
            },
        }
    }
}

/// Whether a check's use was recorded. A use that could not be recorded is
/// named, so a missing record is never read as no use.
#[derive(Debug, Clone, Serialize)]
pub struct UseEventView {
    /// Whether the use event was recorded.
    pub recorded: bool,
    /// Its index in the grant log, when recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    /// Why it was not recorded, when it was not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A permitted decision and the authority path it rests on.
#[derive(Debug, Clone, Serialize)]
pub struct PermitView {
    /// Always true: a refusal is answered as a refusal.
    pub permitted: bool,
    /// The grant exercised.
    pub grant: String,
    /// The grant and every ancestor, from the grant to its root.
    pub path: Vec<String>,
    /// The person the root grant is held by.
    pub responsible: String,
    /// The actions the grant carries.
    pub scope: Vec<String>,
    /// The model version the grant was judged under.
    pub model_version: u64,
    /// The revision the decision was made at.
    pub revision: u64,
    /// For a check, whether its use was recorded. Absent for an explanation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_event: Option<UseEventView>,
}

impl From<&Permit> for PermitView {
    fn from(permit: &Permit) -> Self {
        Self {
            permitted: true,
            grant: permit.grant.to_string(),
            path: permit.path.iter().map(ToString::to_string).collect(),
            responsible: permit.root_person.to_string(),
            scope: names(&permit.actions),
            model_version: permit.model_version,
            revision: permit.revision,
            use_event: permit.use_event.as_ref().map(|used| match used {
                Ok(index) => UseEventView {
                    recorded: true,
                    index: Some(*index),
                    reason: None,
                },
                Err(error) => UseEventView {
                    recorded: false,
                    index: None,
                    reason: Some(error.to_string()),
                },
            }),
        }
    }
}

/// One holder on a page of the who-can answer, with its decision.
#[derive(Debug, Clone, Serialize)]
pub struct HolderView {
    /// The holder.
    pub holder: String,
    /// The decision that permits it.
    #[serde(flatten)]
    pub permit: PermitView,
}

/// One page of the who-can answer.
#[derive(Debug, Clone, Serialize)]
pub struct WhoPage {
    /// The permitted holders on this page, in holder order.
    pub holders: Vec<HolderView>,
    /// The revision every decision on the page was made at.
    pub revision: u64,
    /// Whether no permitted holder remains after this page.
    pub complete: bool,
    /// The last holder of this page to continue after, or null when complete.
    pub next: Option<String>,
}

/// The permission model grants are judged against.
#[derive(Debug, Clone, Serialize)]
pub struct ModelView {
    /// The model's version.
    pub version: u64,
    /// Each relation's name, with the actions it carries.
    pub relations: BTreeMap<String, Vec<String>>,
}

impl From<&Model> for ModelView {
    fn from(model: &Model) -> Self {
        Self {
            version: model.version(),
            relations: model
                .relations()
                .map(|(relation, actions)| {
                    (
                        relation.to_string(),
                        actions
                            .iter()
                            .map(|action| action.as_str().to_owned())
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}
