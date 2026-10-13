//! Supplied session time, checked failure and unchanged stored session bytes.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::ManualClock;
use lys_core::clock::{Clock, ClockSource};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::error::ServerError;
use lys_identity_server::session::Sessions;

#[path = "clock/digest.rs"]
mod digest;
use digest::source_digest;

type TestResult = Result<(), Box<dyn Error>>;
const T0: i64 = 1_700_000_000;

#[test]
fn clock_wait_ratchet() -> TestResult {
    for fixture in FIXTURE_RATCHET {
        fixture.check(fixture.source)?;
    }
    Ok(())
}

#[test]
fn clock_wait_ratchet_refuses_each_base_call() {
    let issuer = &FIXTURE_RATCHET[0];
    let helper =
        "fn let_a_second_pass() { std::thread::sleep(std::time::Duration::from_secs(2)); }";
    for site in [
        "let again = bench.path(\"issuer-again.pem\");",
        "let at_issue = bench.path(\"issuer-at-issue.pem\");",
        "let later = bench.path(\"later.pem\");",
    ] {
        assert!(issuer.source.contains(site));
        let changed = issuer
            .source
            .replacen(site, &format!("let_a_second_pass(); {site}"), 1);
        assert!(issuer.check(&format!("{changed}\n{helper}")).is_err());
    }
    let requests = &FIXTURE_RATCHET[2];
    let site = "clock.set(1_700_000_003);";
    assert!(requests.source.contains(site));
    let changed = requests.source.replacen(
        site,
        "tokio::time::sleep(std::time::Duration::from_secs(3)).await;",
        1,
    );
    assert!(requests.check(&changed).is_err());
}

#[test]
fn clock_wait_ratchet_refuses_introduced_and_hidden_waits() {
    let fixture = &FIXTURE_RATCHET[1];
    for introduced in [
        "std::thread::sleep(std::time::Duration::from_secs(1));",
        "while std::time::Instant::now() < end {}",
        "renamed_helper();",
        "let callback = renamed_helper; callback();",
    ] {
        let source =
            fixture
                .source
                .replacen("let fixture =", &format!("{introduced} let fixture ="), 1);
        assert_ne!(source, fixture.source);
        assert!(fixture.check(&source).is_err(), "{introduced}");
    }
}

#[test]
fn clock_wait_ratchet_accepts_only_lexical_trivia() -> TestResult {
    let fixture = &FIXTURE_RATCHET[1];
    fixture.check(&format!(
        "/* sleep /* nested */ */\n{}\n// timeout\n",
        fixture.source
    ))?;
    assert_ne!(source_digest("ab c")?, source_digest("a bc")?);
    assert_ne!(source_digest("\"a // b\"")?, source_digest("\"a b\"")?);
    assert_eq!(
        source_digest("r##\"/* text */\"##")?,
        source_digest(" r##\"/* text */\"## ")?
    );
    assert_ne!(source_digest("'a' 'static")?, source_digest("'b' 'static")?);
    assert_eq!(
        source_digest("use x::{a, b}; use y::c;")?,
        source_digest("use y::c; use x::{b, a,};")?
    );
    assert_eq!(source_digest("f(a) [a]")?, source_digest("f(a,) [a,]")?);
    assert_ne!(source_digest("(a,)")?, source_digest("(a)")?);
    assert_eq!(source_digest("(a,b,)")?, source_digest("(a,b)")?);
    assert!(source_digest("/* open").is_err());
    assert!(source_digest("\"open").is_err());
    Ok(())
}

fn actor() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.example.test", "subject")?,
        Provenance::new(AuthMethod::Oidc, 90),
    ))
}

fn source(clock: &Arc<ManualClock>) -> ClockSource {
    let provider: Arc<dyn Clock> = Arc::<identity_contract::harness::ManualClock>::clone(clock);
    ClockSource::Supplied(provider)
}

#[test]
fn session_clock_is_shared_and_expiry_at_equality_refuses() -> TestResult {
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::new_with_clock(10, false, source(&clock));
    let cookie = sessions.begin(actor()?)?;
    assert_eq!(
        clock.reads(),
        1,
        "begin samples once and passes time through pruning"
    );
    let entry = sessions.session(Some(&cookie))?;
    assert_eq!(entry.started_at, u64::try_from(T0)?);
    assert_eq!(entry.ends_at, u64::try_from(T0 + 10)?);
    clock.set(T0 + 9);
    assert!(sessions.is_live(&entry.id)?);
    assert_eq!(sessions.current(Some(&cookie))?, entry.id);
    clock.set(T0 + 10);
    assert!(!sessions.is_live(&entry.id)?);
    assert!(matches!(
        sessions.session(Some(&cookie)),
        Err(ServerError::NotSignedIn)
    ));
    assert!(sessions.live(|_| true)?.is_empty());
    let reads = clock.reads();
    assert!(!sessions.is_live("unknown")?);
    assert!(matches!(
        sessions.session(Some("lys_directory_session=unknown")),
        Err(ServerError::NotSignedIn)
    ));
    assert_eq!(
        clock.reads(),
        reads,
        "missing indexed entries need no clock read"
    );
    Ok(())
}

#[test]
fn separate_session_owners_do_not_share_time() -> TestResult {
    let first = Arc::new(ManualClock::new(T0));
    let second = Arc::new(ManualClock::new(T0 + 100));
    let a = Sessions::new_with_clock(10, false, source(&first));
    let b = Sessions::new_with_clock(10, false, source(&second));
    let ac = a.begin(actor()?)?;
    let bc = b.begin(actor()?)?;
    first.set(T0 + 10);
    assert!(matches!(
        a.session(Some(&ac)),
        Err(ServerError::NotSignedIn)
    ));
    assert_eq!(b.session(Some(&bc))?.started_at, u64::try_from(T0 + 100)?);
    Ok(())
}

#[test]
fn clock_conversion_refuses_before_session_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 10, false, source(&clock))?;
    for at in [T0, -1] {
        clock.set(at);
        clock.refuse(at == T0);
        assert!(matches!(
            sessions.begin(actor()?),
            Err(ServerError::ClockUnavailable { .. })
        ));
        assert!(
            !path.exists(),
            "a refused reading cannot write a session file"
        );
    }
    clock.refuse(false);
    clock.set(T0);
    assert!(sessions.live(|_| true)?.is_empty());
    Ok(())
}

#[test]
fn failed_clock_does_not_prune_or_rewrite_kept_sessions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 10, false, source(&clock))?;
    let cookie = sessions.begin(actor()?)?;
    let before = std::fs::read(&path)?;
    clock.refuse(true);
    assert!(matches!(
        sessions.live(|_| true),
        Err(ServerError::ClockUnavailable { .. })
    ));
    assert!(matches!(
        sessions.end_matching(|_| true),
        Err(ServerError::ClockUnavailable { .. })
    ));
    assert_eq!(std::fs::read(&path)?, before);
    clock.refuse(false);
    assert_eq!(
        sessions.session(Some(&cookie))?.started_at,
        u64::try_from(T0)?
    );
    Ok(())
}

#[test]
fn stored_sessions_reopen_byte_equal_under_production_defaults() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 4_102_444_800, false, source(&clock))?;
    let cookie = sessions.begin(actor()?)?;
    let entry = sessions.session(Some(&cookie))?;
    let before = std::fs::read(&path)?;
    drop(sessions);
    let reopened = Sessions::open(path.clone(), 600, false)?;
    assert_eq!(reopened.session(Some(&cookie))?, entry);
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn clock_failure_has_its_own_wire_tag_and_status() {
    use axum::response::IntoResponse;
    let error = ServerError::ClockUnavailable {
        reason: "provider read refused".to_owned(),
    };
    assert_eq!(error.name(), "ClockUnavailable");
    assert!(error.to_string().contains("provider read refused"));
    assert_eq!(
        error.into_response().status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
}

#[test]
fn failed_clock_refuses_before_opening_stored_sessions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let before = b"unreadable session bytes";
    std::fs::write(&path, before)?;
    let clock = Arc::new(ManualClock::new(-1));
    let opened = Sessions::open_with_clock(path.clone(), 10, false, source(&clock));
    assert!(matches!(opened, Err(ServerError::ClockUnavailable { .. })));
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn session_work_counts_end_at_completed_storage() -> TestResult {
    let clock = Arc::new(ManualClock::new(T0));
    let directory = tempfile::tempdir()?;
    let file = directory.path().join("sessions.json");
    let sessions = Sessions::open_with_clock(file.clone(), 10, false, source(&clock))?;
    let before = clock.reads();
    let started = std::time::Instant::now();
    let cookie = sessions.begin(actor()?)?;
    let elapsed = started.elapsed();
    let reads = clock
        .reads()
        .checked_sub(before)
        .ok_or("clock count decreased")?;
    assert_eq!(reads, 1);
    let stored = std::fs::read(&file)?;
    assert!(!stored.is_empty());
    let before = clock.reads();
    let entry = sessions.session(Some(&cookie))?;
    assert_eq!(clock.reads() - before, 1);
    assert_eq!(std::fs::read(&file)?, stored);
    eprintln!(
        "CLOCK_SESSION {}",
        serde_json::json!({
            "operation": "begin", "provider_reads": reads,
            "elapsed_nanos": elapsed.as_nanos(), "stored_bytes": stored.len(),
            "physical_syncs": null, "history_visits": null,
        })
    );
    let before = clock.reads();
    let started = std::time::Instant::now();
    sessions.end(&entry.id, |_| true)?;
    let elapsed = started.elapsed();
    let reads = clock
        .reads()
        .checked_sub(before)
        .ok_or("clock count decreased")?;
    assert_eq!(reads, 1);
    assert_ne!(std::fs::read(&file)?, stored);
    assert!(!sessions.is_live(&entry.id)?);
    assert_eq!(
        clock.reads() - before,
        1,
        "missing ids need no further read"
    );
    eprintln!(
        "CLOCK_SESSION {}",
        serde_json::json!({
            "operation": "end", "provider_reads": reads,
            "elapsed_nanos": elapsed.as_nanos(), "physical_syncs": null, "history_visits": null,
        })
    );
    Ok(())
}

struct ReviewedSource {
    name: &'static str,
    source: &'static str,
    digest: [u8; 32],
}

impl ReviewedSource {
    fn check(&self, source: &str) -> TestResult {
        let source = if self.name == "sessions" {
            source
                .split_once("\nconst FIXTURE_RATCHET:")
                .ok_or("fixture inventory is missing")?
                .0
        } else {
            source
        };
        if source_digest(source)? != self.digest {
            return Err(format!(
                "{} has unreviewed fixture tokens; clock waits are not qualified",
                self.name
            )
            .into());
        }
        Ok(())
    }
}

const FIXTURE_RATCHET: &[ReviewedSource] = &[
    ReviewedSource {
        name: "issuer",
        source: include_str!("../../lys/tests/ca_log/issuer_cert.rs"),
        digest: [
            0x0e, 0xc0, 0xda, 0x3b, 0x1f, 0x85, 0x3d, 0xfd, 0xf5, 0x1e, 0xe3, 0x3a, 0x91, 0x7e,
            0x4c, 0x2f, 0x89, 0x21, 0xf5, 0x1c, 0x2d, 0x30, 0x7c, 0x36, 0x94, 0xaf, 0x3c, 0x46,
            0x5b, 0x46, 0x06, 0x26,
        ],
    },
    ReviewedSource {
        name: "support",
        source: include_str!("../../lys/tests/ca_log/support.rs"),
        digest: [
            0x78, 0x7a, 0xc1, 0x95, 0x83, 0x89, 0xce, 0xdc, 0xf6, 0x41, 0xc0, 0xfb, 0x33, 0xa7,
            0x8e, 0x39, 0xcc, 0xc8, 0x5d, 0x38, 0xfa, 0xf1, 0x46, 0x05, 0x5e, 0x36, 0xfb, 0x52,
            0x60, 0x60, 0x30, 0x25,
        ],
    },
    ReviewedSource {
        name: "requests",
        source: concat!(
            include_str!("requests.rs"),
            "\n",
            include_str!("requests/replay_cases.rs")
        ),
        digest: [
            0x7a, 0x6f, 0x8e, 0x5e, 0x20, 0x7f, 0x5e, 0x0c, 0x91, 0x13, 0x3e, 0x96, 0x30, 0xcf,
            0x32, 0x41, 0x32, 0x23, 0x61, 0x52, 0xd7, 0xcc, 0xc7, 0xed, 0x1b, 0xce, 0xe6, 0x9e,
            0x37, 0xad, 0xba, 0x08,
        ],
    },
    ReviewedSource {
        name: "cli",
        source: include_str!("../../lys/examples/clock-cli.rs"),
        digest: [
            0xff, 0x2a, 0xfd, 0x69, 0x5e, 0xfc, 0xc3, 0xa0, 0x7c, 0xc3, 0x1f, 0x1c, 0x33, 0xa7,
            0x65, 0x53, 0x73, 0x3c, 0x3c, 0x28, 0x81, 0xd0, 0xd7, 0x2b, 0x2b, 0x34, 0xfb, 0xe5,
            0x55, 0x1e, 0xa9, 0x69,
        ],
    },
    ReviewedSource {
        name: "creation",
        source: include_str!("../../lys-core/tests/clock.rs"),
        digest: [
            0x93, 0x3f, 0xbf, 0xb1, 0x9d, 0x24, 0x03, 0x78, 0xa7, 0x5d, 0x8c, 0x8b, 0x8f, 0xd4,
            0x6f, 0x01, 0xc9, 0x61, 0x8f, 0x07, 0x9f, 0xd0, 0xe4, 0xbb, 0xca, 0x58, 0xb5, 0x8b,
            0x66, 0xa3, 0x90, 0x04,
        ],
    },
    ReviewedSource {
        name: "harness",
        source: include_str!("../../../tests/identity_contract/src/harness.rs"),
        digest: [
            0xb3, 0x93, 0x03, 0x3b, 0x0b, 0x4c, 0x70, 0xae, 0xd6, 0x4a, 0x78, 0x58, 0x4e, 0xb3,
            0x0a, 0x67, 0x79, 0x69, 0xc0, 0x37, 0x78, 0x29, 0x97, 0xfd, 0x95, 0x34, 0x4a, 0xbe,
            0x75, 0x76, 0x99, 0x21,
        ],
    },
    ReviewedSource {
        name: "serving",
        source: include_str!("../../../tests/identity_contract/src/harness_serve.rs"),
        digest: [
            0xfc, 0x8e, 0xb2, 0x09, 0xe3, 0xba, 0xe6, 0xc5, 0x01, 0xc2, 0x3c, 0xbb, 0xb5, 0xc3,
            0x79, 0x0c, 0x99, 0x0a, 0xad, 0x84, 0xfa, 0xab, 0x85, 0xdc, 0x37, 0xd3, 0x93, 0xf8,
            0x55, 0xd9, 0x52, 0x10,
        ],
    },
    ReviewedSource {
        name: "sessions",
        source: include_str!("clock.rs"),
        digest: [
            0x3e, 0x24, 0x2b, 0x63, 0x94, 0x6c, 0x52, 0x41, 0xa2, 0x58, 0xf3, 0xf6, 0x4f, 0xa4,
            0x1c, 0x69, 0x6c, 0xb2, 0xb1, 0x7e, 0xec, 0x3e, 0xea, 0x81, 0x80, 0xd9, 0x0e, 0x8b,
            0xcf, 0xfe, 0x79, 0xb0,
        ],
    },
    ReviewedSource {
        name: "tokens",
        source: include_str!("clock/digest.rs"),
        digest: [
            0xf3, 0x13, 0x8e, 0x00, 0x71, 0xc9, 0xb1, 0x7e, 0x61, 0x38, 0x66, 0xcd, 0xce, 0x3f,
            0xc0, 0x6c, 0x87, 0x37, 0x27, 0x34, 0xb7, 0x40, 0x6d, 0xea, 0x60, 0x57, 0x94, 0x41,
            0x0e, 0x8c, 0xef, 0xd1,
        ],
    },
];
