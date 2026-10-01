#![cfg(test)]
//! Run credentials belong only to the generated native config.

use lys_home::harness::lys_mcp::{self, LysMcp};
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
