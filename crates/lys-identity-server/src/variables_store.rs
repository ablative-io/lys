//! The variables as they are kept (AGENTS-001 R2): a leaf store of their
//! own, one leaf for each patch, folded through the agents log engine and
//! sealed in its signed snapshot, so a restart keeps every map and every
//! revision where it stood.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::FileLeafStore;

use crate::agents_log::{Kept, RecordLog};
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::session::now;
use crate::variables_state::{Patched, Read, Scope, Variables, VariablesError, checked_name};

/// The origin the variables' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/variables";

fn unavailable(reason: String) -> ServerError {
    VariablesError::Unavailable { reason }.into()
}

/// The variables, read from their leaf store and appended to it.
pub type VariableStore = RecordLog<Variables, FileLeafStore>;

/// The variables behind one lock.
pub type VariablesKept = Kept<Variables>;

/// The variables in the directory `config` names, their snapshots signed by
/// `key`, saying through `say` how the log started; none when it names no
/// directory.
pub fn configured(
    config: &Config,
    key: Arc<Ed25519Identity>,
    say: &Say,
) -> Result<Option<VariablesKept>, ServerError> {
    let Some(dir) = config.variables_dir.as_deref() else {
        return Ok(None);
    };
    let store = open(dir, key)?;
    say(&format!(
        "variables log {}, holding {} scopes",
        store.start(),
        store.held().scopes.len()
    ));
    Ok(Some(Kept::new(store, unavailable)))
}

/// The variables kept in the directory `dir`, created when it does not exist.
pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<VariableStore, ServerError> {
    RecordLog::open(dir, ORIGIN, key, &unavailable)
}

/// A patch as asked: the revision read, the author, the values (null
/// removes a key) and an optional expiry for the keys set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// The scope.
    pub scope: Scope,
    /// The revision the caller read; 0 for a scope never patched.
    pub revision: u64,
    /// Who patches.
    pub author: String,
    /// The values, by name; null removes the key.
    pub values: BTreeMap<String, serde_json::Value>,
    /// When the keys set expire, in seconds since the Unix epoch.
    pub expires_at: Option<u64>,
}

/// Keep `patch`, refusing a stale revision, an empty patch, a malformed name
/// or an expiry already past, and answer the scope as it then reads.
pub fn patch(variables: &VariablesKept, patch: Patch) -> Result<Read, ServerError> {
    if patch.values.is_empty() {
        return Err(VariablesError::Malformed {
            reason: "the patch names no key: set a value, or null to remove one".to_owned(),
        }
        .into());
    }
    for name in patch.values.keys() {
        checked_name(name)?;
    }
    let at = now();
    if patch.expires_at.is_some_and(|expiry| expiry <= at) {
        return Err(VariablesError::Malformed {
            reason: "expires_at has passed: an expiry is an instant after now".to_owned(),
        }
        .into());
    }
    variables.with(|log, unavailable| {
        let held = log.held().map(&patch.scope).revision;
        if held != patch.revision {
            return Err(VariablesError::Stale {
                scope: patch.scope.name(),
                held,
                given: patch.revision,
            }
            .into());
        }
        let (set, removed): (Vec<_>, Vec<_>) = patch
            .values
            .into_iter()
            .partition(|(_, value)| !value.is_null());
        let patched = Patched {
            scope: patch.scope.clone(),
            revision: held + 1,
            author: patch.author,
            set: set.into_iter().collect(),
            removed: removed.into_iter().map(|(name, _)| name).collect(),
            expires_at: patch.expires_at,
            at,
        };
        log.append(patched, unavailable)?;
        Ok(log.held().read(&patch.scope, at))
    })
}

/// `scope`'s variables as they read now.
pub fn read(variables: &VariablesKept, scope: &Scope) -> Result<Read, ServerError> {
    variables.with(|log, _| Ok(log.held().read(scope, now())))
}
