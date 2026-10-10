//! AGENTS-004 R2: a replacement runner finds and rebinds the owners a dead
//! one started, proved at the kernel; a second client binds and rebinds at
//! the same generation; an uncertain append is resolved by its id and never
//! becomes two; a reused pid, a stale generation and a wrong identity are
//! refused by name with the live owner untouched.
//!
//! The owner under test is the `fake_seat_owner` example (the real owner
//! steps with no harness). The harness send count and the identity
//! server's `AuthorityUnavailable` hold are the battery's and the identity
//! server's own tests; what is proved here is the runner side.

use std::error::Error;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::Client;
use lys_runner::peer::{self, Leader, StartIdentity};
use lys_runner::protocol::{Act, Answer, hex};
use lys_runner::seat_owner::protocol::{ClientKind, OwnerAnswer, OwnerCommand};
use lys_runner::seat_owner::record::Cursors;
use lys_runner::seat_owner::recovery::{self, Found, Unreachable};
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

fn pid_of(pid: u32) -> Result<rustix::process::Pid, Box<dyn Error>> {
    Ok(rustix::process::Pid::from_raw(i32::try_from(pid)?).ok_or("pid 0")?)
}

fn alive(pid: u32) -> bool {
    pid_of(pid).is_ok_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

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
    Ok((serde_json::from_str(line.trim_end())?, child))
}

fn ask(
    seat: &OwnedSeat,
    key: &Arc<Ed25519Identity>,
    command: OwnerCommand,
) -> Result<Answer, Box<dyn Error>> {
    let client = Client::new(seat.endpoint.socket.clone(), Arc::clone(key));
    Ok(client.ask(&Act::Owner { command })?)
}

fn hello(
    seat: &OwnedSeat,
    client: ClientKind,
    key: &Arc<Ed25519Identity>,
) -> Result<Answer, Box<dyn Error>> {
    ask(
        seat,
        key,
        OwnerCommand::Hello {
            binding: seat.binding.clone(),
            client,
            client_start: own_leader()?,
        },
    )
}

fn view_of(answer: Answer) -> Result<lys_runner::seat_owner::protocol::OwnerView, Box<dyn Error>> {
    match answer {
        Answer::Owner {
            answer: OwnerAnswer::Bound { view } | OwnerAnswer::Status { view },
        } => Ok(view),
        other => Err(format!("no view: {other:?}").into()),
    }
}

fn refusal_of(answer: Answer) -> Result<String, Box<dyn Error>> {
    match answer {
        Answer::Refused { refusal, .. } => Ok(refusal),
        other => Err(format!("not a refusal: {other:?}").into()),
    }
}

fn journal_lines(seat: &OwnedSeat) -> Result<usize, Box<dyn Error>> {
    Ok(
        std::fs::read_to_string(seat.endpoint.dir.join("seat-owners.journal"))?
            .lines()
            .count(),
    )
}

#[test]
fn seat_survives_runner_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let state = dir.path().join("state");
    let (seat, mut runner) = runner_with_owner(&state, "restart", &key)?;
    let before = view_of(hello(&seat, ClientKind::Runner, &key)?)?;

    // The runner dies without a goodbye.
    rustix::process::kill_process(pid_of(runner.id())?, rustix::process::Signal::KILL)?;
    runner.wait()?;
    assert!(
        alive(seat.endpoint.owner.pid),
        "the owner is alive throughout"
    );

    // The replacement runner opens the same state: it finds the owner by
    // its record, proves it at the kernel, indexes it and launches nothing.
    let replacement = Sessions::open(&state, 1 << 16)?;
    let owned = replacement.owned_seats()?;
    assert_eq!(owned, vec![seat.clone()], "the same seat, from the record");
    assert!(replacement.unreachable_owners()?.is_empty());
    assert!(
        replacement.status(None)?.sessions.is_empty(),
        "zero replacement harnesses"
    );
    let (found, counts) = recovery::recover(&state)?;
    assert_eq!(found.len(), 1);
    assert!(found[0].live());
    assert_eq!(counts.record_visits, 1, "one directory visited, no history");

    // It binds the same session and generation and the next correlated
    // request is accepted.
    let after = view_of(hello(&seat, ClientKind::Runner, &key)?)?;
    assert_eq!(after.owner, before.owner);
    assert_eq!(after.lease.generation, before.lease.generation);
    assert_eq!(after.binding, before.binding);
    let moved = ask(
        &seat,
        &key,
        OwnerCommand::Cursors {
            intent: "0000000000000000000000000000000a".to_owned(),
            cursors: Cursors {
                receipt: 1,
                feed: 0,
                hook: 0,
            },
        },
    )?;
    assert!(
        matches!(
            moved,
            Answer::Owner {
                answer: OwnerAnswer::Cursors { .. }
            }
        ),
        "{moved:?}"
    );
    rustix::process::kill_process(
        pid_of(seat.endpoint.owner.pid)?,
        rustix::process::Signal::TERM,
    )?;
    Ok(())
}

#[test]
fn seat_survives_identity_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let state = dir.path().join("state");
    let (seat, mut runner) = runner_with_owner(&state, "identity", &key)?;

    // The identity server binds, an accepted hook moves the hook cursor
    // once, then the server is gone and its replacement binds again at the
    // same session and generation: the cursor is still one, the hook is
    // not charged twice.
    let first = view_of(hello(&seat, ClientKind::IdentityServer, &key)?)?;
    let intent = "000000000000000000000000000000b1".to_owned();
    let hook = OwnerCommand::Cursors {
        intent,
        cursors: Cursors {
            receipt: 0,
            feed: 0,
            hook: 1,
        },
    };
    ask(&seat, &key, hook.clone())?;
    let lines = journal_lines(&seat)?;
    ask(&seat, &key, hook)?;
    assert_eq!(journal_lines(&seat)?, lines, "the same intent is one line");
    let again = view_of(hello(&seat, ClientKind::IdentityServer, &key)?)?;
    assert_eq!(again.lease.generation, first.lease.generation);
    assert_eq!(again.binding.session, "identity");
    assert_eq!(again.cursors.hook, 1, "the accepted hook is charged once");

    drop(runner.stdin.take());
    runner.wait()?;
    rustix::process::kill_process(
        pid_of(seat.endpoint.owner.pid)?,
        rustix::process::Signal::TERM,
    )?;
    Ok(())
}

#[test]
fn seat_reconnect_keeps_uncertainty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let state = dir.path().join("state");
    let (seat, mut runner) = runner_with_owner(&state, "uncertain", &key)?;

    // The owner records the receipt cursor as possibly sent; the reply is
    // lost; both clients restart and send the same intent again. It is the
    // same operation, one line, and a cursor that claims more than was
    // received is refused rather than invented.
    let intent = "000000000000000000000000000000c1".to_owned();
    let possibly_sent = OwnerCommand::Cursors {
        intent,
        cursors: Cursors {
            receipt: 3,
            feed: 0,
            hook: 0,
        },
    };
    ask(&seat, &key, possibly_sent.clone())?;
    let lines = journal_lines(&seat)?;
    rustix::process::kill_process(pid_of(runner.id())?, rustix::process::Signal::KILL)?;
    runner.wait()?;
    let replacement = Sessions::open(&state, 1 << 16)?;
    assert_eq!(replacement.owned_seats()?.len(), 1);
    view_of(hello(&seat, ClientKind::Runner, &key)?)?;
    view_of(hello(&seat, ClientKind::IdentityServer, &key)?)?;
    ask(&seat, &key, possibly_sent)?;
    assert_eq!(
        journal_lines(&seat)?,
        lines,
        "the resent intent is not a second line"
    );
    let back = ask(
        &seat,
        &key,
        OwnerCommand::Cursors {
            intent: "000000000000000000000000000000c2".to_owned(),
            cursors: Cursors {
                receipt: 2,
                feed: 0,
                hook: 0,
            },
        },
    )?;
    assert_eq!(refusal_of(back)?, "seat_owner_cursor_regression");
    assert_eq!(
        view_of(ask(&seat, &key, OwnerCommand::Status)?)?
            .cursors
            .receipt,
        3
    );
    rustix::process::kill_process(
        pid_of(seat.endpoint.owner.pid)?,
        rustix::process::Signal::TERM,
    )?;
    Ok(())
}

#[test]
fn seat_reconnect_refuses_foreign_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let state = dir.path().join("state");
    let (seat, mut runner) = runner_with_owner(&state, "foreign", &key)?;

    // A reused pid: a record naming a live pid with another start identity
    // is unreachable, named so, and not the owner.
    let reused = OwnedSeat {
        endpoint: lys_runner::seat_owner::protocol::OwnerEndpoint {
            owner: Leader {
                pid: std::process::id(),
                start: StartIdentity("macos:0.000000".to_owned()),
            },
            ..seat.endpoint.clone()
        },
        ..seat.clone()
    };
    assert!(matches!(
        recovery::prove(&reused),
        Some(Unreachable::PidReused { .. })
    ));
    let exited = OwnedSeat {
        endpoint: lys_runner::seat_owner::protocol::OwnerEndpoint {
            owner: Leader {
                pid: 4_000_000,
                start: StartIdentity("macos:0.000000".to_owned()),
            },
            ..seat.endpoint.clone()
        },
        ..seat.clone()
    };
    assert!(matches!(
        recovery::prove(&exited),
        Some(Unreachable::Exited | Unreachable::Unproved { .. })
    ));
    let found = Found {
        seat: reused,
        unreachable: recovery::prove(&seat),
    };
    assert!(found.live(), "the real owner is live: {found:?}");

    // A stale lease generation and a wrong public identity each name the
    // mismatch; the live owner answers afterwards, untouched.
    let dead = OwnedSeat {
        binding: lys_runner::seat_owner::protocol::OwnerBinding {
            generation: 9,
            ..seat.binding.clone()
        },
        ..seat.clone()
    };
    assert_eq!(
        refusal_of(hello(&dead, ClientKind::Runner, &key)?)?,
        "seat_owner_generation_stale"
    );
    let wrong = OwnedSeat {
        binding: lys_runner::seat_owner::protocol::OwnerBinding {
            seat: "somebody-else".to_owned(),
            ..seat.binding.clone()
        },
        ..seat.clone()
    };
    assert_eq!(
        refusal_of(hello(&wrong, ClientKind::Runner, &key)?)?,
        "seat_owner_binding_mismatch"
    );
    let forged = ask(
        &seat,
        &key,
        OwnerCommand::Hello {
            binding: seat.binding.clone(),
            client: ClientKind::Runner,
            client_start: Leader {
                pid: std::process::id(),
                start: StartIdentity("macos:0.000000".to_owned()),
            },
        },
    )?;
    assert_eq!(refusal_of(forged)?, "seat_owner_client_unproved");
    assert!(alive(seat.endpoint.owner.pid));
    assert_eq!(
        view_of(hello(&seat, ClientKind::Runner, &key)?)?.owner,
        seat.endpoint.owner
    );

    drop(runner.stdin.take());
    runner.wait()?;
    rustix::process::kill_process(
        pid_of(seat.endpoint.owner.pid)?,
        rustix::process::Signal::TERM,
    )?;
    Ok(())
}
