#![cfg(test)]
//! Managed and terminal launches select executable files with the same PATH rule.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn an_absolute_fork_keeps_parent_and_leaf_symlink_spelling_for_invocation()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let real = directory.path().join("real");
    std::fs::create_dir(&real)?;
    let binary = real.join("binary");
    std::fs::write(&binary, "#!/bin/sh\nexit 0\n")?;
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700))?;
    std::os::unix::fs::symlink("binary", real.join("chosen-fork"))?;
    let declared = directory.path().join("declared");
    std::os::unix::fs::symlink(&real, &declared)?;
    let fork = declared.join("chosen-fork");
    assert_ne!(fork, fork.canonicalize()?);
    assert_eq!(
        super::selected(fork.to_str().ok_or("path is not text")?, &BTreeMap::new())?,
        fork
    );
    assert_eq!(
        Path::new(&super::executable(&fork)?.path),
        binary.canonicalize()?
    );
    Ok(())
}

#[test]
fn managed_selection_skips_relative_entries_and_non_executable_files()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let blocked = directory.path().join("blocked");
    let installed = directory.path().join("installed");
    std::fs::create_dir(&blocked)?;
    std::fs::create_dir(&installed)?;
    for (folder, mode) in [(&blocked, 0o600), (&installed, 0o700)] {
        let program = folder.join("claude");
        std::fs::write(&program, "#!/bin/sh\nexit 0\n")?;
        std::fs::set_permissions(program, std::fs::Permissions::from_mode(mode))?;
    }
    let path = std::env::join_paths([Path::new("."), &blocked, &installed])?;
    let environment = BTreeMap::from([(
        "PATH".to_owned(),
        path.to_str().ok_or("path is not text")?.to_owned(),
    )]);
    assert_eq!(
        super::selected("claude", &environment)?,
        installed.join("claude")
    );
    Ok(())
}
