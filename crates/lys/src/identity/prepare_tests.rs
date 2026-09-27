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
fn the_bootstrap_api_key_grants_only_clients_and_secret_reads() -> Result<(), Box<dyn Error>> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(bootstrap_api_key())?;
    let request: serde_json::Value = serde_json::from_slice(&decoded)?;
    assert_eq!(request["name"], API_KEY_NAME);
    let groups: Vec<&str> = request["access"]
        .as_array()
        .ok_or("access is not a list")?
        .iter()
        .filter_map(|entry| entry["group"].as_str())
        .collect();
    assert_eq!(groups, ["Clients", "Secrets"]);
    assert_eq!(
        request["access"][1]["access_rights"],
        serde_json::json!(["read"])
    );
    Ok(())
}
