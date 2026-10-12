//! Supplied session time, checked failure and unchanged stored session bytes.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::ManualClock;
use lys_core::clock::{Clock, ClockSource};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::error::ServerError;
use lys_identity_server::session::Sessions;

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
    let provider: Arc<dyn Clock> = clock.clone();
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

fn quoted_end(bytes: &[u8], start: usize) -> Result<usize, Box<dyn Error>> {
    let quote = bytes[start];
    let mut end = start + 1;
    while end < bytes.len() {
        if bytes[end] == quote {
            return Ok(end + 1);
        }
        if bytes[end] == b'\\' {
            end += 1;
        }
        end += 1;
    }
    Err("unterminated fixture literal".into())
}

fn raw_end(bytes: &[u8], start: usize) -> Result<Option<usize>, Box<dyn Error>> {
    if bytes[start] != b'r' {
        return Ok(None);
    }
    let mut quote = start + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return Ok(None);
    }
    let hashes = quote - start - 1;
    let mut end = quote + 1;
    while end < bytes.len() {
        if bytes[end] == b'"'
            && bytes
                .get(end + 1..end + 1 + hashes)
                .is_some_and(|tail| tail.iter().all(|byte| *byte == b'#'))
        {
            return Ok(Some(end + 1 + hashes));
        }
        end += 1;
    }
    Err("unterminated raw fixture literal".into())
}

fn comment_end(bytes: &[u8], start: usize) -> Result<usize, Box<dyn Error>> {
    let mut depth = 1;
    let mut end = start + 2;
    while end + 1 < bytes.len() {
        match &bytes[end..end + 2] {
            b"/*" => {
                depth += 1;
                end += 2;
            }
            b"*/" => {
                depth -= 1;
                end += 2;
                if depth == 0 {
                    return Ok(end);
                }
            }
            _ => end += 1,
        }
    }
    Err("unterminated fixture comment".into())
}

fn import_leaves(tokens: &[Vec<u8>], depth: usize) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    if depth > 32 {
        return Err("fixture import exceeds review bound".into());
    }
    let Some(open) = tokens.iter().position(|token| token == b"{") else {
        let mut leaf = Vec::new();
        for token in tokens {
            leaf.extend(u64::try_from(token.len())?.to_le_bytes());
            leaf.extend(token);
        }
        return Ok(vec![leaf]);
    };
    if tokens.last().is_none_or(|token| token != b"}") {
        return Err("unreviewed fixture import syntax".into());
    }
    let mut leaves = Vec::new();
    let mut start = open + 1;
    let mut nesting = 0;
    for (at, token) in tokens.iter().enumerate().skip(open + 1) {
        if nesting == 0 && (token == b"," || at == tokens.len() - 1) {
            if start < at {
                let joined: Vec<Vec<u8>> = tokens[..open]
                    .iter()
                    .chain(&tokens[start..at])
                    .cloned()
                    .collect();
                leaves.extend(import_leaves(&joined, depth + 1)?);
            }
            start = at + 1;
        }
        if token == b"{" {
            nesting += 1;
        }
        if token == b"}" && nesting > 0 {
            nesting -= 1;
        }
    }
    leaves.sort();
    Ok(leaves)
}

fn normalized_tokens(tokens: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut normalized = Vec::new();
    let mut delimiters = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        if tokens[at] == b"use" {
            let mut imports = Vec::new();
            while tokens.get(at).is_some_and(|token| token == b"use") {
                let end = tokens[at..]
                    .iter()
                    .position(|token| token == b";")
                    .ok_or("unterminated fixture import")?
                    + at;
                imports.extend(import_leaves(&tokens[at + 1..end], 0)?);
                at = end + 1;
            }
            imports.sort();
            normalized.push(b"use".to_vec());
            normalized.extend(imports);
            normalized.push(b";".to_vec());
            continue;
        }
        let token = &tokens[at];
        if token == b"(" || token == b"[" || token == b"{" {
            let call = token == b"("
                && at > 0
                && tokens[at - 1]
                    .first()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
            delimiters.push((token.clone(), call, 0_usize));
        }
        let trailing = token == b","
            && tokens.get(at + 1).is_some_and(|next| {
                next == b"}"
                    || next == b"]"
                    || next == b")"
                        && delimiters
                            .last()
                            .is_some_and(|(_, call, commas)| *call || *commas > 0)
            });
        if token == b","
            && let Some((_, _, commas)) = delimiters.last_mut()
        {
            *commas += 1;
        }
        if !trailing {
            normalized.push(token.clone());
        }
        if token == b")" || token == b"]" || token == b"}" {
            let (open, ..) = delimiters.pop().ok_or("unbalanced fixture delimiter")?;
            if !matches!(
                (open.as_slice(), token.as_slice()),
                (b"(", b")") | (b"[", b"]") | (b"{", b"}")
            ) {
                return Err("mismatched fixture delimiter".into());
            }
        }
        at += 1;
    }
    if !delimiters.is_empty() {
        return Err("unclosed fixture delimiter".into());
    }
    Ok(normalized)
}

fn source_digest(source: &str) -> Result<[u8; 32], Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    if source.len() > 2_000_000 {
        return Err("fixture source exceeds review bound".into());
    }
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if bytes.get(at..at + 2) == Some(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if bytes.get(at..at + 2) == Some(b"/*") {
            at = comment_end(bytes, at)?;
            continue;
        }
        let end = if let Some(end) = raw_end(bytes, at)? {
            end
        } else if bytes[at] == b'"'
            || (bytes[at] == b'\''
                && (bytes.get(at + 1) == Some(&b'\\') || bytes.get(at + 2) == Some(&b'\'')))
        {
            quoted_end(bytes, at)?
        } else if bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_' {
            let mut end = at + 1;
            while bytes
                .get(end)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                end += 1;
            }
            end
        } else if bytes[at].is_ascii() {
            at + 1
        } else {
            return Err("unreviewed non-ASCII fixture syntax".into());
        };
        tokens.push(bytes[at..end].to_vec());
        at = end;
    }
    let mut digest = Sha256::new();
    for token in normalized_tokens(&tokens)? {
        digest.update(u64::try_from(token.len())?.to_le_bytes());
        digest.update(token);
    }
    Ok(digest.finalize().into())
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
        source: include_str!("requests.rs"),
        digest: [
            0x40, 0xb5, 0x58, 0x62, 0xe7, 0xd9, 0x7e, 0xd6, 0x5f, 0xda, 0xae, 0x15, 0xb3, 0x07,
            0x52, 0xb9, 0x21, 0x30, 0x90, 0x3a, 0x77, 0x67, 0x44, 0x0b, 0xac, 0xe3, 0xd4, 0x73,
            0xd3, 0x71, 0x68, 0x45,
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
            0x3a, 0x25, 0xab, 0xb2, 0x52, 0xdc, 0x66, 0xe2, 0x38, 0x9c, 0x45, 0x29, 0x9f, 0x02,
            0x7b, 0xb8, 0x10, 0x2c, 0x88, 0x34, 0x06, 0x1a, 0x9a, 0x32, 0xcf, 0x2a, 0xab, 0xc0,
            0x16, 0xce, 0x7d, 0x02,
        ],
    },
    ReviewedSource {
        name: "serving",
        source: include_str!("../../../tests/identity_contract/src/harness_serve.rs"),
        digest: [
            0xcd, 0x51, 0xbd, 0x0f, 0x09, 0x7e, 0x40, 0xb7, 0xa8, 0x38, 0x2c, 0x43, 0x8e, 0x72,
            0x53, 0xc9, 0xd5, 0x05, 0x96, 0x58, 0x65, 0x47, 0x35, 0x87, 0x4d, 0x64, 0xbb, 0x94,
            0x20, 0xea, 0x71, 0xc2,
        ],
    },
    ReviewedSource {
        name: "sessions",
        source: include_str!("clock.rs"),
        digest: [
            0xda, 0xc6, 0xbc, 0x58, 0xb0, 0xe4, 0x31, 0x0e, 0x5a, 0xbd, 0x4c, 0xc5, 0x0c, 0x60,
            0xae, 0x62, 0x39, 0x67, 0xd1, 0xca, 0x7e, 0xf9, 0x59, 0xe6, 0x06, 0xc6, 0x10, 0x24,
            0x8e, 0xf4, 0x55, 0xd4,
        ],
    },
];
