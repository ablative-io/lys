use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_revokes_only_its_own_audience_passes_with_its_run_proof() -> TestResult {
    let held = Held::open().await?;
    identity_contract::apps::registered(&held.service, &held.ada, "liminal").await?;
    let started = held
        .ok(
            &format!("/agents/{}/start-command", held.agent()),
            &json!({"operation": operation()?, "machine": held.machine}),
        )
        .await?;
    let session = started["session"].as_str().ok_or("no session")?;
    let (_, pass, seat) = held.rendered(session)?;
    let headers = [(PASS_HEADER, pass.as_str()), (SEAT_HEADER, seat.as_str())];
    let route = format!("/agents/{}/pass", held.agent());
    let body = serde_json::to_vec(&json!({"audience":"liminal"}))?;
    let (status, issued) = held
        .service
        .post_carrying(&route, &headers, body.clone())
        .await?;
    assert_eq!(status, 200, "{issued}");
    let authorization = format!(
        "Bearer {}",
        issued["pass"].as_str().ok_or("no audience pass")?
    );
    let prepared = serde_json::to_vec(&json!({
        "operation": "stop-seat", "grant": lys_identity::grants::GrantId::from_bytes([7; 16]).to_string(),
        "target": {"kind":"liminal.doc", "id":"1", "action":"write"},
        "words": "stop", "request_digest": lys_pass::drafts::request_digest("stop"),
    }))?;
    let bearer = [("authorization", authorization.as_str())];
    let (_, before) = held
        .service
        .post_carrying("/product-drafts", &bearer, prepared.clone())
        .await?;
    assert_eq!(before["refusal"], "product_draft_grant_refused", "{before}");
    let revoke = format!("{route}/revoke");
    let (status, answer) = held
        .service
        .post_carrying(&revoke, &headers, body.clone())
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["audience"], "liminal");
    assert_eq!(answer["revoked"], 1);
    let (_, after) = held
        .service
        .post_carrying("/product-drafts", &bearer, prepared)
        .await?;
    assert_eq!(after["refusal"], "TokenUnknown", "{after}");
    assert_eq!(
        answer["offline_refusal"],
        "offline_pass_revocation_unsupported"
    );
    let (status, again) = held
        .service
        .post_carrying(&revoke, &headers, body.clone())
        .await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["revoked"], 0);
    let (status, refused) = held
        .service
        .post_carrying(&revoke, &[], body.clone())
        .await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "agent_pass_unproven");
    let bad = serde_json::to_vec(&json!({"audience":"unknown-app"}))?;
    let (status, refused) = held.service.post_carrying(&revoke, &headers, bad).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "audience_not_approved");
    let bad = serde_json::to_vec(&json!({"audience":"liminal","extra":true}))?;
    let (status, refused) = held.service.post_carrying(&revoke, &headers, bad).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "RequestMalformed");
    let other = held.seeded.people[0].agents[1].id;
    let other = format!("/agents/{other}/pass/revoke");
    let (status, refused) = held.service.post_carrying(&other, &headers, body).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "agent_pass_unproven");
    Ok(())
}
