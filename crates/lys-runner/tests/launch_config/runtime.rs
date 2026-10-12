//! Generated runtime configuration carries only its admitted inputs.

use super::*;

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
        requires_controls: false,
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
            tracking: None,
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
        tracking: None,
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
        tracking: None,
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
        tracking: None,
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
            tracking: None,
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

#[test]
fn old_config_stays_manual_and_control_requirement_is_typed() -> TestResult {
    let old = serde_json::json!({"files":[],"argument_files":{},"environment_paths":{},"working_directory":false});
    let config: Config = serde_json::from_value(old.clone())?;
    assert_eq!(serde_json::to_value(config)?, old);
    let mut required = old;
    required["requires_controls"] = serde_json::json!(true);
    let config: Config = serde_json::from_value(required.clone())?;
    assert_eq!(serde_json::to_value(config)?, required);
    required["requires_controls"] = serde_json::json!("true");
    assert!(serde_json::from_value::<Config>(required).is_err());
    Ok(())
}

/// A run outside the proxy, as every run on a plan is, is still handed the
/// runner's program and socket: Claude Code reports a plan's windows to its
/// status line and nowhere else, and that line's command reads the two
/// variables (`lys_home::harness::rendering::STATUS_LINE_COMMAND`).
#[test]
fn a_run_outside_the_proxy_is_handed_the_status_lines_program_and_socket() -> TestResult {
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
    let client = Client::new(socket.clone(), key);
    let mut asked = launch();
    asked.session = "plan-session".to_owned();
    asked.config = None;
    asked.directory = dir
        .path()
        .to_str()
        .ok_or("fixture path is not text")?
        .to_owned();
    asked.columns = 500;
    asked.arguments = vec![
        "-c".to_owned(),
        "printf 'program=%s socket=%s;\\n' \"${LYS_PROGRAM:+named}\" \"$LYS_RUNNER_SOCKET\""
            .to_owned(),
    ];
    let Answer::Started { .. } = client.ask(&Act::Start {
        tracking: None,
        lys_mcp: None,
        proxy: None,
        launch: Box::new(asked),
    })?
    else {
        return Err("the runner did not start the declared process".into());
    };
    let expected = format!(
        "program=named socket={};",
        socket.to_str().ok_or("the socket path is not text")?
    );
    let Answer::Matched { .. } = client.ask(&Act::Wait {
        session: "plan-session".to_owned(),
        cursor: Some(0),
        pattern: expected,
        regex: false,
    })?
    else {
        return Err("the run was not handed the runner's program and socket".into());
    };
    serving.stop()?;
    Ok(())
}
