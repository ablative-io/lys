#![cfg(test)]
//! Package identity refuses missing and forged artifacts without executing them.

use std::error::Error;

use lys_runner::codex_policy_contract::verify_package;

#[test]
fn a_version_banner_does_not_establish_package_identity() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("bin"))?;
    std::fs::write(dir.path().join("bin/codex"), "codex-cli 0.36.0")?;
    std::fs::write(dir.path().join("codex-package.json"), "{}")?;
    let refusal = verify_package(dir.path())
        .err()
        .ok_or("forged package accepted")?;
    assert!(
        refusal
            .to_string()
            .starts_with("codex_policy_contract_unsupported:")
    );
    assert!(refusal.to_string().contains("codex-package.json"));
    Ok(())
}

#[test]
fn missing_manifest_names_the_path() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let refusal = verify_package(dir.path())
        .err()
        .ok_or("missing package accepted")?;
    assert!(
        refusal
            .to_string()
            .contains(&dir.path().join("codex-package.json").display().to_string())
    );
    Ok(())
}

#[test]
fn a_copied_trusted_manifest_does_not_authenticate_changed_executable_bytes()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("bin"))?;
    std::fs::write(dir.path().join("bin/codex"), "codex-cli 0.0.0")?;
    std::fs::write(
        dir.path().join("codex-package.json"),
        include_str!("fixtures/codex-package.json"),
    )?;
    let refusal = verify_package(dir.path())
        .err()
        .ok_or("changed binary accepted")?;
    assert!(refusal.to_string().contains("bin/codex"));
    assert!(refusal.to_string().contains("executable differs"));
    Ok(())
}
