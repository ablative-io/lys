//! Startup frame checks preserve the bytes handed to the executed program.

use super::{execute_permission, startup_frame};
use std::io::Write;
use std::process::{Command, Stdio};

fn exec_after_startup(entry: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (mut reader, mut writer) = std::io::pipe()?;
    let first = b"{\"id\":\"lys-initialize\",\"method\":\"initialize\"}\n";
    let mut bytes = Vec::new();
    if entry {
        bytes.extend_from_slice(b"{\"startup\":true}\n");
    }
    bytes.extend_from_slice(b"{\"execute\":true}\n");
    bytes.extend_from_slice(first);
    writer.write_all(&bytes)?;
    drop(writer);
    if entry {
        assert_eq!(
            startup_frame(&mut reader)?,
            serde_json::json!({"startup":true})
        );
    }
    execute_permission(&mut reader)?;
    let output = Command::new("/bin/cat")
        .stdin(reader)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;
    assert!(output.status.success());
    assert_eq!(output.stdout, first);
    Ok(())
}

#[test]
fn startup_permission_preserves_the_first_frame_for_the_executed_program()
-> Result<(), Box<dyn std::error::Error>> {
    exec_after_startup(false)
}

#[test]
fn startup_entry_takes_only_its_own_line_before_permission_and_exec()
-> Result<(), Box<dyn std::error::Error>> {
    exec_after_startup(true)
}
