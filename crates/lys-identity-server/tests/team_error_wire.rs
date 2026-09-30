#![cfg(test)]
//! Team refusals retain their status, name and complete response body.

use std::error::Error;

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use lys_identity_server::error::ServerError;
use lys_identity_server::error_team::TeamError;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

async fn assert_wire(
    error: ServerError,
    status: StatusCode,
    refusal: &str,
    reason: &str,
) -> TestResult {
    let response = error.into_response();
    assert_eq!(response.status(), status, "{refusal}");
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let expected = serde_json::to_vec(&json!({
        "refusal": refusal,
        "reason": reason,
        "fields": [],
    }))?;
    assert_eq!(body.as_ref(), expected.as_slice(), "{refusal}");
    Ok(())
}

#[tokio::test]
async fn unavailable() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::Unavailable {
            reason: "the team log could not be read".to_owned(),
        }),
        StatusCode::SERVICE_UNAVAILABLE,
        "TeamsUnavailable",
        "TeamsUnavailable: the team log could not be read",
    )
    .await
}

#[tokio::test]
async fn unknown_team() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::Unknown),
        StatusCode::NOT_FOUND,
        "TeamUnknown",
        "TeamUnknown: no team by that id was ever created",
    )
    .await
}

#[tokio::test]
async fn reused_change() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::Reused { operation: "change-1".to_owned() }),
        StatusCode::CONFLICT,
        "TeamReused",
        "TeamReused: operation `change-1` already names a team act in other words: send this act under a new operation id",
    ).await
}

#[tokio::test]
async fn retired_team() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::Retired {
            team: "team-1".to_owned(),
        }),
        StatusCode::CONFLICT,
        "TeamRetired",
        "TeamRetired: team `team-1` is retired and takes no more changes",
    )
    .await
}

#[tokio::test]
async fn member_already_held() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::MemberHeld),
        StatusCode::CONFLICT,
        "TeamMemberHeld",
        "TeamMemberHeld: that member is already in the team",
    )
    .await
}

#[tokio::test]
async fn absent_member() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::MemberAbsent),
        StatusCode::CONFLICT,
        "TeamMemberAbsent",
        "TeamMemberAbsent: that member is not in the team",
    )
    .await
}

#[tokio::test]
async fn parent_cycle() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::ParentCycle {
            team: "team-1".to_owned(),
            parent: "team-2".to_owned(),
        }),
        StatusCode::CONFLICT,
        "team_parent_cycle",
        "team_parent_cycle: team `team-1` cannot have parent `team-2` because that closes a cycle",
    )
    .await
}

#[tokio::test]
async fn lead_is_not_a_member() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::LeadNotMember {
            team: "team-1".to_owned(),
            lead: "agent-1".to_owned(),
        }),
        StatusCode::CONFLICT,
        "team_lead_not_member",
        "team_lead_not_member: agent `agent-1` is not an admitted member of team `team-1`",
    )
    .await
}

#[tokio::test]
async fn unknown_member() -> TestResult {
    assert_wire(
        ServerError::Team(TeamError::MemberUnknown),
        StatusCode::NOT_FOUND,
        "TeamMemberUnknown",
        "TeamMemberUnknown: a member is a person or agent the directory holds and has not retired",
    )
    .await
}
