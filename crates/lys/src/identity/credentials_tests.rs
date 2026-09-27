use std::error::Error;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::*;
use crate::identity::private_files::mode_of;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The shortest run of a secret's bytes that counts as a leak. Eight random
/// alphanumeric characters recur by chance with probability about 62^-8 per
/// position, so a hit is a leak, not noise.
const WINDOW: usize = 8;

/// Whether `text` contains any `WINDOW`-byte run of `secret`.
fn leaks(text: &str, secret: &str) -> bool {
    secret
        .as_bytes()
        .windows(WINDOW)
        .any(|window| text.as_bytes().windows(WINDOW).any(|run| run == window))
}

#[test]
fn every_credential_debug_and_display_is_redacted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (credentials, _) = Credentials::load_or_generate(dir.path())?;
    let everything = format!("{credentials:?}");
    let mut checked = 0;
    for kind in CredentialKind::ALL {
        let secret = credentials.get(kind)?;
        assert!(secret.expose().len() >= 44, "{kind:?} is too short");
        let debug = format!("{secret:?}");
        let display = format!("{secret}");
        let kind_debug = format!("{kind:?}");
        for text in [&debug, &display, &kind_debug, &everything] {
            assert!(!leaks(text, secret.expose()), "{kind:?} leaked into {text}");
        }
        assert_eq!(debug, "Secret(<redacted>)");
        assert_eq!(display, "<redacted>");
        checked += 1;
    }
    assert_eq!(checked, CredentialKind::ALL.len());
    Ok(())
}

#[test]
fn a_client_secret_is_redacted_and_kept_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join(CREDENTIALS_DIR))?;
    // Stand-ins for a secret Rauthy issues: generated here, never real.
    let issued = Secret::alphanumeric(40);
    assert!(!leaks(&format!("{issued:?} {issued}"), issued.expose()));
    let first = store_client_secret(dir.path(), "platform", &issued)?;
    let other = Secret::alphanumeric(40);
    let second = store_client_secret(dir.path(), "platform", &other)?;
    assert_eq!(first, WriteOutcome::Created);
    assert_eq!(second, WriteOutcome::Reused);
    let kept = std::fs::read_to_string(client_secret_path(dir.path(), "platform"))?;
    assert_eq!(kept, issued.expose());
    Ok(())
}

#[test]
fn credentials_are_generated_once_and_reused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (first, created) = Credentials::load_or_generate(dir.path())?;
    let (second, reused) = Credentials::load_or_generate(dir.path())?;
    assert_eq!(created.len(), CredentialKind::ALL.len());
    assert!(created.iter().all(|(_, outcome)| *outcome == WriteOutcome::Created));
    assert_eq!(reused.len(), CredentialKind::ALL.len());
    assert!(reused.iter().all(|(_, outcome)| *outcome == WriteOutcome::Reused));
    for kind in CredentialKind::ALL {
        assert_eq!(first.get(kind)?.expose(), second.get(kind)?.expose(), "{kind:?}");
        assert_eq!(mode_of(&kind.path(dir.path()), kind.name())? & 0o077, 0, "{kind:?}");
    }
    Ok(())
}

#[test]
fn generated_credentials_are_distinct_and_well_formed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (credentials, _) = Credentials::load_or_generate(dir.path())?;
    let values: Vec<&str> = CredentialKind::ALL
        .iter()
        .map(|kind| credentials.get(*kind).map(Secret::expose))
        .collect::<IdentityResult<_>>()?;
    for (index, value) in values.iter().enumerate() {
        assert!(values[index + 1..].iter().all(|other| other != value));
    }
    assert!(credentials.get(CredentialKind::RauthyApiKey)?.expose().len() >= 64);
    let key = STANDARD.decode(credentials.get(CredentialKind::RauthyEncryptionKey)?.expose())?;
    assert_eq!(key.len(), 32);
    Ok(())
}

#[test]
fn a_credential_readable_by_others_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    Credentials::load_or_generate(dir.path())?;
    let path = CredentialKind::RauthyApiKey.path(dir.path());
    let mut permissions = std::fs::metadata(&path)?.permissions();
    let loosened = mode_of(&path, "rauthy_api_key")? | 0o044;
    set_mode_bits(&mut permissions, loosened);
    std::fs::set_permissions(&path, permissions)?;
    let refused = load_one(dir.path(), CredentialKind::RauthyApiKey);
    if mode_of(&path, "rauthy_api_key")? & 0o044 == 0 {
        // A platform without permission bits cannot express the case.
        return Ok(());
    }
    assert!(
        matches!(refused, Err(IdentityError::PrivateFileTooOpen { .. })),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_missing_credential_is_named_not_generated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing = load_one(dir.path(), CredentialKind::RauthyApiKey);
    assert!(
        matches!(&missing, Err(IdentityError::SecretMissing { resource, .. }) if resource == "rauthy_api_key"),
        "{missing:?}"
    );
    assert!(!CredentialKind::RauthyApiKey.path(dir.path()).exists());
    Ok(())
}

fn set_mode_bits(permissions: &mut std::fs::Permissions, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(mode);
    }
    #[cfg(not(unix))]
    {
        let _ = (permissions, mode);
    }
}
