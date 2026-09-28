#![cfg(test)]
use std::path::Path;

use super::super::credentials::{Credential, Shape};
use super::*;

/// Whether `text` holds any eight consecutive characters of `secret`.
fn leaks(text: &str, secret: &str) -> bool {
    let bytes = secret.as_bytes();
    bytes.windows(8.min(bytes.len())).any(|window| {
        text.as_bytes()
            .windows(window.len())
            .any(|seen| seen == window)
    })
}

#[test]
fn every_kind_has_a_distinct_snake_case_name() {
    let kinds = [
        ErrorKind::ConfigUnreadable,
        ErrorKind::ConfigInvalid,
        ErrorKind::IssuerInvalid,
        ErrorKind::RedirectInvalid,
        ErrorKind::DatabaseAddressInvalid,
        ErrorKind::ClientInvalid,
        ErrorKind::SecretMissing,
        ErrorKind::SecretMalformed,
        ErrorKind::PrivateFileModeOpen,
        ErrorKind::PrivateFileIo,
        ErrorKind::RenderFailed,
        ErrorKind::RauthyUnreachable,
        ErrorKind::RauthyUncertain,
        ErrorKind::RauthyUnauthorized,
        ErrorKind::RauthyForbidden,
        ErrorKind::RauthyBadRequest,
        ErrorKind::RauthyServerError,
        ErrorKind::RauthyUnexpected,
        ErrorKind::ReadBackMismatch,
        ErrorKind::ThemeInvalid,
        ErrorKind::Unready,
    ];
    let mut names: Vec<&str> = kinds.iter().map(|kind| kind.name()).collect();
    assert!(
        names
            .iter()
            .all(|name| name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
    );
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), kinds.len());
}

/// The kinds whose stable name says the issuer's product, and the name a
/// person reads instead, written apart from `lys_name`.
const SAID_IN_LYS_WORDS: [(ErrorKind, &str); 7] = [
    (ErrorKind::RauthyUnreachable, "sign_in_service_unreachable"),
    (
        ErrorKind::RauthyUncertain,
        "sign_in_service_outcome_uncertain",
    ),
    (
        ErrorKind::RauthyUnauthorized,
        "sign_in_service_unauthorized",
    ),
    (ErrorKind::RauthyForbidden, "sign_in_service_forbidden"),
    (ErrorKind::RauthyBadRequest, "sign_in_service_bad_request"),
    (ErrorKind::RauthyServerError, "sign_in_service_server_error"),
    (
        ErrorKind::RauthyUnexpected,
        "sign_in_service_unexpected_response",
    ),
];

#[test]
fn a_failure_said_in_lys_words_never_names_the_issuers_product() {
    let mut said = 0;
    for (kind, lys_name) in SAID_IN_LYS_WORDS {
        let error = IdentityError::new(
            kind,
            "call Rauthy",
            "rauthy",
            "Rauthy at RAUTHY_LISTEN_PORT answered; spicedb and postgres are up",
        )
        .at(Path::new("/state/rauthy-db-password"));
        let plain = error.to_string();
        assert!(plain.starts_with(kind.name()), "{plain}");
        let text = error.said_in_lys_words().to_string();
        assert!(text.starts_with(&format!("{lys_name}: ")), "{text}");
        let outside_path = text.replace("/state/rauthy-db-password", "");
        assert!(
            !outside_path.to_ascii_lowercase().contains("rauthy"),
            "{text}"
        );
        assert!(
            !outside_path.to_ascii_lowercase().contains("spicedb"),
            "{text}"
        );
        assert!(!outside_path.contains("postgres"), "{text}");
        assert!(
            text.contains("(/state/rauthy-db-password)"),
            "the path is kept exactly"
        );
        said += 1;
    }
    assert_eq!(said, 7);
    let unchanged = IdentityError::new(ErrorKind::Unready, "wait", "services", "")
        .said_in_lys_words()
        .to_string();
    assert_eq!(unchanged, "unready: wait services");
}

#[test]
fn display_names_kind_operation_resource_and_path() {
    let error = IdentityError::new(
        ErrorKind::SecretMissing,
        "read credential",
        "rauthy-db-password",
        "run lys identity prepare",
    )
    .at(Path::new("/state/rauthy-db-password"));
    assert_eq!(
        error.to_string(),
        "secret_missing: read credential rauthy-db-password (/state/rauthy-db-password): run lys identity prepare"
    );
}

/// Every error type and credential type under the identity module, built
/// around a generated secret, formatted with Debug and Display: none shows
/// any of its bytes.
#[test]
fn no_error_or_credential_formats_a_generated_secret() {
    let generated = Credential::generate("rauthy-bootstrap-api-secret", Shape::Alphanumeric(64));
    let secret = generated.expose().to_string();
    let mut rendered = vec![format!("{generated:?}"), format!("{generated}")];

    let wrong_length = format!("{secret}x");
    let malformed =
        Credential::from_stored("key", wrong_length.as_bytes(), Shape::Alphanumeric(64));
    let wrong_key = format!("lys01/{secret}");
    let malformed_key = Credential::from_stored("key", wrong_key.as_bytes(), Shape::EncryptionKey);
    let body = format!("{{\"message\":\"rejected\",\"echo\":\"{secret}\"}}");
    let status = super::super::rauthy::status_error(400, body.as_bytes(), "cambium");
    let errors: Vec<IdentityError> = [malformed.err(), malformed_key.err(), status]
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(errors.len(), 3);
    for error in &errors {
        rendered.push(format!("{error:?}"));
        rendered.push(format!("{error}"));
        rendered.push(format!("{:?}", error.kind()));
    }
    let cli_error = crate::commands::error::CliError::from(IdentityError::new(
        ErrorKind::RauthyUnauthorized,
        "call Rauthy",
        "clients",
        "status 401",
    ));
    rendered.push(format!("{cli_error:?}"));
    rendered.push(format!("{cli_error}"));
    for text in &rendered {
        assert!(!leaks(text, &secret), "{text}");
    }
}
