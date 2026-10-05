//! Reporting targets and accountable people remain separate in every answer.

use lys_identity::IdentityId;
use serde::{Deserialize, Serialize};

use crate::directory_views::DirectoryReceiptView;
use crate::error::ServerError;

/// The identities that can receive a reporting edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportingKind {
    /// A person at the end of an accountability chain.
    Person,
    /// An agent that reports onward.
    Agent,
}

/// The exact edge an operation recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReportingEdge {
    /// The chosen identity.
    pub id: String,
    /// Whether the target is a person or agent.
    pub kind: ReportingKind,
}

/// An immediate reporting target as it is currently displayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReportingTarget {
    /// The chosen identity.
    pub id: String,
    /// Whether the target is a person or agent.
    pub kind: ReportingKind,
    /// The target's current display name.
    pub display_name: String,
}

/// The active person reached by the reporting chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccountablePerson {
    /// The accountable person's identity.
    pub id: String,
    /// Their current display name.
    pub display_name: String,
}

/// An inactive identity interrupting the reporting chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReportingGap {
    /// The identity at the gap.
    pub identity: String,
    /// Its current lifecycle state.
    pub state: String,
}

/// The change in accountable person proved by the reporting event.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResponsibilityChanged {
    /// The receipt of the atomic event.
    pub receipt: DirectoryReceiptView,
    /// The earlier accountable person.
    pub from: String,
    /// The resulting accountable person.
    pub to: String,
}

/// The original answer to one reporting-edge operation.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReportsToChanged {
    /// The chosen edge.
    pub reports_to: ReportingEdge,
    /// The resulting accountable person.
    pub responsible: String,
    /// The receipt of the reporting change.
    pub receipt: DirectoryReceiptView,
    /// The responsibility transition, absent when the person stayed the same.
    pub responsibility: Option<ResponsibilityChanged>,
}

pub(crate) fn edge(identity: IdentityId) -> Result<ReportingEdge, ServerError> {
    let kind = match identity {
        IdentityId::Person(_) => ReportingKind::Person,
        IdentityId::Agent(_) => ReportingKind::Agent,
        IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => {
            return Err(lys_identity::IdentityError::AnswersToUnknown {
                identity: identity.to_string(),
            }
            .into());
        }
    };
    Ok(ReportingEdge {
        id: identity.to_string(),
        kind,
    })
}
