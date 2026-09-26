use std::error::Error;
use std::path::PathBuf;

use super::*;
use crate::identity::credentials::{Credentials, DECLARED};
use crate::identity::rauthy::Transport;

type TestResult = Result<(), Box<dyn Error>>;

/// The failure name each variant's message must begin with. The match has
/// no wildcard: a new variant does not compile until it is named here, and
/// so cannot escape the redaction test below.
fn expected_name(error: &IdentityError) -> String {
    match error {
        IdentityError::Io { .. } => "io_failed".to_string(),
        IdentityError::ConfigInvalid { .. } => "config_invalid".to_string(),
        IdentityError::ConfigValue { name, .. } => (*name).to_string(),
        IdentityError::InvalidIssuer { .. } => "invalid_issuer".to_string(),
        IdentityError::InvalidRedirect { .. } => "invalid_redirect_uri".to_string(),
        IdentityError::SecretMissing { .. } => "secret_missing".to_string(),
        IdentityError::SecretInvalid { .. } => "secret_invalid".to_string(),
        IdentityError::PrivateModeUnrestricted { .. } => "private_mode_unrestricted".to_string(),
        IdentityError::PrivateWriteUnconfirmed { .. } => "private_write_unconfirmed".to_string(),
        IdentityError::Unreachable { service, .. } => format!("{service}_unreachable"),
        IdentityError::UncertainOutcome { .. } => "uncertain_outcome".to_string(),
        IdentityError::UncertainUnresolved { .. } => "uncertain_outcome_unresolved".to_string(),
        IdentityError::RauthyStatus { status, .. } => status_name(*status).to_string(),
        IdentityError::RauthyResponse { .. } => "rauthy_response_invalid".to_string(),
        IdentityError::ReconcileMismatch { .. } => "reconcile_mismatch".to_string(),
        IdentityError::ThemeInvalid { .. } => "theme_invalid".to_string(),
        IdentityError::ContrastBelowTier { .. } => "contrast_below_tier".to_string(),
        IdentityError::ThemeValueUnresolved { .. } => "theme_value_unresolved".to_string(),
        IdentityError::Unready { .. } => "unready".to_string(),
    }
}

/// One of every variant, each built the way the code builds it: naming a
/// credential and its path, never holding its value.
fn every_variant(secrets: &std::path::Path) -> Vec<IdentityError> {
    let key = secrets.join("rauthy_api_key_secret");
    let io = || std::io::Error::other("fixture");
    vec![
        IdentityError::Io {
            operation: "read private file",
            path: key.clone(),
            source: io(),
        },
        IdentityError::ConfigInvalid {
            path: PathBuf::from("identity.toml"),
            reason: "unknown field `extra`".to_string(),
        },
        IdentityError::ConfigValue {
            name: "invalid_database_host",
            field: "database.host".to_string(),
            value: "localhost".to_string(),
            reason: "a loopback address names the container itself",
        },
        IdentityError::InvalidIssuer {
            value: "http://id.example.net".to_string(),
            reason: "plain http is accepted only for a loopback host",
        },
        IdentityError::InvalidRedirect {
            client: "platform",
            value: "http://app.example.net/cb".to_string(),
            reason: "plain http is accepted only for a loopback host",
        },
        IdentityError::SecretMissing {
            name: "rauthy_api_key_secret",
            path: key.clone(),
        },
        IdentityError::SecretInvalid {
            name: "rauthy_api_key_secret",
            path: key.clone(),
            reason: "not the declared number of ASCII letters and digits",
        },
        IdentityError::PrivateModeUnrestricted {
            path: key.clone(),
            mode: 0o644,
            expected: 0o600,
        },
        IdentityError::PrivateWriteUnconfirmed { path: key },
        IdentityError::Unreachable {
            service: "rauthy",
            operation: "list clients",
            resource: "/auth/v1/clients".to_string(),
            address: "127.0.0.1:8080".to_string(),
            source: io(),
        },
        IdentityError::UncertainOutcome {
            operation: "create client",
            resource: "/auth/v1/clients".to_string(),
            reason: "no complete response head arrived".to_string(),
        },
        IdentityError::UncertainUnresolved {
            operation: "create client",
            resource: "/auth/v1/clients/platform".to_string(),
        },
        IdentityError::RauthyStatus {
            operation: "list clients",
            resource: "/auth/v1/clients".to_string(),
            status: 401,
            message: "No valid session".to_string(),
        },
        IdentityError::RauthyResponse {
            operation: "read theme",
            resource: "/auth/v1/theme/platform".to_string(),
            reason: "missing field `dark`".to_string(),
        },
        IdentityError::ReconcileMismatch {
            resource: "client/platform".to_string(),
            reason: "redirect_uris does not read back as configured".to_string(),
        },
        IdentityError::ThemeInvalid {
            path: PathBuf::from("deploy/identity/rauthy-themes.json"),
            reason: "mode must be dark".to_string(),
        },
        IdentityError::ContrastBelowTier {
            client: "cambium".to_string(),
            pair: "theme_moon over ink",
            ratio: "2.10".to_string(),
            tier: "3:1 large text, icons and components",
        },
        IdentityError::ThemeValueUnresolved {
            client: "cambium".to_string(),
            field: "btn_text",
            value: "currentColor".to_string(),
        },
        IdentityError::Unready {
            failures: vec![
                "database_unreachable: refused [database at 192.0.2.10:5432]".to_string(),
            ],
        },
    ]
}

#[test]
fn every_error_names_its_failure_and_carries_no_generated_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let credentials = Credentials::load_or_generate(dir.path(), true)?;
    let secrets: Vec<String> = credentials
        .iter()
        .map(|(_, secret, _)| secret.expose().to_string())
        .collect();
    assert_eq!(secrets.len(), DECLARED.len());
    let errors = every_variant(dir.path());
    let mut checked = 0;
    for error in &errors {
        let display = error.to_string();
        let debug = format!("{error:?} {error:#?}");
        let name = expected_name(error);
        assert!(
            display.starts_with(&name),
            "{display} does not begin with {name}"
        );
        for secret in &secrets {
            assert!(
                !display.contains(secret.as_str()),
                "a secret reached: {display}"
            );
            assert!(
                !debug.contains(secret.as_str()),
                "a secret reached: {debug}"
            );
            checked += 1;
        }
    }
    assert_eq!(errors.len(), 19, "one of every variant");
    assert_eq!(checked, errors.len() * DECLARED.len());

    // The transport outcome an exchange fails with is the other error type
    // under identity/. It has Debug only; both variants are rendered.
    let transports = [
        Transport::NotSent(std::io::Error::other("connection refused")),
        Transport::Uncertain("no complete response head arrived".to_string()),
    ];
    let mut transport_checked = 0;
    for transport in &transports {
        let debug = format!("{transport:?} {transport:#?}");
        for secret in &secrets {
            assert!(
                !debug.contains(secret.as_str()),
                "a secret reached: {debug}"
            );
            transport_checked += 1;
        }
    }
    assert_eq!(transport_checked, 2 * DECLARED.len());
    Ok(())
}

#[test]
fn a_status_error_is_named_for_its_status() {
    let names: Vec<&str> = [400, 401, 403, 404, 500].map(status_name).to_vec();
    assert_eq!(
        names,
        [
            "rauthy_bad_request",
            "rauthy_unauthorized",
            "rauthy_unauthorized",
            "rauthy_not_found",
            "rauthy_status"
        ]
    );
}
