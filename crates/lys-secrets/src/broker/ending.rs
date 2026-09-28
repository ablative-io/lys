//! Ending a lease, and the part of the access seam that asks about leases.
//!
//! Only two identities may discover a lease: its holder, and the person it
//! is acted for: the holder itself when the holder is a person, or a person
//! the permission source says the holder acts for (the `member` relation on
//! `person/<id>`); for a handle lent on, also the person a handle above it
//! is so acted for. Everyone else is answered as if the lease did not
//! exist.
//! Only the person acted for may revoke it; its holder relinquishes it, a
//! separate act. Whether a secret's owner may end every handle derived from
//! it is not decided (ADR-095, proposed), so ownership, and seeing the
//! secret, confer nothing here.
//!
//! A revoke or a relinquish stops issuing at once, records the end with its
//! act, its caller and its instant, and moves the lease's provider state to
//! unconfirmed, all through [`Broker::end_with_upstream_pending`]; the
//! system behind is asked apart, and only its acknowledgement, delivered
//! through [`Broker::deliver_upstream_ack`], confirms it. A lease that has
//! already ended is refused by name with how, when and by whom it first
//! ended, and nothing is recorded, so its first end record stays its only
//! one.
//!
//! Ending a handle ends every handle lent on from it that still stands, one
//! drop line each. Each line names the act, the one who ended it, the
//! operation id and the line of handles from the one ended down to it. The
//! screen's ending (`end_handle`) is carried under an operation id the
//! person made once for it: a resend with the same id and the same handle
//! answers what was recorded and appends nothing, and the same id with
//! another handle is refused `OperationReused`.
//!
//! What the broker knows of endings is a fold of the audit log, by the one
//! function below, at a start and when a snapshot is written alike.

use crate::audit::{AuditKind, AuditLine};
use crate::encoding::hex;
use crate::error::{LeaseRefusal, LendingRefusal, OwnerChangeRefusal, SecretsError};
use crate::handle::{HandleId, new_operation_id};
use crate::permission::PermissionCheck;
use crate::store::{Recipients, Scope};

use super::folded::Handles;
use super::lineage::chain;
use super::owner::checked;
use super::scope::with_via;
use super::{Broker, LeaseView};

/// How a revoke's drop line opens.
const ENDED_BY: &str = "ended by ";
/// How a relinquish's drop line opens.
const RELINQUISHED_BY: &str = "relinquished by ";
/// What stands between the one ending and the line of handles.
const ALONG: &str = " along ";
/// What stands between two handles of the line.
const STEP: &str = " > ";

/// The act a lease was ended by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndAct {
    /// The person the lease is acted for revoked it.
    Revoke,
    /// Its holder gave it back.
    Relinquish,
}

impl EndAct {
    /// The act as a route reads it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Revoke => "revoke",
            Self::Relinquish => "relinquish",
        }
    }

    /// How the act's drop line opens.
    fn opening(self) -> &'static str {
        match self {
            Self::Revoke => ENDED_BY,
            Self::Relinquish => RELINQUISHED_BY,
        }
    }

    /// Who the act's caller is, as a refusal of its name says.
    fn whose(self) -> &'static str {
        match self {
            Self::Revoke => "the person revoking a lease",
            Self::Relinquish => "the holder relinquishing a lease",
        }
    }

    /// The act's code in the folded state.
    pub(super) fn code(self) -> u8 {
        match self {
            Self::Revoke => 1,
            Self::Relinquish => 2,
        }
    }

    /// The act a code in the folded state names.
    pub(super) fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Revoke),
            2 => Some(Self::Relinquish),
            _ => None,
        }
    }
}

/// How a lease came to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndWay {
    /// The person it is acted for revoked it.
    Revoke,
    /// Its holder relinquished it.
    Relinquish,
    /// Its time window closed.
    Expired,
    /// The operator holding the store dropped it from the command line, or
    /// a handle above it was so dropped: an act of the store's keeper, like
    /// an emergency stop, outside the rule of who may revoke.
    Dropped,
}

impl EndWay {
    /// The way as a route reads it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Revoke => "revoke",
            Self::Relinquish => "relinquish",
            Self::Expired => "expired",
            Self::Dropped => "dropped",
        }
    }
}

impl From<EndAct> for EndWay {
    fn from(act: EndAct) -> Self {
        match act {
            EndAct::Revoke => Self::Revoke,
            EndAct::Relinquish => Self::Relinquish,
        }
    }
}

/// How, when and by whom a lease first ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEnd {
    /// How it ended.
    pub way: EndWay,
    /// When, in milliseconds since the epoch: the instant of the act, or
    /// the end of the window; none for an operator's drop.
    pub at_ms: Option<i64>,
    /// Who ended it, for a revoke or a relinquish.
    pub by: Option<String>,
}

/// Who ended a handle, how and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// The one who ended it: the person acted for, or the holder.
    pub by: String,
    /// The operation id the ending was carried under.
    pub operation: String,
    /// The handle whose ending ended this one: itself, or the handle above
    /// it that was ended.
    pub root: String,
    /// The act it was ended by.
    pub act: EndAct,
    /// When, in milliseconds since the epoch, as the broker's clock read.
    pub at_ms: i64,
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

/// The system behind a lease's credential, asked to revoke it once the
/// lease has ended here. Asking confirms nothing: the acknowledgement comes
/// apart, through [`Broker::deliver_upstream_ack`].
pub trait SystemBehind {
    /// Asks the system behind to revoke the credential behind the lease
    /// `lease`, issued from `secret`.
    fn ask_revoke(&mut self, lease: &str, secret: &str);
}

impl<F: FnMut(&str, &str)> SystemBehind for F {
    fn ask_revoke(&mut self, lease: &str, secret: &str) {
        self(lease, secret);
    }
}

/// The act, the one who ended it and the ended handle a drop line names.
fn parsed(outcome: &str) -> Option<(EndAct, String, String)> {
    let (act, rest) = [EndAct::Revoke, EndAct::Relinquish]
        .into_iter()
        .find_map(|act| outcome.strip_prefix(act.opening()).map(|rest| (act, rest)))?;
    let (by, line) = rest.split_once(ALONG)?;
    let root = line.split(' ').next()?;
    if by.is_empty() || root.is_empty() {
        return None;
    }
    Some((act, by.to_owned(), root.to_owned()))
}

/// The state after `line`: an ending's drop line marks its handle dropped
/// and records the act, who ended it, when, under which operation, and from
/// which handle.
pub(super) fn fold(handles: &mut Handles, line: &AuditLine) {
    if line.kind != AuditKind::Drop {
        return;
    }
    let (Some(id), Some(operation)) = (&line.handle, &line.operation) else {
        return;
    };
    let Some((act, by, root)) = parsed(&line.outcome) else {
        return;
    };
    if let Some(record) = handles.get_mut(id) {
        record.dropped = true;
        record.ended = Some(Ended {
            by,
            operation: operation.clone(),
            root,
            act,
            at_ms: line.at_ms,
        });
    }
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
    /// The seam's lease discovery: whether `identity` may discover the lease
    /// `id`. Only its holder and the person it is acted for may; seeing or
    /// owning the secret it was issued from discovers nothing. A lease never
    /// issued is discovered by no one.
    pub fn discovers_lease(&self, identity: &str, id: &HandleId) -> bool {
        self.discovers_record(identity, id.as_str())
    }

    /// The seam's lease revoke: whether `identity` may revoke the lease
    /// `id`. Only the person it is acted for may; while ADR-095 is proposed
    /// no one else, the secret's owner included.
    pub fn revokes_lease(&self, identity: &str, id: &HandleId) -> bool {
        self.acted_for_under(identity, id.as_str())
    }

    /// The seam's lease discovery, over the record id `id`: its holder, or
    /// the person it is acted for.
    pub(super) fn discovers_record(&self, identity: &str, id: &str) -> bool {
        let held = self
            .handles
            .get(id)
            .is_some_and(|record| record.identity == identity);
        held || self.acted_for_under(identity, id)
    }

    /// Whether `identity` is the person the lease `id` is acted for: the
    /// person its holder acts for, or, for a handle lent on, the person the
    /// holder of a handle above it acts for.
    fn acted_for_under(&self, identity: &str, id: &str) -> bool {
        self.handles.contains_key(id)
            && chain(&self.handles, id).iter().any(|at| {
                self.handles
                    .get(at)
                    .is_some_and(|record| self.acts_for(identity, &record.identity))
            })
    }

    /// Whether `identity` holds the lease `id`.
    fn holds(&self, identity: &str, id: &HandleId) -> bool {
        self.handles
            .get(id.as_str())
            .is_some_and(|record| record.identity == identity)
    }

    /// Revokes the lease `id` as `caller`, the person it is acted for, and
    /// asks `behind` to revoke the credential behind it. Answers the lease
    /// as it then stands: issuing stopped, its provider state pending.
    ///
    /// # Errors
    ///
    /// `NotFound` when `caller` may not discover the lease, naming nothing;
    /// `AlreadyEnded` when the person acted for revokes a lease that has
    /// ended, recording nothing; `HolderRelinquishes` for its holder who is
    /// not the person acted for; `RevokeNotPermitted` for any other caller
    /// the seam lets discover it; and `Failed` when the end cannot be
    /// recorded.
    pub fn revoke_lease(
        &mut self,
        caller: &str,
        id: &HandleId,
        behind: &mut dyn SystemBehind,
    ) -> Result<LeaseView, LeaseRefusal> {
        if !self.discovers_lease(caller, id) {
            return Err(LeaseRefusal::NotFound);
        }
        let lease = id.as_str().to_owned();
        if !self.revokes_lease(caller, id) {
            if self.holds(caller, id) {
                return Err(LeaseRefusal::HolderRelinquishes { lease });
            }
            return Err(LeaseRefusal::RevokeNotPermitted { lease });
        }
        self.end_lease(EndAct::Revoke, caller, id, behind)
    }

    /// Relinquishes the lease `id` as `caller`, its holder, recorded as a
    /// relinquish and never as a revoke, and asks `behind` to revoke the
    /// credential behind it. Answers the lease as it then stands.
    ///
    /// # Errors
    ///
    /// `NotFound` when `caller` may not discover the lease, naming nothing;
    /// `RelinquishNotPermitted` when `caller` is not its holder;
    /// `AlreadyEnded` when the holder relinquishes a lease that has ended,
    /// recording nothing; and `Failed` when the end cannot be recorded.
    pub fn relinquish_lease(
        &mut self,
        caller: &str,
        id: &HandleId,
        behind: &mut dyn SystemBehind,
    ) -> Result<LeaseView, LeaseRefusal> {
        if !self.discovers_lease(caller, id) {
            return Err(LeaseRefusal::NotFound);
        }
        if !self.holds(caller, id) {
            return Err(LeaseRefusal::RelinquishNotPermitted {
                lease: id.as_str().to_owned(),
            });
        }
        self.end_lease(EndAct::Relinquish, caller, id, behind)
    }

    /// Ends the lease `id` by `act` as `caller`, unless it has already
    /// ended, then asks `behind` and answers the lease as it stands.
    fn end_lease(
        &mut self,
        act: EndAct,
        caller: &str,
        id: &HandleId,
        behind: &mut dyn SystemBehind,
    ) -> Result<LeaseView, LeaseRefusal> {
        if let Some(end) = self.lease_end(id.as_str()) {
            return Err(LeaseRefusal::AlreadyEnded {
                lease: id.as_str().to_owned(),
                end,
            });
        }
        self.end_with_upstream_pending(act, caller, id)?;
        let secret = self
            .handles
            .get(id.as_str())
            .map(|record| record.secret.clone())
            .ok_or(SecretsError::HandleUnknown)?;
        behind.ask_revoke(id.as_str(), &secret);
        self.lease_view(caller, id)
    }

    /// Ends the lease `id` by `act` as `caller`: stops issuing under it and
    /// under every handle lent on from it that still stands, records the end
    /// with its act, its caller and its instant, one drop line each, and
    /// moves the lease's provider state to unconfirmed, read as `pending`,
    /// with one audit line for the move. Every act that ends a lease with
    /// its provider state pending ends it here.
    ///
    /// # Errors
    ///
    /// `InvalidName` for a caller holding whitespace, `Random` when no
    /// operation id can be drawn, and the audit log's refusals.
    pub(crate) fn end_with_upstream_pending(
        &mut self,
        act: EndAct,
        caller: &str,
        id: &HandleId,
    ) -> Result<Vec<String>, SecretsError> {
        spoken(act.whose(), caller)?;
        let operation = hex(&new_operation_id()?);
        let ended = self.end_line(act, caller, id, &operation, None)?;
        self.upstream_pending(id)?;
        Ok(ended)
    }

    /// How, when and by whom the lease `id` first ended, when it has: its
    /// end record, else an operator's drop of it or of a handle above it,
    /// else the close of its window.
    pub(super) fn lease_end(&self, id: &str) -> Option<LeaseEnd> {
        let record = self.handles.get(id)?;
        if let Some(ended) = &record.ended {
            return Some(LeaseEnd {
                way: ended.act.into(),
                at_ms: Some(ended.at_ms),
                by: Some(ended.by.clone()),
            });
        }
        if self.line_dropped(id) {
            return Some(LeaseEnd {
                way: EndWay::Dropped,
                at_ms: None,
                by: None,
            });
        }
        ((self.clock)() > record.not_after_ms).then_some(LeaseEnd {
            way: EndWay::Expired,
            at_ms: Some(record.not_after_ms),
            by: None,
        })
    }

    /// Ends the handle `id` as `person`, under the operation id `operation`,
    /// asked through the screen service `via` when one carried the person's
    /// word. Every handle lent on from it that still stands is ended with
    /// it, each with its own drop line naming the line of handles down to
    /// it. The provider's part of the revocation is not touched.
    ///
    /// # Errors
    ///
    /// `HandleUnknown` for a handle never issued or one `person` may not
    /// discover through the seam (neither its holder nor the person it is
    /// acted for), the two not told apart; `LendingNotPermitted` for its
    /// holder when not the person acted for; `OperationMissing` for no
    /// operation id or one of the wrong shape; `OperationReused` for an id
    /// already applied by `person` to another handle; `InvalidName` for a
    /// person or service name holding whitespace; and the audit log's
    /// refusals.
    pub fn end_handle(
        &mut self,
        person: &str,
        id: &HandleId,
        via: Option<&str>,
        operation: Option<&str>,
    ) -> Result<HandleEnded, SecretsError> {
        if !self.discovers_lease(person, id) {
            return Err(SecretsError::HandleUnknown);
        }
        let acted_for = self.revokes_lease(person, id);
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
        if !acted_for {
            return Err(SecretsError::from(LendingRefusal::NotActedFor {
                person: person.to_owned(),
                handle: id.as_str().to_owned(),
            }));
        }
        if self.line_dropped(id.as_str()) {
            return Ok(HandleEnded::AlreadyEnded);
        }
        spoken("the person ending a handle", person)?;
        if let Some(service) = via {
            spoken("the service carrying an ending", service)?;
        }
        let ended = self.end_line(EndAct::Revoke, person, id, operation, via)?;
        Ok(HandleEnded::Ended { ended })
    }

    /// Writes one drop line by `act` as `by` for the handle `id` and for
    /// every handle lent on from it that still stands, and folds each.
    fn end_line(
        &mut self,
        act: EndAct,
        by: &str,
        id: &HandleId,
        operation: &str,
        via: Option<&str>,
    ) -> Result<Vec<String>, SecretsError> {
        let mut ended = Vec::new();
        for path in self.standing_below(id.as_str()) {
            let Some(at) = path.last() else {
                continue;
            };
            let Some(record) = self.handles.get(at) else {
                continue;
            };
            let (identity, secret) = (record.identity.clone(), record.secret.clone());
            let outcome = with_via(
                format!("{}{by}{ALONG}{}", act.opening(), path.join(STEP)),
                via,
            );
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
        Ok(ended)
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

    /// The handle `person` ended under `operation`, when there is one.
    fn ended_under(&self, person: &str, operation: &str) -> Option<String> {
        self.handles.values().find_map(|record| {
            record
                .ended
                .as_ref()
                .filter(|ended| {
                    ended.by == person && ended.operation == operation && ended.root == record.id
                })
                .map(|ended| ended.root.clone())
        })
    }

    /// Every handle `person` ended under `operation`, `root` first.
    fn ended_with(&self, person: &str, operation: &str, root: &str) -> Vec<String> {
        let mut ended: Vec<String> = self
            .handles
            .values()
            .filter(|record| {
                record
                    .ended
                    .as_ref()
                    .is_some_and(|ended| ended.by == person && ended.operation == operation)
            })
            .map(|record| record.id.clone())
            .collect();
        ended.sort_by_key(|id| id != root);
        ended
    }

    /// The line of handles from `root` down to each handle at or below it
    /// that still stands, `root` first and the nearest before the deeper.
    fn standing_below(&self, root: &str) -> Vec<Vec<String>> {
        let mut paths: Vec<Vec<String>> = self
            .handles
            .keys()
            .filter(|id| !self.line_dropped(id))
            .filter_map(|id| {
                let mut path = chain(&self.handles, id);
                let at = path.iter().position(|above| above == root)?;
                path.truncate(at.saturating_add(1));
                path.reverse();
                Some(path)
            })
            .collect();
        paths.sort_by(|one, other| one.len().cmp(&other.len()).then_with(|| one.cmp(other)));
        paths
    }
}
