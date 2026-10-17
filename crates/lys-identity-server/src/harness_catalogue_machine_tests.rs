//! Catalogue and profile acceptance use one chosen machine's signed runner replies.

use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_runner::{Act, Answer};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn a_chosen_computers_catalogue_and_profile_use_only_its_reported_programmes()
-> Result<(), Box<dyn std::error::Error>> {
    profile_on_machine("/Users/tom/.local/share/claude/versions/2.1.288/claude").await
}

#[tokio::test]
async fn an_absolute_fork_is_saved_on_a_computer_that_does_not_report_it()
-> Result<(), Box<dyn std::error::Error>> {
    profile_on_machine("/opt/forks/claude-fork").await
}

#[tokio::test]
async fn a_command_profile_accepts_the_absolute_program_reported_by_its_machine()
-> Result<(), Box<dyn std::error::Error>> {
    profile_on_machine("claude").await
}

async fn profile_on_machine(program: &str) -> Result<(), Box<dyn std::error::Error>> {
    let absolute = program == "claude";
    let (service, (seeded, machine, unreadable, stop, worker)) =
        Service::start_with(move |config| {
            let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "member"],
            )?;
            let machine = lys_identity::OperationId::generate()?.to_string();
            let socket = config.log_dir.with_file_name("programs.sock");
            let listener = tokio::net::UnixListener::bind(&socket)?;
            let key = lys_core::Ed25519Identity::load(&config.event_key_file)?.public_key_bytes();
            let mut network = crate::network_store::NetworkStore::open(
                config.network_file.as_deref().ok_or("network absent")?,
            )?;
            network.name(crate::network_store::Machine {
                id: machine.clone(),
                name: "Chosen computer".to_owned(),
                kind: "Computer".to_owned(),
                runtime: Some("lys-runner".to_owned()),
                team: None,
                creation_team: None,
                slots: 1,
                may_run: Vec::new(),
                may_reach: Vec::new(),
                may_run_roles: Vec::new(),
                named_by: seeded.people[0].id.to_string(),
                named_at: 1,
                retired: None,
            })?;
            network.name_runner(
                &machine,
                Some(crate::runner_client::RunnerRecord::Socket {
                    path: socket.to_string_lossy().into_owned(),
                }),
            )?;
            let (unreadable, discovery_refused) = tokio::sync::watch::channel(false);
            let (stop, mut stopped) = tokio::sync::oneshot::channel();
            let worker = tokio::spawn(async move {
                serve_programs(&listener, &key, &discovery_refused, &mut stopped, absolute).await
            });
            Ok((seeded, machine, unreadable, stop, worker))
        })
        .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let (status, catalogue) = service
        .get(&format!("/harnesses?machine={machine}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200);
    let builds = catalogue["programs"]
        .as_array()
        .ok_or("programmes absent")?
        .iter()
        .flat_map(|program| program["builds"].as_array().into_iter().flatten())
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 2);
    assert!(builds.iter().all(|build| {
        build["program"].as_str().is_some_and(|command| {
            command == "claude"
                || command == "codex"
                || (absolute && command == "/installed/bin/claude")
        })
    }));
    assert!(builds.iter().any(|build| {
        build["package"]
            .as_str()
            .is_some_and(|package| package.contains("2.1.300"))
    }));
    let agent = seeded.people[0].agents[0].id.to_string();
    assert_profiles(&service, &machine, &cookie, &agent, program, &unreadable).await?;
    stop.send(()).map_err(|()| "runner ended")?;
    worker.await??;
    Ok(())
}

#[tokio::test]
async fn a_disconnected_machine_returns_words_and_no_service_host_programmes()
-> Result<(), Box<dyn std::error::Error>> {
    let table = crate::budgets_feed::entry_tests::fixture().await?;
    let answer =
        super::machine_view(&table.state, &super::Catalogue::embedded()?, "disconnected").await?;
    assert!(!answer.programs.is_empty());
    assert!(
        answer
            .programs
            .iter()
            .all(|program| program.builds.is_empty())
    );
    assert!(
        answer
            .reason
            .as_deref()
            .is_some_and(|words| words.contains("runner_unreachable"))
    );
    table.finish().await
}

async fn assert_invalid_programs(
    service: &Service,
    route: &str,
    cookie: &str,
    body: &mut serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, invalid) in [
        ("empty", ""),
        ("spaces", "two words"),
        ("control", "claude\n"),
        ("delete control", "claude\u{7f}"),
        ("c1 control", "claude\u{9f}"),
        ("equals", "claude=x"),
        ("parent", ".."),
        ("current", "."),
        ("leading dash", "-rf"),
        ("backslash", "C:\\bin\\claude"),
        ("relative", "bin/claude"),
        ("parent component", "/bin/../claude"),
        ("trailing slash", "/bin/claude/"),
    ] {
        body["operation"] = json!(lys_identity::OperationId::generate()?.to_string());
        body["harness"]["program"] = json!(invalid);
        let (status, refusal) = service.post(route, Some(cookie), body).await?;
        assert_eq!(status, 400, "{name}: {refusal}");
        assert_eq!(refusal["refusal"], "RequestMalformed", "{name}: {refusal}");
        assert_eq!(
            refusal["reason"],
            format!(
                "RequestMalformed: {}",
                lys_home::harness::rendering_launch::PROGRAM_RULE
            )
        );
    }
    Ok(())
}

async fn serve_programs(
    listener: &tokio::net::UnixListener,
    key: &[u8; 32],
    discovery_refused: &tokio::sync::watch::Receiver<bool>,
    stopped: &mut tokio::sync::oneshot::Receiver<()>,
    absolute: bool,
) -> std::io::Result<()> {
    loop {
        let (stream, _) = tokio::select! { accepted = listener.accept() => accepted?, result = &mut *stopped => { result.map_err(std::io::Error::other)?; return Ok::<_, std::io::Error>(()); } };
        let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
        let mut stream = BufReader::new(stream);
        stream
            .get_mut()
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await?;
        let mut line = String::new();
        stream.read_line(&mut line).await?;
        let act = lys_runner::protocol::verify_request(&line, key, &greeting)
            .map_err(std::io::Error::other)?;
        let answer = if act == Act::Programs && *discovery_refused.borrow() {
            Answer::refusal(&lys_runner::RunnerError::refused(
                "harness_programs_unreadable",
                "the chosen programme version report is not UTF-8",
            ))
        } else if act == Act::Programs {
            Answer::Programs {
                found: lys_runner::dial::programs::Programs {
                    programs: vec![
                        lys_runner::dial::programs::HarnessProgram {
                            command: Some("claude".to_owned()),
                            program: if absolute {
                                "/installed/bin/claude"
                            } else {
                                "claude"
                            }
                            .to_owned(),
                            version: "2.1.300".to_owned(),
                            adapter: "claude-stream-json/1".to_owned(),
                        },
                        lys_runner::dial::programs::HarnessProgram {
                            command: Some("codex".to_owned()),
                            program: "codex".to_owned(),
                            version: "0.161.0-alpha.3".to_owned(),
                            adapter: "codex-app-server/1".to_owned(),
                        },
                    ],
                    reason: None,
                },
            }
        } else {
            Answer::refusal(&lys_runner::RunnerError::refused(
                "fixture_standing_unavailable",
                "this fixture answers programme discovery only",
            ))
        };
        stream
            .get_mut()
            .write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes())
            .await?;
    }
}

async fn assert_profiles(
    service: &Service,
    machine: &str,
    cookie: &str,
    agent: &str,
    program: &str,
    unreadable: &tokio::sync::watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let description: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let mut body = json!({"operation":lys_identity::OperationId::generate()?.to_string(),"from_version":0,"model_access":["default"],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"", "runs_on":machine,
        "harness":{"name":"Claude Code", "program":"missing", "package":"claude 2.1.300", "description":description["description"]}});
    let route = format!("/agents/{agent}/provisioning");
    assert_invalid_programs(service, &route, cookie, &mut body).await?;
    body["operation"] = json!(lys_identity::OperationId::generate()?.to_string());
    body["harness"]["program"] = json!("missing");
    let (status, refusal) = service.post(&route, Some(cookie), &body).await?;
    assert_eq!(status, 409);
    assert_eq!(refusal["refusal"], "harness_program_unknown");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains("claude version 2.1.300")
                && words.contains("codex version 0.161.0-alpha.3"))
    );
    body["harness"]["program"] = json!("claude");
    body["model_access"] = json!([description["models"][0]["id"]]);
    unreadable.send(true)?;
    let (status, refused) = service.post(&route, Some(cookie), &body).await?;
    assert_eq!(status, 409);
    assert_eq!(refused["refusal"], "harness_programs_unreadable");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|words| words.contains("not UTF-8"))
    );
    unreadable.send(false)?;
    body["harness"]["program"] = json!(program);
    let (status, profile) = service.post(&route, Some(cookie), &body).await?;
    assert_eq!(status, 200, "profile save for {program}: {profile}");
    assert_eq!(
        profile["profile"]["harness"]["program"],
        body["harness"]["program"]
    );
    unreadable.send(true)?;
    for (index, program) in [
        "/opt/forks/claude-fork",
        "/missing/bin/claude-fork",
        "/opt/forks/Claude fork",
    ]
    .iter()
    .enumerate()
    {
        body["operation"] = json!(lys_identity::OperationId::generate()?.to_string());
        body["from_version"] = json!(index + 1);
        body["harness"]["program"] = json!(program);
        let (status, profile) = service.post(&route, Some(cookie), &body).await?;
        assert_eq!(status, 200, "absolute profile save: {profile}");
        assert_eq!(profile["profile"]["harness"]["program"], *program);
    }
    Ok(())
}
