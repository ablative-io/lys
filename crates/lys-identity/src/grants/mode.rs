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
        let grant = judge_root(
            directory,
            &self.model,
            self.root_authority,
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
