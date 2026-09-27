//! Gates on the one-owner lock: a child holding a duplicate of the lock
//! descriptor never keeps a session held, and a second owner in this process
//! is refused by name however the path is spelled.

use crate::error::HomeError;
use crate::record::{Home, Session};

type Gate = Result<(), Box<dyn std::error::Error>>;

fn home() -> Result<(tempfile::TempDir, Home), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    Ok((dir, home))
}

/// A child given a duplicate of the lock descriptor, as a child spawned
/// between the fork and the exec has, outlives the owner; the next owner
/// still opens the session.
#[cfg(unix)]
#[test]
fn a_child_holding_a_duplicate_descriptor_does_not_keep_the_session_held() -> Gate {
    use std::process::{Command, Stdio};
    let (_dir, home) = home()?;
    let session = home.create_session("s1", "/work", None)?;
    let duplicate = session.lock_file().try_clone()?;
    let mut child = Command::new("/bin/sleep")
        .arg("60")
        .stdin(Stdio::from(duplicate))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    drop(session);
    let reopened = home.open_session("s1");
    child.kill()?;
    child.wait()?;
    let reopened = reopened?;
    assert_eq!(reopened.header().id, "s1");
    Ok(())
}

#[test]
fn a_second_owner_in_this_process_is_refused_naming_this_process() -> Gate {
    let (_dir, home) = home()?;
    let session = home.create_session("s1", "/work", None)?;
    let file = session.file().to_path_buf();
    let respelled = home
        .root()
        .join("sessions")
        .join("..")
        .join("sessions")
        .join("s1.jsonl");
    for path in [file.clone(), respelled] {
        let refused = Session::open(&path);
        let pid = std::process::id();
        assert!(
            matches!(&refused, Err(HomeError::SessionHeld { holder: Some(h), .. }) if *h == pid),
            "{refused:?}"
        );
        let message = refused.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(message.contains("in this process"), "{message}");
    }
    drop(session);
    let reopened = Session::open(&file)?;
    assert_eq!(reopened.header().id, "s1");
    Ok(())
}
