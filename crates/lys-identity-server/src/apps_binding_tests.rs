#![cfg(test)]
//! The provider's lookup of an app's sign-in client: no client completes a
//! sign-in before its app is approved or after it is retired, and an
//! approved client only with its own secret and a listed address.

use std::error::Error;

use serde_json::json;

use super::{sha256_hex, sign_in_client, sign_in_redirect};
use crate::apps_error::AppError;
use crate::apps_state::{Approved, By, Client, Decided, Held, Line, Registered, SignInSet};

type Outcome = Result<(), Box<dyn Error>>;

const APP: &str = "fixture_notes";
const SECRET: &str = "a-fixture-client-secret";
const BACK: &str = "https://app.example.test/signed-in";

#[tokio::test]
async fn unbound_login_cannot_act_as_a_person_on_app_routes() -> Outcome {
    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::{ADMINISTRATOR, Service};

    let (service, _) = Service::start_with(|config| {
        Ok(
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "member"],
            )?,
        )
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: "unbound".to_owned(),
            email: "scope@example.test".to_owned(),
        })
        .await?;
    let (status, answer) = service.get("/apps", Some(&cookie)).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NoPerson");
    Ok(())
}

fn registered() -> Result<Held, Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Registered(Registered {
        operation: "register".to_owned(),
        app: APP.to_owned(),
        name: "The notes fixture".to_owned(),
        redirects: vec![BACK.to_owned()],
        schema: json!({"kinds": {}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))?;
    Ok(held)
}

fn approved() -> Result<Held, Box<dyn Error>> {
    let mut held = registered()?;
    held.hold(Line::Approved(Approved {
        operation: "approve".to_owned(),
        app: APP.to_owned(),
        client: Client {
            client_id: APP.to_owned(),
            secret_sha256: sha256_hex(SECRET),
        },
        binding: None,
        by: By::Start,
        at: 2,
    }))?;
    held.hold(Line::SignInSet(SignInSet {
        operation: "approve".to_owned(),
        app: APP.to_owned(),
        redirects: vec![BACK.to_owned()],
        profile: false,
        by: By::Start,
        at: 2,
    }))?;
    Ok(held)
}

fn refusal(held: &Held, secret: &str, redirect: &str) -> Result<AppError, Box<dyn Error>> {
    match sign_in_client(held, APP, secret, redirect) {
        Ok(_) => Err("the client completed a sign-in".into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn a_pending_apps_client_cannot_complete_a_sign_in() -> Outcome {
    let refused = refusal(&registered()?, SECRET, BACK)?;
    assert!(
        matches!(refused, AppError::AppNotApproved { .. }),
        "{refused}"
    );
    Ok(())
}

/// Authorize presents no secret: the lookup without one admits an approved
/// app's listed address and refuses an unlisted one, a pending app, and an id
/// no app holds, each by the same name the secret-bearing lookup gives.
#[test]
fn the_lookup_without_a_secret_judges_the_approval_and_the_address() -> Outcome {
    let held = approved()?;
    assert_eq!(sign_in_redirect(&held, APP, BACK)?.registered.app, APP);
    let unlisted = sign_in_redirect(&held, APP, "https://elsewhere.example.test/");
    assert!(
        matches!(unlisted, Err(AppError::RedirectInvalid { .. })),
        "{unlisted:?}"
    );
    let unknown = sign_in_redirect(&held, "nobody_here", BACK);
    assert!(
        matches!(unknown, Err(AppError::CredentialRefused { .. })),
        "{unknown:?}"
    );
    let registered = registered()?;
    let pending = sign_in_redirect(&registered, APP, BACK);
    assert!(
        matches!(pending, Err(AppError::AppNotApproved { .. })),
        "{pending:?}"
    );
    Ok(())
}

#[test]
fn an_approved_client_completes_a_sign_in_with_its_secret_to_a_listed_address() -> Outcome {
    let held = approved()?;
    let app = sign_in_client(&held, APP, SECRET, BACK)?;
    assert_eq!(app.registered.app, APP);
    Ok(())
}

#[test]
fn another_secret_is_refused_and_never_named() -> Outcome {
    let refused = refusal(&approved()?, "not-the-secret", BACK)?;
    assert!(
        matches!(refused, AppError::CredentialRefused { .. }),
        "{refused}"
    );
    assert!(!refused.to_string().contains("not-the-secret"));
    Ok(())
}

#[test]
fn an_address_the_registration_does_not_list_is_refused() -> Outcome {
    let refused = refusal(&approved()?, SECRET, "https://elsewhere.example.test/")?;
    assert!(
        matches!(refused, AppError::RedirectInvalid { .. }),
        "{refused}"
    );
    Ok(())
}

#[test]
fn a_retired_apps_client_cannot_complete_a_sign_in() -> Outcome {
    let mut held = approved()?;
    held.hold(Line::Retired(Decided {
        operation: "retire".to_owned(),
        app: APP.to_owned(),
        reason: String::new(),
        by: By::Start,
        at: 3,
    }))?;
    let refused = refusal(&held, SECRET, BACK)?;
    assert!(matches!(refused, AppError::AppRetired { .. }), "{refused}");
    Ok(())
}
