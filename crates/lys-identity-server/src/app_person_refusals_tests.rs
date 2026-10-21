use super::super::{Access, held};
use super::Table;
use super::fixture::{TestResult, acts, operation, refused};
use axum::http::{HeaderMap, header};
use lys_identity::{OperationId, Transition};
use serde_json::json;

#[tokio::test]
async fn a_persons_app_pass_never_administers_another_persons_agent_even_for_the_administrator()
-> TestResult {
    for administrator in [false, true] {
        let mut table = Table::app_fixture().await?;
        if administrator {
            table.administrator()?;
        }
        let pass = table.pass("cambium", false, true)?;
        refused(&table.send("POST", "/agents", &pass,
            &json!({"operation": operation()?, "display_name": "Other", "answers_to": table.other_person()?}), &[]).await?,
            403, "NotAdmitted");
        refused(
            &table
                .send(
                    "POST",
                    &format!("/identities/{}/transitions", table.other),
                    &pass,
                    &json!({"operation": operation()?, "transition": "activate"}),
                    &[],
                )
                .await?,
            403,
            "NotAdmitted",
        );
        refused(
            &table
                .send(
                    "POST",
                    &format!("/agents/{}/certificates", table.other),
                    &pass,
                    &json!({"operation": operation()?, "request": "invalid"}),
                    &[],
                )
                .await?,
            404,
            "AgentNotVisible",
        );
        assert_eq!(table.state_of(&table.other)?, "active");
    }
    Ok(())
}

#[tokio::test]
async fn an_app_pass_cannot_make_any_other_transition_or_activate_a_person() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    for transition in ["suspend", "reinstate", "retire", "unknown"] {
        refused(&table.send("POST", &format!("/identities/{}/transitions", table.own), &pass,
            &json!({"operation": operation()?, "transition": transition, "reason": "not permitted"}), &[]).await?,
            403, "NotAdmitted");
    }
    refused(
        &table
            .send(
                "POST",
                &format!("/identities/{}/transitions", table.person),
                &pass,
                &json!({"operation": operation()?, "transition": "activate"}),
                &[],
            )
            .await?,
        403,
        "NotAdmitted",
    );
    assert_eq!(table.state_of(&table.own)?, "active");
    Ok(())
}

#[tokio::test]
async fn an_app_pass_only_activates_a_registered_agent() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    for next in [Some(Transition::Suspend), Some(Transition::Retire), None] {
        refused(
            &table
                .send(
                    "POST",
                    &format!("/identities/{}/transitions", table.own),
                    &pass,
                    &json!({"operation": operation()?, "transition": "activate"}),
                    &[],
                )
                .await?,
            409,
            "TransitionRefused",
        );
        if let Some(next) = next {
            let actor = table
                .state
                .sessions
                .live(|_| true)?
                .into_iter()
                .find(|entry| entry.id == table.session)
                .ok_or("session absent")?
                .actor;
            crate::routes::with_directory(&table.state, |directory| {
                directory.transition(
                    actor,
                    OperationId::generate()?,
                    crate::routes::identity_id(&table.own)?,
                    next,
                    "lifecycle fixture",
                    crate::session::now(),
                )?;
                Ok(())
            })?;
        }
    }
    assert_eq!(table.state_of(&table.own)?, "retired");
    Ok(())
}

#[tokio::test]
async fn a_revoked_or_unknown_app_pass_is_refused_on_all_three_acts() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    table.revoke(&pass)?;
    for credential in [pass, "unknown".to_owned()] {
        for (path, body) in acts(&table.own)? {
            refused(
                &table.send("POST", &path, &credential, &body, &[]).await?,
                401,
                "TokenUnknown",
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn an_ended_issuing_session_is_token_unknown_on_all_three_acts() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    table.state.sessions.end(&table.session, |_| true)?;
    for (path, body) in acts(&table.own)? {
        refused(
            &table.send("POST", &path, &pass, &body, &[]).await?,
            401,
            "TokenUnknown",
        );
    }
    Ok(())
}

#[tokio::test]
async fn an_app_pass_with_a_cookie_is_refused_on_all_three_acts() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    let cookie = table.cookie()?;
    for (path, body) in acts(&table.own)? {
        let answer = table
            .send("POST", &path, &pass, &body, &[("cookie", &cookie)])
            .await?;
        refused(&answer, 401, "PassRefused");
        assert!(
            answer.1["reason"]
                .as_str()
                .ok_or("reason absent")?
                .contains("mixed_credentials")
        );
    }
    Ok(())
}

#[tokio::test]
async fn an_agent_holder_pass_is_refused_on_all_three_acts() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass_for("cambium", &table.own, false, false)?;
    for (path, body) in acts(&table.own)? {
        refused(
            &table.send("POST", &path, &pass, &body, &[]).await?,
            403,
            "ProductPassPersonRequired",
        );
    }
    Ok(())
}

#[tokio::test]
async fn expired_or_tampered_passes_are_refused_without_using_a_cookie() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    let (header, rest) = pass.split_once('.').ok_or("pass shape")?;
    let (payload, signature) = rest.split_once('.').ok_or("pass shape")?;
    let changed = format!(
        "{}{}",
        if signature.starts_with('A') { "B" } else { "A" },
        &signature[1..]
    );
    let tampered = format!("{header}.{payload}.{changed}");
    for credential in [table.pass("cambium", true, false)?, tampered] {
        for (path, body) in acts(&table.own)? {
            refused(
                &table.send("POST", &path, &credential, &body, &[]).await?,
                401,
                "PassRefused",
            );
            let cookie = table.cookie()?;
            refused(
                &table
                    .send("POST", &path, &credential, &body, &[("cookie", &cookie)])
                    .await?,
                401,
                "PassRefused",
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn an_unapproved_unknown_or_retired_app_is_refused_on_all_three_acts() -> TestResult {
    use crate::apps_state::{By, Decided, Line};
    let table = Table::app_fixture().await?;
    table.register_app("pending_app", false)?;
    crate::apps_api::with_apps(&table.state, |apps, _| {
        apps.keep(Line::Retired(Decided {
            operation: "retire-cambium".to_owned(),
            app: "cambium".to_owned(),
            reason: "no longer approved".to_owned(),
            by: By::Start,
            at: 3,
        }))?;
        Ok(())
    })?;
    for (app, name) in [
        ("pending_app", "app_not_approved"),
        ("unknown_app", "credential_refused"),
        ("cambium", "app_retired"),
    ] {
        let pass = table.pass(app, false, true)?;
        for (path, body) in acts(&table.own)? {
            let answer = table.send("POST", &path, &pass, &body, &[]).await?;
            assert_eq!(answer.1["refusal"], name, "{}", answer.1);
            assert_ne!(answer.0, 200);
        }
    }
    Ok(())
}

#[tokio::test]
async fn an_app_pass_never_becomes_an_administrator_pass_on_other_routes() -> TestResult {
    let mut table = Table::app_fixture().await?;
    table.administrator()?;
    let pass = table.pass("cambium", false, true)?;
    refused(
        &table.get("/directory/people", &pass).await?,
        401,
        "NotSignedIn",
    );
    for path in [
        format!("/agents/{}/certificates", table.own),
        "/me".to_owned(),
    ] {
        refused(
            &table
                .send("GET", &path, &pass, &serde_json::Value::Null, &[])
                .await?,
            401,
            "NotSignedIn",
        );
    }
    Ok(())
}

#[tokio::test]
async fn an_app_pass_uses_its_issuing_sessions_actor_and_never_another_persons_session()
-> TestResult {
    use sha2::{Digest, Sha256};
    let mut table = Table::app_fixture().await?;
    let person = table.person;
    let pass = table.pass("cambium", false, true)?;
    let expected = table
        .state
        .sessions
        .live(|_| true)?
        .into_iter()
        .find(|entry| entry.id == table.session)
        .ok_or("session absent")?
        .actor;
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, format!("Bearer {pass}").parse()?);
    let actor = crate::provider::app_person_actor(&table.state, &headers)?.ok_or("actor absent")?;
    assert_eq!(actor, expected);
    table.administrator()?;
    table.revoke(&pass)?;
    let provider = table.state.provider.as_ref().ok_or("provider absent")?;
    held(&provider.tokens)?.insert(
        crate::routes::hex(&Sha256::digest(pass.as_bytes())),
        Access {
            session_id: table.session.clone(),
            subject: person.to_string(),
            app: "cambium".to_owned(),
            profile: false,
            expires_at: crate::session::now() + 60,
            kind: None,
        },
        crate::session::now(),
    )?;
    for (path, body) in acts(&table.own)? {
        refused(
            &table.send("POST", &path, &pass, &body, &[]).await?,
            401,
            "TokenUnknown",
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_persons_app_pass_cannot_accompany_an_operator_header() -> TestResult {
    let table = Table::app_fixture().await?;
    let pass = table.pass("cambium", false, true)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, format!("Bearer {pass}").parse()?);
    headers.insert("lys-operator", "fixture".parse()?);
    let error = crate::provider::app_person_actor(&table.state, &headers)
        .err()
        .ok_or("mixed credentials admitted")?;
    assert_eq!(error.name(), "PassRefused");
    assert!(error.to_string().contains("mixed_credentials"));
    Ok(())
}

#[tokio::test]
async fn an_app_pass_admits_only_with_a_first_grant_on_its_own_kind() -> TestResult {
    let mut table = Table::app_fixture().await?;
    table.administrator()?;
    let pass = table.pass("cambium", false, true)?;
    let admit = |grant: serde_json::Value| -> TestResult<serde_json::Value> {
        Ok(json!({"operation": operation()?, "display_name": "Bea Example",
            "email": "bea@example.test", "grant": grant}))
    };
    let other = admit(json!({"route": "api", "resource": {"kind": "fixture_product.agent", "id": "a-1"}, "relation": "viewer"}))?;
    let answer = table.send("POST", "/people/admit", &pass, &other, &[]).await?;
    refused(&answer, 403, "not_your_app");
    assert_eq!(answer.1["receipt"]["completed"], json!([]), "{}", answer.1);
    let answer = table.send("POST", "/people/admit", &pass, &admit(serde_json::Value::Null)?, &[]).await?;
    refused(&answer, 400, "app_admit_grant_required");
    assert_eq!(answer.1["receipt"]["completed"], json!([]), "{}", answer.1);
    let undeclared = admit(json!({"route": "api", "resource": {"kind": "cambium.workspace", "id": "w-1"}, "relation": "member"}))?;
    refused(&table.send("POST", "/people/admit", &pass, &undeclared, &[]).await?, 403, "kind_not_registered");
    Ok(())
}
