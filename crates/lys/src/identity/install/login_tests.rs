#![cfg(test)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use super::{KEPT, login_from};

/// A variable the invoking shell could carry and no service may: the
/// shape of a harness's config folder.
const LEAK: &str = "LYS_TEST_HARNESS_CONFIG_DIR";

/// This process's variables with the leak added.
fn process_with_leak() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let mut process: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os().collect();
    process.push((LEAK.into(), "/leaked".into()));
    process
}

#[test]
fn the_login_environment_keeps_the_login_and_nothing_of_the_invoking_process()
-> Result<(), Box<dyn Error>> {
    let environment = login_from(process_with_leak())?;
    let variables: Vec<(&str, &str)> = environment.variables().collect();
    assert!(
        !variables.iter().any(|(name, _)| *name == LEAK),
        "the leak must not be kept"
    );
    for (name, _) in &variables {
        assert!(
            *name == "PATH" || KEPT.contains(name),
            "{name} is not a login variable"
        );
    }
    let home = std::env::var("HOME")?;
    assert!(
        variables.contains(&("HOME", home.as_str())),
        "HOME is the login's"
    );
    let (_, path) = variables
        .iter()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(!path.is_empty(), "PATH must not be empty");
    let record = environment.record();
    assert_eq!(record.path, *path);
    assert!(!record.kept.iter().any(|name| name == "PATH"));
    assert!(record.kept.iter().any(|name| name == "HOME"));
    Ok(())
}

#[test]
fn a_login_profile_that_prints_does_not_become_part_of_path() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let shell = dir.path().join("noisy-shell");
    // A login shell whose profile prints first, answers as /bin/sh would, and
    // whose logout file prints after.
    std::fs::write(
        &shell,
        "#!/bin/sh\necho 'Last login: noise from a profile'\n/bin/sh \"$@\"\necho 'bye from a logout file'\n",
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), shell.into()));
    let environment = login_from(process)?;
    let (_, path) = environment
        .variables()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(
        !path.contains("noise"),
        "profile output became PATH: {path}"
    );
    assert!(!path.contains("bye"), "logout output became PATH: {path}");
    assert!(!path.contains('\n'), "PATH carries a line break: {path}");
    assert!(
        path.split(':')
            .any(|entry| entry == "/bin" || entry == "/usr/bin"),
        "{path}"
    );

    // A shell that never answers its PATH is refused by name.
    let mute = dir.path().join("mute-shell");
    std::fs::write(&mute, "#!/bin/sh\necho 'nothing'\n")?;
    std::fs::set_permissions(&mute, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), mute.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("a mute shell must be refused")?;
    assert!(
        refusal.to_string().contains("did not answer its PATH"),
        "{refusal}"
    );

    // A shell that opens its answer and never closes it is refused by name too.
    let open = dir.path().join("open-shell");
    std::fs::write(&open, "#!/bin/sh\nprintf 'LYS_LOGIN_PATH:/bin'\n")?;
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), open.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("an unclosed answer must be refused")?;
    assert!(
        refusal
            .to_string()
            .contains("did not close its PATH answer"),
        "{refusal}"
    );

    // A shell that answers its PATH but not the login's ANTHROPIC_BASE_URL is
    // refused by name, never taken as naming none.
    let half = dir.path().join("half-shell");
    std::fs::write(
        &half,
        "#!/bin/sh\nprintf 'LYS_LOGIN_PATH:/bin:LYS_LOGIN_PATH_END'\n",
    )?;
    std::fs::set_permissions(&half, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), half.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("an answer without the base must be refused")?;
    assert!(
        refusal
            .to_string()
            .contains("did not answer its ANTHROPIC_BASE_URL"),
        "{refusal}"
    );
    Ok(())
}
