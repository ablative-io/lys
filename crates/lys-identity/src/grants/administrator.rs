//! Standing is an unlimited source; only the resulting grant is recorded.

use super::admission::active;
use super::delegation::{Authority, PassOnRequest, judge_pass_on, recipients};
use super::{
    GrantChange, GrantError, GrantEvent, GrantId, Grants, Mode, PassOn, Recorded,
    RelationshipStore, Source,
};
use crate::IdentityId;
use crate::projection::Projection;
use lys_log_store::LeafStore;

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Pass on the administrator's standing through the ordinary recipient rules.
    ///
    /// # Errors
    /// Refuses any caller other than the active root person, invalid terms,
    /// an inadmissible recipient, reused operations or an unavailable log or engine.
    pub fn delegate_administrator(
        &mut self,
        directory: &Projection,
        request: &PassOnRequest,
        mode: Mode,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        if request.caller != IdentityId::Person(self.root_authority) {
            return Err(GrantError::RootAuthorityRefused {
                caller: request.caller.to_string(),
            });
        }
        active(directory, request.caller)?;
        self.settle_for_change()?;
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return match event.change() {
                GrantChange::Issue(grant)
                    if {
                        let parts = grant.parts();
                        parts.source == Source::Administrator
                            && parts.issuer == request.caller
                            && parts.holder == request.recipient
                            && parts.responsible == request.responsible
                            && parts.resource == request.resource
                            && parts.relation == request.relation
                            && parts.pass_on == request.pass_on
                            && parts.window == request.window
                            && grant.mode() == mode
                            && !grant.is_once()
                    } =>
                {
                    self.answer(event, receipt)
                }
                _ => Err(GrantError::OperationReused {
                    operation: request.operation.to_string(),
                }),
            };
        }
        let kind = request.resource.kind();
        let actions = self.model.kinds().get(kind).map_or_else(
            || {
                self.model
                    .relations_on(kind)
                    .flat_map(|(_, actions)| actions.iter().cloned())
                    .collect()
            },
            |kind| kind.actions.clone(),
        );
        let pass_on = PassOn::to(actions, recipients())?;
        let grant = judge_pass_on(
            directory,
            &self.model,
            request,
            &Authority {
                source: Source::Administrator,
                resource: None,
                pass_on: &pass_on,
                mode,
            },
            GrantId::generate()?,
        )?;
        self.commit(GrantEvent::new(
            request.operation,
            request.caller,
            at,
            GrantChange::Issue(Box::new(grant)),
        )?)
    }
}
