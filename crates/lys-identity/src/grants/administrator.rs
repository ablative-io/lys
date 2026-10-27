//! Standing is an unlimited source; only the resulting grant is recorded.

use super::admission::active;
use super::delegation::{Authority, PassOnRequest, judge_pass_on, recipients};
use super::{
    GrantChange, GrantError, GrantEvent, GrantId, Grants, Mode, PassOn, Recorded,
    RelationshipStore, Source,
};
use crate::projection::Projection;
use crate::{IdentityId, PersonId};
use lys_log_store::LeafStore;

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Pass on the administrator's standing through the ordinary recipient rules.
    ///
    /// The install's root person gives from standing; so does an agent of an
    /// organisation shown to hold a live grant carrying organisation.grant on
    /// it (administrator standing, 24 October 2026). The standing itself is
    /// an unlimited source: the holding is read, never spent, and only the
    /// resulting grant is recorded.
    ///
    /// # Errors
    /// Refuses any caller other than the active root person or an agent
    /// holding organisation.grant on its own organisation, invalid terms,
    /// an inadmissible recipient, reused operations or an unavailable log or engine.
    pub fn delegate_administrator(
        &mut self,
        directory: &Projection,
        request: &PassOnRequest,
        mode: Mode,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.standing_of(directory, request, at)?;
        active(directory, request.caller)?;
        within_standing(directory, request.caller, request.recipient)?;
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
                // Standing passed on carries the recipient's own
                // organisation, read from the directory.
                organisation: directory.organisation_of(request.recipient),
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

    /// The people who may give `recipient` access from standing, read from
    /// the directory alone: the install's root person, while `recipient` is
    /// within the root person's organisation or the root person is of none.
    /// A hint for a surface: the gift itself is judged by
    /// [`Grants::delegate_administrator`].
    #[must_use]
    pub fn standing_givers(&self, directory: &Projection, recipient: IdentityId) -> Vec<PersonId> {
        let root = IdentityId::Person(self.root_authority);
        if within_standing(directory, root, recipient).is_ok() {
            vec![self.root_authority]
        } else {
            Vec::new()
        }
    }

    /// Whose standing `request.caller` gives from: the install's, as its
    /// root person; or its organisation's, as an agent holding a live grant
    /// carrying organisation.grant on it. Anyone else is refused by name.
    fn standing_of(
        &mut self,
        directory: &Projection,
        request: &PassOnRequest,
        at: u64,
    ) -> Result<(), GrantError> {
        let refused = || GrantError::RootAuthorityRefused {
            caller: request.caller.to_string(),
        };
        match request.caller {
            IdentityId::Person(person) if person == self.root_authority => Ok(()),
            IdentityId::Agent(_) => {
                let organisation = directory
                    .organisation_of(request.caller)
                    .ok_or_else(refused)?;
                self.holds_organisation_grant(
                    directory,
                    request.caller,
                    request.route,
                    organisation,
                    at,
                )
                .map_err(|error| match error {
                    GrantError::NotHeld { .. }
                    | GrantError::Revoked { .. }
                    | GrantError::Expired { .. }
                    | GrantError::NotStarted { .. }
                    | GrantError::PermissionAbsent { .. } => refused(),
                    other => other,
                })
            }
            IdentityId::Person(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_)
            | IdentityId::Organisation(_) => Err(refused()),
        }
    }
}

/// Standing is passed on within the giver's organisation (ACCESS-009 R3); a
/// giver of no organisation passes it to anyone.
fn within_standing(
    directory: &Projection,
    giver: IdentityId,
    recipient: IdentityId,
) -> Result<(), GrantError> {
    if let Some(organisation) = directory.organisation_of(giver)
        && directory.organisation_of(recipient) != Some(organisation)
    {
        return Err(GrantError::OrganisationOutside {
            grant: "the administrator's standing".to_owned(),
            organisation: organisation.to_string(),
            recipient: recipient.to_string(),
        });
    }
    Ok(())
}
