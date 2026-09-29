#![cfg(test)]
//! An operator can administer an install but cannot mint a personal session,
//! in either memory-only or persisted-session configurations.

use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::error::ServerError;
use lys_identity_server::session::Sessions;
use std::error::Error;

#[test]
fn operator_authority_cannot_be_converted_into_a_personal_session() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let memory = Sessions::new(3600, false);
    let persistent = Sessions::open(path.clone(), 3600, false)?;
    for sessions in [memory, persistent] {
        let actor = Actor::new(
            LoginBinding::new("https://issuer.example", "admin")?,
            Provenance::new(AuthMethod::Operator, 1),
        );
        let error = sessions
            .begin(actor)
            .err()
            .ok_or("operator became a personal session")?;
        assert!(
            matches!(error, ServerError::OperatorRefused { .. }),
            "{error}"
        );
        assert!(!path.exists(), "refusal must not write a session file");
    }
    Ok(())
}
