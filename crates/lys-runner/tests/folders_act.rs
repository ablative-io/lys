#![cfg(test)]
//! The `folders` act: a runner names the folders directly inside one folder
//! of its computer, so a person chooses where an agent works and never
//! types a path. A file is not a folder, a link is the folder it names, and
//! a folder that cannot be read is refused by name.

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::{Act, Answer, Client, Options, Runner, RunnerError};

type TestResult = Result<(), Box<dyn Error>>;

fn folders(
    client: &Client,
    under: Option<String>,
) -> Result<Result<(String, Vec<String>), String>, RunnerError> {
    match client.ask(&Act::Folders { under }) {
        Ok(Answer::Folders { under, folders }) => Ok(Ok((under, folders))),
        Ok(other) => Err(RunnerError::Malformed {
            reason: format!("the folders act answered {other:?}"),
        }),
        Err(RunnerError::Refused { refusal, .. }) => Ok(Err(refusal)),
        Err(other) => Err(other),
    }
}

#[test]
fn a_runner_names_the_folders_inside_one_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let options = Options {
        socket: dir.path().join("runner.sock"),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    };
    let serving = Runner::open(&options)?.spawn();
    let client = Client::new(options.socket, Arc::clone(&key));

    let held = dir.path().join("held");
    std::fs::create_dir(&held)?;
    std::fs::create_dir(held.join("receipts"))?;
    std::fs::create_dir(held.join("archive"))?;
    std::fs::write(held.join("notes.txt"), b"a file is not a folder")?;
    std::os::unix::fs::symlink(held.join("receipts"), held.join("linked"))?;
    std::os::unix::fs::symlink(held.join("nothing-here"), held.join("dangling"))?;
    let named = held.to_str().ok_or("the test folder is not UTF-8")?;

    assert_eq!(
        folders(&client, Some(named.to_owned()))?,
        Ok((
            named.to_owned(),
            vec![
                "archive".to_owned(),
                "linked".to_owned(),
                "receipts".to_owned()
            ]
        ))
    );
    assert_eq!(
        folders(&client, Some(format!("{named}/receipts")))?,
        Ok((format!("{named}/receipts"), Vec::new())),
        "a folder holding none answers none"
    );
    assert_eq!(
        folders(&client, Some("held/receipts".to_owned()))?,
        Err("folder_invalid".to_owned())
    );
    assert_eq!(
        folders(&client, Some(format!("{named}/nothing-here")))?,
        Err("folder_unreadable".to_owned())
    );
    assert_eq!(
        folders(&client, Some(format!("{named}/notes.txt")))?,
        Err("folder_unreadable".to_owned()),
        "a file cannot be looked in"
    );
    match folders(&client, None)? {
        Ok((home, _)) => assert!(
            std::path::Path::new(&home).is_absolute(),
            "the home folder is answered as an absolute path: {home}"
        ),
        Err(refusal) => assert_eq!(refusal, "home_unknown"),
    }
    serving.stop()?;
    Ok(())
}
