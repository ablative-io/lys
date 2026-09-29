#![cfg(test)]
//! Registry identity, revision binding and authentication ambiguity regressions.

use axum::http::{HeaderMap, header};

use crate::apps_state::{By, Decided};
use crate::delegation::fixture::{CLIENT, REDIRECT, SECRET, TestResult, app, basic};
use crate::provider::ProductClient;

use super::{Binding, ClientSource, Refusal, authenticate};

#[test]
fn approved_app_resolves_with_exact_secret_and_redirect() -> TestResult {
    let app = app()?;
    let expected = Binding::current(&app)?;
    assert_eq!(
        authenticate(
            &basic(CLIENT, SECRET)?,
            &[],
            |_client| Ok(ClientSource::ApprovedApp(&app)),
            REDIRECT
        )?,
        expected
    );
    assert_eq!(
        authenticate(
            &HeaderMap::new(),
            &[("client_id", CLIENT), ("client_secret", SECRET)],
            |_client| Ok(ClientSource::ApprovedApp(&app)),
            REDIRECT
        )?,
        expected
    );
    assert_eq!(
        authenticate(
            &basic(CLIENT, "wrong-private-secret")?,
            &[],
            |_client| Ok(ClientSource::ApprovedApp(&app)),
            REDIRECT
        ),
        Err(Refusal::ClientRefused)
    );
    let error = authenticate(
        &basic(CLIENT, SECRET)?,
        &[],
        |_client| Ok(ClientSource::ApprovedApp(&app)),
        "https://other.example/return",
    )
    .err()
    .ok_or("wrong redirect passed")?;
    assert_eq!(error, Refusal::ClientRefused);
    assert!(!error.to_string().contains(SECRET));
    Ok(())
}

#[test]
fn equal_client_ids_in_configured_registry_do_not_gain_app_authority() -> TestResult {
    let app = app()?;
    let configured = ProductClient {
        client_id: CLIENT.to_owned(),
        secret_sha256: app
            .approved
            .as_ref()
            .ok_or("approval missing")?
            .client
            .secret_sha256
            .clone(),
        redirect_uris: vec![REDIRECT.to_owned()],
    };
    assert_eq!(
        authenticate(
            &basic(CLIENT, SECRET)?,
            &[],
            |_client| Ok(ClientSource::Configured(&configured)),
            REDIRECT
        ),
        Err(Refusal::ClientAuthority)
    );
    Ok(())
}

#[test]
fn mixed_and_duplicate_authentication_refuse_before_registry_resolution() -> TestResult {
    let configured = ProductClient {
        client_id: CLIENT.to_owned(),
        secret_sha256: "irrelevant".to_owned(),
        redirect_uris: vec![],
    };
    let single = basic(CLIENT, SECRET)?;
    let mut duplicate = single.clone();
    duplicate.append(header::AUTHORIZATION, single[header::AUTHORIZATION].clone());
    for (headers, form) in [
        (duplicate, vec![]),
        (single.clone(), vec![("client_id", CLIENT)]),
        (
            single,
            vec![("client_id", CLIENT), ("client_secret", SECRET)],
        ),
        (
            HeaderMap::new(),
            vec![
                ("client_id", CLIENT),
                ("client_id", CLIENT),
                ("client_secret", SECRET),
            ],
        ),
        (HeaderMap::new(), vec![("client_id", CLIENT)]),
    ] {
        let looked_up = std::cell::Cell::new(false);
        assert_eq!(
            authenticate(
                &headers,
                &form,
                |_client| {
                    looked_up.set(true);
                    Ok(ClientSource::Configured(&configured))
                },
                REDIRECT
            ),
            Err(Refusal::ClientAuthentication)
        );
        assert!(
            !looked_up.get(),
            "ambiguous authentication reached the registry"
        );
    }
    Ok(())
}

#[test]
fn changed_registration_approval_version_credential_or_redirect_invalidates_binding() -> TestResult
{
    let original = app()?;
    let binding = Binding::current(&original)?;
    let mut changed = Vec::new();
    let mut registration = original.clone();
    registration.registered.operation = "register-again".to_owned();
    changed.push(registration);
    let mut approval = original.clone();
    approval
        .approved
        .as_mut()
        .ok_or("approval missing")?
        .operation = "approve-again".to_owned();
    changed.push(approval);
    let mut version = original.clone();
    version
        .versions
        .last_mut()
        .ok_or("version missing")?
        .operation = "replace-version".to_owned();
    changed.push(version);
    let mut credential = original.clone();
    credential
        .approved
        .as_mut()
        .ok_or("approval missing")?
        .client
        .secret_sha256 = "a".repeat(64);
    changed.push(credential);
    let mut redirects = original.clone();
    redirects
        .registered
        .redirects
        .push("https://app.example.test/another".to_owned());
    changed.push(redirects);
    let mut retired = original;
    retired.retired = Some(Decided {
        operation: "retire".to_owned(),
        app: CLIENT.to_owned(),
        reason: String::new(),
        by: By::Start,
        at: 3,
    });
    changed.push(retired);
    for app in changed {
        assert_eq!(binding.check_current(&app), Err(Refusal::BindingChanged));
    }
    Ok(())
}

#[test]
fn redirect_set_order_is_irrelevant_but_duplicates_are_refused() -> TestResult {
    let mut app = app()?;
    app.registered
        .redirects
        .push("https://app.example.test/another".to_owned());
    let before = Binding::current(&app)?;
    app.registered.redirects.reverse();
    assert_eq!(Binding::current(&app)?, before);
    app.registered.redirects.push(REDIRECT.to_owned());
    assert_eq!(Binding::current(&app), Err(Refusal::BindingInvalid));
    Ok(())
}
