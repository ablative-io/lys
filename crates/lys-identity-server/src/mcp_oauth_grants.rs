//! What a person may pass on to an app when they approve it: each live
//! grant they hold that lets them pass actions on to an agent, offered once
//! per relation the permission model defines on its kind whose actions all
//! fall within what may be passed on. What they choose is passed on to the
//! app's agent as their own delegation, use only, so the app holds exactly
//! what was chosen and never the person's own authority.

use lys_identity::grants::{PassOn, RecipientKind};
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
}

impl Offer {
    /// The words the approval page shows for it.
    pub(crate) fn words(&self) -> String {
        format!("{} {} {}", self.relation, self.kind, self.id)
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
            if grant.holder() != holder || record.revoked().is_some() || !grant.window().contains(at)
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

/// Pass each chosen offer on from `person` to `agent`, use only.
pub(crate) fn pass_on(
    state: &AppState,
    person: PersonId,
    agent: &str,
    picked: &[Offer],
) -> Result<(), ServerError> {
    for offer in picked {
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
        with_grants(state, |judged| {
            let request = body.request(IdentityId::Person(person))?;
            judged.apps.admit_kind(None, &offer.kind)?;
            judged.grants.delegate(judged.directory, &request, now())?;
            Ok(())
        })?;
    }
    Ok(())
}
