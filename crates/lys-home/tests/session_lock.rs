//! The session lock across processes: while this process owns a session, the
//! built binary is refused by name, naming this process as the holder; once
//! the owner lets go, the binary gets past the lock.

use std::process::Command;

use lys_home::Home;

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn import(
    home: &std::path::Path,
    transcript: &std::path::Path,
) -> std::io::Result<(Option<i32>, String)> {
    let output = Command::new(BIN)
        .arg("import")
        .arg("--home")
        .arg(home)
        .arg("--claude-code")
        .arg(transcript)
        .args(["--session", "s1"])
        .output()?;
    Ok((
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

#[test]
fn another_process_is_refused_naming_the_holder_until_the_owner_lets_go() -> Outcome {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    let transcript = dir.path().join("absent.jsonl");
    let session = home.create_session("s1", "/work", None)?;
    let (status, stderr) = import(&root, &transcript)?;
    assert_eq!(status, Some(1), "{stderr}");
    let holder = format!("held by process {}", std::process::id());
    assert!(stderr.contains(&holder), "{stderr}");
    drop(session);
    let (status, stderr) = import(&root, &transcript)?;
    assert_eq!(status, Some(1), "{stderr}");
    assert!(stderr.contains("already exists"), "{stderr}");
    Ok(())
}
