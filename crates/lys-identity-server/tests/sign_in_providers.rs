#![cfg(test)]

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

    let held = rauthy.providers()?;
    assert_eq!(held.len(), 1);
    assert_eq!(held[0]["typ"], "google");
    let provider = service.issuer.provider_base();
    assert_eq!(held[0]["issuer"], provider);
    assert_eq!(
        held[0]["authorization_endpoint"],
        format!("{provider}/authorize")
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
    let held = rauthy.providers()?;
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
    let held = rauthy.providers()?;
    assert_eq!(held[0]["typ"], "github");
    assert_eq!(
        held[0]["token_endpoint"],
        format!(
            "{}/login/oauth/access_token",
            service.issuer.provider_base()
        )
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
    let held = rauthy.providers()?;
    assert_eq!(held[1]["name"], "Microsoft");
    assert_eq!(held[1]["typ"], "oidc");
    assert_eq!(
        held[1]["issuer"],
        format!(
            "{}/contoso.onmicrosoft.com/v2.0",
            service.issuer.provider_base()
        )
    );
    Ok(())
}

#[tokio::test]
async fn the_redirect_address_shown_is_the_one_the_provider_is_asked_with() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let (status, shown) = service.get("/sign-in-providers", Some(&ada)).await?;
    assert_eq!(status, 200, "{shown}");
    let expected = format!("{}/auth/v1/providers/callback", service.base);
    assert_eq!(shown["redirect_address"], expected.as_str());
    let body = json!({ "provider": "google", "client_id": "123-abc.apps.googleusercontent.com", "client_secret": SECRET });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &body)
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["redirect_address"], expected.as_str());
    let asked = service.issuer.provider_redirects()?;
    assert_eq!(
        asked,
        [expected],
        "the provider was proved with the address shown"
    );
    Ok(())
}

#[tokio::test]
async fn a_client_id_the_provider_rejects_is_refused_in_the_providers_words() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    for provider in ["google", "github"] {
        let body = json!({ "provider": provider, "client_id": "rejected-client", "client_secret": SECRET });
        let answer = service
            .post("/sign-in-providers", Some(&ada), &body)
            .await?;
        refused(&answer, 400, "ProviderRefused");
        let reason = answer.1["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("The OAuth client was not found."),
            "{reason}"
        );
    }
    assert!(
        rauthy.providers()?.is_empty(),
        "a refused provider is never saved"
    );
    let (status, offered) = service.get("/sign-in/providers", None).await?;
    assert_eq!(status, 200, "{offered}");
    assert_eq!(offered["providers"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_saved_provider_is_offered_on_the_sign_in_page() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let body =
        json!({ "provider": "github", "client_id": "Iv1.contract", "client_secret": SECRET });
    let (status, set) = service
        .post("/sign-in-providers", Some(&ada), &body)
        .await?;
    assert_eq!(status, 200, "{set}");
    let (status, offered) = service.get("/sign-in/providers", None).await?;
    assert_eq!(status, 200, "{offered}");
    let providers = offered["providers"].as_array().ok_or("a list")?;
    assert_eq!(providers.len(), 1, "{offered}");
    assert_eq!(providers[0]["name"], "GitHub");
    assert_eq!(providers[0]["provider"], "github");
    assert!(
        !offered.to_string().contains("Iv1.contract"),
        "the sign-in page is not given the client id: {offered}"
    );
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_sets_providers_and_bad_credentials_are_refused() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
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
    Ok(())
}

/// An issuer API nobody listens at is refused with the connection's own
/// cause, never reqwest's bare "error sending request", and without its
/// address.
#[tokio::test]
async fn an_issuer_api_that_refuses_the_connection_is_named_with_its_cause() -> TestResult {
    let rauthy = FakeRauthy::start().await?;
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = closed.local_addr()?;
    drop(closed);
    let settings = SignInProvidersSettings {
        api: format!("http://{address}"),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) =
        Service::start_setting(GRANT_MODEL, None, None, Some(settings), |_| Ok(())).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let answer = service.get("/sign-in-providers", Some(&ada)).await?;
    refused(&answer, 503, "SignInProvidersUnavailable");
    let reason = answer.1["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("error sending request: ") && reason.contains("Connection refused"),
        "{reason}"
    );
    assert!(!reason.contains(&address.to_string()), "{reason}");
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
