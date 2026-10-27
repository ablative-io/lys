//! Organisation standing behind an agent's act (24 October 2026). A grant
//! of `read` on `person:all`, or of `agent.create` on `agent:all`, is one
//! way an agent is admitted to a route; the other is its standing in its
//! own organisation: `organisation.read` admits it to every read within the
//! organisation, `organisation.admit` to registering a person,
//! `organisation.grant` to the grant acts and `organisation.administer` to
//! every other act, as [`standing_for`] names them. The door
//! (`route_actions`) and the handler-judged acts (`agent_acts`) ask the
//! agent's own grant first and its standing second, and record the use on
//! whichever admits. Standing reaches Lys's own kinds only: an approved
//! app's kinds are reached by the app's schema and the grants on them, and
//! the organisation itself by its own acts. A person or agent of another
//! organisation is never reached: the question is not asked, so the refusal
//! names nothing of them.

use std::str::FromStr;

use lys_identity::grants::shipped::{WITHHELD_FROM_AGENTS, standing_for};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, PersonId};

use crate::error::ServerError;

/// Whether standing can reach `kind`: Lys's own kinds, never an app's
/// (`{app}.{kind}`) and never the organisation, which its own acts reach.
#[must_use]
pub(crate) fn covers(kind: &str) -> bool {
    !kind.contains('.') && kind != "organisation"
}

/// The standing question behind `action` on `resource` for `agent`: the
/// standing act on the agent's own organisation. None when standing does
/// not reach the kind, the act is one no agent may be given, the agent is
/// of no organisation, or the resource is a person, a person's account or
/// an agent of another.
///
/// # Errors
/// Refuses an organisation id or standing act that is not a well-formed
/// resource or action name.
pub(crate) fn behind(
    directory: &Projection,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
) -> Result<Option<ExerciseRequest>, ServerError> {
    if !covers(resource.kind()) || WITHHELD_FROM_AGENTS.contains(&action.as_str()) {
        return Ok(None);
    }
    let Some(organisation) = directory.organisation_of(IdentityId::Agent(agent)) else {
        return Ok(None);
    };
    if let Some(identity) = named(resource)
        && directory.organisation_of(identity) != Some(organisation)
    {
        return Ok(None);
    }
    Ok(Some(ExerciseRequest {
        caller: IdentityId::Agent(agent),
        route: Route::Api,
        resource: Resource::new("organisation", &organisation.to_string())?,
        action: Action::new(standing_for(action.as_str()))?,
    }))
}

/// The person or agent `resource` names, when its id is one: an account is
/// its person's.
fn named(resource: &Resource) -> Option<IdentityId> {
    match resource.kind() {
        "person" | "account" => PersonId::from_str(resource.id())
            .ok()
            .map(IdentityId::Person),
        "agent" => AgentId::from_str(resource.id()).ok().map(IdentityId::Agent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use lys_identity::grants::Resource;
    use lys_identity::{IdentityId, PersonId};

    use super::{covers, named};

    #[test]
    fn an_account_is_its_persons_so_another_organisations_is_never_reached()
    -> Result<(), Box<dyn std::error::Error>> {
        let person = PersonId::generate()?;
        for kind in ["person", "account"] {
            let resource = Resource::new(kind, &person.to_string())?;
            assert_eq!(named(&resource), Some(IdentityId::Person(person)), "{kind}");
        }
        assert_eq!(named(&Resource::new("machine", &person.to_string())?), None);
        Ok(())
    }

    #[test]
    fn standing_reaches_lys_kinds_and_neither_an_apps_nor_the_organisation() {
        for kind in ["person", "agent", "grant", "machine", "role", "request"] {
            assert!(covers(kind), "{kind}");
        }
        for kind in ["notes.workspace", "docs.page", "organisation"] {
            assert!(!covers(kind), "{kind}");
        }
    }
}
