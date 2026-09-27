use std::error::Error;
use std::path::PathBuf;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The shipped example, read as the operator copies it.
const EXAMPLE: &str = include_str!("../../../../deploy/identity/config.example.toml");

/// The example with `from` replaced by `to`; refuses an edit that matches
/// nothing, so a test can never pass on an unedited file.
fn edited(text: &str, from: &str, to: &str) -> TestResult<String> {
    if text.matches(from).count() != 1 {
        return Err(format!("expected exactly one {from:?} in the example").into());
    }
    Ok(text.replacen(from, to, 1))
}

fn load(text: &str) -> TestResult<(tempfile::TempDir, IdentityResult<DeploymentConfig>)> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text)?;
    let loaded = DeploymentConfig::load(&path);
    Ok((dir, loaded))
}

#[test]
fn example_config_loads_with_the_local_origin_and_the_local_database() -> TestResult {
    let (dir, loaded) = load(EXAMPLE)?;
    let config = loaded?;
    assert_eq!(config.issuer(), "http://localhost:8080/auth/v1/");
    assert_eq!(config.pub_url()?, "localhost:8080");
    assert_eq!(config.rp_id()?, "localhost");
    assert!(!config.proxy_mode()?);
    assert!(config.uses_local_database());
    assert_eq!(config.database_probe(), "127.0.0.1:55432");
    assert_eq!(config.state_dir(), dir.path().join("state"));
    let ids: Vec<&str> = config
        .managed_clients()
        .iter()
        .map(|(_, client)| client.id.as_str())
        .collect();
    assert_eq!(ids, ["platform", "cambium"]);
    assert!(config.clients.platform.pkce_s256);
    assert_eq!(config.clients.cambium.signing_alg, SigningAlg::RS256);
    Ok(())
}

#[test]
fn an_external_database_is_probed_where_configured_and_never_locally() -> TestResult {
    let text = edited(EXAMPLE, "host = \"postgres\"", "host = \"192.0.2.10\"")?;
    let (_dir, loaded) = load(&text)?;
    let config = loaded?;
    assert!(!config.uses_local_database());
    assert_eq!(config.database_probe(), "192.0.2.10:5432");
    Ok(())
}

#[test]
fn a_tls_origin_needs_a_trusted_proxy_and_then_runs_in_proxy_mode() -> TestResult {
    let origin = edited(
        EXAMPLE,
        "public_origin = \"http://localhost:8080\"",
        "public_origin = \"https://identity.example.test\"",
    )?;
    let (_dir, refused) = load(&origin)?;
    assert!(
        matches!(&refused, Err(IdentityError::InvalidConfigValue { field, .. }) if field == "rauthy.trusted_proxies"),
        "{refused:?}"
    );
    let proxied = edited(&origin, "trusted_proxies = []", "trusted_proxies = [\"10.0.0.2/32\"]")?;
    let (_dir, loaded) = load(&proxied)?;
    let config = loaded?;
    assert!(config.proxy_mode()?);
    assert_eq!(config.issuer(), "https://identity.example.test/auth/v1/");
    assert_eq!(config.pub_url()?, "identity.example.test");
    Ok(())
}

#[test]
fn every_invalid_issuer_is_refused_by_name() -> TestResult {
    let bad = [
        "http://identity.example.test",
        "https://identity.example.test/auth",
        "ftp://localhost:8080",
        "localhost:8080",
        "http://user@localhost:8080",
    ];
    let mut refused = 0;
    for origin in bad {
        let text = edited(
            EXAMPLE,
            "public_origin = \"http://localhost:8080\"",
            &format!("public_origin = \"{origin}\""),
        )?;
        let (_dir, loaded) = load(&text)?;
        assert!(
            matches!(loaded, Err(IdentityError::InvalidIssuer { .. })),
            "{origin}: {loaded:?}"
        );
        refused += 1;
    }
    assert_eq!(refused, bad.len());
    Ok(())
}

#[test]
fn every_invalid_redirect_is_refused_by_name() -> TestResult {
    let bad = [
        "http://localhost:3000/auth/callback#fragment",
        "http://cambium.example.test/callback",
        "https://*.example.test/callback",
        "/relative/callback",
    ];
    let mut refused = 0;
    for uri in bad {
        let text = edited(
            EXAMPLE,
            "redirect_uris = [\"http://localhost:3000/auth/callback\"]",
            &format!("redirect_uris = [\"{uri}\"]"),
        )?;
        let (_dir, loaded) = load(&text)?;
        assert!(
            matches!(loaded, Err(IdentityError::InvalidRedirectUri { .. })),
            "{uri}: {loaded:?}"
        );
        refused += 1;
    }
    assert_eq!(refused, bad.len());
    Ok(())
}

#[test]
fn the_builtin_client_id_is_reserved() -> TestResult {
    let text = edited(EXAMPLE, "id = \"cambium\"", "id = \"rauthy\"")?;
    let (_dir, loaded) = load(&text)?;
    assert!(
        matches!(&loaded, Err(IdentityError::BuiltinClientReserved { field }) if field == "clients.cambium.id"),
        "{loaded:?}"
    );
    Ok(())
}

#[test]
fn the_two_clients_must_have_distinct_ids() -> TestResult {
    let text = edited(EXAMPLE, "id = \"cambium\"", "id = \"platform\"")?;
    let (_dir, loaded) = load(&text)?;
    assert!(
        matches!(loaded, Err(IdentityError::DuplicateClientId { .. })),
        "{loaded:?}"
    );
    Ok(())
}

#[test]
fn a_third_client_or_an_unknown_key_is_not_configuration() -> TestResult {
    let third = format!("{EXAMPLE}\n[clients.other]\nid = \"other\"\n");
    let (_dir, loaded) = load(&third)?;
    assert!(matches!(loaded, Err(IdentityError::ConfigInvalid { .. })), "{loaded:?}");
    let unknown = edited(EXAMPLE, "port = 5432", "port = 5432\npassword = \"x\"")?;
    let (_dir, loaded) = load(&unknown)?;
    assert!(matches!(loaded, Err(IdentityError::ConfigInvalid { .. })), "{loaded:?}");
    Ok(())
}

#[test]
fn a_value_that_would_break_the_env_file_is_refused() -> TestResult {
    let text = edited(EXAMPLE, "name = \"identity\"", "name = \"identity$db\"")?;
    let (_dir, loaded) = load(&text)?;
    assert!(
        matches!(&loaded, Err(IdentityError::InvalidConfigValue { field, .. }) if field == "database.name"),
        "{loaded:?}"
    );
    Ok(())
}

#[test]
fn a_missing_file_is_named_unreadable() {
    let loaded = DeploymentConfig::load(&PathBuf::from("/nonexistent/lys/identity.toml"));
    assert!(
        matches!(loaded, Err(IdentityError::ConfigUnreadable { .. })),
        "{loaded:?}"
    );
}
