//! Owner readiness is the ready line or a closed pipe, never elapsed time.

use std::io::{BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;

use super::{Meter, abandon, start_failed, wait_ready};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_owner_refused_on_its_ready_line_is_ended_not_left_running() -> TestResult {
    let mut owner = Command::new("/bin/sh")
        .args(["-c", "echo not-a-ready-line; exec cat"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let pipe = owner.stdout.take().ok_or("no pipe")?;
    let refused = wait_ready(&mut BufReader::new(pipe), &Meter::default())
        .err()
        .ok_or("a line that is no ready line was taken as one")?;
    let ended = abandon(&mut owner, &refused);
    assert_eq!(ended.name(), refused.name());
    assert!(ended.to_string().contains("the owner was ended"), "{ended}");
    assert!(
        owner.try_wait()?.is_some(),
        "the refused owner was not reaped"
    );
    Ok(())
}

struct Observed {
    pipe: std::io::PipeReader,
    entered: Option<mpsc::Sender<()>>,
}

impl Read for Observed {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if let Some(entered) = self.entered.take() {
            entered.send(()).map_err(std::io::Error::other)?;
        }
        self.pipe.read(bytes)
    }
}

#[test]
fn readiness_stays_pending_until_the_ready_line_arrives() -> TestResult {
    let (pipe, mut writer) = std::io::pipe()?;
    let (entered, reading) = mpsc::channel();
    let (done, answer) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let pipe = Observed {
            pipe,
            entered: Some(entered),
        };
        let result = wait_ready(&mut BufReader::new(pipe), &Meter::default());
        done.send(result).map_err(|error| error.to_string())
    });
    reading.recv()?;
    assert!(matches!(answer.try_recv(), Err(mpsc::TryRecvError::Empty)));
    let owner = crate::peer::Leader {
        pid: 42,
        start: crate::peer::StartIdentity("owner-start".to_owned()),
    };
    writeln!(
        writer,
        "{}",
        super::super::protocol::ready_line(
            "runner",
            std::path::Path::new("/owner.sock"),
            &owner,
            "build"
        )
    )?;
    let (runner, socket, announced, build) = answer.recv()??;
    assert_eq!(runner, "runner");
    assert_eq!(socket, std::path::Path::new("/owner.sock"));
    assert_eq!(announced, owner);
    assert_eq!(build, "build");
    worker
        .join()
        .map_err(|error| format!("ready reader panicked: {error:?}"))??;
    Ok(())
}

#[test]
fn pipe_close_refuses_a_start_that_never_wrote_ready() -> TestResult {
    let (reader, writer) = std::io::pipe()?;
    drop(writer);
    let refused = wait_ready(&mut BufReader::new(reader), &Meter::default())
        .err()
        .ok_or("a closed pipe was accepted")?;
    assert_eq!(refused.name(), start_failed("").name());
    assert!(refused.to_string().contains("exited before it was ready"));
    Ok(())
}
