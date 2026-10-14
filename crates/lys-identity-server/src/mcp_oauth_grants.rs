//! What a person may pass on to an app when they approve it: each live
//! grant they hold that lets them pass actions on to an agent, offered once
//! per relation the permission model defines on its kind whose actions all
//! fall within what may be passed on. What they choose is passed on to the
//! app's agent as their own delegation, use only, so the app holds exactly
//! what was chosen and never the person's own authority.

use lys_identity::grants::{GrantId, Model, PassOn, RecipientKind, RevokeRequest, Route};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::json;

use crate::error::ServerError;
use crate::grant_contract::DelegateBody;
use crate::grants::with_grants;
use crate::routes::AppState;
use crate::session::now;

#[path = "mcp_oauth_choices.rs"]
mod choices;
pub(crate) use choices::grouped;

/// One thing a person may pass on to an app.
pub(crate) struct Offer {
    pub(crate) key: String,
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) relation: String,
    /// The declared actions, retained exactly for consent binding.
    pub(crate) actions: Vec<String>,
}

pub(super) fn words(kind: &str, id: &str, actions: &[String]) -> String {
    let actions: Vec<_> = actions
        .iter()
        .map(|action| action.replace(['.', '_', '-'], " "))
        .collect();
    let doing = match actions.as_slice() {
        [] => "nothing".to_owned(),
        [only] => only.clone(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    };
    format!("Let it {doing} the {kind} called {id}")
}

/// Every offer `person` may make to an app now.
pub(crate) fn offers(state: &AppState, person: PersonId) -> Result<Vec<Offer>, ServerError> {
    let holder = IdentityId::Person(person);
    let model = state.grant_setup.model()?;
    let at = now();
    with_grants(state, |judged| {
        let mut offers = Vec::new();
        for record in judged.grants.book().records() {
            let grant = record.grant();
            if grant.holder() != holder
                || record.revoked().is_some()
                || !grant.window().contains(at)
            {
                continue;
            }
            let PassOn::To {
                actions,
                recipients,
            } = grant.pass_on()
            else {
                continue;
            };
            if !recipients.contains(&RecipientKind::Agent) {
                continue;
            }
            let resource = grant.resource();
            for (relation, allowed) in model.relations_on(resource.kind()) {
                if allowed.is_subset(actions)
                    && allowed
                        .iter()
                        .all(|action| model.agent_may_hold(resource.kind(), action))
                {
                    offers.push(Offer {
                        key: format!("{}:{}", grant.id(), relation.as_str()),
                        source: grant.id().to_string(),
                        kind: resource.kind().to_owned(),
                        id: resource.id().to_owned(),
                        relation: relation.as_str().to_owned(),
                        actions: allowed
                            .iter()
                            .map(|action| action.as_str().to_owned())
                            .collect(),
                    });
                }
            }
        }
        Ok(offers)
    })
}

/// The person's offers named by `chosen`, refused whole when any key is
/// not one of the person's offers now, so nothing is made for a forged
/// choice.
pub(crate) fn chosen(
    state: &AppState,
    person: PersonId,
    chosen: &[String],
) -> Result<Vec<Offer>, ServerError> {
    if chosen.is_empty() {
        return Ok(Vec::new());
    }
    let mut offered = grouped(offers(state, person)?)?;
    chosen
        .iter()
        .map(|key| {
            offered
                .iter()
                .position(|offer| offer.key == *key)
                .map(|at| offered.swap_remove(at).pick())
                .ok_or_else(|| ServerError::RequestMalformed {
                    reason: "a chosen permission is not one this person may pass on".to_owned(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map(|groups| groups.into_iter().flatten().collect())
}

/// Pass every chosen offer on from `person` to `agent`, use only, or none:
/// every delegation is formed before any is made, all are made under one
/// hold of the grants, and when one is refused those already made are
/// revoked before the refusal is answered.
pub(crate) fn pass_on(
    state: &AppState,
    person: PersonId,
    agent: &str,
    picked: &[Offer],
) -> Result<(), ServerError> {
    let caller = IdentityId::Person(person);
    let requests = picked
        .iter()
        .map(|offer| {
            let body: DelegateBody = serde_json::from_value(json!({
                "operation": OperationId::generate()?.to_string(),
                "route": "browser",
                "source": offer.source,
                "recipient": agent,
                "responsible": person.to_string(),
                "resource": {"kind": offer.kind, "id": offer.id},
                "relation": offer.relation,
                "pass_on": {"kind": "use_only"},
                "window": {"starts_at": 0, "ends_at": null},
            }))
            .map_err(|error| ServerError::RequestMalformed {
                reason: format!("a chosen permission does not form a delegation: {error}"),
            })?;
            Ok((offer.kind.as_str(), body.request(caller)?))
        })
        .collect::<Result<Vec<_>, ServerError>>()?;
    with_grants(state, |judged| {
        if picked
            .iter()
            .any(|offer| !still_offered(judged.grants.model(), offer))
        {
            return Err(ServerError::RequestMalformed {
                reason:
                    "the chosen permissions changed; connect again and choose what the app may do"
                        .to_owned(),
            });
        }
        let mut made = Vec::new();
        for (kind, request) in &requests {
            let passed = judged
                .apps
                .admit_kind(None, kind)
                .map_err(ServerError::from)
                .and_then(|()| {
                    judged
                        .grants
                        .delegate(judged.directory, request, now())
                        .map_err(ServerError::from)
                });
            match passed {
                Ok(recorded) => made.push(recorded.event.grant()),
                Err(refused) => return withdrawn(judged.grants, caller, &made, refused),
            }
        }
        Ok(())
    })
}

fn still_offered(model: &Model, offer: &Offer) -> bool {
    model.relations_on(&offer.kind).any(|(relation, actions)| {
        relation.as_str() == offer.relation
            && actions.len() == offer.actions.len()
            && actions.iter().all(|action| {
                offer.actions.iter().any(|chosen| chosen == action.as_str())
                    && model.agent_may_hold(&offer.kind, action)
            })
    })
}

/// Revoke each grant `made` so far and answer `refused`, or a refusal
/// naming the grants still standing when a revocation is itself refused.
fn withdrawn(
    grants: &mut crate::grants::GrantState,
    caller: IdentityId,
    made: &[GrantId],
    refused: ServerError,
) -> Result<(), ServerError> {
    let mut standing = Vec::new();
    for grant in made {
        let revoked = OperationId::generate().map_err(ServerError::from).and_then(|operation| {
            grants
                .revoke(
                    &RevokeRequest {
                        operation,
                        caller,
                        route: Route::Browser,
                        grant: *grant,
                        reason: "the app's approval was refused: every chosen permission is passed on or none".to_owned(),
                    },
                    now(),
                )
                .map_err(ServerError::from)
        });
        if let Err(error) = revoked {
            standing.push(format!("{grant} ({error})"));
        }
    }
    if standing.is_empty() {
        return Err(refused);
    }
    Err(ServerError::ConfigInvalid {
        reason: format!(
            "the app's approval was refused ({refused}) and these permissions it was given could not be taken back: {}",
            standing.join(", ")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{Offer, still_offered, words};
    use lys_identity::grants::{Action, Model, Relation};

    #[test]
    fn a_relation_cannot_gain_acts_between_consent_and_delegation()
    -> Result<(), Box<dyn std::error::Error>> {
        let relation = Relation::new("viewer")?;
        let read = Action::new("read")?;
        let write = Action::new("write")?;
        let before = Model::new(1, [(relation.clone(), [read.clone()].into())])?;
        let after = Model::new(2, [(relation, [read, write].into())])?;
        let chosen = offer(&["read"]);
        assert!(still_offered(&before, &chosen));
        assert!(!still_offered(&after, &chosen));
        Ok(())
    }

    fn offer(actions: &[&str]) -> Offer {
        Offer {
            key: "g:viewer".to_owned(),
            source: "g".to_owned(),
            kind: "project".to_owned(),
            id: "lys".to_owned(),
            relation: "viewer".to_owned(),
            actions: actions.iter().map(|action| (*action).to_owned()).collect(),
        }
    }

    #[test]
    fn an_offer_reads_as_what_the_app_may_do_to_which_thing() {
        assert_eq!(
            words("project", "lys", &offer(&["read"]).actions),
            "Let it read the project called lys"
        );
        assert_eq!(
            words(
                "project",
                "lys",
                &offer(&["read", "write", "grant revoke"]).actions
            ),
            "Let it read, write and grant revoke the project called lys"
        );
        assert!(!words("project", "lys", &offer(&["read"]).actions).contains("viewer"));
    }
}
