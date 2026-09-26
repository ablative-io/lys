use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use super::*;
use crate::identity::private_files::{DIR_MODE, FILE_MODE, mode_of};

type TestResult = Result<(), Box<dyn Error>>;

/// Loads the shipped example config from a copy in `dir`.
fn example_in(dir: &Path) -> Result<LoadedConfig, Box<dyn Error>> {
    let example =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/config.example.toml");
    let path = dir.join("identity.toml");
    std::fs::copy(example, &path)?;
    Ok(LoadedConfig::load(&path)?)
}

/// Every file under `root`, found by walking the tree rather than by
/// trusting the declared list.
fn files_under(root: &Path) -> Result<BTreeSet<PathBuf>, std::io::Error> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.insert(path);
            }
        }
    }
    Ok(found)
}

#[test]
fn prepare_creates_exactly_the_declared_private_files_owner_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let loaded = example_in(dir.path())?;
    let report = prepare(&loaded)?;
    let declared: BTreeSet<PathBuf> = declared_files(&loaded.private_dir).into_iter().collect();
    let found = files_under(&loaded.private_dir)?;
    assert_eq!(found, declared, "the private tree is not the declared set");
    assert_eq!(declared.len(), DECLARED.len() + 1);
    let mut restricted = 0;
    for path in &found {
        assert_eq!(mode_of(path)?, FILE_MODE, "{}", path.display());
        restricted += 1;
    }
    assert_eq!(restricted, declared.len());
    assert_eq!(mode_of(&loaded.private_dir)?, DIR_MODE);
    assert_eq!(mode_of(&loaded.private_dir.join(SECRETS_DIR))?, DIR_MODE);
    assert!(
        report
            .credentials
            .iter()
            .all(|(_, provenance)| *provenance == Provenance::Generated)
    );
    Ok(())
}

#[test]
fn the_environment_file_carries_every_variable_compose_reads() -> TestResult {
    let dir = tempfile::tempdir()?;
    let loaded = example_in(dir.path())?;
    let report = prepare(&loaded)?;
    let env = std::fs::read_to_string(&report.env_file)?;
    let compose = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/compose.yaml"),
    )?;
    let mut required = BTreeSet::new();
    for piece in compose.split("${").skip(1) {
        let name: String = piece
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        // `$${...}` escapes are the container's own shell variables.
        if name.starts_with("LYS_IDENTITY_") {
            required.insert(name);
        }
    }
    let mut present = 0;
    for name in &required {
        assert!(
            env.lines()
                .any(|line| line.starts_with(&format!("{name}="))),
            "identity.env lacks {name}"
        );
        present += 1;
    }
    assert_eq!(present, required.len());
    assert!(
        present > DECLARED.len(),
        "compose reads fewer variables than there are secrets"
    );
    assert!(env.contains("LYS_IDENTITY_PG_HOST=db.example.net\n"));
    Ok(())
}

#[test]
fn a_second_prepare_reuses_every_credential_and_renders_the_same_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let loaded = example_in(dir.path())?;
    let first = prepare(&loaded)?;
    let before = std::fs::read(&first.env_file)?;
    let second = prepare(&loaded)?;
    assert!(
        second
            .credentials
            .iter()
            .all(|(_, provenance)| *provenance == Provenance::Reused)
    );
    assert_eq!(std::fs::read(&second.env_file)?, before);
    Ok(())
}

#[test]
fn a_prepared_deployment_missing_a_secret_is_refused_and_nothing_is_generated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let loaded = example_in(dir.path())?;
    let report = prepare(&loaded)?;
    let before = std::fs::read(&report.env_file)?;
    let removed = loaded.private_dir.join(SECRETS_DIR).join("rauthy_enc_key");
    std::fs::remove_file(&removed)?;
    let message = prepare(&loaded)
        .err()
        .ok_or("a missing secret was accepted")?
        .to_string();
    assert!(
        message.starts_with("secret_missing: rauthy_enc_key"),
        "got {message}"
    );
    assert!(!removed.exists(), "a substitute key was generated");
    assert_eq!(
        std::fs::read(&report.env_file)?,
        before,
        "identity.env was rewritten"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_private_file_opened_to_others_is_refused() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir()?;
    let loaded = example_in(dir.path())?;
    prepare(&loaded)?;
    let opened = loaded
        .private_dir
        .join(SECRETS_DIR)
        .join("pg_admin_password");
    std::fs::set_permissions(&opened, std::fs::Permissions::from_mode(0o644))?;
    let message = prepare(&loaded)
        .err()
        .ok_or("a world-readable secret was accepted")?
        .to_string();
    assert!(
        message.starts_with("private_mode_unrestricted"),
        "got {message}"
    );
    Ok(())
}
