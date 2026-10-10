//! The variables an agent and a session keep (AGENTS-001 R2): for each
//! scope a map of lowercase names to JSON values, each with the revision of
//! the patch that set it, its author and an optional expiry, folded from the
//! variables log's leaves. A patch carries the revision it read; a stale one
//! is refused by name, so a focus written before a compaction is never
//! overwritten blind.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::agents_log::Folded;

/// Everything the variables refuse, each by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VariablesError {
    /// The variables are not configured, or their log could not be read or written.
    #[error("variables_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// The patch carried a revision that is not the one held.
    #[error(
        "variables_stale: {scope} is at revision {held}, not {given}: read it again and patch from what you read"
    )]
    Stale {
        /// The scope.
        scope: String,
        /// The revision held.
        held: u64,
        /// The revision the patch carried.
        given: u64,
    },
    /// A name or a value the variables cannot hold.
    #[error("variables_malformed: {reason}")]
    Malformed {
        /// Why.
        reason: String,
    },
}

impl VariablesError {
    /// How the refusal is answered over HTTP.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Stale { .. } => StatusCode::CONFLICT,
            Self::Malformed { .. } => StatusCode::BAD_REQUEST,
        }
    }

    /// The stable refusal name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "variables_unavailable",
            Self::Stale { .. } => "variables_stale",
            Self::Malformed { .. } => "variables_malformed",
        }
    }
}

/// Whose variables: an agent's or a session's.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[schema(as = VariablesScope)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    /// An agent's, kept across its sessions.
    Agent {
        /// The agent.
        id: String,
    },
    /// A session's, kept for that session.
    Session {
        /// The session.
        id: String,
    },
}

impl Scope {
    /// Its name, for refusals and receipts.
    pub fn name(&self) -> String {
        match self {
            Self::Agent { id } => format!("agent {id}"),
            Self::Session { id } => format!("session {id}"),
        }
    }

    /// Its key in the fold: `agent:<id>` or `session:<id>`, a string so the
    /// fold serialises as JSON.
    pub fn key(&self) -> String {
        match self {
            Self::Agent { id } => format!("agent:{id}"),
            Self::Session { id } => format!("session:{id}"),
        }
    }
}

/// One variable as held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = VariableHeld)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    /// The value.
    pub value: serde_json::Value,
    /// The scope revision of the patch that set it.
    pub revision: u64,
    /// Who set it.
    pub author: String,
    /// When it expires, in seconds since the Unix epoch; none for never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// When it was set, in seconds since the Unix epoch.
    pub at: u64,
}

/// One scope's variables as held.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = VariablesHeld)]
#[serde(deny_unknown_fields)]
pub struct Map {
    /// The scope's revision: the count of patches kept.
    pub revision: u64,
    /// The variables, by name, expired ones included until read.
    pub values: BTreeMap<String, Variable>,
}

/// One leaf of the variables log: a patch kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patched {
    /// The scope.
    pub scope: Scope,
    /// The revision this patch makes: the held revision plus one.
    pub revision: u64,
    /// Who patched.
    pub author: String,
    /// The keys set, with their values.
    pub set: BTreeMap<String, serde_json::Value>,
    /// The keys removed.
    pub removed: Vec<String>,
    /// The expiry of the keys set, when the patch named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// The variables as their log folds them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variables {
    /// Each scope's map, by the scope's key.
    pub scopes: BTreeMap<String, Map>,
}

impl Folded for Variables {
    type Line = Patched;
    const DOMAIN: &'static str = "lys/identity/variables-state/v1";
    const FORMAT: &'static str = "lys-variables-state/v1";
    const KIND: &'static str = "variables";

    fn hold(&mut self, patched: Patched) -> Result<(), String> {
        let map = self.scopes.entry(patched.scope.key()).or_default();
        if patched.revision != map.revision + 1 {
            return Err(format!(
                "{} is at revision {}, so the next patch is {}, not {}",
                patched.scope.name(),
                map.revision,
                map.revision + 1,
                patched.revision
            ));
        }
        for name in &patched.removed {
            map.values.remove(name);
        }
        for (name, value) in patched.set {
            map.values.insert(
                name,
                Variable {
                    value,
                    revision: patched.revision,
                    author: patched.author.clone(),
                    expires_at: patched.expires_at,
                    at: patched.at,
                },
            );
        }
        map.revision = patched.revision;
        Ok(())
    }
}

/// A scope's variables as read at `now`: the live values, and the names
/// that have expired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = VariablesRead)]
#[serde(deny_unknown_fields)]
pub struct Read {
    /// The scope.
    pub scope: Scope,
    /// The scope's revision, to carry on the next patch.
    pub revision: u64,
    /// The live variables, by name.
    pub values: BTreeMap<String, Variable>,
    /// The names held but expired at the read: absent above, named here.
    pub expired: Vec<String>,
}

impl Variables {
    /// The map of `scope`, empty at revision 0 when never patched.
    pub fn map(&self, scope: &Scope) -> Map {
        self.scopes.get(&scope.key()).cloned().unwrap_or_default()
    }

    /// `scope`'s variables as read at `now`.
    pub fn read(&self, scope: &Scope, now: u64) -> Read {
        let map = self.map(scope);
        let (live, expired): (Vec<_>, Vec<_>) = map
            .values
            .into_iter()
            .partition(|(_, variable)| variable.expires_at.is_none_or(|expiry| expiry > now));
        Read {
            scope: scope.clone(),
            revision: map.revision,
            values: live.into_iter().collect(),
            expired: expired.into_iter().map(|(name, _)| name).collect(),
        }
    }
}

/// Refuse a variable name that is not lowercase letters, digits or `_`,
/// starting with a letter, 1 to 64 long.
pub fn checked_name(name: &str) -> Result<(), VariablesError> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if ok {
        Ok(())
    } else {
        Err(VariablesError::Malformed {
            reason: format!(
                "`{name}` is not a variable name: lowercase letters, digits and _, starting with a letter, 1 to 64 long"
            ),
        })
    }
}
