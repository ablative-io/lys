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
        let sessions = Sessions::open(dir.path(), 4096)?;
        sessions.start(Launch {
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
        })?;
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
