//! Grant enforcement against the selected relationship reading.

use super::Judged;
use crate::spicedb::Relationships;
use lys_identity::grants::{ExerciseRequest, GrantError, Grants};
use lys_identity::projection::Projection;
use lys_log_store::LeafStore;

/// Refuse unless the permission engine, where one is named, gives the caller
/// the action on the resource. The grants' own decision is made first and
/// nothing is recorded, so a refusal the grants name is answered by its name.
pub(crate) fn engine_permits<S: LeafStore>(
    grants: &mut Grants<S, Relationships>,
    directory: &Projection,
    request: &ExerciseRequest,
    at: u64,
) -> Result<(), GrantError> {
    if matches!(grants.relationships(), Relationships::Memory(_)) {
        return Ok(());
    }
    let permit = grants.explain(directory, request, at, None)?;
    let Relationships::SpiceDb(engine) = grants.relationships() else {
        return Ok(());
    };
    if engine.check(&request.resource, &request.action, request.caller, at)? {
        return Ok(());
    }
    Err(GrantError::PermissionAbsent {
        grant: permit.grant.to_string(),
    })
}

/// Whether a decision is an exercise, recorded as a use, or a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The caller is about to act: a permit is recorded as a use.
    Exercise,
    /// A question: nothing is recorded.
    Explain,
}

/// [`decide`] for a caller the giving paths judge, except that a caller
/// holding an agent pass was already judged, and its use recorded, by the
/// guarded router on the route's own declared action: judging it again here
/// would record a second use and spend a one-time grant before the act.
pub(crate) fn decide_as<S: LeafStore>(
    actor: &lys_identity::Actor,
    judged: &mut Judged<'_, S>,
    request: &ExerciseRequest,
    at: u64,
    at_least: Option<u64>,
    decision: Decision,
) -> Result<(), GrantError> {
    if matches!(
        actor.provenance().method(),
        lys_identity::AuthMethod::AgentPass(_)
    ) {
        return Ok(());
    }
    decide(judged, request, at, at_least, decision).map(drop)
}

/// The grants' decision on `request`, reaching from its resource to each
/// parent it is placed in that its schema lists, nearest first, answered
/// with the permit and the resource whose grant permits it. The first
/// resource's refusal is the one answered when none permits.
pub(crate) fn decide<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    request: &ExerciseRequest,
    at: u64,
    at_least: Option<u64>,
    decision: Decision,
) -> Result<(lys_identity::grants::Permit, lys_identity::grants::Resource), GrantError> {
    let mut first = None;
    for resource in judged.apps.reach(&request.resource) {
        let asked = ExerciseRequest {
            resource: resource.clone(),
            ..request.clone()
        };
        // A caller admitted by a grant token rests on that token's grant
        // alone; any other grant it holds is never considered.
        let only = crate::agent_signature::token_grant(asked.caller);
        let decided = if let Some(prepared) = judged.prepared {
            prepared.frame(judged, &asked).and_then(|frame| {
                if let Some(required) = at_least
                    && frame.revision() < required
                {
                    return Err(GrantError::StaleDecision {
                        required,
                        projected: frame.revision(),
                    });
                }
                if !prepared.permits(&asked)? {
                    let permit = judged.grants.explain_in_by(&frame, &asked, only, at)?;
                    return Err(GrantError::PermissionAbsent {
                        grant: permit.grant.to_string(),
                    });
                }
                match decision {
                    Decision::Exercise => judged.grants.check_in_by(&frame, &asked, only, at),
                    Decision::Explain => judged.grants.explain_in_by(&frame, &asked, only, at),
                }
            })
        } else {
            engine_permits(judged.grants, judged.directory, &asked, at).and_then(
                |()| match decision {
                    Decision::Exercise => {
                        judged
                            .grants
                            .check_by(judged.directory, &asked, only, at, at_least)
                    }
                    Decision::Explain => {
                        judged
                            .grants
                            .explain_by(judged.directory, &asked, only, at, at_least)
                    }
                },
            )
        };
        match decided {
            Ok(permit) => return Ok((permit, resource)),
            Err(error) => {
                first.get_or_insert(error);
            }
        }
    }
    Err(first.unwrap_or_else(|| GrantError::NotHeld {
        identity: request.caller.to_string(),
        resource: request.resource.to_string(),
        action: request.action.to_string(),
    }))
}
