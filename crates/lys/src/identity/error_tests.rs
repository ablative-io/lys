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
        ErrorKind::NotInstalled,
        ErrorKind::BinaryMissing,
        ErrorKind::VersionUnreadable,
        ErrorKind::UpgradeFailed,
        ErrorKind::InstallBuildDiffers,
        ErrorKind::UpgradeBuildDiffers,
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
