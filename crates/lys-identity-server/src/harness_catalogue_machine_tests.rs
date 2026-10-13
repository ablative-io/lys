//! Catalogue and profile acceptance use one chosen machine's signed runner replies.

use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_runner::{Act, Answer};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn a_chosen_computers_catalogue_and_profile_use_only_its_reported_programmes()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, (seeded, machine, stop, worker)) = Service::start_with(|config| {
        let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(config, [ADMINISTRATOR, "member"])?;
        let machine = lys_identity::OperationId::generate()?.to_string();
        let socket = config.log_dir.with_file_name("programs.sock");
        let listener = tokio::net::UnixListener::bind(&socket)?;
        let key = lys_core::Ed25519Identity::load(&config.event_key_file)?.public_key_bytes();
        let mut network = crate::network_store::NetworkStore::open(config.network_file.as_deref().ok_or("network absent")?)?;
        network.name(crate::network_store::Machine { id: machine.clone(), name: "Chosen computer".to_owned(), kind: "Computer".to_owned(), runtime: Some("lys-runner".to_owned()),
            team: None, creation_team: None, slots: 1, may_run: Vec::new(), may_reach: Vec::new(), may_run_roles: Vec::new(), named_by: seeded.people[0].id.to_string(), named_at: 1, retired: None })?;
        network.name_runner(&machine, Some(crate::runner_client::RunnerRecord::Socket { path: socket.to_string_lossy().into_owned() }))?;
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            loop {
                let (stream, _) = tokio::select! { accepted = listener.accept() => accepted?, result = &mut stopped => { result.map_err(std::io::Error::other)?; return Ok::<_, std::io::Error>(()); } };
                let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                let mut stream = BufReader::new(stream);
                stream.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
                let mut line = String::new(); stream.read_line(&mut line).await?;
                let act = lys_runner::protocol::verify_request(&line, &key, &greeting).map_err(std::io::Error::other)?;
                let answer = if act == Act::Programs {
                    Answer::Programs { found: lys_runner::dial::programs::Programs { programs: vec![
                        lys_runner::dial::programs::HarnessProgram { program: "/chosen/bin/claude".to_owned(), version: "2.1.300".to_owned(), adapter: "claude-stream-json/1".to_owned() },
                        lys_runner::dial::programs::HarnessProgram { program: "/chosen/bin/codex".to_owned(), version: "0.161.0-alpha.3".to_owned(), adapter: "codex-app-server/1".to_owned() },
                    ], reason: None } }
                } else { Answer::refusal(&lys_runner::RunnerError::refused("fixture_standing_unavailable", "this fixture answers programme discovery only")) };
                stream.get_mut().write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes()).await?;
            }
        });
        Ok((seeded, machine, stop, worker))
    }).await?;
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
        build["program"]
            .as_str()
            .is_some_and(|path| path.starts_with("/chosen/bin/"))
    }));
    assert!(builds.iter().any(|build| {
        build["package"]
            .as_str()
            .is_some_and(|package| package.contains("2.1.300"))
    }));
    let description: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let mut body = json!({"operation":lys_identity::OperationId::generate()?.to_string(),"from_version":0,"model_access":["default"],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"", "runs_on":machine,
        "harness":{"name":"Claude Code", "program":"/chosen/bin/missing", "package":"claude 2.1.300", "description":description["description"]}});
    let route = format!("/agents/{agent}/provisioning");
    let (status, refusal) = service.post(&route, Some(&cookie), &body).await?;
    assert_eq!(status, 409);
    assert_eq!(refusal["refusal"], "harness_program_unknown");
    assert!(refusal["reason"].as_str().is_some_and(
        |words| words.contains("/chosen/bin/claude") && words.contains("/chosen/bin/codex")
    ));
    body["harness"]["program"] = json!("/chosen/bin/claude");
    body["model_access"] = json!([description["models"][0]["id"]]);
    let (status, profile) = service.post(&route, Some(&cookie), &body).await?;
    stop.send(()).map_err(|()| "runner ended")?;
    worker.await??;
    assert_eq!(status, 200);
    assert_eq!(
        profile["profile"]["harness"]["program"],
        "/chosen/bin/claude"
    );
    Ok(())
}

#[tokio::test]
async fn a_disconnected_machine_returns_words_and_no_service_host_programmes()
-> Result<(), Box<dyn std::error::Error>> {
    let table = crate::budgets_feed::entry_tests::fixture().await?;
    let answer =
        super::machine_view(&table.state, &super::Catalogue::embedded()?, "disconnected").await?;
    assert!(answer.programs.is_empty());
    assert!(
        answer
            .reason
            .as_deref()
            .is_some_and(|words| words.contains("runner_unreachable"))
    );
    table.finish().await
}
