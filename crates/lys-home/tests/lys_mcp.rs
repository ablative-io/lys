#![cfg(test)]
//! Run credentials belong only to the generated native config.

use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use std::collections::BTreeMap;

use lys_home::harness::lys_mcp::{self, LysMcp, Seat};
use lys_home::harness::rendering_launch::File;
use lys_home::record::blocks::Hash;
use serde_json::{Value, json};
use std::error::Error;

const PASS: &str = "fixture-run-pass-only";
type TestResult = Result<(), Box<dyn Error>>;

#[derive(PartialEq)]
struct Redacted<T>(T);

impl<T> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("config contents redacted")
    }
}

type Fixture = (Vec<File>, Vec<String>, BTreeMap<String, String>);

fn fixture(codex: bool) -> Fixture {
    let instructions = File {
        path: "instructions.txt".to_owned(),
        text: "kept instructions".to_owned(),
        sha256: "kept-digest".to_owned(),
    };
    if codex {
        return (
            vec![instructions],
            vec!["--model".to_owned(), "gpt-6.1-sol".to_owned()],
            BTreeMap::new(),
        );
    }
    let text = r#"{"mcpServers":{"other":{"type":"http","url":"https://other.invalid/mcp"}},"native":true}"#;
    (
        vec![
            File {
                path: "mcp.json".to_owned(),
                text: text.to_owned(),
                sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
            },
            instructions,
        ],
        vec!["--mcp-config".to_owned(), "mcp.json".to_owned()],
        BTreeMap::new(),
    )
}

fn entry() -> LysMcp {
    LysMcp {
        url: "https://service.invalid/api/mcp".to_owned(),
        pass: PASS.to_owned(),
        seat: None,
    }
}

#[test]
fn generated_config_debug_does_not_show_its_contents() {
    let file = File {
        path: "mcp.json".to_owned(),
        text: PASS.to_owned(),
        sha256: "fixture-digest".to_owned(),
    };
    assert!(!format!("{file:?}").contains(PASS));
}

#[test]
fn claude_entry_is_exact_and_changes_no_other_member_or_file() -> TestResult {
    let (mut files, mut arguments, mut environment) = fixture(false);
    let before: Value = serde_json::from_str(&files[0].text)?;
    let other = files[1].clone();
    lys_mcp::render(&mut files, &mut arguments, &mut environment, &entry())?;
    let mut after: Value = serde_json::from_str(&files[0].text)?;
    let added = after["mcpServers"]
        .as_object_mut()
        .ok_or("no servers")?
        .remove("lys")
        .ok_or("no lys")?;
    assert_eq!(
        Redacted(added),
        Redacted(
            json!({"type":"http","url":"https://service.invalid/api/mcp","headers":{"lys-agent-pass":PASS}})
        )
    );
    assert_eq!(Redacted(after), Redacted(before));
    assert_eq!(files[1], other);
    assert_eq!(files[0].sha256, Hash::of(files[0].text.as_bytes()).as_str());
    assert!(!format!("{files:?}").contains(PASS));
    assert!(!format!("{:?}", entry()).contains(PASS));
    Ok(())
}

#[test]
fn codex_entry_is_exact_and_changes_no_other_member_or_file() -> TestResult {
    let (mut files, mut arguments, mut environment) = fixture(true);
    let before = (files.clone(), arguments.clone());
    lys_mcp::render(&mut files, &mut arguments, &mut environment, &entry())?;
    assert_eq!(files, before.0);
    assert_eq!(arguments[..2], before.1[..]);
    assert_eq!(
        arguments[2..],
        [
            "-c".to_owned(),
            "mcp_servers.lys={\"env_http_headers\" = {\"lys-agent-pass\" = \"LYS_AGENT_PASS\"}, \"url\" = \"https://service.invalid/api/mcp\"}".to_owned(),
        ]
    );
    assert_eq!(
        environment
            .get(lys_mcp::CODEX_PASS_VARIABLE)
            .map(String::as_str),
        Some(PASS)
    );
    assert_eq!(environment.len(), 1);
    assert!(!format!("{files:?} {arguments:?}").contains(PASS));
    Ok(())
}

#[test]
fn duplicates_are_refused_atomically_for_both_harnesses() -> TestResult {
    for codex in [false, true] {
        let (mut files, mut arguments, mut environment) = fixture(codex);
        lys_mcp::render(&mut files, &mut arguments, &mut environment, &entry())?;
        let before = files.clone();
        let error = lys_mcp::render(&mut files, &mut arguments, &mut environment, &entry())
            .err()
            .ok_or("duplicate accepted")?;
        assert_eq!(error.name(), "LysMcpDuplicate");
        assert!(!format!("{error:?} {error}").contains(PASS));
        assert_eq!(files, before);
    }
    Ok(())
}

#[test]
fn invalid_inputs_and_configs_never_echo_the_pass_or_change_files() -> TestResult {
    for codex in [false, true] {
        for change in 0..6 {
            let (mut files, mut arguments, mut environment) = fixture(codex);
            let mut entry = entry();
            match change {
                0 => entry.url = format!("file:///{PASS}"),
                1 => entry.pass.push('\n'),
                2 => entry.url = "https://service.invalid/other".to_owned(),
                3 => {
                    files.retain(|file| file.path != "mcp.json");
                    environment.insert(lys_mcp::CODEX_PASS_VARIABLE.to_owned(), "x".to_owned());
                }
                4 if codex => arguments.extend(["-c".to_owned(), "mcp_servers.lys={}".to_owned()]),
                4 => files[0].text = format!("broken {PASS}"),
                5 if codex => {
                    environment.insert(lys_mcp::CODEX_SEAT_VARIABLE.to_owned(), "x".to_owned());
                }
                5 => files.push(files[0].clone()),
                _ => return Err("unknown fixture".into()),
            }
            let before = files.clone();
            let error = lys_mcp::render(&mut files, &mut arguments, &mut environment, &entry)
                .err()
                .ok_or("invalid config accepted")?;
            assert!(!format!("{error:?} {error} {entry:?}").contains(PASS));
            assert_eq!(files, before);
        }
    }
    Ok(())
}

/// A seated run's config carries the runner's signature over the pass by the
/// seat key Lys delegated, and the delegation itself; never the seat key.
#[test]
fn a_seated_run_carries_the_runners_signature_over_the_pass_and_never_its_key() -> TestResult {
    let certificate = Ed25519Identity::from_seed(&zeroize::Zeroizing::new([7_u8; 32]));
    let seed = [9_u8; 32];
    let seat_key = Ed25519Identity::from_seed(&zeroize::Zeroizing::new(seed));
    let seat_public = seat_key.public_key_bytes();
    let delegated = lys_mcp::delegation_bytes(
        "agent-1",
        "serial-1",
        "session-1",
        &seat_public,
        4_102_444_800,
    );
    let delegation = lys_mcp::hex(&sign_attestation(&delegated, &certificate).to_cose_bytes());
    let seat = Seat {
        agent: "agent-1".to_owned(),
        session: "session-1".to_owned(),
        serial: "serial-1".to_owned(),
        not_after: 4_102_444_800,
        delegation: delegation.clone(),
        key: lys_mcp::hex(&seed),
    };
    for codex in [false, true] {
        let (mut files, mut arguments, mut environment) = fixture(codex);
        let mut seated = entry();
        seated.seat = Some(seat.clone());
        lys_mcp::render(&mut files, &mut arguments, &mut environment, &seated)?;
        let text = files[0].text.clone();
        assert!(!text.contains(&seat.key), "the seat key is never written");
        let header = if codex {
            assert!(!format!("{arguments:?}").contains(&seat.key));
            environment
                .get(lys_mcp::CODEX_SEAT_VARIABLE)
                .ok_or("no seat header")?
                .to_owned()
        } else {
            let root: Value = serde_json::from_str(&text)?;
            root["mcpServers"]["lys"]["headers"][lys_mcp::SEAT_HEADER]
                .as_str()
                .ok_or("no seat header")?
                .to_owned()
        };
        let words: Vec<&str> = header.split(' ').collect();
        let [serial, not_after, public, carried, signature] = words.as_slice() else {
            return Err("the seat header is not five words".into());
        };
        assert_eq!(
            (*serial, *not_after, *carried),
            ("serial-1", "4102444800", delegation.as_str())
        );
        assert_eq!(*public, lys_mcp::hex(&seat_public));
        let cose = lys_mcp::unhex(signature).ok_or("signature not hex")?;
        verify_attestation_bytes_by_signer(
            &cose,
            &lys_mcp::pass_bytes("agent-1", "session-1", PASS),
            &seat_public,
        )?;
        let delegation_cose = lys_mcp::unhex(carried).ok_or("delegation not hex")?;
        verify_attestation_bytes_by_signer(
            &delegation_cose,
            &delegated,
            &certificate.public_key_bytes(),
        )?;
        assert!(
            verify_attestation_bytes_by_signer(
                &cose,
                &lys_mcp::pass_bytes("agent-1", "session-1", "another-pass"),
                &seat_public,
            )
            .is_err(),
            "the signature is over this pass only"
        );
    }
    Ok(())
}

#[test]
fn a_seat_whose_key_is_not_a_seed_is_refused_and_no_file_changes() -> TestResult {
    let (mut files, mut arguments, mut environment) = fixture(false);
    let before = files.clone();
    let mut seated = entry();
    seated.seat = Some(Seat {
        agent: "agent-1".to_owned(),
        session: "session-1".to_owned(),
        serial: "serial-1".to_owned(),
        not_after: 1,
        delegation: "00".to_owned(),
        key: "not-hex".to_owned(),
    });
    let refused = lys_mcp::render(&mut files, &mut arguments, &mut environment, &seated)
        .err()
        .ok_or("a seat without a key was accepted")?;
    assert_eq!(refused.name(), "LysMcpInvalid");
    assert_eq!(Redacted(files), Redacted(before));
    Ok(())
}
