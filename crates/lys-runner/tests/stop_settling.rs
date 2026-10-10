#![cfg(test)]
//! A settling stop of everything is the server's own ask, sent again under
//! a pull in force. It leaves a session an earlier ask already stopped
//! exactly as it is: no signal, no kill, no wait on its exit. Only a person
//! asking again ends at once what a hang-up did not. Every wait below ends
//! on the runner's answer or the session's own output, never a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::{Act, Answer, Client, EndedHow, Launch, Options, Runner};

type TestResult = Result<(), Box<dyn Error>>;

/// The session under test.
const SESSION: &str = "still-exiting";

/// A harness that is still exiting: it takes the first hang-up, says so,
/// and heeds no later one, then answers each line it is typed. Nothing but
/// a kill ends it.
const STILL_EXITING: &str = "import signal, sys\n\
def hung(signum, frame):\n\
\x20   signal.signal(signal.SIGHUP, signal.SIG_IGN)\n\
\x20   print('hung-up', flush=True)\n\
signal.signal(signal.SIGHUP, hung)\n\
print('ready', flush=True)\n\
for line in sys.stdin:\n\
\x20   print('got-' + line.strip(), flush=True)\n";

fn waited(client: &Client, pattern: &str) -> TestResult {
    let act = Act::Wait {
        session: SESSION.to_owned(),
        cursor: Some(0),
        pattern: pattern.to_owned(),
        regex: false,
    };
    match client.ask(&act)? {
        Answer::Matched { .. } => Ok(()),
        other => Err(format!("waiting for {pattern} answered {other:?}").into()),
    }
}

fn stop_everything(by: &str, settling: bool) -> Act {
    Act::StopEverything {
        by: by.to_owned(),
        reason: "the test pulled the cord".to_owned(),
        kill: false,
        settling,
    }
}

/// What a stop of everything answered: the sessions ended and those running.
fn stopped(answer: Answer) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
    match answer {
        Answer::StoppedEverything {
            sessions, running, ..
        } => Ok((sessions, running)),
        other => Err(format!("stopping everything answered {other:?}").into()),
    }
}

#[test]
fn a_settling_ask_leaves_a_session_still_exiting_and_a_second_person_s_ask_ends_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let socket = dir.path().join("runner.sock");
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    })?;
    let serving = runner.spawn();
    let client = Client::new(socket.clone(), Arc::clone(&key));
    let started = client.ask(&Act::Start {
        lys_mcp: None,
        proxy: None,
        launch: Box::new(Launch {
            session: SESSION.to_owned(),
            program: "/usr/bin/python3".to_owned(),
            arguments: vec!["-c".to_owned(), STILL_EXITING.to_owned()],
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        }),
    })?;
    assert!(matches!(started, Answer::Started { .. }), "{started:?}");
    waited(&client, "ready")?;

    // A person pulls the cord. The session is hung up and does not exit, so
    // the ask stays unanswered: nothing ends its wait but the exit.
    let other = Client::new(socket, key);
    let first_ask = std::thread::spawn(move || other.ask(&stop_everything("person-one", false)));
    waited(&client, "hung-up")?;

    // The server settles a start under that pull. The session the pull
    // already asked is answered as it stands, running, and at once.
    let settled = stopped(client.ask(&stop_everything("person-one", true))?)?;
    assert_eq!(settled, (Vec::new(), vec![SESSION.to_owned()]));
    // It was not killed: it still answers what it is typed.
    let typed = client.ask(&Act::Input {
        session: SESSION.to_owned(),
        text: "alive".to_owned(),
        enter: true,
    })?;
    assert!(matches!(typed, Answer::Delivered { .. }), "{typed:?}");
    waited(&client, "got-alive")?;

    // A second person's ask ends at once what the hang-up did not, and both
    // asks are answered with the session ended.
    let second = stopped(client.ask(&stop_everything("person-two", false))?)?;
    assert_eq!(second, (vec![SESSION.to_owned()], Vec::new()));
    let answered = first_ask
        .join()
        .map_err(|_panicked| "the first ask panicked")??;
    assert_eq!(stopped(answered)?, (vec![SESSION.to_owned()], Vec::new()));

    // Its end is recorded as stopped by the one who asked first.
    let Answer::Output { output } = client.ask(&Act::Read {
        session: SESSION.to_owned(),
        cursor: None,
        lines: None,
        bytes: None,
        follow: false,
    })?
    else {
        return Err("read answered no output".into());
    };
    let ended = output.ended.ok_or("the session is not recorded ended")?;
    assert_eq!(ended.how, EndedHow::Stopped);
    assert_eq!(
        ended.stopped.map(|words| words.by).as_deref(),
        Some("person-one")
    );
    serving.stop()?;
    Ok(())
}
