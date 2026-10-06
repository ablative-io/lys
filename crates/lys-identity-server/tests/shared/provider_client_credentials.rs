#![cfg(test)]

//! An app's virtual client credentials at the token exchange (DIRECTORY-081):
//! issued once, confirmed by the broker while the apps' record holds them
//! live, refused once revoked or once their app is retired, refused by name
//! and never judged by digest while the broker is down, and ended at the
//! broker when it next answers an administrator's act.

use super::*;

/// Issue `app` a client credential under `operation`, answering the answer.
async fn issue(
    service: &Service,
    cookie: &str,
    app: &str,
    operation: &str,
) -> Result<Value, Box<dyn Error>> {
    ok(post(
        service,
        &format!("/apps/{app}/credentials/issue"),
        Auth::Cookie(cookie),
        &json!({ "operation": operation }),
    )
    .await?)
}

/// The value an issue answered.
fn value(issued: &Value) -> Result<String, Box<dyn Error>> {
    Ok(issued["credential"]
        .as_str()
        .ok_or("an issue answers its value once")?
        .to_owned())
}

/// Ask the token endpoint as `client` with `credential`, in the Basic header
/// or in the form, for `grant`.
async fn token(
    service: &Service,
    client: &str,
    credential: &str,
    basic: bool,
    grant: &[(&str, &str)],
) -> Result<(u16, Value), Box<dyn Error>> {
    let mut form: Vec<(&str, &str)> = grant.to_vec();
    let mut request = reqwest::Client::new().post(format!("{}/oauth/token", service.base));
    if basic {
        request = request.header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{client}:{credential}"))
            ),
        );
    } else {
        form.push(("client_id", client));
        form.push(("client_secret", credential));
    }
    let answer = request.form(&form).send().await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

/// Sign a person in to the fixture product with `credential`, answering the
/// token endpoint's status and body.
async fn sign_in_with(
    service: &Service,
    cookie: &str,
    credential: &str,
    basic: bool,
) -> Result<(u16, Value), Box<dyn Error>> {
    let verifier = "a-verifier-long-enough-for-pkce-0123456789abcdef";
    let code = code(service, cookie, &challenge_of(verifier)).await?;
    token(
        service,
        PRODUCT,
        credential,
        basic,
        &[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", CALLBACK),
            ("code_verifier", verifier),
        ],
    )
    .await
}

/// The fixture product's credentials as its view lists them.
async fn listed(service: &Service, cookie: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let app = ok(identity_contract::apps::get(
        service,
        &format!("/apps/{PRODUCT}"),
        Auth::Cookie(cookie),
    )
    .await?)?;
    Ok(app["client_credentials"]
        .as_array()
        .ok_or("the app lists its credentials")?
        .clone())
}

#[tokio::test]
async fn an_issued_credential_signs_a_product_in_until_it_is_revoked_and_a_refresh_with_it_is_refused()
-> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    let operation = OperationId::generate()?.to_string();
    let first = issue(&service, &cookie, PRODUCT, &operation).await?;
    let first_value = value(&first)?;
    assert!(
        first_value.starts_with(&format!("lys-client.{PRODUCT}.")),
        "the value names its app"
    );
    let first_id = first["credential_id"].as_str().ok_or("an id")?.to_owned();
    // The same operation again answers the same id and no value.
    let again = issue(&service, &cookie, PRODUCT, &operation).await?;
    assert_eq!(again["credential_id"], first_id.as_str());
    assert!(again["credential"].is_null(), "{again}");
    // Both ways a product presents it sign a person in.
    let (status, answer) = sign_in_with(&service, &cookie, &first_value, true).await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = sign_in_with(&service, &cookie, &first_value, false).await?;
    assert_eq!(status, 200, "{answer}");
    let before = answer["access_token"]
        .as_str()
        .ok_or("an access token")?
        .to_owned();
    // A second credential, and both are live.
    let second = issue(
        &service,
        &cookie,
        PRODUCT,
        &OperationId::generate()?.to_string(),
    )
    .await?;
    let second_value = value(&second)?;
    let held = listed(&service, &cookie).await?;
    assert_eq!(held.len(), 2);
    assert!(held.iter().all(|credential| credential["live"] == true));
    assert!(!json!(held).to_string().contains(&first_value));
    // Revoked: refused from the next exchange, the other still signs in.
    let revoked = ok(post(
        &service,
        &format!("/apps/{PRODUCT}/credentials/{first_id}/revoke"),
        Auth::Cookie(&cookie),
        &json!({ "operation": OperationId::generate()?.to_string(), "reason": "rotated" }),
    )
    .await?)?;
    assert_eq!(revoked["client_credentials"][0]["live"], false);
    assert_eq!(
        revoked["client_credentials"][0]["revoked_reason"],
        "rotated"
    );
    assert_eq!(custody.ended(PRODUCT), vec![first_id.clone()]);
    let (status, refused) = sign_in_with(&service, &cookie, &first_value, true).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"], "invalid_client");
    assert_eq!(refused["refusal"], "credential_refused");
    assert!(!refused.to_string().contains(&first_value));
    // A token issued before the revocation stands until its own expiry.
    let (status, info) = userinfo_with(&service, &before).await?;
    assert_eq!(status, 200, "{info}");
    let (status, answer) = sign_in_with(&service, &cookie, &second_value, false).await?;
    assert_eq!(status, 200, "{answer}");
    // A refresh with the revoked credential is refused as the credential,
    // before its grant is read; with the live one it is refused as a grant
    // the provider does not serve.
    let refresh = [("grant_type", "refresh_token"), ("refresh_token", "any")];
    let (status, refused) = token(&service, PRODUCT, &first_value, true, &refresh).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "credential_refused");
    let (status, refused) = token(&service, PRODUCT, &second_value, true, &refresh).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "RequestMalformed");
    Ok(())
}

#[tokio::test]
async fn another_apps_credential_and_a_changed_one_are_refused() -> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    drop(custody);
    let mut body = registration(SECOND, &workspace_schema(SECOND))?;
    body["redirects"] = json!([BACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    approve(&service, &cookie, SECOND).await?;
    let others = value(
        &issue(
            &service,
            &cookie,
            SECOND,
            &OperationId::generate()?.to_string(),
        )
        .await?,
    )?;
    let (status, refused) = sign_in_with(&service, &cookie, &others, true).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "credential_refused");
    let own = value(
        &issue(
            &service,
            &cookie,
            PRODUCT,
            &OperationId::generate()?.to_string(),
        )
        .await?,
    )?;
    let mut changed = own.clone();
    let last = changed.pop().ok_or("a value")?;
    changed.push(if last == '0' { '1' } else { '0' });
    let (status, refused) = sign_in_with(&service, &cookie, &changed, true).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "credential_refused");
    Ok(())
}

#[tokio::test]
async fn with_the_broker_down_a_credential_is_refused_by_name_and_never_judged_by_digest()
-> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    let credential = value(
        &issue(
            &service,
            &cookie,
            PRODUCT,
            &OperationId::generate()?.to_string(),
        )
        .await?,
    )?;
    custody.down();
    let (status, refused) = sign_in_with(&service, &cookie, &credential, true).await?;
    assert_ne!(status, 200, "{refused}");
    // Judged by the approved digest it would be credential_refused; it is the
    // broker's absence, by name.
    assert_eq!(refused["refusal"], "SecretsUnavailable", "{refused}");
    custody.up();
    let (status, answer) = sign_in_with(&service, &cookie, &credential, true).await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_credential_of_an_app_retired_while_the_broker_was_down_never_authenticates_after_it_returns()
-> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    let issued = issue(
        &service,
        &cookie,
        PRODUCT,
        &OperationId::generate()?.to_string(),
    )
    .await?;
    let credential = value(&issued)?;
    let id = issued["credential_id"].as_str().ok_or("an id")?.to_owned();
    custody.down();
    // Retirement is never refused for the broker's absence.
    let retired = ok(post(
        &service,
        &format!("/apps/{PRODUCT}/retire"),
        Auth::Cookie(&cookie),
        &json!({ "operation": OperationId::generate()?.to_string(), "reason": "done" }),
    )
    .await?)?;
    assert_eq!(retired["state"], "retired");
    assert_eq!(retired["client_credentials"][0]["live"], false);
    assert_eq!(
        retired["client_credentials"][0]["ended_by_retirement"],
        true
    );
    assert_eq!(retired["client_credentials"][0]["ended_at_broker"], false);
    custody.up();
    // The app is refused app_retired before the broker is asked.
    let (status, refused) = token(
        &service,
        PRODUCT,
        &credential,
        true,
        &[
            ("grant_type", "authorization_code"),
            ("code", "any"),
            ("redirect_uri", CALLBACK),
            ("code_verifier", "any"),
        ],
    )
    .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "app_retired");
    assert!(custody.ended(PRODUCT).is_empty());
    // The broker's digest is ended when it next answers an administrator's act.
    let mut body = registration(SECOND, &workspace_schema(SECOND))?;
    body["redirects"] = json!([BACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    approve(&service, &cookie, SECOND).await?;
    issue(
        &service,
        &cookie,
        SECOND,
        &OperationId::generate()?.to_string(),
    )
    .await?;
    assert_eq!(custody.ended(PRODUCT), vec![id]);
    assert_eq!(listed(&service, &cookie).await?[0]["ended_at_broker"], true);
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_issues_and_revokes() -> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    drop(custody);
    let bea = service
        .sign_in(Login {
            subject: "bea-subject".to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let refused = post(
        &service,
        &format!("/apps/{PRODUCT}/credentials/issue"),
        Auth::Cookie(&bea),
        &json!({ "operation": OperationId::generate()?.to_string() }),
    )
    .await?;
    assert_ne!(refused.0, 200, "{}", refused.1);
    let issued = issue(
        &service,
        &cookie,
        PRODUCT,
        &OperationId::generate()?.to_string(),
    )
    .await?;
    let id = issued["credential_id"].as_str().ok_or("an id")?;
    let refused = post(
        &service,
        &format!("/apps/{PRODUCT}/credentials/{id}/revoke"),
        Auth::Cookie(&bea),
        &json!({ "operation": OperationId::generate()?.to_string() }),
    )
    .await?;
    assert_ne!(refused.0, 200, "{}", refused.1);
    assert_eq!(listed(&service, &cookie).await?[0]["live"], true);
    Ok(())
}

#[tokio::test]
async fn a_broker_that_holds_no_sealed_client_secret_is_refused_by_name_and_nothing_is_kept()
-> TestResult {
    let (service, cookie, _person, custody) = table_kept(CODE_SECONDS).await?;
    custody.lose(PRODUCT);
    let refused = post(
        &service,
        &format!("/apps/{PRODUCT}/credentials/issue"),
        Auth::Cookie(&cookie),
        &json!({ "operation": OperationId::generate()?.to_string() }),
    )
    .await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "AppClientNoCustody", "{}", refused.1);
    assert!(listed(&service, &cookie).await?.is_empty());
    Ok(())
}
