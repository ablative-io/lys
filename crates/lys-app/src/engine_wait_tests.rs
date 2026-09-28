#![cfg(test)]

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::mpsc;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
const NOT_YET: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n";

#[test]
fn an_answer_is_heard_from_its_status_line() {
    let cases: [(&[u8], bool, Heard); 7] = [
        (OK, false, Heard::Ready),
        (b"HTTP/1.0 200 OK\r\n", true, Heard::Ready),
        (NOT_YET, false, Heard::NotYet),
        (b"HTTP/1.1 500 Internal", true, Heard::NotYet),
        (b"HTTP/1.1 2", false, Heard::More),
        (b"HTTP/1.1 2", true, Heard::NotYet),
        (b"", true, Heard::NotYet),
    ];
    let mut heard_count = 0;
    for (answer, closed, expected) in cases {
        assert_eq!(heard(answer, closed), expected, "{answer:?} {closed}");
        heard_count += 1;
    }
    assert_eq!(heard_count, 7);
}

/// A state folder that exists is watched beside the socket folders, once;
/// one that does not exist is not watched at its ancestor.
#[test]
fn a_round_watches_the_socket_folders_and_the_state_folders_that_exist() -> TestResult {
    let home = tempfile::tempdir()?;
    let run = home.path().join(".docker/run");
    let data = home.path().join("Data");
    std::fs::create_dir_all(&run)?;
    std::fs::create_dir_all(&data)?;
    let sockets = [run.join("docker.sock")];
    let state = [data.clone(), data.join("vms/0"), run.clone()];
    assert_eq!(round_folders(&sockets, &state), [run, data]);
    Ok(())
}

/// Reads one request from `stream` and answers it with `answer`.
fn answer_one(listener: &UnixListener, answer: &[u8]) -> std::io::Result<()> {
    let (mut stream, _) = listener.accept()?;
    let mut request = [0_u8; 512];
    let read = stream.read(&mut request)?;
    assert!(request[..read].starts_with(b"GET /_ping "));
    stream.write_all(answer)
}

/// The engine is made only after the first round is armed and has held
/// nothing, so the wait can end only through the folder's notice: the
/// second round holds the request the new engine answers.
#[cfg(target_os = "macos")]
#[test]
fn the_wait_ends_on_the_folder_notice_when_the_engine_starts() -> TestResult {
    let home = tempfile::tempdir()?;
    let run = home.path().join(".docker/run");
    std::fs::create_dir_all(&run)?;
    let socket = run.join("docker.sock");
    let (start, started) = mpsc::channel::<()>();
    let maker = socket.clone();
    let aside = home.path().join("aside.sock");
    let engine = std::thread::spawn(move || -> std::io::Result<()> {
        started.recv().map_err(std::io::Error::other)?;
        // Made listening beside the folder and moved in whole, so the
        // folder's notice comes only once the engine accepts: a socket made
        // in place is refused in the instant between its bind and listen.
        let listener = UnixListener::bind(&aside)?;
        std::fs::rename(&aside, &maker)?;
        answer_one(&listener, OK)
    });
    let mut rounds = Vec::new();
    let answered = wait_with(std::slice::from_ref(&socket), &[], &mut |held| {
        rounds.push(held);
        if rounds.len() == 1 {
            assert!(start.send(()).is_ok(), "the engine thread is gone");
        }
    })?;
    assert_eq!(answered, socket);
    assert_eq!(rounds.first(), Some(&0), "{rounds:?}");
    assert_eq!(rounds.last(), Some(&1), "{rounds:?}");
    engine
        .join()
        .map_err(|panic| format!("the engine thread failed: {panic:?}"))??;
    Ok(())
}

/// Docker Desktop's start: the socket is there before the engine answers.
/// The request is held, the engine answers only once the wait is asleep,
/// and the answer itself ends the wait in its first round, with no folder
/// changing.
#[cfg(target_os = "macos")]
#[test]
fn a_socket_made_before_its_engine_answers_ends_the_wait_when_it_answers() -> TestResult {
    let home = tempfile::tempdir()?;
    let run = home.path().join(".docker/run");
    std::fs::create_dir_all(&run)?;
    let socket = run.join("docker.sock");
    let listener = UnixListener::bind(&socket)?;
    let (ready, is_ready) = mpsc::channel::<()>();
    let engine = std::thread::spawn(move || -> std::io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = [0_u8; 512];
        let read = stream.read(&mut request)?;
        assert!(request[..read].starts_with(b"GET /_ping "));
        is_ready.recv().map_err(std::io::Error::other)?;
        stream.write_all(OK)
    });
    let mut rounds = Vec::new();
    let answered = wait_with(std::slice::from_ref(&socket), &[], &mut |held| {
        rounds.push(held);
        assert!(ready.send(()).is_ok(), "the engine thread is gone");
    })?;
    assert_eq!(answered, socket);
    assert_eq!(rounds, [1], "one round, holding one request");
    engine
        .join()
        .map_err(|panic| format!("the engine thread failed: {panic:?}"))??;
    Ok(())
}

/// An engine that answers "not yet" at once is not asked again until its
/// state folder changes; then it is asked once more and answers. It is
/// asked exactly twice, so the "not yet" is never asked in a loop.
#[cfg(target_os = "macos")]
#[test]
fn an_engine_that_says_not_yet_is_asked_again_when_its_state_folder_changes() -> TestResult {
    let home = tempfile::tempdir()?;
    let run = home.path().join(".docker/run");
    let data = home.path().join("Data");
    std::fs::create_dir_all(&run)?;
    std::fs::create_dir_all(&data)?;
    let socket = run.join("docker.sock");
    let listener = UnixListener::bind(&socket)?;
    let (said, heard_not_yet) = mpsc::channel::<()>();
    let engine = std::thread::spawn(move || -> std::io::Result<usize> {
        answer_one(&listener, NOT_YET)?;
        said.send(()).map_err(std::io::Error::other)?;
        answer_one(&listener, OK)?;
        Ok(2)
    });
    let changer = data.clone();
    let change = std::thread::spawn(move || -> std::io::Result<()> {
        heard_not_yet.recv().map_err(std::io::Error::other)?;
        std::fs::write(changer.join("backend.sock"), b"")
    });
    let mut rounds = Vec::new();
    let answered = wait_with(std::slice::from_ref(&socket), &[data], &mut |held| {
        rounds.push(held);
    })?;
    assert_eq!(answered, socket);
    assert_eq!(
        rounds,
        [1, 1],
        "a round per notice, each holding one request"
    );
    let asked = engine
        .join()
        .map_err(|panic| format!("the engine thread failed: {panic:?}"))??;
    assert_eq!(asked, 2);
    change
        .join()
        .map_err(|panic| format!("the changing thread failed: {panic:?}"))??;
    Ok(())
}

/// With no folder to watch at all the wait is refused by name.
#[cfg(target_os = "macos")]
#[test]
fn a_wait_with_nothing_to_watch_is_refused_by_name() {
    let refused = wait_with(&[], &[], &mut |_| {}).err();
    assert_eq!(
        refused.map(|refusal| refusal.name),
        Some("engine_watch_failed")
    );
}

#[test]
fn the_wait_source_holds_the_folder_and_answer_notices() -> TestResult {
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/engine_wait.rs"))?;
    assert!(source.contains("EVFILT_VNODE"));
    assert!(source.contains("EVFILT_READ"));
    Ok(())
}
