//! A root grant held by draft or by two (ACCESS-001 R1): issued as the root
//! authority issues any root grant, judged by the same rules, and carrying its
//! mode. Its exercise is refused `ModeHeld` by [`Grants::check_by`]; its act
//! is taken through an approved draft.

use lys_log_store::LeafStore;

use super::admission::{RootRequest, judge_root};
use super::authority::{Grants, Recorded, root_matches};
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent};
use super::permission::RelationshipStore;
use super::types::{GrantId, Mode};
use crate::projection::Projection;

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Issue a root grant to a person in `mode`, as [`Grants::issue_root`]
    /// issues an outright one. A retry of the operation in the same mode is
    /// answered as first recorded; the operation asked again in another mode
    /// is refused `OperationReused`.
    ///
    /// # Errors
    /// - `GrantError::LogUnavailable` if the store or checkpoint cannot be read.
    /// - `GrantError::LeafNotAnEvent` if a stored leaf is missing or invalid.
    /// - `GrantError::OperationUnresolved` if an earlier change cannot be settled.
    /// - `GrantError::OperationReused` if the operation names another request or mode.
    /// - `GrantError::RootAuthorityRefused` if the caller is not the configured root person.
    /// - `GrantError::IdentityNotActive` if the caller or holder is inactive.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` or `IdentityError::RandomSourceUnavailable` if a required person is absent or a grant id cannot be generated.
    /// - `GrantError::RelationUnknown` if the model defines neither the requested relation nor role.
    /// - `GrantError::WithheldFromAgents` if pass-on would give agents withheld actions.
    /// - `GrantError::MachineRefused` if pass-on would give machines human responsibilities.
    /// - `GrantError::AuthorityAbsent` if grant actions or pass-on members are empty.
    /// - `GrantError::PassOnOutside` if pass-on exceeds exercise authority.
    /// - `GrantError::ModelInvalid` if the model version is zero.
    /// - `GrantError::LineageMalformed` if an issued root has an invalid holder.
    /// - `GrantError::EventMismatch` if the grant disagrees with the event operation or caller.
    /// - `GrantError::GrantExists` if the generated grant id already exists.
    /// - `GrantError::GrantUnknown` if a committed projection delta names an absent grant.
    /// - `GrantError::ResourceKindUnheld` if the permission engine cannot hold the resource kind.
    /// - `GrantError::AppendRefused` if the grant event is proved not committed.
    /// - `GrantError::ProjectionPending` if the committed event has not reached the readable relationships.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read, revision or write methods; that exact variant is propagated.
    pub fn issue_root_in(
        &mut self,
        directory: &Projection,
        request: &RootRequest,
        mode: Mode,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        if !mode.is_held() {
            return self.issue_root(directory, request, at);
        }
        self.settle_for_change()?;
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return match event.change() {
                GrantChange::Issue(grant) if root_matches(request, grant, mode) => {
                    self.answer(event, receipt)
                }
                _ => Err(GrantError::OperationReused {
                    operation: request.operation.to_string(),
                }),
            };
        }
        let authority = self.root_authority_for(directory, request, at)?;
        let grant = judge_root(
            directory,
            &self.model,
            authority,
            request,
            GrantId::generate()?,
        )?
        .with_mode(mode);
        self.commit(GrantEvent::new(
            request.operation,
            request.caller,
            at,
            GrantChange::Issue(Box::new(grant)),
        )?)
    }
}
