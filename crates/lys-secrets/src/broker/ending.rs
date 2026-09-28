//! A person ending a handle held for them. Only the person the handle's
//! holder acts for may end it: the holder itself when the holder is a
//! person, or a person the permission source says the holder acts for (the
//! `member` relation on `person/<id>`). Whether a secret's owner may end
//! every handle derived from it is not decided, so ownership alone confers
//! nothing here.
//!
//! Ending a handle ends every handle lent on from it that still stands, one
//! drop line each. Each line names the person, the operation id and the
//! line of handles from the one ended down to it. The ending is carried
//! under an operation id the person made once for it: a resend with the same
//! id and the same handle answers what was recorded and appends nothing, and
//! the same id with another handle is refused `OperationReused`.
//!
//! Ending here is one fact. Whether the provider has revoked the grant
//! behind the handle is another, kept apart (see `revocation`) and never
//! inferred from this one.
//!
//! What the broker knows of endings is a fold of the audit log, by the one
//! function below, at a start and when a snapshot is written alike.

use crate::audit::{AuditKind, AuditLine};
use crate::error::{LendingRefusal, OwnerChangeRefusal, SecretsError};
use crate::handle::HandleId;
use crate::permission::PermissionCheck;
use crate::store::{Recipients, Scope};

use super::lineage::MAX_DEPTH;
use super::owner::checked;
use super::scope::with_via;
use super::{Broker, Handles, Work};

/// How an ending's drop line opens.
const ENDED_BY: &str = "ended by ";
/// What stands between the person and the line of handles.
const ALONG: &str = " along ";
/// What stands between two handles of the line.
const STEP: &str = " > ";

/// Who ended a handle and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// The person who ended it.
    pub by: String,
    /// The operation id the ending was carried under.
    pub operation: String,
    /// The handle whose ending ended this one: itself, or the handle above
    /// it that was ended.
    pub root: String,
}

/// What asking to end a handle came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleEnded {
    /// The handle, and every handle lent on from it that still stood, were
    /// ended now: their ids, the one asked for first.
    Ended {
        /// The handles ended.
        ended: Vec<String>,
    },
    /// The operation id was applied before to this same handle; nothing is
    /// ended again. The handles that ending ended, the one asked for first.
    Repeated {
        /// The handles ended under the operation id.
        ended: Vec<String>,
    },
    /// The handle was already dropped or ended, or one above it was;
    /// nothing is appended.
    AlreadyEnded,
}

/// The person and the ended handle an ending's drop line names.
fn parsed(outcome: &str) -> Option<(String, String)> {
    let rest = outcome.strip_prefix(ENDED_BY)?;
    let (by, line) = rest.split_once(ALONG)?;
    let root = line.split(' ').next()?;
    if by.is_empty() || root.is_empty() {
        return None;
    }
    Some((by.to_owned(), root.to_owned()))
}

/// The state after `line`: an ending's drop line marks its handle dropped
/// and records who ended it, under which operation, and from which handle.
pub(super) fn fold(handles: &mut Handles, line: &AuditLine) {
    if line.kind != AuditKind::Drop {
        return;
    }
    let (Some(id), Some(operation)) = (&line.handle, &line.operation) else {
        return;
    };
    let Some((by, root)) = parsed(&line.outcome) else {
        return;
    };
    handles.end(
        id,
        &Ended {
            by,
            operation: operation.clone(),
            root,
        },
    );
}

/// Refuses a name an ending's drop line could not be read back with.
fn spoken(what: &'static str, name: &str) -> Result<(), SecretsError> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(SecretsError::InvalidName {
            what,
            name: name.to_owned(),
            reason: "is empty or holds whitespace",
        });
    }
    Ok(())
}

impl<P: PermissionCheck> Broker<P> {
    /// Ends the handle `id` as `person`, under the operation id `operation`,
    /// asked through the screen service `via` when one carried the person's
    /// word. Every handle lent on from it that still stands is ended with
    /// it, each with its own drop line naming the line of handles down to
    /// it. The provider's part of the revocation is not touched.
    ///
    /// # Errors
    ///
    /// `HandleUnknown` for a handle never issued or on a secret `person` may
    /// not discover, the two not told apart; `LendingNotPermitted` when
    /// `person` is not the person the holder acts for; `OperationMissing`
    /// for no operation id or one of the wrong shape; `OperationReused` for
    /// an id already applied by `person` to another handle; `InvalidName`
    /// for a person or service name holding whitespace; and the audit log's
    /// refusals.
    pub fn end_handle(
        &mut self,
        person: &str,
        id: &HandleId,
        via: Option<&str>,
        operation: Option<&str>,
    ) -> Result<HandleEnded, SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        if !self.discovers(person, &record.secret) {
            return Err(SecretsError::HandleUnknown);
        }
        let acted_for = self.acts_for(person, &record.identity);
        let operation = checked(&format!("handle {id}"), operation)?;
        if let Some(root) = self.ended_under(person, operation) {
            if root == id.as_str() {
                return Ok(HandleEnded::Repeated {
                    ended: self.ended_with(person, operation, &root),
                });
            }
            return Err(SecretsError::from(OwnerChangeRefusal::Reused {
                operation: operation.to_owned(),
                secret: format!("handle {root}"),
            }));
        }
        if self.line_dropped(id.as_str()) {
            return Ok(HandleEnded::AlreadyEnded);
        }
        if !acted_for {
            return Err(SecretsError::from(LendingRefusal::NotActedFor {
                person: person.to_owned(),
                handle: id.as_str().to_owned(),
            }));
        }
        spoken("the person ending a handle", person)?;
        if let Some(service) = via {
            spoken("the service carrying an ending", service)?;
        }
        let mut ended = Vec::new();
        for path in self.standing_below(id.as_str()) {
            let Some(at) = path.last() else {
                continue;
            };
            let Some(record) = self.handles.get(at) else {
                continue;
            };
            let (identity, secret) = (record.identity.clone(), record.secret.clone());
            let outcome = with_via(format!("{ENDED_BY}{person}{ALONG}{}", path.join(STEP)), via);
            let mut line = self.line(
                AuditKind::Drop,
                (Some(at), Some(&identity), Some(&secret)),
                Some((operation, "")),
                None,
                &outcome,
            );
            line.request = None;
            self.append(&line)?;
            fold(&mut self.handles, &line);
            ended.push(at.clone());
        }
        Ok(HandleEnded::Ended { ended })
    }

    /// Whether `holder` acts for `person`: it is that person, or the
    /// permission source says it acts for them.
    fn acts_for(&self, person: &str, holder: &str) -> bool {
        let own = holder == person && Recipients::PeopleOnly.admits(person);
        own || self
            .permissions
            .member_of(holder, &Scope::Personal(person.to_owned()).target())
            .is_ok()
    }

    /// The handle `person` ended under `operation`, when there is one: the
    /// first, in order of id, that is its own ending's root. Only that handle
    /// is visited.
    pub(super) fn ended_under(&self, person: &str, operation: &str) -> Option<String> {
        let root = self
            .handles
            .roots_ended_by(person, operation)
            .into_iter()
            .next()?;
        Work::count(&self.work.visited, 1);
        Some(root.to_owned())
    }

    /// Every handle `person` ended under `operation`, `root` first. Only
    /// those handles are visited.
    pub(super) fn ended_with(&self, person: &str, operation: &str, root: &str) -> Vec<String> {
        let mut ended: Vec<String> = self
            .handles
            .ended_by(person, operation)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let visited = u64::try_from(ended.len()).unwrap_or(u64::MAX);
        Work::count(&self.work.visited, visited);
        ended.sort_by_key(|id| id != root);
        ended
    }

    /// The line of handles from `root` down to each handle at or below it
    /// that still stands, `root` first and the nearest before the deeper.
    /// It walks down from `root` through each handle's children, so it
    /// visits the handles at or below `root` and no other, and no deeper
    /// than a line of handles is counted.
    pub(super) fn standing_below(&self, root: &str) -> Vec<Vec<String>> {
        let mut paths: Vec<Vec<String>> = Vec::new();
        let mut walk: Vec<Vec<String>> = Vec::new();
        if self.handles.contains_key(root) {
            walk.push(vec![root.to_owned()]);
        }
        while let Some(path) = walk.pop() {
            Work::count(&self.work.visited, 1);
            let Some(at) = path.last() else {
                continue;
            };
            if path.len() < MAX_DEPTH {
                for child in self.handles.children(at) {
                    if !path.iter().any(|above| above == child) {
                        let mut below = path.clone();
                        below.push(child.to_owned());
                        walk.push(below);
                    }
                }
            }
            if !self.line_dropped(at) {
                paths.push(path);
            }
        }
        paths.sort_by(|one, other| one.len().cmp(&other.len()).then_with(|| one.cmp(other)));
        paths
    }
}
