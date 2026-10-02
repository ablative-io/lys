#![cfg(test)]
//! Run credentials belong only to the generated native config.

use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use lys_home::harness::lys_mcp::{self, LysMcp, Seat};
use lys_home::harness::rendering_launch::File;
use lys_home::record::blocks::Hash;
use serde_json::{Value, json};
use std::collections::BTreeMap;
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

fn fixture(codex: bool) -> (Vec<File>, BTreeMap<String, String>) {
    let (path, root, text) = if codex {
        (
            "config.toml",
            "CODEX_HOME",
            "model = 'fixture-model'\nmcp_servers = { other = { url = 'https://other.invalid/mcp' } }\n",
        )
    } else {
        (
            "mcp.json",
            "CLAUDE_CONFIG_DIR",
            r#"{"mcpServers":{"other":{"type":"http","url":"https://other.invalid/mcp"}},"native":true}"#,
        )
    };
    (
        vec![
            File {
                path: path.to_owned(),
                text: text.to_owned(),
                sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
            },
            File {
                path: "instructions.txt".to_owned(),
                text: "kept instructions".to_owned(),
                sha256: "kept-digest".to_owned(),
            },
        ],
        BTreeMap::from([(root.to_owned(), String::new())]),
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
    let (mut files, roots) = fixture(false);
    let before: Value = serde_json::from_str(&files[0].text)?;
    let other = files[1].clone();
    lys_mcp::render(&mut files, &roots, &entry())?;
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
    let (mut files, roots) = fixture(true);
    let before: toml::Value = toml::from_str(&files[0].text)
        .map_err(|error: toml::de::Error| format!("invalid native TOML at {:?}", error.span()))?;
    let other = files[1].clone();
    lys_mcp::render(&mut files, &roots, &entry())?;
    let mut after: toml::Value = toml::from_str(&files[0].text)
        .map_err(|error: toml::de::Error| format!("invalid native TOML at {:?}", error.span()))?;
    let added = after["mcp_servers"]
        .as_table_mut()
        .ok_or("no servers")?
        .remove("lys")
        .ok_or("no lys")?;
    let expected_text = format!(
        "url = 'https://service.invalid/api/mcp'\nhttp_headers = {{ lys-agent-pass = '{PASS}' }}\n"
    );
    let expected: toml::Value = toml::from_str(&expected_text)
        .map_err(|error: toml::de::Error| format!("invalid expected TOML at {:?}", error.span()))?;
    assert_eq!(Redacted(added), Redacted(expected));
    assert_eq!(Redacted(after), Redacted(before));
    assert_eq!(files[1], other);
    assert_eq!(files[0].sha256, Hash::of(files[0].text.as_bytes()).as_str());
    assert!(!format!("{files:?}").contains(PASS));
    Ok(())
}

#[test]
fn duplicates_are_refused_atomically_for_both_harnesses() -> TestResult {
    for codex in [false, true] {
        let (mut files, roots) = fixture(codex);
        lys_mcp::render(&mut files, &roots, &entry())?;
        let before = files.clone();
        let error = lys_mcp::render(&mut files, &roots, &entry())
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
            let (mut files, mut roots) = fixture(codex);
            let mut entry = entry();
            match change {
                0 => entry.url = format!("file:///{PASS}"),
                1 => entry.pass.push('\n'),
                2 => roots.clear(),
                3 => {
                    roots.insert("CODEX_HOME".to_owned(), "wrong".to_owned());
                    roots.insert("CLAUDE_CONFIG_DIR".to_owned(), String::new());
                }
                4 => files[0].text = format!("broken {PASS}"),
                5 => files.push(files[0].clone()),
                _ => return Err("unknown fixture".into()),
            }
            let before = files.clone();
            let error = lys_mcp::render(&mut files, &roots, &entry)
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
        let (mut files, roots) = fixture(codex);
        let mut seated = entry();
        seated.seat = Some(seat.clone());
        lys_mcp::render(&mut files, &roots, &seated)?;
        let text = files[0].text.clone();
        assert!(!text.contains(&seat.key), "the seat key is never written");
        let header = if codex {
            let root: toml::Value = toml::from_str(&text)?;
            root["mcp_servers"]["lys"]["http_headers"][lys_mcp::SEAT_HEADER]
                .as_str()
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
    let (mut files, roots) = fixture(false);
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
    let refused = lys_mcp::render(&mut files, &roots, &seated)
        .err()
        .ok_or("a seat without a key was accepted")?;
    assert_eq!(refused.name(), "LysMcpInvalid");
    assert_eq!(Redacted(files), Redacted(before));
    Ok(())
}
