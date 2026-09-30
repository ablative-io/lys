//! Directory disappearance after preflight refuses exec rather than running at home.
#![cfg(test)]

use std::error::Error;
use std::io::Read;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

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
