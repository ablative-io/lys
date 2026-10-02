//! What a person may pass on to an app when they approve it: each live
//! grant they hold that lets them pass actions on to an agent, offered once
//! per relation the permission model defines on its kind whose actions all
//! fall within what may be passed on. What they choose is passed on to the
//! app's agent as their own delegation, use only, so the app holds exactly
//! what was chosen and never the person's own authority.

use lys_identity::grants::{GrantId, PassOn, RecipientKind, RevokeRequest, Route};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::json;

use crate::error::ServerError;
use crate::grant_contract::DelegateBody;
use crate::grants::with_grants;
use crate::routes::AppState;
use crate::session::now;

/// One thing a person may pass on to an app.
pub(crate) struct Offer {
    pub(crate) key: String,
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) relation: String,
    /// What the app may then do, each action's name read as words.
    pub(crate) actions: Vec<String>,
}

impl Offer {
    /// The words the approval page shows for it: what the app may do, and
    /// to which thing, so a person reads the effect rather than a code.
    pub(crate) fn words(&self) -> String {
        let doing = match self.actions.as_slice() {
            [] => "nothing".to_owned(),
            [only] => only.clone(),
            [first @ .., last] => format!("{} and {last}", first.join(", ")),
        };
        format!("Let it {doing} the {} called {}", self.kind, self.id)
    }
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
                if allowed.is_subset(actions) {
                    offers.push(Offer {
                        key: format!("{}:{}", grant.id(), relation.as_str()),
                        source: grant.id().to_string(),
                        kind: resource.kind().to_owned(),
                        id: resource.id().to_owned(),
                        relation: relation.as_str().to_owned(),
                        actions: allowed
                            .iter()
                            .map(|action| action.as_str().replace(['.', '_', '-'], " "))
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
    let mut offered = offers(state, person)?;
    chosen
        .iter()
        .map(|key| {
            offered
                .iter()
                .position(|offer| offer.key == *key)
                .map(|at| offered.swap_remove(at))
                .ok_or_else(|| ServerError::RequestMalformed {
                    reason: "a chosen permission is not one this person may pass on".to_owned(),
                })
        })
        .collect()
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
    use super::Offer;

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
            offer(&["read"]).words(),
            "Let it read the project called lys"
        );
        assert_eq!(
            offer(&["read", "write", "grant revoke"]).words(),
            "Let it read, write and grant revoke the project called lys"
        );
        assert!(!offer(&["read"]).words().contains("viewer"));
    }
}
