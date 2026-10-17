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

const WILD: &str = "https://*.tail39833a.ts.net/signin/return";
const ONE: &str = "https://toms-macbook-pro.tail39833a.ts.net/signin/return";

/// The reason a return address is refused, or `admitted`.
fn judged(address: &str) -> String {
    match crate::apps_api::redirect(address) {
        Ok(_) => "admitted".to_owned(),
        Err(AppError::RedirectInvalid { reason, .. }) => reason.to_owned(),
        Err(other) => other.to_string(),
    }
}

/// DIRECTORY-097 R1: an https address whose host is one wildcard label under
/// a domain of two or more labels is admitted as a registration's and the
/// settings' addresses are; an http wildcard, a bare wildcard, a one-label
/// suffix, a second wildcard label and a wildcard inside a label are each
/// refused `redirect_invalid` with its own reason.
#[test]
fn a_wildcard_label_under_a_domain_is_admitted_with_https_and_its_kin_refused_by_reason() -> Outcome
{
    assert_eq!(crate::apps_api::redirect(WILD)?, WILD);
    assert_eq!(
        crate::apps_api::redirect("https://*.example.com:8443/x?y=1")?,
        "https://*.example.com:8443/x?y=1"
    );
    assert_eq!(
        judged("http://*.example.com/x"),
        "is neither https nor http to this machine"
    );
    assert_eq!(
        judged("http://*/x"),
        "is neither https nor http to this machine"
    );
    assert_eq!(
        judged("https://*/x"),
        "is a bare wildcard; a wildcard is one label under a domain of two or more labels"
    );
    assert_eq!(
        judged("https://*.net/x"),
        "names a wildcard under a one-label suffix; the domain under the wildcard needs two or more labels"
    );
    assert_eq!(
        judged("https://*.*.example.com/x"),
        "has more than one wildcard label; only the first label of the host may be a wildcard"
    );
    assert_eq!(
        judged("https://a*.example.com/x"),
        "has a wildcard inside a label; the wildcard is the whole first label"
    );
    assert_eq!(
        judged("https:/a*.example.com/x"),
        "has a wildcard inside a label; the wildcard is the whole first label"
    );
    assert_eq!(
        judged("https://u@*.example.com/x"),
        "names a user beside a wildcard; a wildcard address names machines and no user"
    );
    assert_eq!(
        judged("https://*.evil.example@good.example/cb"),
        "has a wildcard outside the host; the wildcard is the whole first label of the host"
    );
    assert_eq!(
        judged("https://*.example.com/a*b"),
        "has a wildcard outside the host; the wildcard is the whole first label of the host"
    );
    assert_eq!(
        judged("https://*.example.com./x"),
        "has an empty label in the domain under the wildcard"
    );
    assert_eq!(
        judged("https://*.bücher.example/x"),
        "names a domain outside ASCII under the wildcard; the domain is written as a browser sends it, in ASCII"
    );
    assert_eq!(judged("https://*.example.com/x#top"), "carries a fragment");
    // R1's rule is two or more labels: a two-label public suffix is admitted
    // by it; refusing public suffixes by name is a brief of its own.
    assert_eq!(
        crate::apps_api::redirect("https://*.co.uk/x")?,
        "https://*.co.uk/x"
    );
    Ok(())
}

/// An approved app whose sign-in settings list `redirects`.
fn approved_listing(redirects: &[&str]) -> Result<Held, Box<dyn Error>> {
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
        redirects: redirects
            .iter()
            .map(|address| (*address).to_owned())
            .collect(),
        profile: false,
        by: By::Start,
        at: 2,
    }))?;
    Ok(held)
}

/// DIRECTORY-097 R2: with a wildcard entry listed, a sign-in's redirect is
/// admitted when its host is exactly one label under the entry's suffix,
/// compared without regard to ASCII case, with the entry's scheme, port,
/// path and query; a second label, no label, another port, another path,
/// another query, http, a label with a character outside a-z, 0-9 and
/// hyphen, and a longer suffix are each refused by the existing reason.
/// An exact entry beside it still admits byte for byte.
#[test]
fn a_wildcard_entry_admits_one_label_under_its_suffix_and_nothing_else() -> Outcome {
    let held = approved_listing(&[BACK, WILD])?;
    assert_eq!(sign_in_redirect(&held, APP, ONE)?.registered.app, APP);
    assert_eq!(
        sign_in_redirect(
            &held,
            APP,
            "https://TOMS-MacBook-Pro.tail39833a.ts.net/signin/return"
        )?
        .registered
        .app,
        APP
    );
    assert_eq!(sign_in_redirect(&held, APP, BACK)?.registered.app, APP);
    for presented in [
        "https://a.b.tail39833a.ts.net/signin/return",
        "https://tail39833a.ts.net/signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net:8443/signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/return/",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/return?x=1",
        "http://toms-macbook-pro.tail39833a.ts.net/signin/return",
        "https://toms_mac.tail39833a.ts.net/signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net.example.com/signin/return",
        "https://*.tail39833a.ts.net/signin/return",
        "https://evil@toms-macbook-pro.tail39833a.ts.net/signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net./signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net:443/signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/return#x",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/%0Areturn",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/../signin/return",
        "https://toms-macbook-pro.tail39833a.ts.net/signin/./return",
        "https://toms-macbook-pro.tail39833a.ts.net\\signin\\return",
        "HTTPS://toms-macbook-pro.tail39833a.ts.net/signin/return",
    ] {
        match sign_in_redirect(&held, APP, presented) {
            Err(AppError::RedirectInvalid { address, reason }) => {
                assert_eq!(address, presented);
                assert_eq!(reason, "is not an address the app's sign-in settings list");
            }
            other => return Err(format!("{presented} judged {other:?}").into()),
        }
    }
    Ok(())
}

/// DIRECTORY-097 R2: the entry's suffix is compared without regard to ASCII
/// case, as the label is; the rest of the entry is its own text.
#[test]
fn a_wildcard_entry_written_in_any_ascii_case_admits_the_label_under_its_suffix() -> Outcome {
    let held = approved_listing(&["https://*.TAIL39833A.ts.net/signin/return"])?;
    assert_eq!(sign_in_redirect(&held, APP, ONE)?.registered.app, APP);
    assert_eq!(
        sign_in_redirect(&held, APP, "https://TOMS.tail39833a.TS.NET/signin/return")?
            .registered
            .app,
        APP
    );
    match sign_in_redirect(&held, APP, "https://toms.tail39833a.ts.net/SIGNIN/return") {
        Err(AppError::RedirectInvalid { reason, .. }) => {
            assert_eq!(reason, "is not an address the app's sign-in settings list");
        }
        other => return Err(format!("a path in another case judged {other:?}").into()),
    }
    Ok(())
}
