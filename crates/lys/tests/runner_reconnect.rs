//! The explicit CLI reconnect reaches the bridge's owner-only control socket.

use clap::Parser;
use std::error::Error;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;

#[test]
fn cli_reconnect_sends_the_wake_command_to_the_saved_runner_socket() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let socket = root.path().join("runner.sock");
    let parsed = lys::cli::Cli::try_parse_from([
        "lys",
        "runner",
        "reconnect",
        "--socket",
        socket.to_str().ok_or("socket not UTF-8")?,
    ])?;
    let lys::cli::Command::Runner(command) = parsed.command else {
        return Err("reconnect parsed as another command".into());
    };
    let listener = UnixListener::bind(socket.with_extension("reconnect.sock"))?;
    let worker = std::thread::spawn(move || -> std::io::Result<u8> {
        let (mut stream, _) = listener.accept()?;
        let mut command = [0];
        stream.read_exact(&mut command)?;
        stream.write_all(b"{\"reconnecting\":true,\"generation\":1}")?;
        Ok(command[0])
    });
    lys::commands::runner::run(command)?;
    assert_eq!(
        worker.join().map_err(|error| {
            drop(error);
            "control fixture panicked"
        })??,
        b'R'
    );
    Ok(())
}
