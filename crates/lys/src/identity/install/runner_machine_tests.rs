#![cfg(test)]
//! A computer keeps one private key across repeated joins and an older layout.

use std::error::Error;

use lys_core::Ed25519Identity;

use crate::identity::install::layout::Layout;
use crate::identity::private_files;

type Outcome = Result<(), Box<dyn Error>>;

#[test]
fn install_machine_key_is_private_and_reused() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let mut said = Vec::new();
    let first = super::provide(&layout, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(
        layout.runner_machine_key(),
        layout.root.join("keys/runner-machine.key")
    );
    assert_eq!(
        private_files::mode_of(&layout.runner_machine_key())?,
        Some(0o600)
    );
    assert_eq!(said.len(), 1);
    assert!(said[0].contains("runner machine key created"));
    let second = super::provide(&layout, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(first.public_key_bytes(), second.public_key_bytes());
    assert_eq!(said.len(), 1);
    Ok(())
}

#[test]
fn join_machine_key_migrates_the_old_file_without_changing_identity() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    private_files::ensure_dir(&layout.data_dir())?;
    let old = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    let first = Ed25519Identity::load_or_generate(&old)?;
    let mut said = Vec::new();
    let migrated = super::provide(&layout, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(first.public_key_bytes(), migrated.public_key_bytes());
    assert!(!old.exists());
    assert_eq!(
        private_files::mode_of(&layout.runner_machine_key())?,
        Some(0o600)
    );
    assert_eq!(said.len(), 1);
    assert!(said[0].contains("runner machine key moved"));
    let again = super::provide(&layout, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(again.public_key_bytes(), first.public_key_bytes());
    assert_eq!(said.len(), 1);
    Ok(())
}

#[test]
fn join_machine_key_refuses_conflicting_legacy_keys() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let mut said = Vec::new();
    let installed = super::provide(&layout, &mut |line| said.push(line.to_owned()))?;
    private_files::ensure_dir(&layout.data_dir())?;
    let old = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    let legacy = Ed25519Identity::load_or_generate(&old)?;
    let error = match super::provide(&layout, &mut |line| said.push(line.to_owned())) {
        Ok(_) => return Err("a different legacy identity was discarded".into()),
        Err(error) => error,
    };
    assert!(error.to_string().contains("runner_machine_key_conflict"));
    assert_eq!(
        Ed25519Identity::load(&old)?.public_key_bytes(),
        legacy.public_key_bytes()
    );
    assert_eq!(
        Ed25519Identity::load(&layout.runner_machine_key())?.public_key_bytes(),
        installed.public_key_bytes()
    );
    assert_eq!(said.len(), 1);
    Ok(())
}

#[test]
fn install_machine_key_refuses_an_invalid_existing_key_without_replacing_it() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    private_files::ensure_dir(&layout.root.join("keys"))?;
    private_files::write(&layout.runner_machine_key(), b"invalid")?;
    let mut said = Vec::new();
    assert!(super::provide(&layout, &mut |line| said.push(line.to_owned())).is_err());
    assert_eq!(std::fs::read(layout.runner_machine_key())?, b"invalid");
    assert!(said.is_empty());
    Ok(())
}

#[test]
fn install_and_upgrade_server_keys_provide_the_same_private_machine_key() -> Outcome {
    use crate::identity::config::DeploymentConfig;
    use crate::identity::install::{layout, server_keys};

    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let config = DeploymentConfig::parse(&layout::render_deployment(None), layout.root.clone())?;
    crate::identity::prepare::materialise_all(&config)?;
    server_keys(&layout, &config)?;
    let first = Ed25519Identity::load(&layout.runner_machine_key())?;
    assert_eq!(
        private_files::mode_of(&layout.runner_machine_key())?,
        Some(0o600)
    );
    server_keys(&layout, &config)?;
    assert_eq!(
        Ed25519Identity::load(&layout.runner_machine_key())?.public_key_bytes(),
        first.public_key_bytes(),
    );
    Ok(())
}

#[test]
fn upgrade_server_keys_migrate_a_join_without_changing_the_machine_key() -> Outcome {
    use crate::identity::config::DeploymentConfig;
    use crate::identity::install::{layout, server_keys};

    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let config = DeploymentConfig::parse(&layout::render_deployment(None), layout.root.clone())?;
    crate::identity::prepare::materialise_all(&config)?;
    private_files::ensure_dir(&layout.data_dir())?;
    let legacy = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    let joined = Ed25519Identity::load_or_generate(&legacy)?;
    server_keys(&layout, &config)?;
    assert!(!legacy.exists());
    assert_eq!(
        private_files::mode_of(&layout.runner_machine_key())?,
        Some(0o600)
    );
    assert_eq!(
        Ed25519Identity::load(&layout.runner_machine_key())?.public_key_bytes(),
        joined.public_key_bytes(),
    );
    Ok(())
}

#[test]
fn build_machine_record_preserves_the_installs_existing_join() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let key = super::provide(&layout, &mut |_| {})?;
    private_files::ensure_dir(&layout.data_dir())?;
    let computer = "op-11111111111111111111111111111111";
    let identity = "machine-22222222222222222222222222222222";
    let public = lys_runner::protocol::hex(&key.public_key_bytes());
    std::fs::write(
        layout.data_dir().join("network.joins.json"),
        serde_json::to_vec(&serde_json::json!({
            "format": "lys-runner-joins/v1", "codes": [{
                "operation": "op-33333333333333333333333333333333", "machine": computer,
                "issued_by": "person-44444444444444444444444444444444", "issued_at": 1,
                "standing": {"state": "used", "at": 2, "key": public, "identity": identity}
            }]
        }))?,
    )?;
    for _ in 0..2 {
        let record = super::recorded(&layout, &mut |_| {})?;
        assert_eq!(record.computer.as_deref(), Some(computer));
        assert_eq!(record.identity.as_deref(), Some(identity));
        assert_eq!(record.key, public);
        assert_eq!(record.key_file, layout.runner_machine_key());
    }
    Ok(())
}

#[test]
fn build_machine_record_names_an_unjoined_key_and_refuses_unreadable_joins() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    let record = super::recorded(&layout, &mut |_| {})?;
    assert!(record.computer.is_none());
    assert!(record.identity.is_none());
    private_files::ensure_dir(&layout.data_dir())?;
    let path = layout.data_dir().join("network.joins.json");
    std::fs::write(&path, b"broken")?;
    let error = super::recorded(&layout, &mut |_| {})
        .err()
        .ok_or("broken joins were ignored")?;
    assert!(error.to_string().contains("runner machine record"));
    assert_eq!(std::fs::read(path)?, b"broken");
    Ok(())
}

#[test]
fn old_build_record_is_backed_up_before_machine_metadata_is_written() -> Outcome {
    let directory = tempfile::tempdir()?;
    let layout = Layout::at(directory.path().to_owned());
    private_files::ensure_dir(&layout.install_dir())?;
    let old = b"{\"binaries\":{},\"surface\":null,\"environment\":{},\"settings\":{}}";
    std::fs::write(layout.build_record(), old)?;
    let mut said = Vec::new();
    let new = b"{\"binaries\":{},\"runner_machine\":{\"key\":\"public\"}}";
    super::write_build_record(&layout, new, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(
        std::fs::read(
            layout
                .install_dir()
                .join("build.before-runner-machine.json")
        )?,
        old
    );
    assert_eq!(std::fs::read(layout.build_record())?, new);
    assert_eq!(said.len(), 1);
    super::write_build_record(&layout, new, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(said.len(), 1);
    assert_eq!(private_files::mode_of(&layout.build_record())?, Some(0o600));
    Ok(())
}
