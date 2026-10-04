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
            harness: None,
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
        lys_mcp: None,
        proxy: None,
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
        lys_mcp: None,
        proxy: None,
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
        lys_mcp: None,
        proxy: None,
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
        lys_mcp: None,
        proxy: None,
        launch: Box::new(launch),
    };
    assert_eq!(serde_json::to_string(&act)?, legacy);
    let decoded: Act = serde_json::from_str(&legacy)?;
    assert_eq!(decoded, act);
    assert_eq!(serde_json::to_string(&decoded)?, legacy);
    Ok(())
}

const RUN_PASS: &str = "fixture-p77-run-pass-only";

#[derive(PartialEq)]
struct Redacted<T>(T);

impl<T> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("config contents redacted")
    }
}

fn mcp_launch(codex: bool) -> Launch {
    let mut launch = launch();
    launch.arguments = vec!["-c".to_owned(), "printf p77-ready; exec cat".to_owned()];
    let (path, text) = if codex {
        ("instructions.txt", "kept instructions\n")
    } else {
        launch
            .arguments
            .extend(["--mcp-config".to_owned(), "mcp.json".to_owned()]);
        ("mcp.json", r#"{"mcpServers":{}}"#)
    };
    launch.config = Some(Config {
        files: vec![File {
            path: path.to_owned(),
            text: text.to_owned(),
            sha256: hex(&Sha256::digest(text.as_bytes())),
        }],
        argument_files: BTreeMap::new(),
        environment_paths: BTreeMap::new(),
        working_directory: true,
        harness: None,
    });
    launch
}

fn check_records(path: &std::path::Path, native: &std::path::Path) -> TestResult {
    for child in std::fs::read_dir(path)? {
        let child = child?;
        let path = child.path();
        if path == native {
            continue;
        }
        if child.file_type()?.is_dir() {
            check_records(&path, native)?;
        } else if child.file_type()?.is_file() {
            let bytes = std::fs::read(path)?;
            assert!(
                !bytes
                    .windows(RUN_PASS.len())
                    .any(|part| part == RUN_PASS.as_bytes())
            );
        }
    }
    Ok(())
}

#[test]
fn signed_run_pass_reaches_only_the_generated_config_for_both_harnesses() -> TestResult {
    for codex in [false, true] {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 1024)?;
        let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
        let greeting = Greeting::fresh("77");
        let act = Act::Start {
            launch: Box::new(mcp_launch(codex)),
            proxy: None,
            lys_mcp: Some(lys_runner::protocol::LysMcp {
                url: "https://service.invalid/api/mcp".to_owned(),
                pass: RUN_PASS.to_owned(),
                seat: None,
            }),
        };
        let signed = sign_request(&key, &greeting, &act)?;
        let request: Request = serde_json::from_str(&signed)?;
        assert!(!format!("{act:?} {request:?}").contains(RUN_PASS));
        let answer = lys_runner::socket::dispatch(
            &sessions,
            &key.public_key_bytes(),
            &greeting,
            &signed,
            &std::sync::atomic::AtomicBool::new(false),
        );
        assert!(matches!(answer, Answer::Started { .. }));
        assert!(!format!("{answer:?}").contains(RUN_PASS));
        sessions.stop_all()?;
        let config = dir.path().join("sessions/config-session/config");
        let native = config.join(if codex {
            "instructions.txt"
        } else {
            "mcp.json"
        });
        if codex {
            // Codex gets the pass as a command-line setting, never a file.
            assert!(!config.join("config.toml").exists());
        } else {
            let text = std::fs::read_to_string(config.join("mcp.json"))?;
            let config: serde_json::Value = serde_json::from_str(&text)?;
            assert_eq!(
                Redacted(&config["mcpServers"]["lys"]),
                Redacted(
                    &serde_json::json!({"type":"http","url":"https://service.invalid/api/mcp","headers":{"lys-agent-pass":RUN_PASS}})
                )
            );
        }
        check_records(dir.path(), &native)?;
    }
    Ok(())
}

#[test]
fn a_codex_launch_with_no_files_starts_with_its_pass_in_the_environment() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("81");
    let mut launch = mcp_launch(true);
    launch.config.as_mut().ok_or("no config")?.files.clear();
    launch.arguments = vec![
        "-c".to_owned(),
        "printf 'pass:%s\\n' \"$LYS_AGENT_PASS\"; exec cat".to_owned(),
    ];
    let act = Act::Start {
        launch: Box::new(launch),
        proxy: None,
        lys_mcp: Some(lys_runner::protocol::LysMcp {
            url: "https://service.invalid/api/mcp".to_owned(),
            pass: RUN_PASS.to_owned(),
            seat: None,
        }),
    };
    let answer = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &sign_request(&key, &greeting, &act)?,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(matches!(answer, Answer::Started { .. }), "{answer:?}");
    assert!(!format!("{answer:?}").contains(RUN_PASS));
    // The process read the pass from its environment: it is on its screen.
    let again = Greeting::fresh("83");
    let seen = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &again,
        &sign_request(
            &key,
            &again,
            &Act::Wait {
                session: "config-session".to_owned(),
                cursor: Some(0),
                pattern: format!("pass:{RUN_PASS}"),
                regex: false,
            },
        )?,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(matches!(seen, Answer::Matched { .. }), "{seen:?}");
    sessions.stop_all()?;
    let config = dir.path().join("sessions/config-session/config");
    assert!(std::fs::read_dir(&config)?.next().is_none());
    Ok(())
}

#[test]
fn a_launch_naming_a_relative_directory_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("82");
    let mut launch = mcp_launch(false);
    launch.directory = "relative/folder".to_owned();
    launch.config.as_mut().ok_or("no config")?.working_directory = false;
    let act = Act::Start {
        launch: Box::new(launch),
        lys_mcp: None,
        proxy: None,
    };
    let answer = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &sign_request(&key, &greeting, &act)?,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(!matches!(answer, Answer::Started { .. }), "{answer:?}");
    assert!(format!("{answer:?}").contains("not absolute"), "{answer:?}");
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn an_invalid_signed_original_config_is_refused_before_pass_insertion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 1024)?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("78");
    let mut launch = mcp_launch(false);
    launch.config.as_mut().ok_or("no config")?.files[0].sha256 = "wrong".to_owned();
    let act = Act::Start {
        launch: Box::new(launch),
        proxy: None,
        lys_mcp: Some(lys_runner::protocol::LysMcp {
            url: "https://service.invalid/api/mcp".to_owned(),
            pass: RUN_PASS.to_owned(),
            seat: None,
        }),
    };
    let answer = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &sign_request(&key, &greeting, &act)?,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(
        matches!(&answer, Answer::Refused { refusal, .. } if refusal == "launch_config_refused")
    );
    assert!(!format!("{answer:?}").contains(RUN_PASS));
    assert!(!dir.path().join("sessions").exists());
    sessions.stop_all()?;
    check_records(dir.path(), &dir.path().join("absent"))?;
    Ok(())
}

#[test]
fn malformed_signed_pass_payload_does_not_echo_secret_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let greeting = Greeting::fresh("79");
    let act = serde_json::json!({"act":"start","launch":mcp_launch(false),"lys_mcp":{"url":"https://service.invalid/api/mcp","pass":RUN_PASS,RUN_PASS:true}}).to_string();
    let signature = hex(&key.sign(&lys_runner::protocol::signed_bytes(
        1,
        &greeting.runner,
        &greeting.challenge,
        &act,
    )));
    let request = Request {
        version: 1,
        runner: greeting.runner.clone(),
        challenge: greeting.challenge.clone(),
        act,
        signature,
    };
    let error = lys_runner::protocol::verify_parsed(&request, &key.public_key_bytes(), &greeting)
        .err()
        .ok_or("unknown credential field accepted")?;
    assert_eq!(error.name(), "runner_request_malformed");
    assert!(!format!("{request:?} {error:?} {error}").contains(RUN_PASS));
    Ok(())
}

#[test]
fn signed_start_without_pass_leaves_both_native_configs_unchanged() -> TestResult {
    for codex in [false, true] {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 1024)?;
        let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
        let greeting = Greeting::fresh("80");
        let launch = mcp_launch(codex);
        let file = launch.config.as_ref().ok_or("no config")?.files[0].clone();
        let act = Act::Start {
            launch: Box::new(launch),
            lys_mcp: None,
            proxy: None,
        };
        let answer = lys_runner::socket::dispatch(
            &sessions,
            &key.public_key_bytes(),
            &greeting,
            &sign_request(&key, &greeting, &act)?,
            &std::sync::atomic::AtomicBool::new(false),
        );
        assert!(matches!(answer, Answer::Started { .. }));
        sessions.stop_all()?;
        let text = std::fs::read_to_string(
            dir.path()
                .join("sessions/config-session/config")
                .join(file.path),
        )?;
        assert_eq!(Redacted(text), Redacted(file.text));
    }
    Ok(())
}
