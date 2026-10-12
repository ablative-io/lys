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
