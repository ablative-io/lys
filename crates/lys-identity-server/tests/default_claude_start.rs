//! Claude startup requires an explicit native confinement choice.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use identity_contract::apps::{Auth, login, ok, op, post};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service, StageTimer};
use lys_core::Ed25519Identity;
use lys_home::harness::rendering_launch::Launch;
use lys_identity::{Actor, AuthMethod, LoginBinding, OperationId, Profile, Provenance};
use lys_identity_server::launch_template::from_template;
use lys_identity_server::provisioning_store::ProvisioningStore;
use lys_identity_server::routes::open_directory;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Act, Answer, Client, Options, Runner, Serving};
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    dir: tempfile::TempDir,
    key: Arc<Ed25519Identity>,
    serving: Serving,
    broker: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

impl Table {
    async fn open() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = tokio::spawn(async move {
            axum::serve(
                listener,
                axum::Router::new().fallback(|| async { axum::Json(json!({"handles": []})) }),
            )
            .await
        });
        let key_file = dir.path().join("broker.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let secrets = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let socket = dir.path().join("runner.sock");
        let adjusted = socket.clone();
        let state = dir.path().join("runner-state");
        let (service, (serving, key)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            Some(secrets),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
                config.requests_dir = None;
                config.certificates_dir = None;
                config.roles_file = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            move |config| {
                open_directory(config)?.setup_person(
                    Actor::new(
                        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                        Provenance::new(AuthMethod::Oidc, 1),
                    ),
                    OperationId::generate()?,
                    Profile::new("Owner")?,
                    1,
                )?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 1 << 16,
                })?;
                Ok((runner.spawn(), key))
            },
        )
        .await?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        let program = dir.path().join("claude");
        std::fs::write(
            &program,
            "#!/bin/sh\nset -e\ncat \"$CLAUDE_CONFIG_DIR/settings.json\"\nprintf '\\nprofile-read\\n'\nexec cat\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        Ok(Self {
            service,
            cookie,
            dir,
            key,
            serving,
            broker,
        })
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        ok(post(&self.service, path, Auth::Cookie(&self.cookie), body).await?)
    }

    async fn close(self) -> Outcome {
        let stopped = self.serving.stop();
        self.broker.abort();
        match self.broker.await {
            Err(error) if error.is_cancelled() => {}
            Err(error) => return Err(error.into()),
            Ok(result) => result?,
        }
        stopped?;
        Ok(())
    }
}

struct Evidence {
    status: u16,
    answer: Value,
    launch: Option<Launch>,
    live: Value,
}

async fn walk(table: &Table, mode: Option<&str>) -> Result<Evidence, Box<dyn Error>> {
    let registered = table
        .post(
            "/agents",
            &json!({"operation": op()?, "display_name": "New"}),
        )
        .await?;
    let agent = registered["agent"]
        .as_str()
        .ok_or("registration has no agent")?;
    table
        .post(
            &format!("/identities/{agent}/transitions"),
            &json!({
                "operation": op()?, "transition": "activate", "reason": "Start this agent"
            }),
        )
        .await?;
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let mut profile = json!({
        "operation": op()?, "from_version": 0,
        "model_access": ["default"], "tools": [], "skills": [], "mcp_servers": [],
        "instructions": "", "instructions_mode": "keep", "note": "Start this agent",
        "harness": {"name": "Claude Code", "description": catalogue["description"],
                    "program": table.dir.path().join("claude"), "package": "2.1.287"}
    });
    if let Some(mode) = mode {
        profile["permissions"] = json!({"default_mode": mode});
    }
    table
        .post(&format!("/agents/{agent}/provisioning"), &profile)
        .await?;
    table
        .post(
            &format!("/agents/{agent}/provisioning/1/review"),
            &json!({"operation": op()?}),
        )
        .await?;
    let machine = op()?;
    table.post("/network/machines", &json!({
        "operation": machine, "name": "This computer", "kind": "Computer", "runtime": "lys-runner",
        "slots": 0, "may_run": [agent], "may_run_roles": [], "may_reach": []
    })).await?;
    table
        .post(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner": {"kind": "lys"}}),
        )
        .await?;
    let (status, answer) = post(
        &table.service,
        &format!("/agents/{agent}/start-command"),
        Auth::Cookie(&table.cookie),
        &json!({"machine": machine, "operation": op()?}),
    )
    .await?;
    let launch = if mode.is_some() && status == 200 {
        let store = ProvisioningStore::open(&table.service.dir.path().join("provisioning.json"))?;
        let version = store
            .profile(agent)
            .and_then(|profile| profile.versions.last())
            .ok_or("no profile")?;
        let launch = from_template(
            version,
            answer["template"].as_str().ok_or("no template")?,
            answer["template_sha256"].as_str().ok_or("no hash")?,
        )?;
        let session = answer["session"].as_str().ok_or("no session")?;
        let client = Client::new(table.dir.path().join("runner.sock"), Arc::clone(&table.key));
        match client.ask(&Act::Wait {
            session: session.to_owned(),
            cursor: Some(0),
            pattern: "profile-read".to_owned(),
            regex: false,
        })? {
            Answer::Matched { .. } => {}
            answer => {
                return Err(format!("fixture did not read native settings: {answer:?}").into());
            }
        }
        Some(launch)
    } else {
        None
    };
    let (live_status, live) = table
        .service
        .get("/runtime/live", Some(&table.cookie))
        .await?;
    if live_status != 200 {
        return Err(format!("runtime live answered {live_status}: {live}").into());
    }
    Ok(Evidence {
        status,
        answer,
        launch,
        live,
    })
}

async fn evidence(mode: Option<&str>) -> Result<Evidence, Box<dyn Error>> {
    let stage = StageTimer::new("scenario.table_open");
    let table = Table::open().await?;
    drop(stage);
    let stage = StageTimer::new("scenario.driver");
    let result = walk(&table, mode).await.map_err(|error| error.to_string());
    drop(stage);
    let stage = StageTimer::new("scenario.table_close");
    let close = table.close().await;
    drop(stage);
    match (result, close) {
        (Ok(evidence), Ok(())) => Ok(evidence),
        (Err(error), Ok(())) => Err(error.into()),
        (Ok(_), Err(error)) => Err(error),
        (Err(walk), Err(close)) => {
            Err(format!("start failed: {walk}; cleanup failed: {close}").into())
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claude_form_workspace_defaults_start_a_real_runner_with_native_settings() -> Outcome {
    let timing = StageTimer::new("scenario.total");
    let evidence = evidence(Some("workspace-only")).await?;
    assert_eq!(evidence.status, 200, "{}", evidence.answer);
    assert_eq!(evidence.answer["runner"]["state"], "running");
    let launch = evidence.launch.ok_or("no launch")?;
    assert!(
        launch
            .arguments
            .windows(2)
            .any(|pair| pair == ["--model", "default"])
    );
    let settings = launch
        .files
        .iter()
        .find(|file| file.path == "settings.json")
        .ok_or("no settings")?;
    let native: Value = serde_json::from_str(&settings.text)?;
    assert_eq!(native["permissions"]["defaultMode"], "dontAsk");
    assert_eq!(
        native["permissions"]["allow"],
        json!(["Read(./**)", "Edit(./**)"])
    );
    assert_eq!(
        native["permissions"]["deny"],
        json!(["WebFetch", "WebSearch"])
    );
    assert_eq!(
        native["permissions"]["blockReadsOutsideWorkingDirectories"],
        true
    );
    assert_eq!(native["sandbox"]["enabled"], true);
    assert_eq!(native["sandbox"]["failIfUnavailable"], true);
    assert_eq!(native["sandbox"]["allowUnsandboxedCommands"], false);
    assert_eq!(native["sandbox"]["excludedCommands"], json!([]));
    assert_eq!(native["sandbox"]["filesystem"]["allowWrite"], json!([]));
    assert_eq!(native["sandbox"]["network"]["allowedDomains"], json!([]));
    assert_eq!(native["sandbox"]["network"]["strictAllowlist"], true);
    assert_eq!(native["sandbox"]["network"]["allowLocalBinding"], false);
    assert_eq!(native["sandbox"]["network"]["allowAllUnixSockets"], false);
    drop(timing);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claude_without_a_confinement_choice_refuses_before_render_or_start() -> Outcome {
    let timing = StageTimer::new("scenario.total");
    let evidence = evidence(None).await?;
    assert_eq!(evidence.status, 400, "{}", evidence.answer);
    assert_eq!(evidence.answer["refusal"], "PolicyUnrepresentable");
    assert!(
        evidence.answer["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("choose how this agent is confined"))
    );
    assert!(evidence.answer.get("template").is_none());
    assert!(evidence.answer.get("runner").is_none());
    assert_eq!(evidence.live["sessions"], json!([]));
    drop(timing);
    Ok(())
}
