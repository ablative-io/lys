use std::error::Error;
use std::path::{Path, PathBuf};

use super::*;

type TestResult = Result<(), Box<dyn Error>>;

/// The example config the repository ships, which every case edits.
fn example() -> Result<String, std::io::Error> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/config.example.toml");
    std::fs::read_to_string(path)
}

/// `text` with `key` in `[section]` set to the TOML `value`, or `None` when
/// the key is not in that section — so a case can never silently edit
/// nothing.
fn set(text: &str, section: &str, key: &str, value: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut in_section = false;
    let mut replaced = false;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
        }
        if in_section && !replaced && trimmed.starts_with(&format!("{key} =")) {
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(value);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    replaced.then_some(out)
}

/// Writes `text` as a config in `dir` and loads it.
fn load_in(dir: &Path, text: &str) -> Result<Result<LoadedConfig, IdentityError>, Box<dyn Error>> {
    let path = dir.join("identity.toml");
    std::fs::write(&path, text)?;
    Ok(LoadedConfig::load(&path))
}

/// The refusal message for `text`, or an error when it was accepted.
fn refusal_text(text: &str) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let loaded = load_in(dir.path(), text)?;
    Ok(loaded.err().ok_or("the config was accepted")?.to_string())
}

/// Loads the example with one edit applied, and returns the refusal's
/// message, or an error when the edit was accepted.
fn refusal(section: &str, key: &str, value: &str) -> Result<String, Box<dyn Error>> {
    let text =
        set(&example()?, section, key, value).ok_or("the key to edit is not in the example")?;
    refusal_text(&text)
}

#[test]
fn the_shipped_example_validates_and_derives_its_issuer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let loaded = load_in(dir.path(), &example()?)??;
    assert_eq!(loaded.issuer(), "http://localhost:8080/auth/v1/");
    assert_eq!(loaded.private_dir, dir.path().join("private"));
    assert_eq!(loaded.admin.to_string(), "http://127.0.0.1:8080");
    assert_eq!(loaded.spicedb_http.effective_port(), 8443);
    let ids = loaded
        .config
        .clients
        .managed()
        .map(|(_, spec)| spec.id.clone());
    assert_eq!(ids, ["platform", "cambium"]);
    Ok(())
}

#[test]
fn the_database_address_is_configuration_and_nothing_substitutes_it() -> TestResult {
    let text = set(&example()?, "database", "host", "\"192.0.2.10\"").ok_or("no database.host")?;
    let dir = tempfile::tempdir()?;
    let loaded = load_in(dir.path(), &text)??;
    assert_eq!(loaded.config.database.host, "192.0.2.10");
    Ok(())
}

#[test]
fn every_named_refusal_fires_by_its_own_name() -> TestResult {
    let cases = [
        ("database", "host", "\"localhost\"", "invalid_database_host"),
        ("database", "host", "\"127.0.0.1\"", "invalid_database_host"),
        (
            "database",
            "host",
            "\"db.example.net:5432\"",
            "invalid_database_host",
        ),
        ("database", "sslmode", "\"verify-none\"", "invalid_sslmode"),
        ("database", "admin_user", "\"rauthy\"", "invalid_identifier"),
        (
            "rauthy",
            "public_origin",
            "\"http://id.example.net\"",
            "invalid_issuer",
        ),
        (
            "rauthy",
            "public_origin",
            "\"https://id.example.net/auth\"",
            "invalid_issuer",
        ),
        (
            "rauthy",
            "public_origin",
            "\"ftp://id.example.net\"",
            "invalid_issuer",
        ),
        (
            "rauthy",
            "admin_origin",
            "\"https://127.0.0.1:8080\"",
            "invalid_local_origin",
        ),
        (
            "rauthy",
            "trusted_proxy",
            "\"10.0.0.0/8\"",
            "invalid_trusted_proxy",
        ),
        (
            "rauthy",
            "admin_email",
            "\"not an address\"",
            "invalid_admin_email",
        ),
        ("clients.platform", "id", "\"rauthy\"", "invalid_client_id"),
        ("clients.cambium", "id", "\"platform\"", "invalid_client_id"),
        (
            "clients.cambium",
            "signing_alg",
            "\"HS256\"",
            "invalid_signing_alg",
        ),
        (
            "clients.platform",
            "challenges",
            "[\"plain\"]",
            "invalid_challenge",
        ),
        (
            "clients.platform",
            "redirect_uris",
            "[\"http://app.example.net/callback\"]",
            "invalid_redirect_uri",
        ),
        (
            "clients.platform",
            "redirect_uris",
            "[\"https://app.example.net/*\"]",
            "invalid_redirect_uri",
        ),
        (
            "clients.cambium",
            "post_logout_redirect_uris",
            "[\"https://app.example.net/#done\"]",
            "invalid_redirect_uri",
        ),
        (
            "clients.cambium",
            "redirect_uris",
            "[]",
            "invalid_redirect_uri",
        ),
        ("spicedb", "http_publish", "\"eighty\"", "invalid_publish"),
    ];
    let mut fired = 0;
    for (section, key, value, name) in cases {
        let message = refusal(section, key, value)?;
        assert!(
            message.starts_with(name),
            "{section}.{key} = {value}: expected {name}, got {message}"
        );
        fired += 1;
    }
    assert_eq!(fired, cases.len());
    Ok(())
}

#[test]
fn an_unknown_or_missing_key_is_config_invalid_never_a_default() -> TestResult {
    let unknown = set(
        &example()?,
        "database",
        "port",
        "5432\nfallback_host = \"localhost\"",
    )
    .ok_or("no database.port")?;
    let message = refusal_text(&unknown)?;
    assert!(message.starts_with("config_invalid"), "got {message}");
    assert!(message.contains("fallback_host"), "got {message}");

    let missing = example()?
        .lines()
        .filter(|line| !line.trim_start().starts_with("host ="))
        .collect::<Vec<_>>()
        .join("\n");
    let message = refusal_text(&missing)?;
    assert!(message.starts_with("config_invalid"), "got {message}");
    assert!(message.contains("host"), "got {message}");
    Ok(())
}

#[test]
fn an_https_origin_needs_its_proxy_and_yields_an_https_issuer() -> TestResult {
    let text = set(
        &example()?,
        "rauthy",
        "public_origin",
        "\"https://id.example.net\"",
    )
    .ok_or("no public_origin")?;
    let without_proxy = refusal_text(&text)?;
    assert!(
        without_proxy.starts_with("invalid_trusted_proxy"),
        "got {without_proxy}"
    );
    let text =
        set(&text, "rauthy", "trusted_proxy", "\"172.16.0.0/12\"").ok_or("no trusted_proxy")?;
    let dir = tempfile::tempdir()?;
    let loaded = load_in(dir.path(), &text)??;
    assert_eq!(loaded.issuer(), "https://id.example.net/auth/v1/");
    assert_eq!(loaded.public.effective_port(), 443);
    Ok(())
}
