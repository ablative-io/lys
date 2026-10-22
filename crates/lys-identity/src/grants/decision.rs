//! Single decisions retain their explanation and record an exercise once.

use super::usage;
use super::{
    ExerciseRequest, Exercised, Frame, Grant, GrantError, GrantId, Grants, Permit,
    RelationshipStore,
};
use crate::projection::Projection;
use lys_log_store::LeafStore;

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Whether the caller may exercise the action on the resource at `at`,
    /// answered with the authority it rests on or refused by the boundary that
    /// blocks it, and the exercise recorded as a use event. `at_least` is a
    /// revision the caller requires the decision to reflect, such as the one a
    /// receipt names. This is the enforcement point: call it when the action is
    /// about to be taken.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::ModeHeld` if the selected grant requires an approved draft.
    /// - `GrantError::EventMismatch` if recording a one-time use produces an inconsistent event.
    /// - `GrantError::GrantUnknown` if its use event names no recorded grant.
    /// - `GrantError::OperationReused` if a generated use operation is already recorded.
    /// - `GrantError::AppendRefused` if a one-time use is proved not committed.
    /// - `GrantError::Identity` wrapping `IdentityError::RandomSourceUnavailable` if a one-time use operation cannot be generated.
    pub fn check(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        self.check_by(directory, request, None, at, at_least)
    }

    /// [`Grants::check`] resting only on `only` when it names a grant: no
    /// other grant the caller holds is considered, so the use is recorded
    /// against that grant or the exercise is refused.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::ModeHeld` if the selected grant requires an approved draft.
    /// - `GrantError::EventMismatch` if recording a one-time use produces an inconsistent event.
    /// - `GrantError::GrantUnknown` if its use event names no recorded grant.
    /// - `GrantError::OperationReused` if a generated use operation is already recorded.
    /// - `GrantError::AppendRefused` if a one-time use is proved not committed.
    /// - `GrantError::Identity` wrapping `IdentityError::RandomSourceUnavailable` if a one-time use operation cannot be generated.
    pub fn check_by(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        let mut permit = self.explain_by(directory, request, only, at, at_least)?;
        self.finish_check(request, at, &mut permit)?;
        Ok(permit)
    }

    pub(super) fn finish_check(
        &mut self,
        request: &ExerciseRequest,
        at: u64,
        permit: &mut Permit,
    ) -> Result<(), GrantError> {
        self.book
            .grant(permit.grant)
            .map_or(Ok(()), Grant::exercisable)?;
        let exercised = Exercised {
            action: request.action.clone(),
            resource: request.resource.clone(),
        };
        let used = self.record_use(request.caller, permit.grant, request.route, at, exercised);
        if let Err(error) = &used {
            usage::note(&mut self.unreported, permit.grant, request.route, at, error);
        }
        if self.is_one_time(permit.grant) {
            // A one-time grant admits only an exercise whose use is recorded;
            // that one leaf spends it.
            used.clone()?;
        }
        permit.use_event = Some(used);
        Ok(())
    }

    /// The same decision as [`Grants::check`], by the same evaluator at the
    /// same revision, and nothing recorded: answering why an identity may act,
    /// or who can, is not an exercise.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    pub fn explain(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        self.explain_by(directory, request, None, at, at_least)
    }

    /// [`Grants::explain`] resting only on `only` when it names a grant.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    pub fn explain_by(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        let settled = self.settle(at_least)?;
        self.unresolved_issue(request)?;
        let frame = super::frame::Frame::read_about(
            self,
            directory,
            settled,
            request.caller,
            &request.resource,
        )?;
        self.explain_only(&frame, request, only, at)
    }
    /// Exercise an already read decision without a second network read.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::ModeHeld` if the selected grant requires an approved draft.
    /// - `GrantError::EventMismatch` if recording a one-time use produces an inconsistent event.
    /// - `GrantError::GrantUnknown` if its use event names no recorded grant.
    /// - `GrantError::OperationReused` if a generated use operation is already recorded.
    /// - `GrantError::AppendRefused` if a one-time use is proved not committed.
    /// - `GrantError::Identity` wrapping `IdentityError::RandomSourceUnavailable` if a one-time use operation cannot be generated.
    pub fn check_in_by(
        &mut self,
        frame: &Frame<'_>,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
    ) -> Result<Permit, GrantError> {
        let mut permit = self.explain_in_by(frame, request, only, at)?;
        self.finish_check(request, at, &mut permit)?;
        Ok(permit)
    }
    /// Judge every exercise against one frame before recording any of its use events.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if an ancestor grant is absent.
    /// - `GrantError::LineageCycle` if the source chain revisits a grant.
    /// - `GrantError::LineageMalformed` if the root holder or standing issuer is not a person.
    /// - `GrantError::IssuerNotHolder` if a child issuer is outside its source authority.
    /// - `GrantError::ResourceOutside` if a child names another resource.
    /// - `GrantError::UseOnly` if an ancestor disallows pass-on.
    /// - `GrantError::RecipientRefused` if the child holder kind is excluded.
    /// - `GrantError::ActionsOutside` if a child carries actions outside its source.
    /// - `GrantError::PassOnBeyondSource` if a child may pass on more than its source permits.
    /// - `GrantError::ExpiryBeyondSource` if a child outlives its source.
    /// - `GrantError::LogUnavailable` if log reconciliation or checkpoint reading fails.
    /// - `GrantError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed event.
    /// - `GrantError::NotHeld` if the caller holds no applicable grant.
    /// - `GrantError::Revoked` if the selected ancestry was revoked or a one-time grant is spent.
    /// - `GrantError::Expired` if a grant or relationship caveat has ended.
    /// - `GrantError::NotStarted` if an ancestry grant has not started.
    /// - `GrantError::IdentityNotActive` if a holder or standing issuer is inactive.
    /// - `GrantError::ResponsibleMismatch` if recorded responsibility differs from the directory.
    /// - `GrantError::Identity` wrapping `IdentityError::IdentityUnknown` if a required identity is absent.
    /// - `GrantError::NotHolder` if responsible-person delegation no longer holds.
    /// - `GrantError::MachineRefused` if a machine was used as a delegation issuer.
    /// - `GrantError::OperationUnresolved` if an applicable issue or revocation remains uncertain.
    /// - `GrantError::StaleDecision` if the frame, relationships or requested minimum revision is stale.
    /// - `GrantError::PermissionAbsent` if the required holder or resource relationship is absent.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::ModeHeld` if the selected grant requires an approved draft.
    /// - `GrantError::EventMismatch` if recording a one-time use produces an inconsistent event.
    /// - `GrantError::GrantUnknown` if its use event names no recorded grant.
    /// - `GrantError::OperationReused` if a generated use operation is already recorded.
    /// - `GrantError::AppendRefused` if a one-time use is proved not committed.
    /// - `GrantError::Identity` wrapping `IdentityError::RandomSourceUnavailable` if a one-time use operation cannot be generated.
    pub fn check_many_in(
        &mut self,
        frame: &Frame<'_>,
        requests: &[ExerciseRequest],
        at: u64,
    ) -> Result<Vec<Permit>, GrantError> {
        let mut permits = requests
            .iter()
            .map(|request| self.explain_in(frame, request, at))
            .collect::<Result<Vec<_>, _>>()?;
        for permit in &permits {
            self.book
                .grant(permit.grant)
                .map_or(Ok(()), Grant::exercisable)?;
        }
        for (request, permit) in requests.iter().zip(&mut permits) {
            self.finish_check(request, at, permit)?;
        }
        Ok(permits)
    }
}
