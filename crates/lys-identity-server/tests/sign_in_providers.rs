//! The administrator sets Google, Microsoft and GitHub sign-in at the issuer
//! from the service: each provider's endpoints come from the issuer's own
//! lookup (GitHub's are fixed), setting one again replaces it under the same
//! name, the secret goes to the issuer and never comes back, a Microsoft
//! provider names its tenant, and anyone but the administrator is refused.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const SECRET: &str = "GOCSPX-contract-test-secret-value";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service set to speak to the fake issuer API, with the administrator
/// signed in.
async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) =
        Service::start_setting(GRANT_MODEL, None, None, Some(settings), |_| Ok(())).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    Ok((service, rauthy, ada))
}

#[tokio::test]
async fn google_is_set_from_the_lookup_and_set_again_replaces_it() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let (status, empty) = service.get("/sign-in-providers", Some(&ada)).await?;
    assert_eq!(status, 200, "{empty}");
    assert_eq!(empty["providers"], json!([]));
    assert_eq!(empty["offered"], json!(["google", "microsoft", "github"]));

    let body = json!({ "provider": "google", "client_id": "123-abc.apps.googleusercontent.com", "client_secret": SECRET });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &body)
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["providers"].as_array().map(Vec::len), Some(1), "{set}");
    assert_eq!(set["providers"][0]["provider"], "google");
    assert_eq!(set["providers"][0]["name"], "Google");
    assert_eq!(set["providers"][0]["enabled"], true);
    assert_eq!(
        set["providers"][0]["client_id"],
        "123-abc.apps.googleusercontent.com"
    );
    assert!(
        !set.to_string().contains(SECRET),
        "the secret never comes back: {set}"
    );

    let held = rauthy.providers();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0]["typ"], "google");
    assert_eq!(held[0]["issuer"], "https://accounts.google.com");
    assert_eq!(
        held[0]["authorization_endpoint"],
        "https://accounts.google.com/authorize"
    );
    assert_eq!(held[0]["scope"], "openid email profile");
    assert_eq!(held[0]["use_pkce"], true);
    assert_eq!(held[0]["client_secret"], SECRET);
    assert_eq!(held[0]["auto_onboarding"], true);
    assert_eq!(held[0]["auto_link"], false);

    let again = json!({ "provider": "google", "client_id": "123-abc.apps.googleusercontent.com", "client_secret": "GOCSPX-rotated" });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &again)
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["providers"].as_array().map(Vec::len), Some(1), "{set}");
    let held = rauthy.providers();
    assert_eq!(held.len(), 1, "set again replaces, never adds");
    assert_eq!(held[0]["client_secret"], "GOCSPX-rotated");
    Ok(())
}

#[tokio::test]
async fn github_has_fixed_endpoints_and_microsoft_names_its_tenant() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let github =
        json!({ "provider": "github", "client_id": "Iv1.contract", "client_secret": SECRET });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &github)
        .await?;
    assert_eq!(status, 200, "{set}");
    let held = rauthy.providers();
    assert_eq!(held[0]["typ"], "github");
    assert_eq!(
        held[0]["token_endpoint"],
        "https://github.com/login/oauth/access_token"
    );
    assert_eq!(held[0]["userinfo_endpoint"], "https://api.github.com/user");
    assert_eq!(held[0]["jwks_endpoint"], Value::Null);
    assert_eq!(held[0]["scope"], "user:email");
    assert_eq!(held[0]["use_pkce"], false);
    assert_eq!(held[0]["client_secret_post"], true);

    let no_tenant =
        json!({ "provider": "microsoft", "client_id": "app-id", "client_secret": SECRET });
    refused(
        &service
            .post("/sign-in-providers", Some(&ada), &no_tenant)
            .await?,
        400,
        "RequestMalformed",
    );
    let tenant_for_github = json!({ "provider": "github", "client_id": "Iv1.contract", "client_secret": SECRET, "tenant": "x" });
    refused(
        &service
            .post("/sign-in-providers", Some(&ada), &tenant_for_github)
            .await?,
        400,
        "RequestMalformed",
    );
    let microsoft = json!({ "provider": "microsoft", "client_id": "app-id", "client_secret": SECRET, "tenant": "contoso.onmicrosoft.com" });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &microsoft)
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["providers"].as_array().map(Vec::len), Some(2), "{set}");
    let held = rauthy.providers();
    assert_eq!(held[1]["name"], "Microsoft");
    assert_eq!(held[1]["typ"], "oidc");
    assert_eq!(
        held[1]["issuer"],
        "https://login.microsoftonline.com/contoso.onmicrosoft.com/v2.0"
    );
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_sets_providers_and_bad_credentials_are_refused() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let body = json!({ "provider": "google", "client_id": "id", "client_secret": SECRET });
    refused(
        &service.post("/sign-in-providers", None, &body).await?,
        401,
        "NotSignedIn",
    );
    let bea = service.sign_in(login("bea-subject")).await?;
    refused(
        &service.get("/sign-in-providers", Some(&bea)).await?,
        403,
        "NotAdmitted",
    );
    refused(
        &service
            .post("/sign-in-providers", Some(&bea), &body)
            .await?,
        403,
        "NotAdmitted",
    );
    for bad in [
        json!({ "provider": "google", "client_id": "", "client_secret": SECRET }),
        json!({ "provider": "google", "client_id": "id", "client_secret": "" }),
        json!({ "provider": "google", "client_id": "with space", "client_secret": SECRET }),
        json!({ "provider": "facebook", "client_id": "id", "client_secret": SECRET }),
        json!({ "provider": "google", "client_id": "id", "client_secret": SECRET, "extra": 1 }),
    ] {
        refused(
            &service.post("/sign-in-providers", Some(&ada), &bad).await?,
            400,
            "RequestMalformed",
        );
    }
    drop(rauthy);
    Ok(())
}

#[tokio::test]
async fn a_service_without_the_issuer_api_says_so() -> TestResult {
    let service = Service::start().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    refused(
        &service.get("/sign-in-providers", Some(&ada)).await?,
        503,
        "SignInProvidersUnavailable",
    );
    Ok(())
}
