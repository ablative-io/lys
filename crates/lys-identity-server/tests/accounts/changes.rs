//! Own-account changes and sign-in revocation, split at the account-change seam.
use super::*;

#[tokio::test]
async fn a_changed_email_signs_in_and_the_old_one_does_not() -> TestResult {
    let (service, _rauthy, ada, mut sink) = table_with_mail(true).await?;
    let (status, shown) = service.get("/me/account", Some(&ada)).await?;
    assert_eq!(status, 200, "{shown}");
    assert_eq!(shown["email"], EMAIL);
    // First provider sign-in records the setup account's missing held address.
    let me = service.get("/me", Some(&ada)).await?.1;
    let (back, binding) = service
        .issuer_answer(identity_contract::fake_issuer::Login {
            subject: me["signed_in"]["subject"]
                .as_str()
                .ok_or("Ada's account")?
                .to_owned(),
            email: EMAIL.to_owned(),
        })
        .await?;
    let (_, location, cookie) = service.get_page(&back, &binding).await?;
    assert_eq!(location.as_deref(), Some("/#/me"));
    assert!(cookie);
    let wrong = service
        .post(
            "/me/account/email",
            Some(&ada),
            &json!({ "email": "ada@lovelace.test", "password": "Not-The-Password-9" }),
        )
        .await?;
    assert_eq!(wrong.0, 400, "{}", wrong.1);
    assert_eq!(wrong.1["refusal"], "AccountRefused");
    let (status, changed) = service
        .post(
            "/me/account/email",
            Some(&ada),
            &json!({ "email": "ada@lovelace.test", "password": PASSWORD }),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["pending"], true);
    assert_eq!(
        service.get("/me/account", Some(&ada)).await?.1["email"],
        EMAIL
    );
    let letter = sink.next().await.ok_or("the email proof")?;
    let code = letter
        .body
        .split("Code: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or("the mailed code")?;
    let held = sink.next().await.ok_or("the held mailbox proof")?;
    assert_eq!(held.to, vec![EMAIL]);
    let held_code = held
        .body
        .split("Code: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or("the held code")?;
    let (status, changed) = service
        .post(
            "/me/account/email/confirm",
            Some(&ada),
            &json!({"code": code, "held_code":held_code}),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["email"], "ada@lovelace.test");
    let signed_in = service.sign_in_with("ada@lovelace.test", PASSWORD).await?;
    assert_eq!(service.get("/me", Some(&signed_in)).await?.0, 200);
    assert!(
        service.sign_in_with(EMAIL, PASSWORD).await.is_err(),
        "the old email no longer signs in"
    );
    Ok(())
}

#[tokio::test]
async fn a_changed_password_needs_the_current_one() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let renewed = "Difference-Engine-1822";
    let wrong = service
        .post(
            "/me/account/password",
            Some(&ada),
            &json!({ "current": "Not-The-Password-9", "password": renewed }),
        )
        .await?;
    assert_eq!(wrong.1["refusal"], "AccountRefused", "{}", wrong.1);
    let (status, answer) = service
        .post(
            "/me/account/password",
            Some(&ada),
            &json!({ "current": PASSWORD, "password": renewed }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    service.sign_in_with(EMAIL, renewed).await?;
    assert!(service.sign_in_with(EMAIL, PASSWORD).await.is_err());
    Ok(())
}

#[tokio::test]
async fn the_administrator_resets_a_password_and_disables_a_person() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let bea = bea(&service, &rauthy, &ada).await?;
    service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let (status, reset) = service
        .post(
            &format!("/directory/people/{bea}/account/password"),
            Some(&ada),
            &json!({ "password": "Bea-New-Password-222" }),
        )
        .await?;
    assert_eq!(status, 200, "{reset}");
    let bea_session = service
        .sign_in_with("bea@example.test", "Bea-New-Password-222")
        .await?;
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-Password-000111")
            .await
            .is_err()
    );

    let not_admin = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&bea_session),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(not_admin.1["refusal"], "NotAdmitted", "{}", not_admin.1);
    let (status, off) = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(status, 200, "{off}");
    assert_eq!(off["enabled"], false);
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-New-Password-222")
            .await
            .is_err()
    );
    let (status, on) = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": true }),
        )
        .await?;
    assert_eq!(status, 200, "{on}");
    service
        .sign_in_with("bea@example.test", "Bea-New-Password-222")
        .await?;

    let (status, moved) = service
        .post(
            &format!("/directory/people/{bea}/account/email"),
            Some(&ada),
            &json!({ "email": "bea@new.example.test" }),
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    service
        .sign_in_with("bea@new.example.test", "Bea-New-Password-222")
        .await?;
    Ok(())
}

async fn product_code(
    service: &Service,
    cookie: &str,
    verifier: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let answer = client
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", PRODUCT),
            ("redirect_uri", CALLBACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("state", "accounts-product"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(answer.status().as_u16(), 303);
    let back = reqwest::Url::parse(
        answer
            .headers()
            .get(reqwest::header::LOCATION)
            .ok_or("missing product redirect")?
            .to_str()?,
    )?;
    back.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| "missing product code".into())
}

async fn product_token(
    service: &Service,
    code: &str,
    verifier: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{PRODUCT}:{}", secret()))
            ),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", CALLBACK),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;
    Ok((answer.status().as_u16(), answer.json().await?))
}

async fn product_user(service: &Service, token: &str) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(token)
        .send()
        .await?;
    Ok((answer.status().as_u16(), answer.json().await?))
}

#[tokio::test]
async fn disabling_an_account_revokes_existing_sessions_codes_and_tokens_across_restart()
-> TestResult {
    let (mut service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let verifier = "accounts-product-verifier-of-enough-length-0123456789";
    let code = product_code(&service, &cookie, verifier).await?;
    let issued = product_token(&service, &code, verifier).await?;
    assert_eq!(issued.0, 200);
    let token = issued.1["access_token"]
        .as_str()
        .ok_or("no access token")?
        .to_owned();
    assert_eq!(product_user(&service, &token).await?.0, 200);
    let pending = product_code(&service, &cookie, verifier).await?;
    let disabled = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(disabled.0, 200, "{}", disabled.1);
    assert_eq!(disabled.1["enabled"], false);
    let kept: Value = serde_json::from_slice(&std::fs::read(
        service.dir.path().join("provider.tokens.json"),
    )?)?;
    assert!(
        kept["tokens"]
            .as_array()
            .ok_or("no stored access table")?
            .is_empty()
    );
    let signed_out = service.get("/me", Some(&cookie)).await?;
    assert_eq!(signed_out.0, 401, "{}", signed_out.1);
    assert_eq!(signed_out.1["refusal"], "NotSignedIn");
    let revoked = product_user(&service, &token).await?;
    assert_eq!(revoked.0, 401, "{}", revoked.1);
    assert_eq!(revoked.1["refusal"], "TokenUnknown");
    let code_refused = product_token(&service, &pending, verifier).await?;
    assert_eq!(code_refused.0, 400, "{}", code_refused.1);
    assert_eq!(code_refused.1["error"], "invalid_grant");
    assert_eq!(code_refused.1["refusal"], "CodeUnknown");
    service.restart().await?;
    let reopened = service.get("/me", Some(&cookie)).await?;
    assert_eq!(reopened.0, 401, "{}", reopened.1);
    assert_eq!(product_user(&service, &token).await?.0, 401);
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-Password-000111")
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn a_callback_prepared_before_disable_cannot_create_a_new_session() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    let subject = user["id"].as_str().ok_or("account id missing")?.to_owned();
    let (callback, binding) = service
        .issuer_answer(identity_contract::fake_issuer::Login {
            subject,
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let disabled = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(disabled.0, 200, "{}", disabled.1);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let answer = client
        .get(format!("{}{callback}", service.base))
        .header(reqwest::header::COOKIE, binding)
        .send()
        .await?;
    let session = answer
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|cookie| cookie.starts_with(&format!("{}=", lys_identity_server::session::COOKIE)));
    assert!(
        session.is_none(),
        "a pre-disable callback recreated a signed-in session"
    );
    Ok(())
}

#[tokio::test]
async fn a_failed_session_revoke_is_named_and_keeps_the_old_cookie_refused() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let sessions = service.dir.path().join("sessions.json");
    std::fs::rename(&sessions, service.dir.path().join("saved-sessions.json"))?;
    std::fs::create_dir(&sessions)?;
    let refused = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(refused.0, 503, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "SessionsUnavailable");
    let signed_out = service.get("/me", Some(&cookie)).await?;
    assert_eq!(signed_out.0, 401, "{}", signed_out.1);
    assert_eq!(signed_out.1["refusal"], "NotSignedIn");
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    assert_eq!(
        user["enabled"], true,
        "issuer changes only after durable session revocation"
    );
    Ok(())
}

#[tokio::test]
async fn a_failed_token_revoke_is_named_and_leaves_the_issuer_account_enabled() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let verifier = "accounts-product-verifier-of-enough-length-0123456789";
    let code = product_code(&service, &cookie, verifier).await?;
    let issued = product_token(&service, &code, verifier).await?;
    assert_eq!(issued.0, 200, "{}", issued.1);
    let tokens = service.dir.path().join("provider.tokens.json");
    std::fs::rename(&tokens, service.dir.path().join("saved-tokens.json"))?;
    std::fs::create_dir(&tokens)?;
    let refused = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(refused.0, 503, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "ProviderUnavailable");
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    assert_eq!(
        user["enabled"], true,
        "issuer changes only after the provider's tokens are revoked"
    );
    Ok(())
}

#[tokio::test]
async fn a_sign_in_the_issuer_api_refuses_is_named_and_begins_no_session() -> TestResult {
    let (mut service, rauthy, ada) = table().await?;
    bea(&service, &rauthy, &ada).await?;
    std::fs::write(rauthy.api_key_file(), "lys$not-the-issuer-key")?;
    service.restart().await?;
    let refused = service
        .post(
            "/sign-in",
            None,
            &json!({ "email": "bea@example.test", "password": "Bea-Password-000111" }),
        )
        .await?;
    assert_eq!(refused.0, 503, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "SignInProvidersRefused");
    Ok(())
}

#[tokio::test]
async fn the_administrator_is_refused_an_email_or_password_the_policy_refuses() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let bea = bea(&service, &rauthy, &ada).await?;
    let (status, email) = service
        .post(
            &format!("/directory/people/{bea}/account/email"),
            Some(&ada),
            &json!({ "email": "not-an-email" }),
        )
        .await?;
    assert_eq!(status, 400, "{email}");
    assert_eq!(email["refusal"], "AccountRefused", "{email}");
    let (status, password) = service
        .post(
            &format!("/directory/people/{bea}/account/password"),
            Some(&ada),
            &json!({ "password": "short" }),
        )
        .await?;
    assert_eq!(status, 400, "{password}");
    assert_eq!(password["refusal"], "AccountRefused", "{password}");
    service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    Ok(())
}

pub(super) fn fixture_password_policy() -> accounts::PasswordPolicy {
    accounts::PasswordPolicy {
        length_min: 14,
        length_max: 128,
        lower_case: Some(1),
        upper_case: Some(1),
        digits: Some(1),
        special: None,
        not_recently_used: None,
    }
}
