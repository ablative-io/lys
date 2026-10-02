#![cfg(test)]
use std::error::Error;
use std::path::Path;

use super::*;

const EXAMPLE: &str = include_str!("../../../../deploy/identity/config.example.toml");

fn write_config(dir: &Path, text: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let path = dir.join("identity.toml");
    std::fs::write(&path, text)?;
    Ok(path)
}

#[test]
fn prepare_creates_every_declared_private_file_owner_only() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let config = write_config(dir.path(), EXAMPLE)?;
    run(&config, true)?;
    let state = dir.path().join("state");
    assert_eq!(
        private_files::mode_of(&state)?,
        Some(private_files::DIR_MODE)
    );
    let declared = declared_private_files();
    assert_eq!(declared.len(), SECRETS.len() + 1);
    let mut counted = 0;
    for name in &declared {
        let mode = private_files::mode_of(&state.join(name))?;
        assert_eq!(mode, Some(private_files::FILE_MODE), "{name}");
        counted += 1;
    }
    let present = std::fs::read_dir(&state)?.count();
    assert_eq!(counted, declared.len());
    assert_eq!(present, declared.len());
    Ok(())
}

#[test]
fn a_second_prepare_reuses_every_credential() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let config = write_config(dir.path(), EXAMPLE)?;
    run(&config, true)?;
    let state = dir.path().join("state");
    let first: Vec<Vec<u8>> = declared_private_files()
        .iter()
        .map(|name| std::fs::read(state.join(name)))
        .collect::<Result<_, _>>()?;
    run(&config, true)?;
    let second: Vec<Vec<u8>> = declared_private_files()
        .iter()
        .map(|name| std::fs::read(state.join(name)))
        .collect::<Result<_, _>>()?;
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn provided_credentials_that_are_missing_are_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let text = EXAMPLE.replace("source = \"generate\"", "source = \"provided\"");
    let config = write_config(dir.path(), &text)?;
    let result = run(&config, true);
    assert!(
        result.is_err_and(|error| error.kind() == ErrorKind::SecretMissing
            && error.to_string().contains("postgres-superuser-password"))
    );
    let generated = dir.path().join("state").join("postgres-superuser-password");
    assert_eq!(private_files::mode_of(&generated)?, None);
    Ok(())
}

#[test]
fn a_state_directory_others_can_read_is_refused() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let config = write_config(dir.path(), EXAMPLE)?;
    let state = dir.path().join("state");
    std::fs::create_dir(&state)?;
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755))?;
    let result = run(&config, true);
    assert!(result.is_err_and(|error| error.kind() == ErrorKind::PrivateFileModeOpen));
    Ok(())
}

#[test]
fn the_issuer_names_itself_on_the_public_origins_scheme_and_trusts_the_gateway_alone()
-> Result<(), Box<dyn Error>> {
    let credentials: Vec<(SecretSpec, Credential)> = SECRETS
        .iter()
        .map(|spec| (*spec, Credential::generate(spec.file, spec.shape)))
        .collect();
    let base = std::path::PathBuf::from("/deployments/dev");
    let local = DeploymentConfig::parse(EXAMPLE, base.clone())?;
    let env = render_env(&local, &credentials)?;
    for line in [
        "RAUTHY_PROXY_MODE=false",
        "RAUTHY_PEER_IP_HEADER_NAME=",
        "IDENTITY_NETWORK=172.29.48.0/24",
        "IDENTITY_GATEWAY=172.29.48.1",
    ] {
        assert!(env.lines().any(|found| found == line), "{line}");
    }
    let trusted = EXAMPLE.replace(
        "trusted_proxies = []",
        "trusted_proxies = [\"172.29.48.1/32\"]",
    );
    let env = render_env(
        &DeploymentConfig::parse(&trusted, base.clone())?,
        &credentials,
    )?;
    for line in [
        "RAUTHY_PROXY_MODE=false",
        "RAUTHY_PEER_IP_HEADER_NAME=X-Forwarded-For",
        "RAUTHY_TRUSTED_PROXIES=\"172.29.48.1/32\"",
    ] {
        assert!(env.lines().any(|found| found == line), "{line}");
    }
    let tls = trusted.replace("\"http://localhost:8480\"", "\"https://id.example.test\"");
    let env = render_env(&DeploymentConfig::parse(&tls, base)?, &credentials)?;
    assert!(env.lines().any(|found| found == "RAUTHY_PROXY_MODE=true"));
    Ok(())
}

#[test]
fn the_rendered_environment_names_the_configured_database_host() -> Result<(), Box<dyn Error>> {
    let text = EXAMPLE
        .replace("bundled = true", "bundled = false")
        .replace("host = \"postgres\"", "host = \"192.0.2.10\"");
    let config = DeploymentConfig::parse(&text, std::path::PathBuf::from("/deployments/dev"))?;
    let credentials: Vec<(SecretSpec, Credential)> = SECRETS
        .iter()
        .map(|spec| (*spec, Credential::generate(spec.file, spec.shape)))
        .collect();
    let env = render_env(&config, &credentials)?;
    assert!(
        env.lines()
            .any(|line| line == "IDENTITY_DB_HOST=192.0.2.10")
    );
    assert!(env.lines().any(|line| line == "COMPOSE_PROFILES="));
    assert!(!env.contains("127.0.0.1") && !env.contains("=postgres\n"));
    for spec in SECRETS {
        assert!(
            env.lines()
                .any(|line| line.starts_with(&format!("{}=", spec.variable)))
        );
    }
    Ok(())
}

#[test]
fn the_install_key_grants_clients_users_providers_keys_and_the_policy_write()
-> Result<(), Box<dyn Error>> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(bootstrap_api_key())?;
    let request: serde_json::Value = serde_json::from_slice(&decoded)?;
    assert_eq!(request["name"], API_KEY_NAME);
    let groups: Vec<&str> = request["access"]
        .as_array()
        .ok_or("access is not a list")?
        .iter()
        .filter_map(|entry| entry["group"].as_str())
        .collect();
    assert_eq!(
        groups,
        ["Clients", "Secrets", "Users", "AuthProviders", "ApiKeys"]
    );
    assert_eq!(
        request["access"][1]["access_rights"],
        serde_json::json!(["read", "update"]),
        "the install writes Lys's password policy, which the issuer keeps under secrets"
    );
    assert_eq!(
        request["access"][2]["access_rights"],
        serde_json::json!(["read", "create", "update"]),
        "the directory service makes and changes accounts"
    );
    assert_eq!(
        request["access"][4]["access_rights"],
        serde_json::json!(["read", "create", "update"]),
        "the install makes the directory service's own, narrower key"
    );
    Ok(())
}

fn rendered_for(text: &str) -> Result<String, Box<dyn Error>> {
    let config = DeploymentConfig::parse(text, std::path::PathBuf::from("/deployments/dev"))?;
    let credentials: Vec<(SecretSpec, Credential)> = SECRETS
        .iter()
        .map(|spec| (*spec, Credential::generate(spec.file, spec.shape)))
        .collect();
    Ok(render_env(&config, &credentials)?.to_string())
}

#[test]
fn a_plain_http_origin_asks_for_a_session_cookie_a_browser_keeps_over_http()
-> Result<(), Box<dyn Error>> {
    let env = rendered_for(EXAMPLE)?;
    assert!(
        env.lines()
            .any(|line| line == "RAUTHY_PUB_URL=localhost:8480"),
        "the example is served over plain http on loopback"
    );
    assert!(
        env.lines()
            .any(|line| line == "RAUTHY_COOKIE_MODE=danger-insecure"),
        "a Secure cookie is dropped by a browser over http, so no session survives the sign-in"
    );
    Ok(())
}

#[test]
fn an_https_origin_keeps_the_host_bound_secure_session_cookie() -> Result<(), Box<dyn Error>> {
    let text = EXAMPLE
        .replace(
            "public_origin = \"http://localhost:8480\"",
            "public_origin = \"https://identity.example.test\"",
        )
        .replace(
            "trusted_proxies = []",
            "trusted_proxies = [\"172.29.48.1/32\"]",
        );
    let env = rendered_for(&text)?;
    assert!(
        env.lines()
            .any(|line| line == "RAUTHY_PUB_URL=identity.example.test")
    );
    assert!(env.lines().any(|line| line == "RAUTHY_COOKIE_MODE=host"));
    Ok(())
}
