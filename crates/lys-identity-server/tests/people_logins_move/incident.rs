//! The three independent holes of 26 October, then repair onto Annabel’s own record.
use super::*;

/// All three legs run before the assertions: fixing only the first
/// hole cannot stop the replay reaching the email and provider holes.
#[tokio::test]
async fn october_26_link_email_and_google_each_refuse_becoming_dean() -> TestResult {
    let (service, rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let annabel = add(&service, &ada, "Annabel", "annabel.personal@example.test").await?;
    let annabel_session = signed_in_as(&service, &annabel, "annabel.personal@example.test").await?;
    let dean_session = signed_in_as(&service, &dean, "dean@example.test").await?;
    let token = dean["letter"]["handed"]
        .as_str()
        .and_then(|link| link.split("/#/added/").nth(1))
        .ok_or("Dean's link")?;
    let link = service
        .post(
            "/organisations/found/prove",
            Some(&annabel_session),
            &json!({"proof": token}),
        )
        .await?;
    // Carry the historical Dean session to independently exercise R2,
    // even though R1 now prevents Annabel obtaining it from the link.
    let email = service
        .post(
            "/me/account/email",
            Some(&dean_session),
            &json!({"email": "annabel@example.test"}),
        )
        .await?;
    let subject = dean["subject"].as_str().ok_or("Dean's account")?;
    // Recreate the historical, mutable issuer account independently of R2.
    // The unpatched issuer joins equal Google/account emails, while Lys
    // still holds Dean's admission address for Dean's person.
    historical_email(&rauthy, subject, "annabel@example.test", true).await?;
    let google: Value = reqwest::Client::new()
        .post(format!("{}/providers/create", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&json!({"name":"Google", "enabled":true, "client_id":"google-client"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let provider = google["id"].as_str().ok_or("Google's provider")?;
    let (back, binding) = service
        .issuer_answer_from(
            Login {
                subject: subject.to_owned(),
                email: "annabel@example.test".to_owned(),
            },
            provider,
        )
        .await?;
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}{back}", service.base))
        .header(reqwest::header::COOKIE, binding)
        .send()
        .await?;
    let provider_cookie = response.headers().contains_key(reqwest::header::SET_COOKIE);
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the provider result")?
        .to_str()?
        .to_owned();
    let provider_url = reqwest::Url::parse(&format!(
        "http://lys.test/{}",
        location.trim_start_matches("/#/")
    ))?;
    let fields: std::collections::BTreeMap<_, _> =
        provider_url.query_pairs().into_owned().collect();
    let expected = [
        link.0 == 409 && link.1["refusal"] == "SignedInAsAnother",
        email.0 == 409 && email.1["refusal"] == "EmailChangeNeedsMail",
        !provider_cookie && location.contains("refused=ProviderEmailMismatch"),
    ];
    assert_eq!(
        expected, [true; 3],
        "independent holes [link, email, provider]: {expected:?}; link={link:?}; email={email:?}; provider={location}"
    );
    assert_eq!(fields.get("provider").map(String::as_str), Some("Google"));
    assert_eq!(
        fields.get("account_email").map(String::as_str),
        Some("a…@example.test")
    );
    assert_eq!(
        fields.get("held_email").map(String::as_str),
        Some("d…@example.test")
    );
    assert_eq!(
        service.get("/me", Some(&annabel_session)).await?.1["person"]["id"],
        annabel["person"]
    );
    // The repair targets Annabel's own record, which already holds a login
    // at this issuer. Neither that login nor her existing session is replaced.
    let source_id = dean["person"].as_str().ok_or("Dean's record")?;
    let target_id = annabel["person"].as_str().ok_or("Annabel's own record")?;
    let target_before = service
        .get(&format!("/identities/{target_id}"), Some(&ada))
        .await?
        .1;
    assert_eq!(target_before["logins"].as_array().map(Vec::len), Some(1));
    let grants_before = service.get("/grants", Some(&ada)).await?;
    let (_, administrator) = service.get("/me", Some(&ada)).await?;
    let size = service.log_size().await?;
    let (status, moved) = service
        .post(
            &format!("/people/{source_id}/logins/move"),
            Some(&ada),
            &move_body(&service, &dean, &json!({"person":target_id}))?,
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    assert_eq!(moved["source"], source_id);
    assert_eq!(moved["target"], target_id);
    assert_eq!(moved["source_grants"], json!([dean["grant"]]));
    assert_eq!(moved["target_grants"], json!([annabel["grant"]]));
    assert_eq!(service.get("/grants", Some(&ada)).await?, grants_before);
    let source = service
        .get(&format!("/identities/{source_id}"), Some(&ada))
        .await?
        .1;
    let target = service
        .get(&format!("/identities/{target_id}"), Some(&ada))
        .await?
        .1;
    assert_eq!(source["display_name"], "Dean Whiting");
    assert_eq!(source["logins"], json!([]));
    assert_eq!(target["display_name"], target_before["display_name"]);
    assert_eq!(target["state"], target_before["state"]);
    let logins = target["logins"].as_array().ok_or("Annabel's two logins")?;
    assert_eq!(logins.len(), 2);
    for added in [&dean, &annabel] {
        assert!(
            logins
                .iter()
                .any(|login| login["issuer"] == service.issuer.issuer()
                    && login["subject"] == added["subject"])
        );
    }
    let receipts = moved["receipts"].as_array().ok_or("the repair batch")?;
    assert_eq!(
        receipts.len(),
        2,
        "unbind and bind, without creating or activating a person"
    );
    let mut indices = Vec::new();
    for receipt in receipts {
        assert_eq!(
            receipt["actor"]["subject"],
            administrator["signed_in"]["subject"]
        );
        let index = receipt["log"]["index"]
            .as_u64()
            .ok_or("the signed batch's leaf")?;
        indices.push(index);
        let (_, signed) = service
            .get(&format!("/receipts/{index}"), Some(&ada))
            .await?;
        assert_eq!(signed["receipt"], *receipt);
    }
    assert_eq!(indices, vec![size, size + 1]);
    assert_eq!(service.log_size().await?, size + 2);
    assert_eq!(
        source["events"].as_array().and_then(|events| events.last()),
        Some(&json!(size))
    );
    let mut target_events = target_before["events"]
        .as_array()
        .ok_or("Annabel's existing events")?
        .clone();
    target_events.push(json!(size + 1));
    assert_eq!(target["events"], json!(target_events));
    assert_eq!(service.get("/me", Some(&dean_session)).await?.0, 401);
    assert_eq!(
        service.get("/me", Some(&annabel_session)).await?.1["person"]["id"],
        target_id
    );
    assert_eq!(service.get("/me", Some(&ada)).await?.0, 200);
    for (added, email) in [
        (&dean, "annabel@example.test"),
        (&annabel, "annabel.personal@example.test"),
    ] {
        let session = signed_in_as(&service, added, email).await?;
        let (status, own) = service.get("/me", Some(&session)).await?;
        assert_eq!(status, 200, "{own}");
        assert_eq!(own["person"]["id"], target_id);
        assert_eq!(own["signed_in"]["subject"], added["subject"]);
        assert_eq!(own["sign_in_identities"].as_array().map(Vec::len), Some(2));
    }
    Ok(())
}
