//! The grant routes' answers, typed so the screens' types cannot drift from
//! what the server writes. A cannot-give answer is also read back by its
//! type, which refuses a reason outside the six by name and defaults none.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::grants::{
    Action, CannotGiveList, CannotGiveReason, CannotGiveSubject, GrantRecord, LastUse, Model,
    PassOn, Permit, Recorded, Route, Source, Unreported, Usage,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::routes::hex;

/// What a grant lets its holder pass on, as the answer writes it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResourceView {
    /// Its kind.
    pub kind: String,
    /// Its id.
    pub id: String,
}

/// When a grant may be exercised.
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
pub struct WindowView {
    /// When it starts, in seconds since the Unix epoch.
    pub starts_at: u64,
    /// When it ends, or null for no end of its own.
    pub ends_at: Option<u64>,
}

/// Permitted exercises whose use events could not be recorded.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UnreportedView {
    /// How many.
    pub count: u64,
    /// When the latest was permitted, in seconds since the Unix epoch.
    pub at: u64,
    /// How the latest arrived.
    pub route: &'static str,
    /// Why its use event was not recorded.
    pub reason: String,
}

/// When a grant was last seen exercised. Not seen says only that no exercise
/// was observed at an enforcement point, never that it was never used.
/// `source` says whether the use reports are whole: `reported` when every
/// permitted exercise this service has seen since it opened has its use
/// event, so `recorded` is the count; `missing` when some have none, named in
/// `unreported`, so `recorded` is not the count and a zero is not no use. A
/// missing report is in no log, so a restart cannot know one from before it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
    /// How many use events the grant log records for the grant.
    pub recorded: u64,
    /// `reported` or `missing`.
    pub source: &'static str,
    /// The exercises with no use event, when the source is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreported: Option<UnreportedView>,
}

impl From<&Usage> for LastUseView {
    fn from(usage: &Usage) -> Self {
        let (seen, at, route, use_event) = match usage.last {
            LastUse::NotSeen => (false, None, None, None),
            LastUse::Seen { at, route, index } => {
                (true, Some(at), Some(route_name(route)), Some(index))
            }
        };
        Self {
            seen,
            at,
            route,
            use_event,
            recorded: usage.recorded,
            source: if usage.unreported.is_some() {
                "missing"
            } else {
                "reported"
            },
            unreported: usage.unreported.as_ref().map(|missing| UnreportedView {
                count: missing.count,
                at: missing.at,
                route: route_name(missing.route),
                reason: missing.reason.clone(),
            }),
        }
    }
}

/// Why a grant does not stand, as the caller may read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct StandingView {
    /// Whether it stands.
    pub stands: bool,
    /// Why not, present only when it does not stand.
    #[serde(flatten)]
    pub refused: Option<RefusedView>,
}

/// A grant, as a caller who may inspect it reads it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
    /// The view of `record`: the exercises of it the log has no report of,
    /// and the standing and effective end the service judged for it.
    #[must_use]
    pub fn new(
        record: &GrantRecord,
        unreported: Option<&Unreported>,
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
            last_use: LastUseView::from(&Usage::of(record, unreported)),
            standing,
            effective_ends_at,
        }
    }
}

/// The grants a caller may see, at one revision.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GrantList {
    /// The grants.
    pub grants: Vec<GrantView>,
    /// The grant log's revision they were read at.
    pub revision: u64,
}

/// Where a recorded change stands in the grant log.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReceiptView {
    /// The event version it was signed under.
    pub version: u64,
    /// The identity that made the request.
    pub caller: String,
    /// The boss whose source grant the responsible person passed on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_holder: Option<String>,
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
                source_holder: None,
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
    /// The redacted failure of the reading used for this permit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<crate::grants::DegradedView>,
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
            degraded: permit.degraded.as_deref().map(crate::grants::DegradedView::from),
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HolderView {
    /// The holder.
    pub holder: String,
    /// The decision that permits it.
    #[serde(flatten)]
    pub permit: PermitView,
}

/// One page of the who-can answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WhoPage {
    /// The permitted holders on this page, in holder order.
    pub holders: Vec<HolderView>,
    /// The revision every decision on the page was made at.
    pub revision: u64,
    /// Whether no permitted holder remains after this page.
    pub complete: bool,
    /// The last holder of this page to continue after, or null when complete.
    pub next: Option<String>,
    /// The reading's degradation, including when the page is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<crate::grants::DegradedView>,
}

/// The permission model grants are judged against.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelView {
    /// Plain sentences for the shipped model’s actions.
    pub action_sentences: BTreeMap<String, String>,
    /// The model's version.
    pub version: u64,
    /// Each relation's name, with the actions it carries.
    pub relations: BTreeMap<String, Vec<String>>,
    /// The acts no agent may be given, so a screen never offers them.
    pub withheld_from_agents: Vec<String>,
}

impl From<&Model> for ModelView {
    fn from(model: &Model) -> Self {
        Self {
            action_sentences: lys_identity::grants::shipped::ACTION_SENTENCES
                .iter()
                .map(|(action, sentence)| ((*action).to_owned(), (*sentence).to_owned()))
                .collect(),
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
            withheld_from_agents: lys_identity::grants::WITHHELD_FROM_AGENTS
                .iter()
                .map(|action| (*action).to_owned())
                .collect(),
        }
    }
}

/// An answer named a cannot-give reason outside the six. It is refused
/// whole, never defaulted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown_cannot_give_reason: `{reason}` is not a cannot-give reason, which is one of sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only or agents_only"
)]
pub struct UnknownCannotGiveReason {
    /// The reason the answer named.
    pub reason: String,
}

/// Why an item cannot be given, using admission's reason names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CannotGiveReasonView(pub CannotGiveReason);

impl utoipa::ToSchema for CannotGiveReasonView {}

impl utoipa::PartialSchema for CannotGiveReasonView {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(CannotGiveReason::ALL.map(CannotGiveReason::name)))
            .build()
            .into()
    }
}

impl Serialize for CannotGiveReasonView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for CannotGiveReasonView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let reason = String::deserialize(deserializer)?;
        CannotGiveReason::from_name(&reason)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom(UnknownCannotGiveReason { reason }))
    }
}

/// What one cannot-give item is about. A grant or service-account item names
/// the caller's own grant; a relation item names the relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "subject", rename_all = "snake_case")]
pub enum CannotGiveSubjectView {
    /// A grant the caller holds.
    Grant {
        /// The caller's grant.
        grant: String,
    },
    /// A relation of the model on the source grant's resource.
    Relation {
        /// The relation's name.
        relation: String,
    },
    /// A service account the caller holds.
    ServiceAccount {
        /// The caller's grant on it.
        grant: String,
    },
    /// The caller's own sign-in identity.
    SignInIdentity,
}

/// One thing the caller cannot give, with its one reason. No item carries a standing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CannotGiveItemView {
    /// What it is.
    #[serde(flatten)]
    pub subject: CannotGiveSubjectView,
    /// The one reason it cannot be given.
    pub reason: CannotGiveReasonView,
    /// Whether it is the grant the form was opened from.
    pub source: bool,
}

/// Everything the caller cannot give the recipient, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CannotGiveAnswer {
    /// The grant the form was opened from.
    pub source: String,
    /// The recipient asked about.
    pub recipient: String,
    /// The items, in order.
    pub items: Vec<CannotGiveItemView>,
}

impl From<&CannotGiveList> for CannotGiveAnswer {
    fn from(list: &CannotGiveList) -> Self {
        Self {
            source: list.source.to_string(),
            recipient: list.recipient.to_string(),
            items: list
                .items
                .iter()
                .map(|item| CannotGiveItemView {
                    subject: match &item.subject {
                        CannotGiveSubject::Grant(grant) => CannotGiveSubjectView::Grant {
                            grant: grant.to_string(),
                        },
                        CannotGiveSubject::Relation(relation) => CannotGiveSubjectView::Relation {
                            relation: relation.as_str().to_owned(),
                        },
                        CannotGiveSubject::ServiceAccount(grant) => {
                            CannotGiveSubjectView::ServiceAccount {
                                grant: grant.to_string(),
                            }
                        }
                        CannotGiveSubject::SignInIdentity => CannotGiveSubjectView::SignInIdentity,
                    },
                    reason: CannotGiveReasonView(item.reason),
                    source: item.source,
                })
                .collect(),
        }
    }
}
