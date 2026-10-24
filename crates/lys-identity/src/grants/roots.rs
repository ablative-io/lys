//! Whose authority a root grant rests on (ACCESS-009 R2): the install's root
//! person for its own collections; otherwise a person holding
//! organisation.grant on the organisation, shown by their grants; and the
//! same standing to take a grant back within an organisation.

use lys_log_store::LeafStore;

use super::admission::{RootAuthority, RootRequest, Route, install_level};
use super::authority::{ExerciseRequest, Grants, Recorded, RevokeRequest};
use super::error::GrantError;
use super::permission::RelationshipStore;
use super::revocation::judge_revoke;
use super::types::{Action, Grant, GrantId, Resource};
use crate::id::{IdentityId, PersonId};
use crate::projection::Projection;

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Whose authority a root on `request.resource` rests on (ACCESS-009
    /// R2): the install's root person for an install-level resource;
    /// otherwise the caller's organisation, once the caller is shown to hold
    /// a live grant carrying organisation.grant on it, and the resource is
    /// not rooted under another organisation.
    ///
    /// # Errors
    /// - `GrantError::RootAuthorityRefused` if the caller is of no organisation, or holds no live organisation.grant on it.
    /// - `GrantError::RootOwnedElsewhere` if a live grant on the resource carries another organisation.
    /// - Whatever reading the caller's grants refuses, other than a grant not held, revoked, expired or not started.
    pub fn root_authority_for(
        &mut self,
        directory: &Projection,
        request: &RootRequest,
        at: u64,
    ) -> Result<RootAuthority, GrantError> {
        if install_level(&request.resource) {
            return Ok(RootAuthority::Install(self.root_authority));
        }
        let refused = || GrantError::RootAuthorityRefused {
            caller: request.caller.to_string(),
        };
        let organisation = directory
            .organisation_of(request.caller)
            .ok_or_else(refused)?;
        if let Some(elsewhere) = self
            .rooted_under(&request.resource, at)
            .filter(|owner| *owner != organisation)
        {
            return Err(GrantError::RootOwnedElsewhere {
                resource: request.resource.to_string(),
                organisation: elsewhere.to_string(),
            });
        }
        self.holds_organisation_grant(directory, request.caller, request.route, organisation, at)
            .map(|()| RootAuthority::Organisation(organisation))
            .map_err(|error| match error {
                GrantError::NotHeld { .. }
                | GrantError::Revoked { .. }
                | GrantError::Expired { .. }
                | GrantError::NotStarted { .. }
                | GrantError::PermissionAbsent { .. } => refused(),
                other => other,
            })
    }

    /// Show `caller` to hold a live grant carrying organisation.grant on
    /// `organisation`, by the same reading a check makes, recording no use.
    pub(super) fn holds_organisation_grant(
        &mut self,
        directory: &Projection,
        caller: IdentityId,
        route: Route,
        organisation: crate::OrganisationId,
        at: u64,
    ) -> Result<(), GrantError> {
        let exercise = ExerciseRequest {
            caller,
            route,
            resource: Resource::new("organisation", &organisation.to_string())?,
            action: Action::new(super::shipped::ORGANISATION_GRANT)?,
        };
        self.explain(directory, &exercise, at, None).map(|_| ())
    }

    /// The organisation `resource` is rooted under: the one a grant on it
    /// that is neither revoked nor ended at `at` carries, when any does.
    fn rooted_under(&self, resource: &Resource, at: u64) -> Option<crate::OrganisationId> {
        self.book
            .live_on_resource(resource)
            .filter(|record| record.revoked().is_none())
            .filter(|record| {
                record
                    .grant()
                    .window()
                    .ends_at()
                    .is_none_or(|ends| at < ends)
            })
            .find_map(|record| record.grant().organisation())
    }

    /// The people who may issue a root on `resource` to someone of
    /// `organisation`, read from the book alone (ACCESS-009 R2): the
    /// install's root person for an install-level resource; otherwise every
    /// person holding a live grant carrying organisation.grant on the
    /// organisation, and nobody while the resource is rooted under another.
    /// A hint for a surface: the issue itself is judged by
    /// [`Grants::issue_root`].
    ///
    /// # Errors
    /// Refuses an organisation whose id the grants cannot name as a resource.
    pub fn rooters(
        &self,
        organisation: Option<crate::OrganisationId>,
        resource: &Resource,
        at: u64,
    ) -> Result<Vec<PersonId>, GrantError> {
        if install_level(resource) {
            return Ok(vec![self.root_authority]);
        }
        let Some(organisation) = organisation else {
            return Ok(Vec::new());
        };
        if self
            .rooted_under(resource, at)
            .is_some_and(|owner| owner != organisation)
        {
            return Ok(Vec::new());
        }
        let on = Resource::new("organisation", &organisation.to_string())?;
        let act = Action::new(super::shipped::ORGANISATION_GRANT)?;
        let mut people = Vec::new();
        for record in self.book.live_on_resource(&on) {
            let held = record.grant();
            if let IdentityId::Person(person) = held.holder()
                && record.revoked().is_none()
                && held.window().contains(at)
                && self.model.actions_of(held).contains(&act)
                && !people.contains(&person)
            {
                people.push(person);
            }
        }
        Ok(people)
    }

    /// Record `grant` as issued, judging nothing: for a fixture writing the
    /// store of a build whose judge admitted it, never for a route.
    ///
    /// # Errors
    /// Refuses operation reuse, a grant the book cannot hold or a failed commit.
    #[cfg(any(test, feature = "test-support"))]
    pub fn record_unjudged(&mut self, grant: Grant, at: u64) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        let (operation, issuer) = (grant.parts().operation, grant.parts().issuer);
        self.commit(super::events::GrantEvent::new(
            operation,
            issuer,
            at,
            super::events::GrantChange::Issue(Box::new(grant)),
        )?)
    }

    /// Revoke a grant, and with it everything derived from it, as its
    /// issuer, a holder of a grant it derives from, the install's root
    /// person over an install-level grant, or a person holding
    /// organisation.grant on the grant's organisation (ACCESS-009 R2),
    /// shown by the caller's grants in `directory`.
    ///
    /// # Errors
    /// Refuses an unauthorized revocation, operation reuse or a failed commit.
    pub fn revoke(
        &mut self,
        directory: &Projection,
        request: &RevokeRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        if let Some(answered) = self.revoke_answered(request)? {
            return answered;
        }
        match judge_revoke(
            &self.book,
            self.standing_over(request.grant),
            request.caller,
            request.grant,
        ) {
            Ok(()) => {}
            Err(GrantError::RevokeRefused { caller, grant }) => {
                let organisation = self.book.grant(request.grant).and_then(Grant::organisation);
                let held = organisation.map(|organisation| {
                    self.holds_organisation_grant(
                        directory,
                        request.caller,
                        request.route,
                        organisation,
                        at,
                    )
                });
                match held {
                    Some(Ok(())) => {}
                    None
                    | Some(Err(
                        GrantError::NotHeld { .. }
                        | GrantError::Revoked { .. }
                        | GrantError::Expired { .. }
                        | GrantError::NotStarted { .. }
                        | GrantError::PermissionAbsent { .. },
                    )) => return Err(GrantError::RevokeRefused { caller, grant }),
                    Some(Err(error)) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
        self.record_revoke(request, at)
    }

    /// The install's root person, when `grant` is on an install-level
    /// resource and so revoked by standing.
    pub(super) fn standing_over(&self, grant: GrantId) -> Option<PersonId> {
        self.book
            .grant(grant)
            .filter(|held| install_level(held.resource()))
            .map(|_| self.root_authority)
    }
}
