#![cfg(test)]
//! DIRECTORY-050 R2: the runner protocol's conformance suite. It runs
//! against any runner given its socket: set `LYS_RUNNER_CONFORMANCE_SOCKET`
//! to the runner's socket and `LYS_RUNNER_CONFORMANCE_KEY` to the file
//! holding the seed of the server key that runner trusts. With neither set
//! it runs against Lys's own runner, started here. It needs `/bin/sh` on the
//! runner's machine.
//!
//! Every act of the protocol is exercised and every refusal the protocol
//! names for a request is shown firing, a request answered once and sent
//! again on another connection among them. The published section is held to
//! the protocol's own types: every act and answer they define is named in it. Every wait ends on the runner's
//! answer, never a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest, OperationState};
use lys_runner::protocol::{Greeting, Reply, Request, hex, signed_bytes};
use lys_runner::{Act, Answer, Client, Key, Launch, Options, PROTOCOL_VERSION, Runner, Serving};

type TestResult = Result<(), Box<dyn Error>>;

/// The runner under test, and what keeps Lys's own alive while it is.
struct Target {
    socket: PathBuf,
    key: Arc<Ed25519Identity>,
    own: Option<(Serving, tempfile::TempDir)>,
}

impl Target {
    fn open() -> Result<Self, Box<dyn Error>> {
        let named = (
            std::env::var_os("LYS_RUNNER_CONFORMANCE_SOCKET"),
            std::env::var_os("LYS_RUNNER_CONFORMANCE_KEY"),
        );
        if let (Some(socket), Some(key)) = named {
            return Ok(Self {
                socket: PathBuf::from(socket),
                key: Arc::new(Ed25519Identity::load(&PathBuf::from(key))?),
                own: None,
            });
        }
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
        Ok(Self {
            socket,
            key,
            own: Some((runner.spawn(), dir)),
        })
    }

    fn client(&self) -> Client {
        Client::new(self.socket.clone(), Arc::clone(&self.key))
    }

    /// Send the line `make` writes for the greeting of a fresh connection,
    /// and read the reply.
    fn raw(
        &self,
        make: impl FnOnce(&Greeting) -> Result<String, Box<dyn Error>>,
    ) -> Result<Reply, Box<dyn Error>> {
        let mut connection = lys_runner::connect(&self.socket)?;
        let greeting = connection.greeting()?;
        let reply = connection.exchange(&make(&greeting)?)?;
        Ok(serde_json::from_str(&reply)?)
    }

    fn close(self) -> TestResult {
        if let Some((serving, _dir)) = self.own {
            serving.stop()?;
        }
        Ok(())
    }
}

/// A session id no other run of the suite uses.
fn session(name: &str) -> String {
    let nonce = lys_runner::protocol::nonce();
    format!("conformance-{name}-{}", &nonce[..16])
}

fn shell(id: &str) -> Launch {
    Launch {
        session: id.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::from([("PS1".to_owned(), "$ ".to_owned())]),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn start(client: &Client, id: &str) -> TestResult {
    match client.ask(&Act::Start {
        lys_mcp: None,
        proxy: None,
        launch: Box::new(shell(id)),
    })? {
        Answer::Started { session, pid, .. } if session == id && pid > 0 => Ok(()),
        other => Err(format!("start answered {other:?}").into()),
    }
}

fn wait(
    client: &Client,
    id: &str,
    pattern: &str,
    regex: bool,
) -> Result<(String, u64), Box<dyn Error>> {
    let act = Act::Wait {
        session: id.to_owned(),
        cursor: Some(0),
        pattern: pattern.to_owned(),
        regex,
    };
    match client.ask(&act)? {
        Answer::Matched {
            matched, cursor, ..
        } => Ok((matched, cursor)),
        other => Err(format!("wait answered {other:?}").into()),
    }
}

fn delivered(answer: &Answer, id: &str) -> bool {
    matches!(answer, Answer::Delivered { session } if session == id)
}

fn refusal(reply: &Reply) -> &str {
    match &reply.answer {
        Answer::Refused { refusal, .. } => refusal,
        _ => "",
    }
}

#[test]
fn status_names_the_protocol() -> TestResult {
    let target = Target::open()?;
    let Answer::Status { status } = target.client().ask(&Act::Status { session: None })? else {
        return Err("status answered no status".into());
    };
    assert_eq!(status.protocol, PROTOCOL_VERSION);
    assert!(!status.runner.is_empty());
    target.close()
}

#[test]
fn every_request_refusal_fires_by_name() -> TestResult {
    let target = Target::open()?;
    let act = serde_json::to_string(&Act::Status { session: None })?;
    let key = Arc::clone(&target.key);
    let signed = move |version: u32, runner: &str, challenge: &str| Request {
        version,
        runner: runner.to_owned(),
        challenge: challenge.to_owned(),
        act: act.clone(),
        signature: hex(&key.sign(&signed_bytes(version, runner, challenge, &act))),
    };
    let line = |request: &Request| serde_json::to_string(request);
    let mut fired = 0;

    let unsigned = target.raw(|greeting| {
        let request = Request {
            signature: String::new(),
            ..signed(PROTOCOL_VERSION, &greeting.runner, &greeting.challenge)
        };
        Ok(line(&request)?)
    })?;
    assert_eq!(refusal(&unsigned), "runner_request_unsigned");
    fired += 1;

    let other_version = target.raw(|greeting| {
        Ok(line(&signed(
            PROTOCOL_VERSION + 1,
            &greeting.runner,
            &greeting.challenge,
        ))?)
    })?;
    assert_eq!(refusal(&other_version), "runner_protocol_mismatch");
    fired += 1;

    let malformed = target.raw(|_greeting| Ok("not json".to_owned()))?;
    assert_eq!(refusal(&malformed), "runner_request_malformed");
    fired += 1;

    let misaddressed = target.raw(|greeting| {
        let elsewhere = hex(&[0x5a; 16]);
        assert_ne!(elsewhere, greeting.runner);
        Ok(line(&signed(
            PROTOCOL_VERSION,
            &elsewhere,
            &greeting.challenge,
        ))?)
    })?;
    assert_eq!(refusal(&misaddressed), "runner_request_misaddressed");
    fired += 1;

    let mut answered = String::new();
    let first = target.raw(|greeting| {
        answered = line(&signed(
            PROTOCOL_VERSION,
            &greeting.runner,
            &greeting.challenge,
        ))?;
        Ok(answered.clone())
    })?;
    assert_eq!(
        refusal(&first),
        "",
        "a request on its own connection is answered"
    );
    let replayed = target.raw(|greeting| {
        assert!(
            !answered.contains(&greeting.challenge),
            "each connection is given its own challenge"
        );
        Ok(answered.clone())
    })?;
    assert_eq!(refusal(&replayed), "runner_request_replayed");
    fired += 1;

    assert_eq!(fired, lys_runner::published::REQUEST_REFUSALS.len());
    target.close()
}

#[test]
fn a_session_is_typed_to_read_and_waited_on() -> TestResult {
    let target = Target::open()?;
    let client = target.client();
    let id = session("type");
    start(&client, &id)?;
    let typed = client.ask(&Act::Input {
        session: id.clone(),
        text: "echo typed-$((40+2))".to_owned(),
        enter: true,
    })?;
    assert!(delivered(&typed, &id), "{typed:?}");
    let (matched, cursor) = wait(&client, &id, "typed-42", false)?;
    assert_eq!(matched, "typed-42");
    let Answer::Output { output } = client.ask(&Act::Read {
        session: id.clone(),
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    })?
    else {
        return Err("read answered no output".into());
    };
    assert!(
        output.text.contains("echo typed-$((40+2))"),
        "the typed line: {output:?}"
    );
    assert!(output.text.contains("typed-42"), "its output: {output:?}");
    assert!(output.cursor >= cursor);
    let (matched, _) = wait(&client, &id, "typed-[0-9]+", true)?;
    assert_eq!(
        matched, "typed-42",
        "a regular expression, asked for by name"
    );
    target.close()
}

#[test]
fn keys_resize_follow_and_end() -> TestResult {
    let target = Target::open()?;
    let client = target.client();
    let id = session("keys");
    start(&client, &id)?;
    let resized = client.ask(&Act::Resize {
        session: id.clone(),
        columns: 101,
        rows: 31,
    })?;
    assert!(delivered(&resized, &id), "{resized:?}");
    client.ask(&Act::Input {
        session: id.clone(),
        text: "stty size; echo keys-$((1+1))".to_owned(),
        enter: false,
    })?;
    let keyed = client.ask(&Act::Keys {
        session: id.clone(),
        keys: vec![Key::Enter],
    })?;
    assert!(delivered(&keyed, &id), "{keyed:?}");
    wait(&client, &id, "keys-2", false)?;
    wait(&client, &id, "31 101", false)?;

    let Answer::Output { output } = client.ask(&Act::Read {
        session: id.clone(),
        cursor: None,
        lines: Some(1),
        bytes: None,
        follow: false,
    })?
    else {
        return Err("read answered no output".into());
    };
    let follower = target.client();
    let (named, from) = (id.clone(), output.cursor);
    let following = std::thread::spawn(move || {
        follower.ask(&Act::Read {
            session: named,
            cursor: Some(from),
            lines: None,
            bytes: None,
            follow: true,
        })
    });
    client.ask(&Act::Input {
        session: id.clone(),
        text: "echo followed".to_owned(),
        enter: true,
    })?;
    let answered = following
        .join()
        .map_err(|_panicked| "the follower panicked")??;
    let Answer::Output { output } = answered else {
        return Err("a following read answered no output".into());
    };
    assert!(output.from == from && !output.text.is_empty(), "{output:?}");

    let Answer::Ended { session, ended } = client.ask(&Act::End {
        session: id.clone(),
    })?
    else {
        return Err("end answered no end".into());
    };
    assert_eq!(session, id);
    assert!(ended.at > 0);
    let refused = client.ask(&Act::Input {
        session: id,
        text: "echo after".to_owned(),
        enter: true,
    });
    assert!(refused.is_err_and(|error| error.name() == "session_ended"));
    let unknown = client.ask(&Act::End {
        session: session_unknown(),
    });
    assert!(unknown.is_err_and(|error| error.name() == "session_unknown"));
    target.close()
}

fn session_unknown() -> String {
    session("never-started")
}

#[test]
fn the_published_section_names_every_act_and_answer_the_protocol_defines() -> TestResult {
    let section = lys_runner::published::section();
    let described = &section["x-lys-runner"];
    let acts = described["acts"]
        .as_object()
        .ok_or("the section names no acts")?;
    let id = "published".to_owned();
    let every_act = [
        Act::ReadBytes {
            session: id.clone(),
            cursor: None,
            follow: false,
        },
        Act::InputBytes {
            session: id.clone(),
            data: vec![0xff],
        },
        Act::Start {
            lys_mcp: None,
            proxy: None,
            launch: Box::new(shell(&id)),
        },
        Act::Input {
            session: id.clone(),
            text: String::new(),
            enter: false,
        },
        Act::Keys {
            session: id.clone(),
            keys: vec![Key::Enter],
        },
        Act::Read {
            session: id.clone(),
            cursor: None,
            lines: None,
            bytes: None,
            follow: false,
        },
        Act::Wait {
            session: id.clone(),
            cursor: None,
            pattern: "p".to_owned(),
            regex: false,
        },
        Act::Resize {
            session: id.clone(),
            columns: 1,
            rows: 1,
        },
        Act::End {
            session: id.clone(),
        },
        Act::Status { session: None },
        Act::Operate {
            operation: Operation {
                operation: "op".to_owned(),
                session: id.clone(),
                request: OperationRequest::Stop,
            },
        },
        Act::Outcome {
            operation: "op".to_owned(),
        },
        Act::Feed {
            cursor: None,
            follow: false,
        },
        Act::Folders { under: None },
        Act::GrantChannel,
        Act::StopEverything {
            by: "person".to_owned(),
            reason: "why".to_owned(),
            kill: false,
        },
    ];
    let mut named = 0;
    for act in &every_act {
        let tag = serde_json::to_value(act)?["act"]
            .as_str()
            .ok_or("an act has no tag")?
            .to_owned();
        assert!(acts.contains_key(&tag), "{tag} is not published");
        assert!(lys_runner::published::ACTS.contains(&tag.as_str()), "{tag}");
        named += 1;
    }
    assert_eq!(
        (named, acts.len()),
        (
            lys_runner::published::ACTS.len(),
            lys_runner::published::ACTS.len()
        )
    );
    let answers = described["answers"]
        .as_array()
        .ok_or("the section names no answers")?;
    let every_answer = [
        Answer::Bytes {
            output: lys_runner::terminal_bytes::ByteOutput {
                session: id.clone(),
                from: 0,
                cursor: 1,
                oldest: 0,
                data: vec![0xff],
                ended: None,
            },
        },
        Answer::Started {
            session: id.clone(),
            pid: 1,
            started_at: 1,
        },
        Answer::Delivered {
            session: id.clone(),
        },
        Answer::Matched {
            session: id.clone(),
            matched: String::new(),
            cursor: 0,
        },
        Answer::Operation {
            outcome: OperationOutcome {
                operation: "op".to_owned(),
                session: id,
                request: "stop".to_owned(),
                state: OperationState::Accepted,
                at: 0,
                words: String::new(),
                text: None,
                ended: None,
            },
        },
        Answer::Feed {
            page: lys_runner::tracking_store::FeedPage {
                format: "f".to_owned(),
                entries: Vec::new(),
                cursor: "c".to_owned(),
            },
        },
        Answer::Folders {
            under: "/".to_owned(),
            folders: Vec::new(),
        },
        Answer::GrantChannel,
        Answer::StoppedEverything {
            sessions: Vec::new(),
            running: Vec::new(),
        },
        Answer::Refused {
            refusal: "r".to_owned(),
            words: String::new(),
            oldest: None,
        },
    ];
    let mut kinds = 0;
    for answer in &every_answer {
        let kind = serde_json::to_value(answer)?["kind"].clone();
        assert!(answers.contains(&kind), "{kind} is not published");
        kinds += 1;
    }
    assert_eq!(kinds, every_answer.len());
    assert_eq!(answers.len(), lys_runner::published::ANSWERS.len());
    Ok(())
}
