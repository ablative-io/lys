use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use super::*;
use crate::identity::private_files::{PRIVATE_DIR_MODE, PRIVATE_FILE_MODE, mode_of};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const EXAMPLE: &str = include_str!("../../../../deploy/identity/config.example.toml");
const COMPOSE: &str = include_str!("../../../../deploy/identity/compose.yaml");

fn config_in(dir: &Path, text: &str) -> TestResult<DeploymentConfig> {
    let path = dir.join("config.toml");
    std::fs::write(&path, text)?;
    Ok(DeploymentConfig::load(&path)?)
}

/// Every regular file under `dir`, relative to it.
fn files_under(dir: &Path) -> TestResult<BTreeSet<String>> {
    let mut found = BTreeSet::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
                found.insert(relative);
            }
        }
    }
    Ok(found)
}

fn env_keys(env: &str) -> BTreeSet<String> {
    env.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(key, _)| key.to_string()))
        .collect()
}

#[test]
fn prepare_creates_exactly_the_declared_private_files_with_restricted_modes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let config = config_in(dir.path(), EXAMPLE)?;
    let report = prepare(&config)?;
    let state = config.state_dir();
    let declared: BTreeSet<String> = declared_private_files().into_iter().collect();
    assert_eq!(declared.len(), 10, "the env file and nine credentials");
    let present = files_under(&state)?;
    assert_eq!(present, declared, "prepare wrote a file outside the declared set");
    let mut checked = 0;
    for relative in &declared {
        let mode = mode_of(&state.join(relative), relative)?;
        assert_eq!(mode, PRIVATE_FILE_MODE, "{relative} has mode {mode:o}");
        checked += 1;
    }
    assert_eq!(checked, declared.len());
    assert_eq!(report.files.len(), declared.len());
    for directory in [state.clone(), state.join(CREDENTIALS_DIR)] {
        let mode = mode_of(&directory, "state_dir")?;
        assert_eq!(mode, PRIVATE_DIR_MODE, "{} has mode {mode:o}", directory.display());
    }
    Ok(())
}

#[test]
fn a_second_prepare_reuses_every_credential_and_leaves_the_env_file_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let config = config_in(dir.path(), EXAMPLE)?;
    let first = prepare(&config)?;
    let env_before = std::fs::read(config.state_dir().join(ENV_FILE))?;
    let second = prepare(&config)?;
    assert!(first.files.iter().all(|(_, outcome)| *outcome == WriteOutcome::Created));
    let outcomes: Vec<WriteOutcome> = second.files.iter().map(|(_, outcome)| *outcome).collect();
    assert_eq!(outcomes[0], WriteOutcome::Unchanged, "the env file");
    assert!(outcomes[1..].iter().all(|outcome| *outcome == WriteOutcome::Reused));
    assert_eq!(std::fs::read(config.state_dir().join(ENV_FILE))?, env_before);
    Ok(())
}

#[test]
fn the_env_file_sets_every_variable_compose_interpolates() -> TestResult {
    let dir = tempfile::tempdir()?;
    let config = config_in(dir.path(), EXAMPLE)?;
    prepare(&config)?;
    let env = std::fs::read_to_string(config.state_dir().join(ENV_FILE))?;
    let keys = env_keys(&env);
    let referenced: BTreeSet<String> = COMPOSE
        .split("${")
        .skip(1)
        .filter_map(|rest| {
            let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
            Some(rest[..end].to_string())
        })
        .collect();
    assert!(referenced.len() >= 20, "compose references {referenced:?}");
    let missing: Vec<&String> = referenced.difference(&keys).collect();
    assert!(missing.is_empty(), "compose needs {missing:?}");
    assert!(env.contains("\nCOMPOSE_PROFILES=local-database\n"));
    assert!(env.contains("\nIDENTITY_DB_HOST=postgres\n"));
    Ok(())
}

#[test]
fn an_external_database_host_reaches_the_env_file_and_disables_the_local_service() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = EXAMPLE.replacen("host = \"postgres\"", "host = \"192.0.2.10\"", 1);
    assert_ne!(text, EXAMPLE);
    let config = config_in(dir.path(), &text)?;
    prepare(&config)?;
    let env = std::fs::read_to_string(config.state_dir().join(ENV_FILE))?;
    assert!(env.contains("\nIDENTITY_DB_HOST=192.0.2.10\n"));
    assert!(env.contains("\nCOMPOSE_PROFILES=\n"));
    assert!(!env.contains("IDENTITY_DB_HOST=postgres"));
    Ok(())
}

#[test]
fn the_bootstrap_api_key_can_read_create_and_update_but_never_delete() -> TestResult {
    let decoded = STANDARD.decode(bootstrap_api_key())?;
    let definition: Value = serde_json::from_slice(&decoded)?;
    assert_eq!(definition["name"], API_KEY_NAME);
    assert!(!definition.to_string().contains("delete"));
    assert!(definition.get("exp").is_none());
    Ok(())
}

#[test]
fn a_state_directory_inside_a_git_work_tree_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join(".git"))?;
    let config = config_in(dir.path(), EXAMPLE)?;
    let refused = prepare(&config);
    assert!(
        matches!(refused, Err(IdentityError::StateDirInsideGit { .. })),
        "{refused:?}"
    );
    assert!(!config.state_dir().exists(), "nothing is written when refused");
    Ok(())
}
