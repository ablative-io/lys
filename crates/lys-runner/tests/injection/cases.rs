#![cfg(test)]
//! Injection admission, attribution and durable-before-terminal ordering.

use super::Sessions;
use crate::injection::{Injection, InputGrant, inject, lys_notice};
use crate::input::Input;
use crate::protocol::Launch;
use sha2::Digest;
use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Capture {
    bytes: Arc<Mutex<Vec<u8>>>,
    feed: std::path::PathBuf,
}
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let feed = std::fs::read_to_string(&self.feed)?;
        if !feed.contains("injection") {
            return Err(io::Error::other(
                "injection leaf missing before terminal write",
            ));
        }
        self.bytes
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Fixture {
    dir: tempfile::TempDir,
    sessions: Arc<Sessions>,
    bytes: Arc<Mutex<Vec<u8>>>,
    terminal: Input,
}
impl Fixture {
    fn new() -> TestResult<Self> {
        Self::at(tempfile::tempdir()?)
    }
    fn at(dir: tempfile::TempDir) -> TestResult<Self> {
        Self::started(dir, false)
    }
    fn owned() -> TestResult<Self> {
        Self::started(tempfile::tempdir()?, true)
    }
    fn started(dir: tempfile::TempDir, owned: bool) -> TestResult<Self> {
        let sessions = Sessions::open(dir.path(), 4096)?;
        let launch = Launch {
            session: "session".to_owned(),
            program: "/bin/cat".to_owned(),
            arguments: Vec::new(),
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        if owned {
            sessions.start_for(launch, "person.owner")?;
        } else {
            sessions.start(launch)?;
        }
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let terminal = {
            let mut table = sessions.lock()?;
            let live = table
                .sessions
                .get_mut("session")
                .ok_or("session missing")?
                .live("session")?;
            std::mem::replace(
                &mut live.writer,
                Input::new(Box::new(Capture {
                    bytes: Arc::clone(&bytes),
                    feed: dir.path().join("feed.jsonl"),
                })),
            )
        };
        Ok(Self {
            dir,
            sessions,
            bytes,
            terminal,
        })
    }
    fn input(sender: &str) -> Injection<'_> {
        Injection {
            session: "session",
            sender,
            bytes: b"sensitive input",
            signed: true,
            cookie: None,
        }
    }
    fn entries(&self) -> TestResult<Vec<serde_json::Value>> {
        std::fs::read_to_string(self.dir.path().join("feed.jsonl"))?
            .lines()
            .map(|line| Ok(serde_json::from_str(line)?))
            .collect()
    }
    fn end(self) -> TestResult {
        self.sessions.stop_all()?;
        drop(self.terminal);
        Ok(())
    }
}

#[test]
fn owned_legacy_input_needs_context_and_start_records_the_person() -> TestResult {
    let fixture = Fixture::owned()?;
    let kept = crate::state::StateFile::open(&tempfile::tempdir()?.path().join("empty"))?.read()?;
    assert!(kept.responsible.is_empty());
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.dir.path().join("sessions.json"))?)?;
    assert_eq!(record["responsible"]["session"], "person.owner");
    for result in [
        fixture.sessions.write("session", b"input"),
        fixture.sessions.input("session", "input", true),
        fixture
            .sessions
            .keys("session", &[crate::protocol::Key::Enter]),
    ] {
        assert_eq!(
            result.expect_err("owned needs context").name(),
            "SessionInputContextMissing"
        );
    }
    let operation = crate::operations::Operation {
        operation: "compact-owned".to_owned(),
        session: "session".to_owned(),
        request: crate::operations::OperationRequest::Compact {
            text: "/compact".to_owned(),
        },
    };
    assert_eq!(
        fixture
            .sessions
            .operate(operation)
            .expect_err("compact needs context")
            .name(),
        "SessionInputContextMissing"
    );
    assert!(
        fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    fixture.end()
}

#[test]
fn old_session_record_migrates_with_explicitly_absent_responsibility() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = br#"{"format":"lys-runner-sessions/v2","sessions":[{"session":"old","pid":null,"leader_start":null,"started_at":7,"columns":80,"rows":24,"ended":{"how":"exited","at":8,"status":0,"signal":null,"reason":null}}]}"#;
    std::fs::write(dir.path().join("sessions.json"), old)?;
    let state = crate::state::StateFile::open(dir.path())?;
    let kept = state.read()?;
    assert_eq!(kept.format, crate::state::FORMAT);
    assert_eq!(kept.sessions.len(), 1);
    assert_eq!(kept.sessions[0].session, "old");
    assert_eq!(kept.sessions[0].started_at, 7);
    assert!(kept.responsible.is_empty());
    state.write(&kept)?;
    assert_eq!(state.read()?, kept);
    Ok(())
}

#[test]
fn every_owned_legacy_act_uses_the_live_input_grant() -> TestResult {
    use crate::legacy_input::InputContext;
    use crate::protocol::{Act, Answer, Greeting, Key, sign_request};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let fixture = Fixture::owned()?;
    let server = lys_core::Ed25519Identity::ephemeral();
    let acts = [
        Act::InputBytes {
            session: "session".to_owned(),
            data: b"raw".to_vec(),
        },
        Act::Input {
            session: "session".to_owned(),
            text: "text".to_owned(),
            enter: true,
        },
        Act::Keys {
            session: "session".to_owned(),
            keys: vec![Key::Enter],
        },
        Act::Operate {
            operation: crate::operations::Operation {
                operation: "compact-explicit".to_owned(),
                session: "session".to_owned(),
                request: crate::operations::OperationRequest::Compact {
                    text: "/compact".to_owned(),
                },
            },
        },
    ];
    let denied = |grant: &InputGrant<'_>| {
        assert_eq!(
            (grant.kind, grant.id, grant.action, grant.sender),
            ("session", "session", "input", "agent.caller")
        );
        assert!(fixture.sessions.table.try_lock().is_ok());
        Err(crate::error::RunnerError::refused(
            "SessionInputNotGranted",
            "revoked",
        ))
    };
    let before = fixture.entries()?;
    for act in &acts {
        for (context, expected) in [
            (None, "SessionInputContextMissing"),
            (
                Some(InputContext {
                    sender: "agent.caller",
                    signed: true,
                    cookie: None,
                    judge: &denied,
                }),
                "SessionInputNotGranted",
            ),
            (
                Some(InputContext {
                    sender: "agent.caller",
                    signed: true,
                    cookie: Some("administrator=borrowed"),
                    judge: &denied,
                }),
                "AgentCookieRefused",
            ),
        ] {
            let greeting = Greeting::fresh(fixture.sessions.runner());
            let line = sign_request(&server, &greeting, act)?;
            let answer = crate::socket::dispatch_for(
                &fixture.sessions,
                &server.public_key_bytes(),
                &greeting,
                &line,
                &AtomicBool::new(false),
                context.as_ref(),
            );
            assert!(matches!(answer, Answer::Refused {refusal, ..} if refusal == expected));
        }
    }
    assert_eq!(fixture.entries()?, before);
    assert!(
        fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let calls = AtomicUsize::new(0);
    let allowed = |grant: &InputGrant<'_>| {
        assert_eq!(grant.sender, "agent.caller");
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    };
    let context = InputContext {
        sender: "agent.caller",
        signed: true,
        cookie: None,
        judge: &allowed,
    };
    for act in &acts {
        let greeting = Greeting::fresh(fixture.sessions.runner());
        let line = sign_request(&server, &greeting, act)?;
        let answer = crate::socket::dispatch_for(
            &fixture.sessions,
            &server.public_key_bytes(),
            &greeting,
            &line,
            &AtomicBool::new(false),
            Some(&context),
        );
        assert!(!matches!(answer, Answer::Refused { .. }), "{answer:?}");
    }
    assert_eq!(calls.load(Ordering::Relaxed), 4);
    let entries = fixture.entries()?;
    let injections: Vec<_> = entries
        .iter()
        .filter(|entry| entry["body"]["kind"] == "injection")
        .collect();
    assert_eq!(injections.len(), 4);
    assert!(
        injections
            .iter()
            .all(|entry| entry["body"]["entry"]["sender"] == "agent.caller")
    );
    assert!(
        !fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    fixture.end()
}

#[test]
fn unowned_legacy_input_records_the_verified_server_key() -> TestResult {
    let fixture = Fixture::new()?;
    let server = lys_core::Ed25519Identity::ephemeral();
    let greeting = crate::protocol::Greeting::fresh(fixture.sessions.runner());
    let line = crate::protocol::sign_request(
        &server,
        &greeting,
        &crate::protocol::Act::InputBytes {
            session: "session".to_owned(),
            data: b"operator".to_vec(),
        },
    )?;
    let answer = crate::socket::dispatch(
        &fixture.sessions,
        &server.public_key_bytes(),
        &greeting,
        &line,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(matches!(answer, crate::protocol::Answer::Delivered { .. }));
    let entries = fixture.entries()?;
    let injections: Vec<_> = entries
        .iter()
        .filter(|entry| entry["body"]["kind"] == "injection")
        .collect();
    assert_eq!(injections.len(), 1);
    assert_eq!(
        injections[0]["body"]["entry"]["sender"],
        format!(
            "server:{}",
            crate::protocol::hex(&server.public_key_bytes())
        )
    );
    assert_eq!(
        *fixture.bytes.lock().map_err(|error| error.to_string())?,
        b"operator"
    );
    fixture.end()
}
#[test]
fn denied_injection_writes_neither_leaf_nor_terminal_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.entries()?;
    let judge = |asked: &InputGrant<'_>| {
        assert_eq!(
            (asked.kind, asked.id, asked.action, asked.sender),
            ("session", "session", "input", "person.owner")
        );
        Err(crate::error::RunnerError::refused(
            "SessionInputNotGranted",
            "no current grant",
        ))
    };
    let error =
        inject(&fixture.sessions, &Fixture::input("person.owner"), &judge).expect_err("denied");
    assert_eq!(error.name(), "SessionInputNotGranted");
    assert_eq!(fixture.entries()?, before);
    assert!(
        fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    fixture.end()
}
#[test]
fn allowed_injection_records_one_sender_leaf_before_terminal_delivery() -> TestResult {
    let fixture = Fixture::new()?;
    let judge = |_: &InputGrant<'_>| -> Result<(), crate::error::RunnerError> {
        assert!(fixture.sessions.table.try_lock().is_ok());
        Ok(())
    };
    inject(
        &fixture.sessions,
        &Fixture::input("agent.exact-sender"),
        &judge,
    )?;
    let entries = fixture.entries()?;
    let injected: Vec<_> = entries
        .iter()
        .filter(|entry| entry["body"]["kind"] == "injection")
        .collect();
    assert_eq!(injected.len(), 1);
    let leaf = &injected[0]["body"]["entry"];
    assert_eq!(leaf["sender"], "agent.exact-sender");
    assert_eq!(leaf["length"], 15);
    assert_eq!(
        leaf["sha256"],
        crate::protocol::hex(&sha2::Sha256::digest(b"sensitive input"))
    );
    assert!(!serde_json::to_string(&entries)?.contains("sensitive input"));
    assert_eq!(
        *fixture.bytes.lock().map_err(|error| error.to_string())?,
        b"sensitive input"
    );
    fixture.end()
}
#[test]
fn lys_notice_has_its_own_path_and_cannot_be_claimed_by_a_sender() -> TestResult {
    let fixture = Fixture::new()?;
    let judge = |_: &InputGrant<'_>| -> Result<(), crate::error::RunnerError> {
        panic!("reserved sender reached the judge")
    };
    assert_eq!(
        inject(&fixture.sessions, &Fixture::input("lys"), &judge)
            .expect_err("reserved sender")
            .name(),
        "InjectionSenderInvalid"
    );
    lys_notice(&fixture.sessions, "session", b"server notice")?;
    let entries = fixture.entries()?;
    let injected: Vec<_> = entries
        .iter()
        .filter(|entry| entry["body"]["kind"] == "injection")
        .collect();
    assert_eq!(injected.len(), 1);
    assert_eq!(injected[0]["body"]["entry"]["sender"], "lys");
    assert_eq!(
        *fixture.bytes.lock().map_err(|error| error.to_string())?,
        b"server notice"
    );
    fixture.end()
}
#[test]
fn signed_input_with_a_cookie_is_refused_before_judgement_and_writes() -> TestResult {
    let fixture = Fixture::new()?;
    let judge = |_: &InputGrant<'_>| -> Result<(), crate::error::RunnerError> {
        panic!("mixed credentials reached judge")
    };
    let mut input = Fixture::input("agent.sender");
    input.cookie = Some("administrator=fixture");
    assert_eq!(
        inject(&fixture.sessions, &input, &judge)
            .expect_err("mixed credentials")
            .name(),
        "AgentCookieRefused"
    );
    assert!(fixture.entries()?.is_empty());
    assert!(
        fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    fixture.end()
}
#[test]
fn an_existing_feed_remains_readable_and_byte_identical_before_new_injections() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = concat!(
        "{\"seq\":0,\"at\":1,\"session\":\"old\",\"body\":{\"kind\":\"boundary\",\"entry\":{\"boundary\":\"turn_end\",\"turn\":null}}}\n",
        "{\"seq\":1,\"at\":1,\"session\":\"old\",\"body\":{\"kind\":\"commit\",\"entry\":{}}}\n"
    );
    std::fs::write(dir.path().join("feed.jsonl"), old)?;
    std::fs::write(
        dir.path().join("feed.index.json"),
        serde_json::to_vec(
            &serde_json::json!({"format":"lys-runner-feed/v1","feed":"0123456789abcdef0123456789abcdef","committed":old.len(),"next_seq":2,"sources":{},"attempts":{}}),
        )?,
    )?;
    let fixture = Fixture::at(dir)?;
    assert_eq!(fixture.sessions.lock()?.feed.page(None)?.entries.len(), 1);
    let judge = |_: &InputGrant<'_>| Ok(());
    inject(&fixture.sessions, &Fixture::input("person.sender"), &judge)?;
    let page = fixture.sessions.lock()?.feed.page(None)?;
    assert_eq!(page.entries.len(), 2);
    assert!(
        matches!(&page.entries[0].body, crate::tracking_store::Body::Boundary(boundary) if boundary.boundary == "turn_end")
    );
    assert!(std::fs::read_to_string(fixture.dir.path().join("feed.jsonl"))?.starts_with(old));
    fixture.end()
}

#[test]
fn a_failed_durable_leaf_prevents_terminal_delivery() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.dir.path().join("feed.jsonl");
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    let judge = |_: &InputGrant<'_>| Ok(());
    let error = inject(&fixture.sessions, &Fixture::input("person.sender"), &judge)
        .expect_err("durability failure");
    assert_eq!(error.name(), "durable_writer_unavailable");
    assert!(
        fixture
            .bytes
            .lock()
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    std::fs::remove_dir(path)?;
    let stopped = fixture
        .sessions
        .stop_all()
        .expect_err("sticky durability failure");
    assert_eq!(stopped.name(), "durable_writer_unavailable");
    drop(fixture.terminal);
    Ok(())
}
