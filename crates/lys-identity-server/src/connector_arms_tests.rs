//! The server's arms for a connector (DIRECTORY-080 R1, box 12.5), each
//! held to what is right for an app's own identity. A connector is
//! responsible for no agent and answers to its approver by its apps line,
//! never by a reporting edge, so these arms refuse it and say why.

use std::error::Error;

use lys_identity::projection::Projection;
use lys_identity::{AgentId, ConnectorId, IdentityError, IdentityId};

use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;

/// A connector sees no agent's sessions: only the administrator, the
/// person responsible for the agent, or the agent itself does.
#[test]
fn a_connector_sees_no_agents_sessions() -> TestResult {
    let directory = Projection::new();
    let connector = IdentityId::Connector(ConnectorId::generate()?);
    let agent = AgentId::generate()?;
    assert!(!crate::runtime_api::sees(
        &directory,
        false,
        connector,
        &agent.to_string()
    ));
    assert!(crate::runtime_api::sees(
        &directory,
        false,
        IdentityId::Agent(agent),
        &agent.to_string()
    ));
    Ok(())
}

/// Nobody reports to a connector: it is refused as a reporting edge by
/// name, as a service account is.
#[test]
fn a_connector_is_no_reporting_edge() -> TestResult {
    let connector = IdentityId::Connector(ConnectorId::generate()?);
    let refused = crate::reporting_views::edge(connector);
    assert!(
        matches!(
            &refused,
            Err(ServerError::Identity(IdentityError::AnswersToUnknown { identity }))
                if *identity == connector.to_string()
        ),
        "{refused:?}"
    );
    Ok(())
}

/// A connector's text is read as a connector, and a prefix that names no
/// kind of identity is refused by name.
#[test]
fn identities_are_read_by_their_prefix() -> TestResult {
    let connector = ConnectorId::generate()?;
    assert_eq!(
        crate::routes::identity_id(&connector.to_string())?,
        IdentityId::Connector(connector)
    );
    let refused = crate::routes::identity_id("robot-00");
    assert!(
        matches!(
            &refused,
            Err(ServerError::Identity(IdentityError::IdentifierMalformed { text, .. }))
                if text == "robot-00"
        ),
        "{refused:?}"
    );
    Ok(())
}
