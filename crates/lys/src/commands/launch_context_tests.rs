use super::{read_at, surface};
use crate::commands::sign_in::{Pass, pass_file};
use crate::identity::private_files;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn save(root: &std::path::Path, server: &str, expires_at: u64) -> Result {
    let pass = Pass {
        server: server.to_owned(),
        person: "person-0123456789abcdef0123456789abcdef".to_owned(),
        expires_at,
        access_token: "fixture-private-access".to_owned(),
        refresh_token: "fixture-private-refresh".to_owned(),
    };
    private_files::ensure_dir(&root.join("client"))?;
    private_files::write(&pass_file(root), &serde_json::to_vec(&pass)?)?;
    Ok(())
}

#[test]
fn no_server_and_no_sign_in_names_the_missing_address_without_a_default() -> Result {
    let root = tempfile::tempdir()?;
    let error = read_at(root.path(), None)
        .err()
        .ok_or("missing state accepted")?
        .to_string();
    assert!(error.contains("server_address_missing"));
    assert!(error.contains("lys sign-in --server <server-address>"));
    assert!(!error.contains("ablative.com.au"));
    Ok(())
}

#[test]
fn missing_person_pass_names_the_exact_explicit_sign_in_command() -> Result {
    let root = tempfile::tempdir()?;
    let error = read_at(root.path(), Some("https://issuer.example/api"))
        .err()
        .ok_or("missing state accepted")?
        .to_string();
    assert!(error.contains("person_pass_missing"));
    assert!(error.contains("lys sign-in --server https://issuer.example/api"));
    Ok(())
}

#[test]
fn an_expired_pass_uses_its_recorded_server_without_echoing_credentials() -> Result {
    let root = tempfile::tempdir()?;
    save(root.path(), "https://issuer.example/api", 1)?;
    let error = read_at(root.path(), None)
        .err()
        .ok_or("expired state accepted")?
        .to_string();
    assert!(error.contains("person_pass_expired"));
    assert!(error.contains("lys sign-in --server https://issuer.example/api"));
    assert!(!error.contains("fixture-private"));
    Ok(())
}

#[test]
fn an_explicit_other_server_refuses_before_any_request() -> Result {
    let root = tempfile::tempdir()?;
    save(root.path(), "https://issuer.example/api", u64::MAX)?;
    let error = read_at(root.path(), Some("https://another.example/api"))
        .err()
        .ok_or("wrong server accepted")?
        .to_string();
    assert!(error.contains("person_pass_wrong_server"));
    assert!(error.contains("lys sign-in --server https://another.example/api"));
    Ok(())
}

#[test]
fn personal_sign_in_does_not_read_a_local_operator_install() -> Result {
    let root = tempfile::tempdir()?;
    save(root.path(), "http://127.0.0.1:12345/api", u64::MAX)?;
    std::fs::write(
        root.path().join("identity.json"),
        "invalid operator configuration",
    )?;
    let context = read_at(root.path(), None)?;
    assert_eq!(context.base, "http://127.0.0.1:12345/api");
    assert_eq!(context.person, "person-0123456789abcdef0123456789abcdef");
    assert!(context.server.separate().is_ok());
    Ok(())
}

#[test]
fn a_stored_address_with_credentials_is_refused_before_it_is_reached() -> Result {
    let root = tempfile::tempdir()?;
    save(
        root.path(),
        "https://name:credential@issuer.example/api",
        u64::MAX,
    )?;
    let error = read_at(root.path(), None)
        .err()
        .ok_or("unsafe server accepted")?
        .to_string();
    assert!(error.contains("server_base_invalid"));
    assert!(!error.contains("credential@"));
    Ok(())
}

#[test]
fn surface_links_keep_the_recorded_prefix() {
    assert_eq!(
        surface("https://issuer.example/lys/api", "agents/new"),
        "https://issuer.example/lys/#/agents/new"
    );
}
