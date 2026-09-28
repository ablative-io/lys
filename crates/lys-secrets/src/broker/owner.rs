//! Owner changes of scope or recipients, each carried under an operation id
//! its caller made once for that change. The broker records the id in the
//! change's audit line with a digest of the change, so a resend of the same
//! change is answered with the outcome recorded the first time and applies
//! nothing, and the same id sent with another change is refused.
//!
//! What the broker knows of these ids is a fold of the audit log, by the
//! one function below, at a start and when a snapshot is written alike.

use std::collections::BTreeMap;

use crate::audit::{AuditKind, AuditLine};
use crate::encoding::{Canonical, hex, sha256};
use crate::error::{LendingRefusal, OwnerChangeRefusal, SecretsError};
use crate::permission::PermissionCheck;

use super::Broker;

const CHANGE_DOMAIN: &str = "lys-secrets/owner-change/v1";
/// How an applied scope change's outcome opens.
pub(super) const SCOPE: &str = "scope ";
/// How an applied recipients change's outcome opens.
pub(super) const RECIPIENTS: &str = "recipients ";
const SHORTEST: usize = 16;
const LONGEST: usize = 64;

/// What an owner change came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerChanged {
    /// The change was applied now.
    Applied,
    /// The operation id was applied before with this same change; nothing
    /// is applied again.
    Repeated {
        /// The outcome recorded when it was applied.
        outcome: String,
    },
}

/// What the log says of one secret's owner changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Operations {
    /// The operation id of the last owner change applied, when it carried
    /// one.
    pub(super) last: Option<String>,
    /// Each operation id applied: the digest of its change and the outcome
    /// recorded.
    pub(super) applied: BTreeMap<String, (String, String)>,
}

/// Each secret's owner changes, by secret name.
pub(super) type Owners = BTreeMap<String, Operations>;

/// The operation id and change digest an owner change is recorded with.
pub(super) type Call = (String, String);

/// Where an owner change stands against the ids already applied.
pub(super) enum Admission {
    /// Applied before with this same change.
    Repeated(String),
    /// Not applied before; applied under this call.
    Fresh(Call),
}

/// The state after `line`: an applied owner change sets the secret's last
/// operation id, and records its id when it carried one.
pub(super) fn fold(owners: &mut Owners, line: &AuditLine) {
    if line.kind != AuditKind::Seal {
        return;
    }
    let Some(secret) = &line.secret else {
        return;
    };
    if !(line.outcome.starts_with(SCOPE) || line.outcome.starts_with(RECIPIENTS)) {
        return;
    }
    let operations = owners.entry(secret.clone()).or_default();
    match (&line.operation, &line.request) {
        (Some(operation), Some(digest)) => {
            operations
                .applied
                .insert(operation.clone(), (digest.clone(), line.outcome.clone()));
            operations.last = Some(operation.clone());
        }
        _ => operations.last = None,
    }
}

/// `operation` when it is an operation id of the right shape.
fn checked<'a>(secret: &str, operation: Option<&'a str>) -> Result<&'a str, SecretsError> {
    let missing = |reason: String| {
        SecretsError::from(OwnerChangeRefusal::Missing {
            secret: secret.to_owned(),
            reason,
        })
    };
    let Some(operation) = operation else {
        return Err(missing("carried no operation id".to_owned()));
    };
    let length = operation.chars().count();
    if !(SHORTEST..=LONGEST).contains(&length) {
        return Err(missing(format!(
            "carried an operation id of {length} characters"
        )));
    }
    let shaped = operation
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if !shaped {
        return Err(missing(
            "carried an operation id with a character outside A-Z, a-z, 0-9, _ and -".to_owned(),
        ));
    }
    Ok(operation)
}

/// The digest of the change `change` to `secret`, in hex.
fn change_digest(secret: &str, change: &str) -> Result<String, SecretsError> {
    let mut encoding = Canonical::new(CHANGE_DOMAIN)?;
    encoding
        .field(secret.as_bytes())?
        .field(change.as_bytes())?;
    Ok(hex(&sha256(&encoding.into_bytes())))
}

impl<P: PermissionCheck> Broker<P> {
    /// Refuses `owner` unless it owns `secret`.
    pub(super) fn owns(&self, owner: &str, secret: &str) -> Result<(), SecretsError> {
        let owns = self
            .store
            .entry(secret)
            .is_some_and(|entry| entry.owner == owner);
        if owns {
            return Ok(());
        }
        Err(SecretsError::from(LendingRefusal::NotPermitted {
            holder: owner.to_owned(),
            secret: secret.to_owned(),
        }))
    }

    /// Where the change `change` to `secret` under `operation` stands.
    ///
    /// # Errors
    ///
    /// `OperationMissing` for no id or one of the wrong shape, and
    /// `OperationReused` for an id applied to the secret with another change.
    pub(super) fn owner_admission(
        &self,
        secret: &str,
        operation: Option<&str>,
        change: &str,
    ) -> Result<Admission, SecretsError> {
        let operation = checked(secret, operation)?;
        let digest = change_digest(secret, change)?;
        let applied = self
            .owners
            .get(secret)
            .and_then(|operations| operations.applied.get(operation));
        match applied {
            None => Ok(Admission::Fresh((operation.to_owned(), digest))),
            Some((recorded, outcome)) if *recorded == digest => {
                Ok(Admission::Repeated(outcome.clone()))
            }
            Some(_other) => Err(SecretsError::from(OwnerChangeRefusal::Reused {
                operation: operation.to_owned(),
                secret: secret.to_owned(),
            })),
        }
    }

    /// Records an applied owner change of `secret` with `outcome`, under
    /// `call` when it carried one, and folds the line it wrote.
    pub(super) fn record_owner_change(
        &mut self,
        owner: &str,
        secret: &str,
        outcome: &str,
        call: Option<&Call>,
    ) -> Result<(), SecretsError> {
        let line = self.line(
            AuditKind::Seal,
            (None, Some(owner), Some(secret)),
            call.map(|(operation, digest)| (operation.as_str(), digest.as_str())),
            None,
            outcome,
        );
        self.append(&line)?;
        fold(&mut self.owners, &line);
        Ok(())
    }

    /// The operation id of the last owner change applied to `secret`, when
    /// it carried one.
    pub(super) fn last_operation(&self, secret: &str) -> Option<String> {
        self.owners
            .get(secret)
            .and_then(|operations| operations.last.clone())
    }
}
