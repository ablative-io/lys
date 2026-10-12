//! Account values cross a local broker closure only at the spawn boundary.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_runner::pty::{Spawn, Spawned};
use lys_runner::rotation::{Rotation, RotationState};
use lys_runner::{RunnerError, Sessions};
use lys_secrets::{
    Broker, BrokerPaths, Holder, IssuedHandle, LocalGrants, Presentation, Secret, SecretRelation,
    Used, new_operation_id,
};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;
const NOW: i64 = 1_800_000_000_000;
const VALUE: &[u8] = b"fixture-account-token-value";
const SECOND: &[u8] = b"fixture-second-account-token-value";

struct LocalBroker {
    dir: tempfile::TempDir,
    broker: Broker<LocalGrants>,
    agent: Ed25519Identity,
    issued: Vec<IssuedHandle>,
}

impl LocalBroker {
    fn open() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let paths = BrokerPaths {
            store_dir: dir.path().join("store"),
            log_dir: dir.path().join("log"),
            store_key: dir.path().join("store.key"),
            audit_key: dir.path().join("audit.key"),
            anchor: dir.path().join("anchor"),
        };
        let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
        let grants = LocalGrants::new();
        for secret in ["first", "second"] {
            grants.grant(SecretRelation {
                identity: "agent".to_owned(),
                secret: secret.to_owned(),
                granted_by: Some("person".to_owned()),
            })?;
        }
        let mut broker = Broker::create(&paths, grants, Box::new(|| NOW))?;
        let holder = Holder {
            identity: "agent".to_owned(),
            key: agent.public_key_bytes(),
        };
        let mut issued = Vec::new();
        for (secret, value) in [("first", VALUE), ("second", SECOND)] {
            broker.seal(secret, "person", &Secret::from_slice(value))?;
            issued.push(broker.issue(&holder, secret, 10, NOW + 60_000)?);
        }
        Ok(Self {
            dir,
            broker,
            agent,
            issued,
        })
    }

    fn rotation(&self) -> Result<RotationState, Box<dyn Error>> {
        Ok(RotationState::new(serde_json::from_value(
            serde_json::json!({
                "accounts": self.issued.iter().map(|handle| handle.id.as_str()).collect::<Vec<_>>(),
                "variable": "LYS_HANDLE_ACCOUNT", "limit": {"signal":"words","words":["usage limit"]},
                "resume_arguments": ["--resume", "conversation"]
            }),
        )?)?)
    }

    fn use_account(
        &mut self,
        id: &str,
        place: &mut dyn FnMut(&[u8]) -> Result<(), RunnerError>,
    ) -> Result<(), RunnerError> {
        let issued = self
            .issued
            .iter()
            .find(|handle| handle.id.as_str() == id)
            .ok_or_else(|| {
                RunnerError::refused(
                    "account_handle_unheld",
                    "the agent does not hold the account",
                )
            })?;
        let presentation = Presentation::sign(
            &issued.id,
            &new_operation_id().map_err(broker_error)?,
            NOW,
            [7; 32],
            &self.agent,
        )
        .map_err(broker_error)?;
        match self
            .broker
            .use_handle(&issued.token, &presentation, |value| place(value.expose()))
            .map_err(broker_error)?
        {
            Used::Forwarded { answer, .. } => answer,
            Used::Retried { .. } => Err(RunnerError::refused(
                "account_use_replayed",
                "no credential was forwarded",
            )),
        }
    }
}

fn broker_error(error: lys_secrets::SecretsError) -> RunnerError {
    RunnerError::refused(error.name(), "the local account broker refused the use")
}

fn output(mut spawned: Spawned) -> Result<String, Box<dyn Error>> {
    let status = spawned.child.wait()?;
    assert!(status.success());
    let mut text = String::new();
    spawned.reader.read_to_string(&mut text)?;
    Ok(text)
}

fn digest_arguments(variable: &str) -> Vec<String> {
    vec![
        "-c".to_owned(),
        format!("printf '%s' \"${variable}\" | /usr/bin/shasum -a 256"),
    ]
}

fn contains_value(bytes: &[u8]) -> bool {
    [VALUE, SECOND]
        .iter()
        .any(|value| bytes.windows(value.len()).any(|window| window == *value))
}

fn never_written(path: &Path) -> TestResult {
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            never_written(&path)?;
        } else {
            assert!(
                !contains_value(&std::fs::read(&path)?),
                "credential written to {}",
                path.display()
            );
        }
    }
    Ok(())
}

#[test]
fn rotation_account_value_reaches_only_the_child_environment() -> TestResult {
    let mut broker = LocalBroker::open()?;
    let rotation = broker.rotation()?;
    let environment = BTreeMap::new();
    for variable in ["CLAUDE_CODE_OAUTH_TOKEN", "CODEX_API_KEY", "OPENAI_API_KEY"] {
        let args = digest_arguments(variable);
        let spawn = Spawn {
            program: "/bin/sh",
            arguments: &args,
            directory: "/",
            environment: &environment,
            columns: 80,
            rows: 24,
        };
        let child = Sessions::spawn_account(&spawn, &rotation, variable, |handle, place| {
            broker.use_account(handle, place)
        })?;
        assert!(output(child)?.starts_with(&format!("{:x}", Sha256::digest(VALUE))));
        assert!(environment.is_empty());
    }
    Ok(())
}

#[test]
fn rotation_account_successor_is_resolved_after_usage_limit_words() -> TestResult {
    let mut broker = LocalBroker::open()?;
    let mut rotation = broker.rotation()?;
    let mut selected = Vec::new();
    let environment = BTreeMap::new();
    let args = digest_arguments("CLAUDE_CODE_OAUTH_TOKEN");
    let spawn = Spawn {
        program: "/bin/sh",
        arguments: &args,
        directory: "/",
        environment: &environment,
        columns: 80,
        rows: 24,
    };
    for value in [VALUE, SECOND] {
        let child = Sessions::spawn_account(
            &spawn,
            &rotation,
            "CLAUDE_CODE_OAUTH_TOKEN",
            |handle, place| {
                selected.push(handle.to_owned());
                broker.use_account(handle, place)
            },
        )?;
        assert!(output(child)?.starts_with(&format!("{:x}", Sha256::digest(value))));
        assert!(rotation.words_in("usage limit"));
        rotation.trip();
        rotation.advance(1);
    }
    assert_eq!(
        selected,
        broker
            .issued
            .iter()
            .map(|h| h.id.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        rotation.resume_arguments(),
        Some(["--resume".to_owned(), "conversation".to_owned()].as_slice())
    );
    assert!(
        rotation
            .moves()
            .last()
            .is_some_and(|movement| movement.to.is_empty())
    );
    Ok(())
}

#[test]
fn rotation_account_value_never_reaches_rendered_files_or_captured_logs() -> TestResult {
    let mut broker = LocalBroker::open()?;
    let rotation = broker.rotation()?;
    let captured = Arc::new(Mutex::new(Vec::new()));
    let logs = Arc::clone(&captured);
    lys_runner::error::also_to(Box::new(move |line| {
        logs.lock()
            .expect("fixture log lock poisoned")
            .push(line.to_owned());
    }))?;
    let mut template: serde_json::Value = serde_json::from_str(include_str!(
        "../../lys-home/tests/fixtures/launch/template.json"
    ))?;
    template["flags"] = serde_json::json!(["--strict-mcp-config", "--setting-sources="]);
    template["slots"]["secrets"]["use_only"] = serde_json::json!([
        {"env":"LYS_HANDLE_ACCOUNT", "handle":rotation.account()}
    ]);
    let template = serde_json::to_string(&template)?;
    let rendered = lys_home::harness::rendering_launch::render(
        "claude-code/template-v1",
        "/bin/sh",
        &template,
        lys_home::harness::launch_fields::InstructionsMode::Append,
    )?;
    let before = format!("{rendered:?}");
    for file in &rendered.files {
        std::fs::write(broker.dir.path().join(&file.path), &file.text)?;
    }
    let args = digest_arguments("CLAUDE_CODE_OAUTH_TOKEN");
    let spawn = Spawn {
        program: "/bin/sh",
        arguments: &args,
        directory: "/",
        environment: &rendered.environment,
        columns: 80,
        rows: 24,
    };
    let child = Sessions::spawn_account(
        &spawn,
        &rotation,
        "CLAUDE_CODE_OAUTH_TOKEN",
        |handle, place| broker.use_account(handle, place),
    )?;
    assert!(output(child)?.starts_with(&format!("{:x}", Sha256::digest(VALUE))));
    assert_eq!(before, format!("{rendered:?}"));
    assert!(!contains_value(before.as_bytes()));
    assert!(!contains_value(template.as_bytes()));
    never_written(broker.dir.path())?;
    for line in captured.lock().map_err(|error| error.to_string())?.iter() {
        assert!(!contains_value(line.as_bytes()), "credential reached log");
    }
    Ok(())
}

#[test]
fn rotation_account_unheld_is_refused_before_spawn() -> TestResult {
    let mut broker = LocalBroker::open()?;
    let rotation = RotationState::new(serde_json::from_str::<Rotation>(
        r#"{"accounts":["unheld"],"variable":"ACCOUNT","limit":{"signal":"words","words":["limit"]}}"#,
    )?)?;
    let environment = BTreeMap::new();
    let args = vec!["-c".to_owned(), "exit 99".to_owned()];
    let spawn = Spawn {
        program: "/bin/sh",
        arguments: &args,
        directory: "/",
        environment: &environment,
        columns: 80,
        rows: 24,
    };
    let error = match Sessions::spawn_account(
        &spawn,
        &rotation,
        "CLAUDE_CODE_OAUTH_TOKEN",
        |handle, place| broker.use_account(handle, place),
    ) {
        Ok(child) => {
            output(child)?;
            return Err("unheld account spawned".into());
        }
        Err(error) => error,
    };
    assert_eq!(error.name(), "account_handle_unheld");
    Ok(())
}

#[test]
fn rotation_account_broker_errors_never_quote_the_value() -> TestResult {
    let broker = LocalBroker::open()?;
    let rotation = broker.rotation()?;
    let environment = BTreeMap::new();
    let args = vec!["-c".to_owned(), "exit 99".to_owned()];
    let spawn = Spawn {
        program: "/bin/sh",
        arguments: &args,
        directory: "/",
        environment: &environment,
        columns: 80,
        rows: 24,
    };
    let error = match Sessions::spawn_account(
        &spawn,
        &rotation,
        "CLAUDE_CODE_OAUTH_TOKEN",
        |_handle, place| {
            place(VALUE)?;
            Err(RunnerError::refused(
                "broker_read_failed",
                String::from_utf8(VALUE.to_vec())
                    .map_err(|error| RunnerError::refused("fixture_invalid", error.to_string()))?,
            ))
        },
    ) {
        Ok(child) => {
            output(child)?;
            return Err("refused account spawned".into());
        }
        Err(error) => error,
    };
    assert_eq!(error.name(), "broker_read_failed");
    assert!(
        !contains_value(error.to_string().as_bytes()),
        "broker refusal exposes credential"
    );
    Ok(())
}

#[test]
fn rotation_account_readable_secret_remains_refused() -> TestResult {
    let mut template: serde_json::Value = serde_json::from_str(include_str!(
        "../../lys-home/tests/fixtures/launch/template.json"
    ))?;
    template["slots"]["secrets"]["readable"] = serde_json::json!(["account"]);
    let error = lys_home::harness::rendering_launch::render(
        "claude-code/template-v1",
        "/bin/sh",
        &serde_json::to_string(&template)?,
        lys_home::harness::launch_fields::InstructionsMode::Append,
    )
    .expect_err("readable account secret admitted");
    assert!(error.contains("readable"));
    Ok(())
}
