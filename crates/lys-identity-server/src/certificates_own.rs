//! Each AI's one certificate. Lys issues an AI its certificate when Lys
//! starts it and it holds none that stands, over a key Lys makes for that
//! certificate and keeps in its own store; the key never goes to the machine
//! the AI runs on. The certificate is the AI's identity only: what it may do
//! is its grants. One that would lapse within [`RENEW_WITHIN`] is replaced at
//! the next start and withdrawn, so the AI stands on one certificate; a
//! replacement leaves the AI's runs running, while a person's withdrawal
//! ends them.

use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_core::ca::create_certificate_request;
use lys_identity::{AgentId, IdentityId, OperationId};

use crate::agent_sight::SeenAgent;
use crate::certificates_issue::{VALID_FOR, issued, with_store};
use crate::certificates_store::Withdrawn;
use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::session::now;

/// How long before it lapses a certificate is replaced at a start.
pub(crate) const RENEW_WITHIN: u64 = 7 * 24 * 60 * 60;

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::CertificatesUnavailable {
        reason: what.to_string(),
    }
}

/// The serials of `agent`'s certificates still standing, each with when it
/// was issued.
fn standing_of(state: &AppState, agent: &str) -> Result<Vec<(String, u64)>, ServerError> {
    with_store(state, |store| {
        Ok(store
            .certificates()
            .filter(|entered| entered.issued.agent == agent && entered.withdrawn.is_none())
            .map(|entered| (entered.issued.serial.clone(), entered.issued.issued_at))
            .collect())
    })
}

/// The newest of `standing` that stands past [`RENEW_WITHIN`] from `at`.
fn lasting(standing: &[(String, u64)], at: u64) -> Option<String> {
    standing
        .iter()
        .filter(|(_, issued_at)| {
            issued_at.saturating_add(VALID_FOR.as_secs()) > at.saturating_add(RENEW_WITHIN)
        })
        .max_by_key(|(_, issued_at)| *issued_at)
        .map(|(serial, _)| serial.clone())
}

/// The serial of the certificate `agent` stands on, issued first when it
/// holds none that stands past [`RENEW_WITHIN`]; any it held before are then
/// withdrawn as replaced.
pub(crate) fn standing(state: &AppState, agent: AgentId) -> Result<String, ServerError> {
    let name = agent.to_string();
    let at = now();
    let before = standing_of(state, &name)?;
    if let Some(serial) = lasting(&before, at) {
        return Ok(serial);
    }
    let responsible = with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?
            .responsible())
    })?
    .ok_or(ServerError::AgentNotVisible)?;
    let serial = OperationId::generate()?.to_string();
    let path = with_store(state, |store| Ok(store.agent_key(&name, &serial)))?
        .ok_or_else(|| unavailable("the certificate log is kept where no agent key can be"))?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&path).map_err(unavailable)?);
    let request = create_certificate_request(&key, &name).map_err(unavailable)?;
    let seen = SeenAgent {
        agent,
        responsible: Some(responsible),
    };
    let made = issued(state, &seen, &request, serial.clone(), at)?;
    with_store(state, |store| {
        // A start beside this one may have issued while the claims were
        // gathered; its certificate is the one stood on, and this one is
        // not entered.
        let now_standing: Vec<(String, u64)> = store
            .certificates()
            .filter(|entered| entered.issued.agent == name && entered.withdrawn.is_none())
            .map(|entered| (entered.issued.serial.clone(), entered.issued.issued_at))
            .collect();
        if let Some(serial) = lasting(&now_standing, at) {
            return Ok(serial);
        }
        store.issue(made)?;
        for (replaced, _) in &now_standing {
            store.withdraw(Withdrawn {
                serial: replaced.clone(),
                by: responsible.to_string(),
                reason: format!("Lys replaced it before it lapsed, with {serial}"),
                withdrawn_at: at,
            })?;
        }
        Ok(serial.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::{RENEW_WITHIN, VALID_FOR, lasting};

    #[test]
    fn a_certificate_near_its_end_is_not_stood_on_and_the_newest_lasting_one_is() {
        let valid = VALID_FOR.as_secs();
        let at = 10 * valid;
        let near_end = at + RENEW_WITHIN - valid;
        let fresh = at - 60;
        let older = at - 120;
        assert_eq!(lasting(&[("near".to_owned(), near_end)], at), None);
        assert_eq!(
            lasting(
                &[
                    ("older".to_owned(), older),
                    ("fresh".to_owned(), fresh),
                    ("near".to_owned(), near_end)
                ],
                at
            ),
            Some("fresh".to_owned())
        );
        assert_eq!(lasting(&[], at), None);
    }
}
