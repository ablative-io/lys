#![cfg(test)]
//! Socket resource bounds and shutdown are proved on connection signals.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;

use super::{Options, Runner};

type TestResult = Result<(), Box<dyn Error>>;

fn runner(dir: &std::path::Path) -> Result<Runner, Box<dyn Error>> {
    Ok(Runner::open(&Options {
        socket: dir.join("runner.sock"),
        state: dir.join("state"),
        server_key: [0; 32],
        scrollback: 4096,
    })?)
}

#[test]
fn an_oversized_line_is_refused_before_parsing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let serving = runner(dir.path())?.spawn();
    let mut stream = UnixStream::connect(dir.path().join("runner.sock"))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut request = vec![b'x'; 1_048_577];
    request.push(b'\n');
    match stream.write_all(&request) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(error) => return Err(error.into()),
    }
    line.clear();
    reader.read_line(&mut line)?;
    drop(reader);
    drop(stream);
    serving.stop()?;
    assert!(line.contains("request_too_large"), "{line}");
    Ok(())
}

#[cfg(target_os = "macos")]
fn threads() -> Result<usize, Box<dyn Error>> {
    let pid = i32::try_from(std::process::id())?;
    let info = libproc::proc_pid::pidinfo::<libproc::task_info::TaskInfo>(pid, 0)?;
    Ok(usize::try_from(info.pti_threadnum)?)
}

#[cfg(target_os = "linux")]
fn threads() -> Result<usize, Box<dyn Error>> {
    Ok(std::fs::read_dir("/proc/self/task")?
        .collect::<Result<Vec<_>, _>>()?
        .len())
}

#[test]
fn idle_connections_do_not_spawn_a_thread_each() -> TestResult {
    let dir = tempfile::tempdir()?;
    let runner = runner(dir.path())?;
    let before = threads()?;
    let serving = runner.spawn();
    let mut clients = Vec::new();
    for _ in 0..12 {
        let stream = UnixStream::connect(dir.path().join("runner.sock"))?;
        let mut reader = BufReader::new(stream);
        let mut greeting = String::new();
        reader.read_line(&mut greeting)?;
        clients.push(reader);
    }
    let added = threads()?.saturating_sub(before);
    drop(clients);
    serving.stop()?;
    assert!(added <= 4, "twelve idle peers added {added} OS threads");
    Ok(())
}

#[test]
fn shutdown_does_not_need_the_listener_path_to_exist() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut serving = runner(dir.path())?.spawn();
    let path = dir.path().join("runner.sock");
    let alternate = dir.path().join("alternate.sock");
    std::fs::hard_link(&path, &alternate)?;
    std::fs::remove_file(&path)?;
    let (entered, ready) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    serving.shutdown_probe = Some(Box::new(move || {
        entered.send(()).map_err(super::socket_failed)?;
        resumed.recv().map_err(super::socket_failed)
    }));
    let stopped = std::thread::spawn(move || serving.stop());
    ready.recv()?;
    match UnixStream::connect(&alternate) {
        Ok(stream) => drop(stream),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
            ) => {}
        Err(error) => return Err(error.into()),
    }
    resume.send(())?;
    let result = stopped
        .join()
        .map_err(|panic| format!("stop panicked: {panic:?}"))?;
    std::fs::remove_file(&alternate)?;
    assert!(result.is_ok(), "{result:?}");
    Ok(())
}

#[test]
fn an_accept_failure_leaves_the_listener_answering_the_next_connection() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut runner = runner(dir.path())?;
    runner.accept_error = Some((
        0,
        std::io::Error::from(std::io::ErrorKind::ConnectionAborted),
    ));
    let serving = runner.spawn();
    let path = dir.path().join("runner.sock");
    let mut first = BufReader::new(UnixStream::connect(&path)?);
    let mut greeting = String::new();
    assert_eq!(first.read_line(&mut greeting)?, 0);
    drop(first);
    let mut next = BufReader::new(UnixStream::connect(&path)?);
    assert!(next.read_line(&mut greeting)? > 0);
    assert!(greeting.contains("challenge"), "{greeting}");
    drop(next);
    serving.stop()?;
    Ok(())
}

#[test]
fn descriptor_exhaustion_recovers_when_an_existing_connection_exits() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut runner = runner(dir.path())?;
    runner.accept_error = Some((1, std::io::Error::from_raw_os_error(nix::libc::EMFILE)));
    let serving = runner.spawn();
    let path = dir.path().join("runner.sock");
    let mut first = BufReader::new(UnixStream::connect(&path)?);
    let mut line = String::new();
    first.read_line(&mut line)?;
    let mut refused = BufReader::new(UnixStream::connect(&path)?);
    line.clear();
    assert_eq!(refused.read_line(&mut line)?, 0);
    drop(refused);
    let mut next = BufReader::new(UnixStream::connect(&path)?);
    drop(first);
    assert!(next.read_line(&mut line)? > 0);
    assert!(line.contains("challenge"), "{line}");
    drop(next);
    serving.stop()?;
    Ok(())
}
