//! AGENTS-004 R1: a supervised seat's owner is an independent process,
//! bound by kernel-proved identity, unique per seat, and the manual
//! lifecycle is untouched by it.
//!
//! The owner under test is the `fake_seat_owner` example: the real owner
//! steps (plan, establish intent, a runner of its own on the owner socket,
//! the kernel-proved Hello) with no harness, since the harness is what the
//! lead's qualification battery brings. Every wait here is a blocking read
//! on a pipe or a socket; nothing sleeps.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::Client;
use lys_runner::harness_control::ManagedLaunch;
use lys_runner::peer::{self, Leader, StartIdentity};
use lys_runner::protocol::{Act, Answer, hex};
use lys_runner::seat_owner::protocol::{ClientKind, OwnerAnswer, OwnerBinding, OwnerCommand};
use lys_runner::seat_owner::sessions::OwnedSeat;
use lys_runner::session::Sessions;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture() -> Result<PathBuf, Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let debug = exe
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("the test binary has no target directory")?;
    let fixture = debug.join("examples").join("fake_seat_owner");
    if !fixture.exists() {
        return Err(format!("{} was not built beside the tests", fixture.display()).into());
    }
    Ok(fixture)
}

fn own_leader() -> Result<Leader, Box<dyn Error>> {
    let pid = std::process::id();
    Ok(Leader {
        pid,
        start: peer::start_identity(pid)?,
    })
}

fn supervised(session: &str) -> ManagedLaunch {
    serde_json::from_value(serde_json::json!({
        "launch": {
            "session": session,
            "program": "claude",
            "arguments": [],
            "directory": "/",
            "environment": {},
            "columns": 120,
            "rows": 40
        },
        "transport": "claude",
        "conversation": "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f",
        "requires_controls": false,
        "owner": {
            "seat": format!("seat-{session}"),
            "session": session,
            "conversation": "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f",
            "generation": 1
        }
    }))
    .expect("the fixture launch reads")
}

fn alive(pid: u32) -> bool {
    let Some(pid) = rustix::process::Pid::from_raw(i32::try_from(pid).unwrap_or(0)) else {
        return false;
    };
    rustix::process::test_kill_process(pid).is_ok()
}

fn kill(pid: u32, signal: rustix::process::Signal) -> Result<(), Box<dyn Error>> {
    let pid = rustix::process::Pid::from_raw(i32::try_from(pid)?).ok_or("pid 0")?;
    rustix::process::kill_process(pid, signal)?;
    Ok(())
}

fn hello(
    seat: &OwnedSeat,
    client_start: Leader,
    key: &Arc<Ed25519Identity>,
) -> Result<Answer, Box<dyn Error>> {
    let client = Client::new(seat.endpoint.socket.clone(), Arc::clone(key));
    Ok(client.ask(&Act::Owner {
        command: OwnerCommand::Hello {
            binding: seat.binding.clone(),
            client: ClientKind::Runner,
            client_start,
        },
    })?)
}

fn bound(answer: Answer) -> Result<lys_runner::seat_owner::protocol::OwnerView, Box<dyn Error>> {
    match answer {
        Answer::Owner {
            answer: OwnerAnswer::Bound { view },
        } => Ok(view),
        other => Err(format!("not bound: {other:?}").into()),
    }
}

fn refusal_of(answer: Answer) -> Result<String, Box<dyn Error>> {
    match answer {
        Answer::Refused { refusal, .. } => Ok(refusal),
        other => Err(format!("not a refusal: {other:?}").into()),
    }
}

/// A runner process that starts an owner for `session` and waits to be
/// killed; answers the seat it printed and the process.
fn runner_with_owner(
    state: &std::path::Path,
    session: &str,
    key: &Arc<Ed25519Identity>,
) -> Result<(OwnedSeat, std::process::Child), Box<dyn Error>> {
    let mut child = Command::new(fixture()?)
        .arg("--as-runner")
        .arg(state)
        .arg(session)
        .arg(hex(&key.public_key_bytes()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("no pipe")?;
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    if line.is_empty() {
        let status = child.wait()?;
        return Err(
            format!("the runner fixture exited {status} before it printed its seat").into(),
        );
    }
    let seat: OwnedSeat = serde_json::from_str(line.trim_end())?;
    Ok((seat, child))
}

#[test]
fn seat_survives_terminal_crash() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let state = dir.path().join("state");
    let (seat, mut runner) = runner_with_owner(&state, "survives", &key)?;
    let owner = seat.endpoint.owner.pid;
    assert!(alive(owner), "the owner runs");

    // The owner is in a session of its own, not the runner's or the test's.
    let owner_pid = rustix::process::Pid::from_raw(i32::try_from(owner)?).ok_or("pid 0")?;
    let owner_sid = rustix::process::getsid(Some(owner_pid))?;
    let own_sid = rustix::process::getsid(None)?;
    assert_ne!(owner_sid, own_sid, "the owner joined a session of its own");

    // The attaching client binds and is told the owner it reached.
    let view = bound(hello(&seat, own_leader()?, &key)?)?;
    assert_eq!(view.owner, seat.endpoint.owner);
    assert_eq!(view.lease.generation, 1);
    assert_eq!(view.binding.session, "survives");

    // The runner that started the owner dies: SIGKILL, no goodbye. A
    // detaching runner (its pipe closed, exit 0) is the second case.
    kill(runner.id(), rustix::process::Signal::KILL)?;
    let status = runner.wait()?;
    assert!(!status.success(), "the runner was killed: {status}");
    assert!(
        alive(owner),
        "the owner outlives the runner that started it"
    );
    let again = bound(hello(&seat, own_leader()?, &key)?)?;
    assert_eq!(
        again.owner, seat.endpoint.owner,
        "the same owner process answers"
    );
    assert_eq!(again.lease.generation, 1, "the same generation");
    assert_eq!(
        again.binding.conversation, seat.binding.conversation,
        "the same conversation"
    );

    let (detached_seat, mut detaching) = runner_with_owner(&state, "detached", &key)?;
    drop(detaching.stdin.take());
    let status = detaching.wait()?;
    assert!(status.success(), "the runner detached cleanly: {status}");
    assert!(alive(detached_seat.endpoint.owner.pid));
    let view = bound(hello(&detached_seat, own_leader()?, &key)?)?;
    assert_eq!(view.owner, detached_seat.endpoint.owner);

    for pid in [owner, detached_seat.endpoint.owner.pid] {
        kill(pid, rustix::process::Signal::TERM)?;
    }
    Ok(())
}

#[test]
fn seat_owner_is_unique() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let sessions = Sessions::open(&dir.path().join("state"), 1 << 16)?;
    sessions.set_server_key(key.public_key_bytes());
    sessions.set_owner_program(fixture()?);

    // Two starts for the same seat binding at once: one owner, the other
    // refused by name, never a second session.
    let managed = supervised("unique");
    let binding = managed.owner.clone().ok_or("supervised")?;
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let starts: Vec<_> = (0..2)
        .map(|_| {
            let sessions = Arc::clone(&sessions);
            let managed = managed.clone();
            let binding = binding.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                sessions.start_owned(managed, binding, None)
            })
        })
        .collect();
    let mut outcomes = Vec::new();
    for start in starts {
        outcomes.push(start.join().map_err(|_| "a start panicked")?);
    }
    let started: Vec<_> = outcomes.iter().filter(|outcome| outcome.is_ok()).collect();
    let refused: Vec<String> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err().map(lys_runner::RunnerError::name))
        .collect();
    assert_eq!(started.len(), 1, "exactly one owner: {outcomes:?}");
    assert_eq!(refused, vec!["seat_owner_held".to_owned()]);
    let seats = sessions.owned_seats()?;
    assert_eq!(seats.len(), 1);
    let seat = seats[0].clone();
    assert!(alive(seat.endpoint.owner.pid));
    assert!(
        sessions.status(None)?.sessions.is_empty(),
        "the runner launched no session of its own for the seat"
    );

    // A forged client: the right pid with another start identity, and a
    // pid the kernel does not confirm. Each is refused by name and the
    // owner is untouched.
    let mut forged = own_leader()?;
    forged.start = StartIdentity("macos:0.000000".to_owned());
    assert_eq!(
        refusal_of(hello(&seat, forged, &key)?)?,
        "seat_owner_client_unproved"
    );
    let foreign = Leader {
        pid: seat.endpoint.owner.pid,
        start: seat.endpoint.owner.start.clone(),
    };
    assert_eq!(
        refusal_of(hello(&seat, foreign, &key)?)?,
        "seat_owner_client_unproved"
    );
    let stale = OwnedSeat {
        binding: OwnerBinding {
            generation: 2,
            ..seat.binding.clone()
        },
        ..seat.clone()
    };
    assert_eq!(
        refusal_of(hello(&stale, own_leader()?, &key)?)?,
        "seat_owner_generation_stale"
    );
    let other = OwnedSeat {
        binding: OwnerBinding {
            seat: "another-seat".to_owned(),
            ..seat.binding.clone()
        },
        ..seat.clone()
    };
    assert_eq!(
        refusal_of(hello(&other, own_leader()?, &key)?)?,
        "seat_owner_binding_mismatch"
    );
    assert!(alive(seat.endpoint.owner.pid), "the owner was not touched");
    let view = bound(hello(&seat, own_leader()?, &key)?)?;
    assert_eq!(view.owner, seat.endpoint.owner);
    kill(seat.endpoint.owner.pid, rustix::process::Signal::TERM)?;
    Ok(())
}

#[test]
fn seat_owner_preserves_manual_lifecycle() -> TestResult {
    // A launch without the typed binding reads as it always did, with no
    // owner: the survival behaviour is selected by the member alone.
    let manual: ManagedLaunch = serde_json::from_value(serde_json::json!({
        "launch": {
            "session": "manual",
            "program": "claude",
            "arguments": [],
            "directory": "/",
            "environment": {},
            "columns": 120,
            "rows": 40
        },
        "transport": "claude",
        "conversation": "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f",
        "requires_controls": false
    }))?;
    assert!(manual.owner.is_none());
    let bytes = serde_json::to_string(&manual)?;
    assert!(
        !bytes.contains("\"owner\""),
        "a manual launch carries no owner member: {bytes}"
    );

    // A runner that serves no socket yet starts no owner and launches
    // nothing, by name.
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(&dir.path().join("state"), 1 << 16)?;
    let supervised = supervised("unserved");
    let binding = supervised.owner.clone().ok_or("supervised")?;
    let refused = sessions
        .start_owned(supervised, binding, None)
        .expect_err("no socket, no owner");
    assert_eq!(refused.name(), "seat_owner_start_failed");
    assert!(sessions.owned_seats()?.is_empty());
    assert!(sessions.status(None)?.sessions.is_empty());
    assert!(
        !dir.path().join("state").join("owners").exists()
            || std::fs::read_dir(dir.path().join("state").join("owners"))?
                .next()
                .is_none(),
        "no owner directory was made for a refused start"
    );

    // A manual session of the same name is a session, not a seat: a
    // supervised start for it is refused by name, and the manual record
    // stands.
    let sessions = Sessions::open(&dir.path().join("state2"), 1 << 16)?;
    sessions.set_server_key([7; 32]);
    let launch: lys_runner::protocol::Launch = serde_json::from_value(serde_json::json!({
        "session": "manual",
        "program": "/bin/sh",
        "arguments": ["-c", "exit 0"],
        "directory": "/",
        "environment": {},
        "columns": 80,
        "rows": 24
    }))?;
    sessions.start(launch)?;
    let supervised = supervised("manual");
    let binding = supervised.owner.clone().ok_or("supervised")?;
    let refused = sessions
        .start_owned(supervised, binding, None)
        .expect_err("a manual session is not promoted");
    assert_eq!(refused.name(), "seat_owner_held");
    assert!(sessions.owned_seats()?.is_empty());
    let mut out = std::io::stderr();
    writeln!(
        out,
        "manual lifecycle: {} sessions, 0 owners",
        sessions.status(None)?.sessions.len()
    )?;
    Ok(())
}
