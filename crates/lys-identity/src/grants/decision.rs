//! Single decisions retain their explanation and record an exercise once.

use super::usage;
use super::{
    ExerciseRequest, Frame, Grant, GrantError, GrantId, Grants, Permit, RelationshipStore,
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
        let used = self.record_use(request.caller, permit.grant, request.route, at);
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
