//! Directory disappearance after preflight refuses exec rather than running at home.
#![cfg(test)]

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[test]
fn a_launch_naming_no_directory_is_refused_by_name_and_never_runs_at_home() {
    // Runs go where the launch says they go: with no directory named there
    // is nowhere to start, and the login's home is never used in its place.
    let environment = BTreeMap::new();
    let spawn = crate::pty::Spawn {
        program: "/bin/echo",
        arguments: &["PROGRAM-WAS-EXECUTED".to_owned()],
        directory: "",
        environment: &environment,
        columns: 80,
        rows: 24,
    };
    let err = crate::pty::spawn(&spawn)
        .err()
        .expect("an empty directory is refused");
    assert_eq!(err.name(), "launch_without_directory");
    assert!(
        err.to_string().contains("names no working directory"),
        "{err}"
    );
}

#[test]
fn removed_working_directory_never_runs_the_requested_program() -> Result<(), Box<dyn Error>> {
    let fixture = tempfile::tempdir()?;
    let directory = fixture.path().join("removed");
    std::fs::create_dir(&directory)?;
    let mut command = CommandBuilder::new("/bin/echo");
    command.arg("PROGRAM-WAS-EXECUTED");
    crate::pty_command::set_directory(&mut command, "/bin/echo", &directory)?;
    std::fs::remove_dir(&directory)?;
    let pair = native_pty_system().openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut reader = pair.master.try_clone_reader()?;
    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);
    let mut output = String::new();
    reader.read_to_string(&mut output)?;
    assert!(!child.wait()?.success());
    assert!(!output.contains("PROGRAM-WAS-EXECUTED"));
    assert!(output.contains(directory.to_str().ok_or("non UTF-8 fixture")?));
    Ok(())
}
