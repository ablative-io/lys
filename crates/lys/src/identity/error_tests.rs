//! The redaction test: every error variant, produced wherever possible by
//! the real operation that fails while generated secrets are in play, is
//! formatted with `Debug` and `Display` and searched for any run of those
//! secrets' bytes.

use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use super::*;
use crate::identity::config::DeploymentConfig;
use crate::identity::credentials::{CredentialKind, Credentials, load_one};
use crate::identity::private_files::{mode_of, write_private};
use crate::identity::rauthy::{RauthyApi, ThemeColours};
use crate::identity::themes::{ThemeMapping, check_contrast};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const EXAMPLE: &str = include_str!("../../../../deploy/identity/config.example.toml");

/// See `credentials_tests.rs`: an eight-byte run of a secret is a leak.
const WINDOW: usize = 8;

fn leaks(text: &str, secret: &str) -> bool {
    secret
        .as_bytes()
        .windows(WINDOW)
        .any(|window| text.as_bytes().windows(WINDOW).any(|run| run == window))
}

/// Every variant by name. The match has no wildcard, so a new variant fails
/// to compile here until the test produces and checks it.
fn variant(error: &IdentityError) -> &'static str {
    match error {
        IdentityError::ConfigUnreadable { .. } => "config_unreadable",
        IdentityError::ConfigInvalid { .. } => "config_invalid",
        IdentityError::InvalidIssuer { .. } => "invalid_issuer",
        IdentityError::InvalidRedirectUri { .. } => "invalid_redirect_uri",
        IdentityError::InvalidConfigValue { .. } => "invalid_config_value",
        IdentityError::BuiltinClientReserved { .. } => "builtin_client_reserved",
        IdentityError::DuplicateClientId { .. } => "duplicate_client_id",
        IdentityError::StateDirInsideGit { .. } => "state_dir_inside_git",
        IdentityError::SecretMissing { .. } => "secret_missing",
        IdentityError::PrivateFileTooOpen { .. } => "private_file_too_open",
        IdentityError::Io { .. } => "io_failed",
        IdentityError::ThemesInvalid { .. } => "themes_invalid",
        IdentityError::ContrastBelowTier { .. } => "theme_contrast_below_tier",
        IdentityError::ThemeValueUnmeasurable { .. } => "theme_value_unmeasurable",
        IdentityError::Unreachable { .. } => "service_unreachable",
        IdentityError::OutcomeUncertain { .. } => "outcome_uncertain",
        IdentityError::RauthyStatus { .. } => "rauthy_status",
        IdentityError::RauthyResponseInvalid { .. } => "rauthy_response_invalid",
        IdentityError::UnmanagedClients { .. } => "unmanaged_clients",
        IdentityError::ServicesUnready { .. } => "services_unready",
    }
}

const VARIANTS: usize = 20;

fn config_error(dir: &Path, from: &str, to: &str) -> TestResult<IdentityError> {
    let path = dir.join(format!("config-{}.toml", to.len()));
    std::fs::write(&path, EXAMPLE.replacen(from, to, 1))?;
    match DeploymentConfig::load(&path) {
        Err(error) => Ok(error),
        Ok(_) => Err(format!("editing {from:?} to {to:?} was accepted").into()),
    }
}

/// Serve one connection on a fresh loopback port. `respond` gets the request
/// head; `None` closes the connection without a response.
fn serve_once(respond: fn(&str) -> Option<String>) -> TestResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            if let Some(response) = respond(&String::from_utf8_lossy(&request)) {
                let written = stream.write_all(response.as_bytes());
                assert!(written.is_ok(), "the fake server could not answer");
            }
        }
    });
    Ok(address)
}

fn closed_port() -> TestResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.to_string())
}

fn result_error<T>(result: IdentityResult<T>, what: &str) -> TestResult<IdentityError> {
    result.err().ok_or_else(|| format!("{what} unexpectedly succeeded").into())
}

fn theme(bg: [u16; 3], btn_text: &str) -> ThemeColours {
    ThemeColours {
        text: bg,
        text_high: bg,
        bg,
        bg_high: bg,
        action: bg,
        accent: bg,
        error: bg,
        btn_text: btn_text.to_string(),
        theme_sun: "hsla(var(--action) / .7)".to_string(),
        theme_moon: "hsla(var(--accent) / .85)".to_string(),
    }
}

#[test]
fn no_error_debug_or_display_carries_a_generated_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let state = dir.path().join("state");
    let (credentials, _) = Credentials::load_or_generate(&state)?;
    let secrets: Vec<String> = CredentialKind::ALL
        .iter()
        .map(|kind| credentials.get(*kind).map(|secret| secret.expose().to_string()))
        .collect::<IdentityResult<_>>()?;
    let api_key = load_one(&state, CredentialKind::RauthyApiKey)?;

    let mut errors = vec![
        result_error(DeploymentConfig::load(&dir.path().join("absent.toml")), "absent config")?,
        config_error(dir.path(), "port = 5432", "port = \"x\"")?,
        config_error(
            dir.path(),
            "public_origin = \"http://localhost:8080\"",
            "public_origin = \"http://identity.example.test\"",
        )?,
        config_error(
            dir.path(),
            "redirect_uris = [\"http://localhost:3000/auth/callback\"]",
            "redirect_uris = [\"http://localhost:3000/#f\"]",
        )?,
        config_error(dir.path(), "name = \"identity\"", "name = \"Identity!\"")?,
        config_error(dir.path(), "id = \"cambium\"", "id = \"rauthy\"")?,
        config_error(dir.path(), "id = \"cambium\"", "id = \"platform\"")?,
    ];

    // A state directory inside a Git work tree.
    let worktree = dir.path().join("repo");
    std::fs::create_dir_all(worktree.join(".git"))?;
    let config_path = worktree.join("config.toml");
    std::fs::write(&config_path, EXAMPLE)?;
    let config = DeploymentConfig::load(&config_path)?;
    errors.push(result_error(crate::identity::prepare::prepare(&config), "prepare in Git")?);

    // Missing, too-open and unwritable private files.
    errors.push(result_error(load_one(&dir.path().join("empty"), CredentialKind::RauthyApiKey), "load")?);
    let open_path = CredentialKind::HiqliteApi.path(&state);
    let mut permissions = std::fs::metadata(&open_path)?.permissions();
    loosen(&mut permissions, mode_of(&open_path, "rauthy_hiqlite_api")? | 0o044);
    std::fs::set_permissions(&open_path, permissions)?;
    if mode_of(&open_path, "rauthy_hiqlite_api")? & 0o044 == 0 {
        errors.push(IdentityError::PrivateFileTooOpen {
            resource: "rauthy_hiqlite_api".to_string(),
            path: open_path,
            mode: 0o644,
        });
    } else {
        errors.push(result_error(load_one(&state, CredentialKind::HiqliteApi), "too open")?);
    }
    let unwritable = dir.path().join("no-such-dir").join("identity.env");
    errors.push(result_error(write_private(&unwritable, secrets[0].as_bytes(), "identity.env"), "write")?);

    // Themes.
    errors.push(result_error(ThemeMapping::load(&dir.path().join("absent.json")), "themes")?);
    errors.push(result_error(check_contrast("platform", &theme([210, 14, 5], "white")), "contrast")?);
    errors.push(result_error(check_contrast("platform", &theme([0, 0, 100], "url(x)")), "css")?);

    // Rauthy, holding the generated API key throughout.
    let unreachable = RauthyApi::new(&closed_port()?, &api_key);
    errors.push(result_error(unreachable.read_client("platform"), "closed port")?);
    let silent = RauthyApi::new(&serve_once(|_| None)?, &api_key);
    errors.push(result_error(silent.read_client("platform"), "silent server")?);
    let echo = RauthyApi::new(
        &serve_once(|request| {
            let body = serde_json::json!({ "message": request }).to_string();
            Some(format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ))
        })?,
        &api_key,
    );
    errors.push(result_error(echo.read_client("platform"), "echoing server")?);
    let garbled = RauthyApi::new(
        &serve_once(|_| Some("HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nnot json!".to_string()))?,
        &api_key,
    );
    errors.push(result_error(garbled.read_client("platform"), "garbled server")?);

    // The last two carry only client ids and service names by construction.
    errors.push(IdentityError::UnmanagedClients {
        ids: "stray".to_string(),
    });
    errors.push(IdentityError::ServicesUnready {
        names: "database (database_unreachable)".to_string(),
    });

    let mut seen: Vec<&'static str> = Vec::new();
    for error in &errors {
        let name = variant(error);
        let rendered = [format!("{error:?}"), error.to_string()];
        assert!(rendered[1].starts_with(name), "{name}: {}", rendered[1]);
        for text in &rendered {
            for secret in &secrets {
                assert!(!leaks(text, secret), "{name} carried a generated secret");
            }
        }
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    assert_eq!(seen.len(), VARIANTS, "variants produced: {seen:?}");
    Ok(())
}

#[test]
fn a_status_error_from_an_echoing_server_is_scrubbed_but_still_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    Credentials::load_or_generate(dir.path())?;
    let api_key = load_one(dir.path(), CredentialKind::RauthyApiKey)?;
    let echo = RauthyApi::new(
        &serve_once(|request| {
            let body = serde_json::json!({ "message": request }).to_string();
            Some(format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ))
        })?,
        &api_key,
    );
    let error = result_error(echo.read_client("platform"), "echoing server")?;
    let text = error.to_string();
    assert!(text.starts_with("rauthy_status: read client platform returned 403 (forbidden)"), "{text}");
    assert!(text.contains("<redacted>"), "the echoed key was not scrubbed: {text}");
    assert!(!leaks(&text, api_key.expose()));
    Ok(())
}

fn loosen(permissions: &mut std::fs::Permissions, mode: u32) {
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

#[test]
fn every_path_in_an_error_is_the_path_that_failed() -> TestResult {
    let missing = PathBuf::from("/nonexistent/lys/identity.toml");
    let error = result_error(DeploymentConfig::load(&missing), "absent config")?;
    assert!(error.to_string().contains("/nonexistent/lys/identity.toml"));
    Ok(())
}
