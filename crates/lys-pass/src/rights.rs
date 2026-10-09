//! Rights name resources already reached by the issuer, never a local graph.

use serde::{Deserialize, Serialize};

use crate::Error;

/// How a grant may be exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The product may act immediately.
    Outright,
    /// One authorized approval is required.
    ByDraft,
    /// Two distinct authorized approvals are required.
    ByTwo,
}

/// The signed holder and the person responsible for a non-person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    /// The directory identity id.
    pub id: String,
    /// The directory identity kind.
    pub kind: String,
    /// The responsible person's identity, required for a non-person.
    pub responsible: Option<String>,
}

/// One explicitly reached resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    /// The application's qualified kind.
    pub kind: String,
    /// The exact placed resource id.
    pub id: String,
}

/// A grant's effective actions on an explicitly reached resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Right {
    /// The resource after issuer-side reach expansion.
    pub resource: Resource,
    /// The effective actions after issuer-side role expansion.
    pub actions: Vec<String>,
    /// The required approval mode.
    pub mode: Mode,
    /// The authority receipt a product records with an act.
    pub grant: String,
}

/// The access pass's signed claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// The trusted Lys issuer.
    pub iss: String,
    /// The holder's identity, equal to the holder id.
    pub sub: String,
    /// The one application audience.
    pub aud: String,
    /// The issue instant in Unix seconds.
    pub iat: u64,
    /// The first invalid instant in Unix seconds.
    pub exp: u64,
    /// An optional first valid instant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<u64>,
    /// The holder and responsibility assertion.
    pub holder: Holder,
    /// Effective rights, already expanded by Lys.
    pub rights: Vec<Right>,
    /// The audience whose rights could not fit; no partial offline decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights_truncated: Option<String>,
}

/// A product's exact permission question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// The resource kind.
    pub kind: String,
    /// The resource id.
    pub id: String,
    /// The action requested.
    pub action: String,
}

impl Target {
    /// Validate the boundary without adding a length limit or default.
    pub fn new(kind: &str, id: &str, action: &str) -> Result<Self, Error> {
        if [kind, id, action].iter().any(|value| value.is_empty() || value.chars().any(char::is_control)) {
            return Err(Error::Invalid("permission target is empty or contains controls"));
        }
        Ok(Self { kind: kind.to_owned(), id: id.to_owned(), action: action.to_owned() })
    }
}

/// An offline answer retaining the exact granting receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision<'a> {
    /// An outright grant permits the act.
    Allowed {
        /// The effective grant id.
        grant: &'a str,
    },
    /// A grant exists but the act requires approval.
    Held {
        /// The effective grant id.
        grant: &'a str,
        /// The required approval mode.
        mode: Mode,
    },
    /// No effective grant in this pass permits the act.
    Refused,
}

pub(crate) fn in_prefix(kind: &str, prefix: &str) -> bool {
    kind == prefix || kind.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('.'))
}

pub(crate) fn audience_owns(kind: &str, audience: &str) -> bool {
    kind.split_once('.').map_or(audience == "lys", |(owner, _)| owner == audience)
}
