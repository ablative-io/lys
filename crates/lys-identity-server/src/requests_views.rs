//! The typed answers of the access request routes, as they go on the wire.

use serde::Serialize;

use crate::grant_contract::ResourceView;
use crate::read_views::PersonSummary;
use crate::requests_store::Decided;

/// The decision on a request.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DecisionView {
    /// The person who decided.
    pub by: String,
    /// The decider's words.
    pub note: String,
    /// The grant the approval issued, null for a request declined.
    pub grant: Option<String>,
    /// When it was decided, in seconds since the Unix epoch.
    pub decided_at: u64,
}

impl From<&Decided> for DecisionView {
    fn from(decided: &Decided) -> Self {
        Self {
            by: decided.by.clone(),
            note: decided.note.clone(),
            grant: decided.grant.clone(),
            decided_at: decided.decided_at,
        }
    }
}

/// One access request.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestView {
    /// The request's id, which is the operation id it was asked with.
    pub id: String,
    /// The identity that asks, and would hold the access.
    pub asked_by: String,
    /// That identity's display name, null when the directory no longer holds it.
    pub asked_by_name: Option<String>,
    /// The person who answers for the asker.
    pub responsible: PersonSummary,
    /// What the access is asked on.
    pub resource: ResourceView,
    /// The relation asked for.
    pub relation: String,
    /// The actions the model resolves the relation to.
    pub actions: Vec<String>,
    /// When the access would end, in seconds since the Unix epoch, null for no end of its own.
    pub ends_at: Option<u64>,
    /// Why it is asked.
    pub why: String,
    /// When it was asked, in seconds since the Unix epoch.
    pub asked_at: u64,
    /// `waiting`, `approved` or `declined`.
    pub state: &'static str,
    /// The people who could approve it as the grants stand now.
    pub approvers: Vec<PersonSummary>,
    /// The grants the caller holds that the access could be lent from, as the grants stand now.
    pub sources: Vec<String>,
    /// Whether the caller could approve it with no source, by issuing the access directly: the
    /// caller is the root authority and a person asks.
    pub can_issue_root: bool,
    /// Whether the caller may approve or decline it: a person who could give the access, or
    /// the root authority.
    pub can_decide: bool,
    /// The person whose approval is being settled, null when none is. While it is not null
    /// only that person's same approval is taken.
    pub held_by: Option<String>,
    /// The decision, null while it waits.
    pub decision: Option<DecisionView>,
}

/// The answer of `GET /requests`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestList {
    /// Every request the caller may see, in the order asked.
    pub requests: Vec<RequestView>,
    /// Counters and continuation, absent when no query was supplied.
    #[serde(flatten)]
    pub page: Option<crate::list_page::Totals>,
}
