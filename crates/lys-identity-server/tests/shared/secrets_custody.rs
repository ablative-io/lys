#![cfg(test)]

//! Application approval and secret custody retain refusal and replay evidence.

use super::*;

#[tokio::test]
async fn only_administrator_can_save_a_current_app_secret_and_the_broker_gets_a_signed_body()
-> TestResult {
    use identity_contract::apps::{Auth, ok, op, post, registration, workspace_schema};
    use identity_contract::harness::ADMINISTRATOR;
    let setup = setup_with_person(ADMINISTRATOR).await?;
    let admin = setup.service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = setup.service.sign_in(login(BEA)).await?;
    let app = "fixture_save";
    ok(post(
        &setup.service,
        "/apps",
        Auth::Cookie(&admin),
        &registration(app, &workspace_schema(app))?,
    )
    .await?)?;
    let approval_operation = op()?;
    let approved = ok(post(
        &setup.service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation":approval_operation}),
    )
    .await?)?;
    assert!(approved["client"].is_null());
    assert_eq!(approved["credentials"]["app"], app);
    // The stand-in has custody of this fixed fixture value; it was not returned
    // by approval. Manual save remains available for already-issued credentials.
    let fixture = "ab".repeat(32);
    let secret = fixture.as_str();
    let prepared = received(&setup.log);
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].path, "/_lys/apps/prepare");
    signed_as_received(&prepared[0], &setup.key)?;
    assert!(!String::from_utf8_lossy(&prepared[0].body).contains(secret));
    setup.log.lock().expect("fixture lock poisoned").clear();
    let replay = ok(post(
        &setup.service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation":approval_operation}),
    )
    .await?)?;
    assert!(replay["client"].is_null());
    assert!(!replay.to_string().contains(secret));
    assert!(
        received(&setup.log).is_empty(),
        "replay must not prepare again"
    );
    let path = format!("/apps/{app}/credentials/save");
    let body = json!({"client_secret":secret});
    assert_eq!(
        post(&setup.service, &path, Auth::Cookie(&bea), &body)
            .await?
            .0,
        403
    );
    assert_ne!(
        post(
            &setup.service,
            &path,
            Auth::Cookie(&admin),
            &json!({"client_secret":"wrong"})
        )
        .await?
        .0,
        200
    );
    let malformed = post(
        &setup.service,
        &path,
        Auth::Cookie(&admin),
        &json!({"client_secret": secret, "owner":"person-other"}),
    )
    .await?;
    assert_eq!(malformed.0, 400);
    assert_eq!(malformed.1["refusal"], "RequestMalformed");
    assert!(!malformed.1.to_string().contains(secret));
    assert!(
        received(&setup.log).is_empty(),
        "refusals never reached the broker"
    );
    let answer = ok(post(&setup.service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert!(!answer.to_string().contains(secret));
    let calls = received(&setup.log);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/_lys/apps/save");
    signed_as_received(&calls[0], &setup.key)?;
    let saved: Value = serde_json::from_slice(&calls[0].body)?;
    assert_eq!(saved["app"], app);
    assert_eq!(saved["client_secret"], secret);
    assert!(
        saved.get("owner").is_none(),
        "browser cannot supply an owner"
    );
    Ok(())
}

#[tokio::test]
async fn approval_with_broker_down_keeps_app_pending() -> TestResult {
    use identity_contract::apps::{Auth, get, ok, op, post, registration, workspace_schema};
    use identity_contract::harness::ADMINISTRATOR;
    let setup = setup_with_broker(ADMINISTRATOR, false).await?;
    let admin = setup.service.sign_in(login(ADMINISTRATOR)).await?;
    let app = "fixture_save_unavailable";
    ok(post(
        &setup.service,
        "/apps",
        Auth::Cookie(&admin),
        &registration(app, &workspace_schema(app))?,
    )
    .await?)?;
    let answer = post(
        &setup.service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation":op()?}),
    )
    .await?;
    let held = ok(get(
        &setup.service,
        &format!("/apps/{app}"),
        Auth::Cookie(&admin),
    )
    .await?)?;
    // Inspect the retained state even when the response incorrectly claims success.
    assert_eq!(
        held["state"], "pending",
        "broker-down approval activated the app without credential custody"
    );
    assert_eq!(answer.0, 502, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "SecretsUnavailable");
    assert!(answer.1.get("client").is_none());
    Ok(())
}

#[tokio::test]
async fn wrong_broker_custody_receipt_keeps_app_pending() -> TestResult {
    use identity_contract::apps::{Auth, get, ok, op, post, registration, workspace_schema};
    use identity_contract::harness::ADMINISTRATOR;
    let setup = setup_with_person(ADMINISTRATOR).await?;
    let admin = setup.service.sign_in(login(ADMINISTRATOR)).await?;
    let app = "fixture_bad_custody";
    ok(post(
        &setup.service,
        "/apps",
        Auth::Cookie(&admin),
        &registration(app, &workspace_schema(app))?,
    )
    .await?)?;
    let answer = post(
        &setup.service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation":op()?}),
    )
    .await?;
    assert_eq!(answer.0, 502, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "SecretsUnavailable");
    let held = ok(get(
        &setup.service,
        &format!("/apps/{app}"),
        Auth::Cookie(&admin),
    )
    .await?)?;
    assert_eq!(held["state"], "pending");
    assert!(held["client_id"].is_null());
    Ok(())
}

#[tokio::test]
async fn approval_without_configured_broker_keeps_app_pending() -> TestResult {
    use identity_contract::apps::{Auth, get, ok, op, post, registration, workspace_schema};
    use identity_contract::harness::ADMINISTRATOR;
    let (service, _seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let app = "fixture_save_unavailable";
    ok(post(
        &service,
        "/apps",
        Auth::Cookie(&admin),
        &registration(app, &workspace_schema(app))?,
    )
    .await?)?;
    let answer = post(
        &service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation":op()?}),
    )
    .await?;
    let held = ok(get(&service, &format!("/apps/{app}"), Auth::Cookie(&admin)).await?)?;
    // Inspect the retained state even when the response incorrectly claims success.
    assert_eq!(
        held["state"], "pending",
        "missing-broker approval activated the app without credential custody"
    );
    assert_eq!(answer.0, 502, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "SecretsUnavailable");
    assert!(answer.1.get("client").is_none());
    Ok(())
}
