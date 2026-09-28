#![cfg(test)]
use std::error::Error;
use std::path::PathBuf;

use super::*;

const EXAMPLE: &str = include_str!("../../../../deploy/identity/config.example.toml");

fn parse(text: &str) -> IdentityResult<DeploymentConfig> {
    DeploymentConfig::parse(text, PathBuf::from("/deployments/dev"))
}

fn refused_as(text: &str, kind: ErrorKind) -> bool {
    parse(text).is_err_and(|error| error.kind() == kind)
}

#[test]
fn the_example_configuration_is_valid() -> Result<(), Box<dyn Error>> {
    let config = parse(EXAMPLE)?;
    assert_eq!(config.state_dir(), PathBuf::from("/deployments/dev/state"));
    assert_eq!(config.pub_url(), "localhost:8480");
    assert_eq!(config.rp_id(), "localhost");
    assert!(!config.public_tls());
    assert_eq!(config.managed_clients()[0].0, ClientRole::Platform);
    assert_eq!(
        config
            .clients
            .cambium
            .as_ref()
            .map(|client| client.token_alg.as_str()),
        Some("RS256")
    );
    assert_eq!(config.clients.platform.challenges, ["S256"]);
    Ok(())
}

#[test]
fn a_database_on_a_network_device_is_configuration() -> Result<(), Box<dyn Error>> {
    let text = EXAMPLE
        .replace("bundled = true", "bundled = false")
        .replace("host = \"postgres\"", "host = \"192.0.2.10\"");
    let config = parse(&text)?;
    assert_eq!(config.database.host, "192.0.2.10");
    Ok(())
}

#[test]
fn a_bundled_flag_that_disagrees_with_the_host_is_refused() {
    let text = EXAMPLE.replace("host = \"postgres\"", "host = \"192.0.2.10\"");
    assert!(refused_as(&text, ErrorKind::DatabaseAddressInvalid));
    let text = EXAMPLE.replace("bundled = true", "bundled = false");
    assert!(refused_as(&text, ErrorKind::DatabaseAddressInvalid));
}

#[test]
fn a_database_host_with_a_scheme_or_credentials_is_refused() {
    for host in ["postgres://db", "user@db", "db:5432", ""] {
        let text = EXAMPLE
            .replace("bundled = true", "bundled = false")
            .replace("host = \"postgres\"", &format!("host = \"{host}\""));
        assert!(
            refused_as(&text, ErrorKind::DatabaseAddressInvalid),
            "{host}"
        );
    }
}

#[test]
fn an_invalid_issuer_is_refused_by_name() {
    for origin in [
        "http://id.example.test",
        "ftp://localhost",
        "http://localhost:8480/auth",
        "https://user@id.example.test",
        "localhost:8480",
    ] {
        let text = EXAMPLE.replace("http://localhost:8480\"", &format!("{origin}\""));
        assert!(refused_as(&text, ErrorKind::IssuerInvalid), "{origin}");
    }
}

#[test]
fn an_https_origin_needs_its_proxy_named() -> Result<(), Box<dyn Error>> {
    let text = EXAMPLE.replace("\"http://localhost:8480\"", "\"https://id.example.test\"");
    assert!(refused_as(&text, ErrorKind::IssuerInvalid));
    let text = text.replace(
        "trusted_proxies = []",
        "trusted_proxies = [\"172.29.48.1/32\"]",
    );
    let config = parse(&text)?;
    assert!(config.public_tls());
    assert_eq!(config.pub_url(), "id.example.test");
    Ok(())
}

#[test]
fn only_the_networks_gateway_may_be_a_trusted_proxy() -> Result<(), Box<dyn Error>> {
    let mut refused = 0;
    for proxy in [
        "10.0.0.0/8",
        "172.16.0.0/12",
        "172.29.48.0/24",
        "172.29.48.2",
        "::1",
    ] {
        let text = EXAMPLE.replace(
            "trusted_proxies = []",
            &format!("trusted_proxies = [\"{proxy}\"]"),
        );
        assert!(refused_as(&text, ErrorKind::IssuerInvalid), "{proxy}");
        refused += 1;
    }
    assert_eq!(refused, 5);
    for proxy in ["172.29.48.1", "172.29.48.1/32"] {
        let text = EXAMPLE.replace(
            "trusted_proxies = []",
            &format!("trusted_proxies = [\"{proxy}\"]"),
        );
        assert_eq!(parse(&text)?.issuer.trusted_proxies, [proxy]);
    }
    Ok(())
}

#[test]
fn the_network_is_a_private_range_written_from_its_first_address() -> Result<(), Box<dyn Error>> {
    let mut refused = 0;
    for network in [
        "172.29.48.1/24",
        "8.8.8.0/24",
        "172.29.48.0/30",
        "172.29.48.0",
        "172.29.48.0/x",
        "fd00::/64",
        "10.0.0.0/7",
    ] {
        let text = EXAMPLE.replace("172.29.48.0/24", network);
        assert!(refused_as(&text, ErrorKind::ConfigInvalid), "{network}");
        refused += 1;
    }
    assert_eq!(refused, 7);
    let config = parse(&EXAMPLE.replace("172.29.48.0/24", "10.4.0.0/16"))?;
    assert_eq!(config.gateway()?, std::net::Ipv4Addr::new(10, 4, 0, 1));
    Ok(())
}

#[test]
fn an_invalid_redirect_is_refused_by_name() {
    for uri in [
        "http://app.example.test/callback",
        "https://app.example.test/callback#fragment",
        "https://app.example.test/*",
        "/relative/callback",
    ] {
        let text = EXAMPLE.replace("http://localhost:8400/auth/oidc/callback", uri);
        assert!(refused_as(&text, ErrorKind::RedirectInvalid), "{uri}");
    }
    let text = EXAMPLE.replace(
        "redirect_uris = [\"http://localhost:8490/auth/callback\"]",
        "redirect_uris = []",
    );
    assert!(refused_as(&text, ErrorKind::RedirectInvalid));
}

#[test]
fn clients_are_distinct_and_never_rauthys_own() {
    let text = EXAMPLE.replace("id = \"cambium\"", "id = \"lys-platform\"");
    assert!(refused_as(&text, ErrorKind::ClientInvalid));
    let text = EXAMPLE.replace("id = \"cambium\"", "id = \"rauthy\"");
    assert!(refused_as(&text, ErrorKind::ClientInvalid));
    let text = EXAMPLE.replace("token_alg = \"RS256\"", "token_alg = \"HS256\"");
    assert!(refused_as(&text, ErrorKind::ClientInvalid));
    let text = EXAMPLE.replace("challenges = [\"S256\"]", "challenges = [\"plain\"]");
    assert!(refused_as(&text, ErrorKind::ClientInvalid));
}

#[test]
fn unknown_fields_and_a_third_client_are_refused() {
    let text = format!("{EXAMPLE}\n[clients.other]\nid = \"other\"\n");
    assert!(refused_as(&text, ErrorKind::ConfigInvalid));
    let text = EXAMPLE.replace("[spicedb]", "[spicedb]\npreshared_key = \"inline\"");
    assert!(refused_as(&text, ErrorKind::ConfigInvalid));
}

#[test]
fn a_missing_configuration_file_is_named() {
    let result = DeploymentConfig::load(std::path::Path::new("/nonexistent/identity.toml"));
    assert!(
        result.is_err_and(|error| error.kind() == ErrorKind::ConfigUnreadable
            && error.to_string().contains("/nonexistent/identity.toml"))
    );
}

#[test]
fn lys_owns_the_password_policy_and_a_configured_one_is_taken_whole() -> Result<(), Box<dyn Error>>
{
    let absent = parse(EXAMPLE)?;
    assert_eq!(absent.password_policy, PasswordPolicy::default());
    assert_eq!(
        (
            absent.password_policy.length_min,
            absent.password_policy.length_max
        ),
        (15, 64),
        "Lys's own policy, NIST SP 800-63B-4 3.1.1.2"
    );
    let text = format!(
        "{EXAMPLE}\n[password_policy]\nlength_min = 12\nlength_max = 48\ndigits = 2\nnot_recently_used = 4\n"
    );
    let configured = parse(&text)?.password_policy;
    assert_eq!(
        configured,
        PasswordPolicy {
            length_min: 12,
            length_max: 48,
            lower_case: None,
            upper_case: None,
            digits: Some(2),
            special: None,
            not_recently_used: Some(4),
        }
    );
    Ok(())
}

#[test]
fn a_password_policy_the_sign_in_service_cannot_hold_is_refused_by_field() {
    let refused = [
        (
            "length_min = 7\nlength_max = 48",
            "password_policy.length_min",
        ),
        (
            "length_min = 12\nlength_max = 11",
            "password_policy.length_max",
        ),
        (
            "length_min = 12\nlength_max = 129",
            "password_policy.length_max",
        ),
        (
            "length_min = 12\nlength_max = 48\nspecial = 0",
            "password_policy.special",
        ),
        (
            "length_min = 12\nlength_max = 48\nnot_recently_used = 11",
            "password_policy.not_recently_used",
        ),
    ];
    for (table, field) in refused {
        let text = format!("{EXAMPLE}\n[password_policy]\n{table}\n");
        let error = parse(&text).err().map(|error| error.to_string());
        let named = error
            .as_deref()
            .is_some_and(|error| error.starts_with("config_invalid") && error.contains(field));
        assert!(named, "{field}: {error:?}");
    }
    assert_eq!(refused.len(), 5);
}
