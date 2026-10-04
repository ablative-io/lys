//! Effective settings are administrator-only metadata and never disclose configuration secrets or private paths.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn configuration_is_the_effective_startup_view_without_secrets() -> TestResult {
    let service = Service::start().await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["source"], "startup_configuration");
    assert_eq!(body["mutable_in_browser"], false);
    // The sign-in service is named on Lys's own origin, as installed; its
    // loopback address is never shown.
    assert_eq!(body["sign_in"]["provider_origin"], service.base.as_str());
    assert!(!body.to_string().contains(service.issuer.loopback()));
    assert!(body["sign_in"]["session_seconds"].as_u64().is_some());
    assert!(body["sign_in"]["secure_cookie"].as_bool().is_some());
    assert_eq!(body["permissions"]["projection"], "local");
    assert_eq!(body["secrets"]["configured"], false);
    assert_eq!(body["storage"]["directory_format"], "signed_leaf_log");
    let text = body.to_string();
    for private in [
        "client_secret",
        "key_file",
        "log_dir",
        "administrator",
        "link_audit_source",
    ] {
        assert!(!text.contains(private), "exposed {private}");
    }
    let signin = body["sign_in"]
        .as_object()
        .ok_or("sign-in metadata missing")?;
    assert_eq!(signin.len(), 3);
    Ok(())
}

#[tokio::test]
async fn configuration_refuses_anonymous_and_other_people() -> TestResult {
    let service = Service::start().await?;
    let (status, _) = service.get("/configuration", None).await?;
    assert_eq!(status, 401);
    let cookie = service
        .sign_in(Login {
            subject: "ordinary-person".to_owned(),
            email: "ordinary@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 403);
    assert_eq!(body["refusal"], "NotAdmitted");
    assert!(body.get("sign_in").is_none());
    Ok(())
}

#[tokio::test]
async fn model_windows_are_declared_kept_and_refused_by_name() -> TestResult {
    use identity_contract::apps::{Auth, send};
    use serde_json::json;
    // The setting is changed by a person: the administrator is one in the directory.
    let (mut service, _seeded) = Service::start_with(|config| {
        Ok(lys_identity_server::dev_seed::seed_configured(
            config,
            [ADMINISTRATOR, "other-subject"],
        )?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    // Nothing is declared at first, and no kept call has named a model.
    assert_eq!(body["organisation"].get("model_windows"), None);
    assert_eq!(body["models_undeclared"], json!([]));
    let zone = body["organisation"]["zone"].clone();
    let version = body["organisation"]["version"]
        .as_u64()
        .ok_or("no version")?;
    let windows = json!({"model-one": 200_000, "small-model": null});
    let put = |body: serde_json::Value| {
        let (service, cookie) = (&service, &cookie);
        async move {
            send(
                service,
                reqwest::Method::PUT,
                "/configuration",
                Auth::Cookie(cookie),
                Some(&body),
            )
            .await
        }
    };
    let (status, answer) =
        put(json!({"zone": zone, "version": version, "model_windows": windows})).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["model_windows"], windows);
    assert_eq!(answer["version"], version + 1);
    // The zone set without the table leaves the table as held.
    let (status, answer) = put(json!({"zone": zone, "version": version + 1})).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["model_windows"], windows);
    // A window of no tokens and a model with no name are refused by name.
    for refused in [json!({"model-one": 0}), json!({" ": 5}), json!({"": null})] {
        let (status, answer) =
            put(json!({"zone": zone, "version": version + 2, "model_windows": refused})).await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "ConfigurationMalformed");
    }
    service.restart().await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["organisation"]["model_windows"], windows);
    assert_eq!(body["organisation"]["version"], version + 2);
    Ok(())
}
