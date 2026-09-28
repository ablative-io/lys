//! A trusted native helper starts without the parent process's environment.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use lys_runner::pty::{Spawn, spawn_isolated};

#[test]
fn child_environment_contains_only_declared_values_and_terminal() -> Result<(), Box<dyn Error>> {
    let environment = BTreeMap::from([("TERM".to_owned(), "lys-test-terminal".to_owned())]);
    let mut child = spawn_isolated(&Spawn {
        program: "/usr/bin/env",
        arguments: &[],
        directory: "/",
        environment: &environment,
        columns: 80,
        rows: 24,
    })?;
    let mut output = String::new();
    child.reader.read_to_string(&mut output)?;
    assert!(child.child.wait()?.success());
    let actual: std::collections::BTreeSet<_> = output.lines().collect();
    assert_eq!(actual, ["TERM=lys-test-terminal", "SHELL="].into());
    Ok(())
}

#[test]
fn agent_variables_are_refused_before_the_trusted_helper_spawns() -> Result<(), Box<dyn Error>> {
    let environment =
        BTreeMap::from([("DYLD_INSERT_LIBRARIES".to_owned(), "/untrusted".to_owned())]);
    let result = spawn_isolated(&Spawn {
        program: "/program-that-must-not-be-reached",
        arguments: &[],
        directory: "/",
        environment: &environment,
        columns: 80,
        rows: 24,
    });
    let error = result.err().ok_or("loader environment accepted")?;
    assert_eq!(error.name(), "containment_entry_refused");
    assert!(error.to_string().contains("DYLD_INSERT_LIBRARIES"));
    Ok(())
}

#[test]
fn missing_directory_is_named_and_never_replaced_by_login_home() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let missing = temporary.path().join("missing");
    let text = missing.to_str().ok_or("fixture path not UTF-8")?;
    let result = spawn_isolated(&Spawn {
        program: "/bin/pwd",
        arguments: &[],
        directory: text,
        environment: &BTreeMap::new(),
        columns: 80,
        rows: 24,
    });
    let error = result.err().ok_or("missing directory accepted")?;
    assert!(error.to_string().contains(text));
    assert!(error.to_string().contains("working directory"));
    Ok(())
}

#[test]
fn requested_directory_and_literal_arguments_reach_the_child() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let directory = temporary.path().join("a space; no shell");
    std::fs::create_dir(&directory)?;
    let canonical = directory.canonicalize()?;
    let mut child = spawn_isolated(&Spawn {
        program: "/bin/pwd",
        arguments: &[],
        directory: directory.to_str().ok_or("fixture path not UTF-8")?,
        environment: &BTreeMap::new(),
        columns: 80,
        rows: 24,
    })?;
    let mut output = String::new();
    child.reader.read_to_string(&mut output)?;
    assert!(child.child.wait()?.success());
    assert_eq!(
        output.trim(),
        canonical.to_str().ok_or("fixture path not UTF-8")?
    );
    Ok(())
}

#[test]
fn missing_and_non_executable_programs_are_refused_before_spawn() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let non_executable = temporary.path().join("not-executable");
    std::fs::write(&non_executable, "not a program")?;
    let missing = temporary.path().join("missing-program");
    for program in [&non_executable, &missing] {
        let text = program.to_str().ok_or("fixture path not UTF-8")?;
        let result = spawn_isolated(&Spawn {
            program: text,
            arguments: &[],
            directory: "/",
            environment: &BTreeMap::new(),
            columns: 80,
            rows: 24,
        });
        let error = result.err().ok_or("invalid program spawned")?;
        assert_eq!(error.name(), "spawn_failed");
        assert!(error.to_string().contains(text));
    }
    Ok(())
}
