//! A start refused after its owner was spawned ends the owner, and a start
//! never waits without bound on an owner's ready line.

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{READY_WITHIN, abandon, ready_within, start_failed};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn alive(pid: u32) -> bool {
    i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

/// An owner that writes a line that is not a ready line and lives on (a
/// stale build, or a path cut at a space) is refused by name, then ended
/// and reaped: before, the start waited on its exit for ever, and an owner
/// refused after spawn ran on, writing to the seat's thread.
#[test]
fn an_owner_refused_on_its_ready_line_is_ended_not_left_running() -> TestResult {
    let mut owner = Command::new("/bin/sh")
        .args(["-c", "echo not-a-ready-line; exec sleep 600"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let pipe = owner.stdout.take().ok_or("no pipe")?;
    let started = Instant::now();
    let refused = ready_within(pipe, READY_WITHIN)
        .err()
        .ok_or("a line that is no ready line was taken as one")?;
    let ended = abandon(&mut owner, &refused);
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "the start waited"
    );
    assert_eq!(ended.name(), refused.name());
    assert!(ended.to_string().contains("the owner was ended"), "{ended}");
    assert!(!alive(owner.id()), "the refused owner still runs");
    Ok(())
}

/// An owner that writes nothing and does not exit is refused by name once
/// the bound passes, never waited on for ever.
#[test]
fn an_owner_that_never_writes_its_ready_line_is_refused_by_name_in_time() -> TestResult {
    let (reader, mut writer) = std::io::pipe()?;
    let started = Instant::now();
    let refused = ready_within(reader, Duration::from_millis(200))
        .err()
        .ok_or("a silent owner was taken as ready")?;
    assert_eq!(refused.name(), start_failed("").name());
    assert!(started.elapsed() < Duration::from_secs(10));
    // The writer stayed open the whole time: the bound, not an end of
    // file, refused it.
    writer.write_all(b"late\n")?;
    Ok(())
}
