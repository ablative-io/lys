//! The typed answers of the access request routes, as they go on the wire.

use serde::Serialize;

use crate::grant_contract::ResourceView;
use crate::read_views::PersonSummary;
use crate::requests_store::Decided;

/// The decision on a request.
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
    /// The decision, null while it waits.
    pub decision: Option<DecisionView>,
}

/// The answer of `GET /requests`.
#[derive(Debug, Clone, Serialize)]
pub struct RequestList {
    /// Every request the caller may see, in the order asked.
    pub requests: Vec<RequestView>,
}
