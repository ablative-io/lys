#![cfg(test)]
//! `lys runner join` as a person runs it: an address another computer
//! cannot reach is refused by name before the code is read or anything is
//! sent; no code on standard input is refused by name; a code read from
//! standard input is sent and never said or kept, while this computer's own
//! key is made where the install keeps the runner's files.

use std::error::Error;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output, Stdio};

type TestResult = Result<(), Box<dyn Error>>;

/// Run `lys runner join` against `server` with `stdin`, its files under
/// `home`.
fn join(home: &Path, server: &str, stdin: &str) -> Result<Output, Box<dyn Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(["runner", "join", "--server", server, "--machine"])
        .arg("op-0123456789abcdef0123456789abcdef")
        .env("LYS_IDENTITY_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("the join has no input")?;
    // A join refused before it reads its input may have closed it already.
    drop(input.write_all(stdin.as_bytes()));
    drop(input);
    Ok(child.wait_with_output()?)
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn an_address_on_this_computer_is_refused_before_anything_is_sent() -> TestResult {
    let home = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    for server in [
        format!("https://127.0.0.1:{port}"),
        format!("https://localhost:{port}"),
        format!("http://127.0.0.1:{port}"),
        "http://lys.example.test".to_owned(),
    ] {
        let output = join(home.path(), &server, "0f1e2d3c\n")?;
        assert!(!output.status.success(), "{server}: {}", said(&output));
        assert!(
            said(&output).contains("runner_join_unreachable"),
            "{server}: {}",
            said(&output)
        );
    }
    assert!(
        listener.accept().is_err(),
        "nothing was sent to this computer's own address"
    );
    assert!(
        std::fs::read_dir(home.path())?.next().is_none(),
        "nothing was kept for a join refused at once"
    );
    Ok(())
}

#[test]
fn no_code_on_standard_input_is_refused_by_name() -> TestResult {
    let home = tempfile::tempdir()?;
    let output = join(home.path(), "https://lys.example.invalid", "")?;
    assert!(!output.status.success(), "{}", said(&output));
    assert!(
        said(&output).contains("runner_join_code_missing"),
        "{}",
        said(&output)
    );
    Ok(())
}

#[test]
fn a_code_read_from_standard_input_is_never_said_or_kept() -> TestResult {
    let home = tempfile::tempdir()?;
    let code = "5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0de5c0";
    // No port is named, so the address is refused when it is dialled, and
    // nothing leaves this computer.
    let output = join(
        home.path(),
        "https://lys.example.invalid",
        &format!("{code}\n"),
    )?;
    assert!(!output.status.success(), "{}", said(&output));
    assert!(!said(&output).contains(code), "{}", said(&output));
    let key = home.path().join("data").join("runner-machine.key");
    assert_eq!(
        std::fs::read(&key)?.len(),
        32,
        "this computer's own key is made"
    );
    let mut dirs = vec![home.path().to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                dirs.push(entry.path());
            } else if entry.file_type()?.is_file() {
                let bytes = std::fs::read(entry.path())?;
                assert!(
                    !bytes
                        .windows(code.len())
                        .any(|window| window == code.as_bytes()),
                    "{} keeps the code",
                    entry.path().display()
                );
            }
        }
    }
    Ok(())
}
