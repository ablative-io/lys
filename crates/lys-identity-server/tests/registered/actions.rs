//! Registered writes refuse while Active controls exercise the same handlers.
use super::support::{OTHER, PERSON, TestResult, fixture, inactive, login, stored};
use identity_contract::harness::ADMINISTRATOR;
use lys_identity::OperationId;
use serde_json::json;

async fn service_account_control(subject: &str, active: bool) -> TestResult {
    let (service, person, other) = fixture(subject, active).await?;
    let cookie = service.sign_in(login(subject)).await?;
    let before = stored(&service)?;
    let owner = if subject == ADMINISTRATOR {
        other
    } else {
        person
    };
    let mut body =
        json!({"operation": OperationId::generate()?.to_string(), "name": "lifecycle fixture"});
    if subject == ADMINISTRATOR {
        body["owner"] = json!(owner.to_string());
    }
    let answer = service
        .post("/service-accounts", Some(&cookie), &body)
        .await?;
    if active {
        assert_eq!(answer.0, 200, "{}", answer.1);
        assert_eq!(answer.1["owner"], owner.to_string());
        assert_ne!(stored(&service)?, before, "Active control made no write");
    } else {
        assert_eq!(
            stored(&service)?,
            before,
            "Registered {subject} changed durable bytes"
        );
        inactive(&answer, person);
    }
    Ok(())
}

#[tokio::test]
async fn registered_person_cannot_create_service_accounts() -> TestResult {
    service_account_control(PERSON, false).await
}

#[tokio::test]
async fn registered_admin_cannot_create_service_accounts_for_another_owner() -> TestResult {
    service_account_control(ADMINISTRATOR, false).await
}

#[tokio::test]
async fn active_person_can_create_service_accounts() -> TestResult {
    service_account_control(PERSON, true).await
}

#[tokio::test]
async fn active_admin_can_create_service_accounts_for_another_owner() -> TestResult {
    service_account_control(ADMINISTRATOR, true).await
}

#[tokio::test]
async fn registered_administrator_cannot_register_people_or_activate_itself() -> TestResult {
    let (service, person, _) = fixture(ADMINISTRATOR, false).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let before = stored(&service)?;
    for (path, body) in [
        (
            "/people".to_owned(),
            json!({"operation": OperationId::generate()?.to_string(), "display_name": "must not be registered"}),
        ),
        (
            format!("/identities/{person}/transitions"),
            json!({"operation": OperationId::generate()?.to_string(), "transition": "activate", "reason": "must not self-activate"}),
        ),
    ] {
        let answer = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(
            stored(&service)?,
            before,
            "Registered admin changed bytes at {path}"
        );
        inactive(&answer, person);
    }
    Ok(())
}

#[tokio::test]
async fn registered_person_reads_and_ends_only_own_sessions() -> TestResult {
    let (service, person, _) = fixture(PERSON, false).await?;
    let cookie = service.sign_in(login(PERSON)).await?;
    let other_cookie = service.sign_in(login(OTHER)).await?;
    let other_answer = service.get("/sessions", Some(&other_cookie)).await?;
    assert_eq!(other_answer.0, 200, "{}", other_answer.1);
    let other_id = other_answer.1["sessions"][0]["id"]
        .as_str()
        .ok_or("other session id")?;
    let own_answer = service.get("/sessions", Some(&cookie)).await?;
    assert_eq!(own_answer.0, 200, "{}", own_answer.1);
    assert_eq!(own_answer.1["person"], person.to_string());
    let own = own_answer.1["sessions"].as_array().ok_or("own sessions")?;
    assert_eq!(own.len(), 1);
    assert_eq!(own[0]["login"]["subject"], PERSON);
    assert_ne!(own[0]["id"], other_id);
    let own_id = own[0]["id"].as_str().ok_or("own session id")?;
    let denied = service
        .post(
            &format!("/sessions/{other_id}/end"),
            Some(&cookie),
            &json!({}),
        )
        .await?;
    assert_eq!(denied.1["refusal"], "SessionUnknown", "{}", denied.1);
    assert_eq!(service.get("/me", Some(&other_cookie)).await?.0, 200);
    let ended = service
        .post(
            &format!("/sessions/{own_id}/end"),
            Some(&cookie),
            &json!({}),
        )
        .await?;
    assert_eq!(ended.0, 200, "{}", ended.1);
    assert_eq!(service.get("/me", Some(&cookie)).await?.0, 401);
    Ok(())
}
