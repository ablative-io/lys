#![cfg(test)]
//! Signed config files are validated before publication and bound to one session.

use lys_core::Ed25519Identity;
use lys_runner::launch_config::{Config, File};
use lys_runner::protocol::{Greeting, Request, hex, sign_request, verify_request};
use lys_runner::session::Sessions;
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, RunnerError};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::symlink;

type TestResult = Result<(), Box<dyn Error>>;

fn launch() -> Launch {
    Launch {
        session: "config-session".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec![
            "-c".to_owned(),
            "cat \"$CONFIG_FILE\"; printf '\\nread-done\\n'".to_owned(),
        ],
        directory: String::new(),
        environment: BTreeMap::new(),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
        config: Some(Config {
            files: vec![File {
                path: "settings.json".to_owned(),
                text: "exact settings".to_owned(),
                sha256: hex(&Sha256::digest(b"exact settings")),
            }],
            argument_files: BTreeMap::new(),
            environment_paths: BTreeMap::from([(
                "CONFIG_FILE".to_owned(),
                "settings.json".to_owned(),
            )]),
            working_directory: true,
        }),
    }
}

#[test]
fn signed_config_is_written_under_the_session_and_read_by_its_process() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let socket = dir.path().join("runner.sock");
    let serving = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.path().to_owned(),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    })?
    .spawn();
    let client = Client::new(socket, key);
    let Answer::Started { .. } = client.ask(&Act::Start {
        launch: Box::new(launch()),
    })?
    else {
        return Err("the runner did not start the declared process".into());
    };
    let Answer::Matched { .. } = client.ask(&Act::Wait {
        session: "config-session".to_owned(),
        cursor: Some(0),
        pattern: "exact settings".to_owned(),
        regex: false,
    })?
    else {
        return Err("the process did not read its signed config".into());
    };
    serving.stop()?;
    let path = dir
        .path()
        .join("sessions/config-session/config/settings.json");
    assert_eq!(std::fs::read(path)?, b"exact settings");
    Ok(())
}

#[test]
fn invalid_contents_are_refused_before_any_config_is_written() -> TestResult {
    for change in [0, 1, 2, 3] {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 1024)?;
        let mut asked = launch();
        let config = asked.config.as_mut().ok_or("no config")?;
        match change {
            0 => config.files[0].path = "../outside".to_owned(),
            1 => config.files[0].sha256 = "wrong".to_owned(),
            2 => config.files.push(config.files[0].clone()),
            3 => {
                config.argument_files.insert(99, "settings.json".to_owned());
            }
            _ => return Err("unexpected fixture".into()),
        }
        let error = sessions
            .start(asked)
            .err()
            .ok_or("invalid config was accepted")?;
        assert_eq!(error.name(), "launch_config_refused");
        assert!(!dir.path().join("sessions").exists());
        assert!(sessions.status(None)?.sessions.is_empty());
        sessions.stop_all()?;
    }
    Ok(())
}

#[test]
fn a_config_is_never_written_through_a_symlink() -> TestResult {
    let dir = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    symlink(outside.path(), dir.path().join("sessions"))?;
    let error = sessions
        .start(launch())
        .err()
        .ok_or("symlink was accepted")?;
    assert_eq!(error.name(), "launch_config_refused");
    assert_eq!(std::fs::read_dir(outside.path())?.count(), 0);
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn a_failed_spawn_can_retry_only_identical_kept_config() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let mut refused = launch();
    refused.program = "/no-such-program".to_owned();
    assert!(sessions.start(refused).is_err());
    let mut changed = launch();
    let file = &mut changed.config.as_mut().ok_or("no config")?.files[0];
    file.text = "changed".to_owned();
    file.sha256 = hex(&Sha256::digest(b"changed"));
    assert_eq!(
        sessions
            .start(changed)
            .err()
            .ok_or("changed config accepted")?
            .name(),
        "launch_config_refused"
    );
    sessions.start(launch())?;
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn an_interrupted_staging_directory_is_replaced_before_publication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let own = dir.path().join("sessions/config-session");
    let staged = own.join("config-writing");
    std::fs::create_dir_all(staged.join("partial"))?;
    std::fs::write(staged.join("settings.json"), b"incomplete settings")?;
    std::fs::write(staged.join("partial/unfinished"), b"unpublished")?;
    sessions.start(launch())?;
    sessions.stop_all()?;
    assert_eq!(
        std::fs::read(own.join("config/settings.json"))?,
        b"exact settings"
    );
    assert!(!staged.exists());
    assert!(!own.join("config/partial").exists());
    Ok(())
}

#[test]
fn an_unpublished_symlink_is_refused_without_removing_its_target() -> TestResult {
    let dir = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let own = dir.path().join("sessions/config-session");
    std::fs::create_dir_all(&own)?;
    std::fs::write(outside.path().join("retained"), b"outside contents")?;
    symlink(outside.path(), own.join("config-writing"))?;
    let error = sessions
        .start(launch())
        .err()
        .ok_or("unpublished symlink was accepted")?;
    sessions.stop_all()?;
    assert_eq!(error.name(), "launch_config_refused");
    assert_eq!(
        std::fs::read(outside.path().join("retained"))?,
        b"outside contents"
    );
    assert!(!own.join("config").exists());
    Ok(())
}

#[test]
fn the_signature_covers_config_files_and_bindings() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("11");
    let act = Act::Start {
        launch: Box::new(launch()),
    };
    let signed = sign_request(&key, &greeting, &act)?;
    assert_eq!(
        verify_request(&signed, &key.public_key_bytes(), &greeting)?,
        act
    );
    let mut request: Request = serde_json::from_str(&signed)?;
    let mut contents: serde_json::Value = serde_json::from_str(&request.act)?;
    contents["launch"]["config"]["files"][0]["text"] = serde_json::json!("different");
    request.act = serde_json::to_string(&contents)?;
    let error = verify_request(
        &serde_json::to_string(&request)?,
        &key.public_key_bytes(),
        &greeting,
    )
    .err()
    .ok_or("unsigned config change was accepted")?;
    assert_eq!(error.name(), "runner_request_unsigned");
    Ok(())
}

#[test]
fn a_signed_start_that_binds_arguments_to_files_reads_back_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("12");
    let mut bound = launch();
    let config = bound.config.as_mut().ok_or("the fixture carries config")?;
    config.argument_files = BTreeMap::from([
        (1, "settings.json".to_owned()),
        (10, "settings.json".to_owned()),
    ]);
    let act = Act::Start {
        launch: Box::new(bound),
    };
    let signed = sign_request(&key, &greeting, &act)?;
    let request: Request = serde_json::from_str(&signed)?;
    assert!(
        request
            .act
            .contains(r#""argument_files":{"1":"settings.json","10":"settings.json"}"#),
        "{}",
        request.act
    );
    assert_eq!(
        verify_request(&signed, &key.public_key_bytes(), &greeting)?,
        act
    );
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousLaunch {
    session: String,
    program: String,
    arguments: Vec<String>,
    directory: String,
    #[serde(default)]
    environment: BTreeMap<String, String>,
    columns: u16,
    rows: u16,
    #[serde(default)]
    rotation: Option<lys_runner::Rotation>,
    #[serde(default)]
    policy: Option<Box<lys_runner::admitted::Admitted>>,
}

#[test]
fn the_previous_launch_reader_refuses_config_by_name() -> TestResult {
    let current = serde_json::to_string(&launch())?;
    let error = serde_json::from_str::<PreviousLaunch>(&current)
        .err()
        .ok_or("previous reader accepted config")?;
    let named = RunnerError::Malformed {
        reason: format!("the act does not read: {error}"),
    };
    assert_eq!(named.name(), "runner_request_malformed");
    assert!(named.to_string().contains("unknown field `config`"));
    Ok(())
}

#[test]
fn a_start_request_keeps_the_existing_wire_bytes() -> TestResult {
    let launch = launch();
    let legacy = format!(
        "{{\"act\":\"start\",\"launch\":{}}}",
        serde_json::to_string(&launch)?
    );
    let act = Act::Start {
        launch: Box::new(launch),
    };
    assert_eq!(serde_json::to_string(&act)?, legacy);
    let decoded: Act = serde_json::from_str(&legacy)?;
    assert_eq!(decoded, act);
    assert_eq!(serde_json::to_string(&decoded)?, legacy);
    Ok(())
}
