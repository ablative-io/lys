//! First-run defaults reach the runner through native workspace permissions.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use identity_contract::apps::{Auth, login, ok, op, post};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
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
        let program = dir.path().join("codex");
        std::fs::write(
            &program,
            "#!/bin/sh\nset -e\ncat \"$CODEX_HOME/config.toml\"\nprintf '\\nprofile-read\\n'\nexec cat\n",
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

async fn walk(table: &Table) -> Outcome {
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
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    table.post(&format!("/agents/{agent}/provisioning"), &json!({
        "operation": op()?, "from_version": 0,
        "model_access": ["gpt-6.1-sol"], "tools": [], "skills": [], "mcp_servers": [],
        "instructions": "", "instructions_mode": "keep", "note": "Start this agent",
        "harness": {"name": "Codex", "description": catalogue["description"],
                    "program": table.dir.path().join("codex"), "package": "codex-cli 0.145.0"},
        "permissions": {"default_mode": "workspace-write"}
    })).await?;
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
    let started = table
        .post(
            &format!("/agents/{agent}/start-command"),
            &json!({"machine": machine, "operation": op()?}),
        )
        .await?;
    assert_eq!(started["runner"]["state"], "running", "{started}");
    let store = ProvisioningStore::open(&table.service.dir.path().join("provisioning.json"))?;
    let version = store
        .profile(agent)
        .and_then(|profile| profile.versions.last())
        .ok_or("no recorded profile")?;
    let launch = from_template(
        version,
        started["template"].as_str().ok_or("no template")?,
        started["template_sha256"].as_str().ok_or("no hash")?,
    )?;
    assert!(
        launch
            .arguments
            .windows(2)
            .any(|pair| pair == ["--sandbox", "workspace-write"])
    );
    assert!(
        launch
            .arguments
            .windows(2)
            .any(|pair| pair == ["--model", "gpt-6.1-sol"])
    );
    let config = launch
        .files
        .iter()
        .find(|file| file.path == "config.toml")
        .ok_or("no config")?;
    let native: toml::Table = config.text.parse()?;
    assert_eq!(
        native["sandbox_workspace_write"]["network_access"].as_bool(),
        Some(false)
    );
    assert_eq!(
        native["sandbox_workspace_write"]["exclude_tmpdir_env_var"].as_bool(),
        Some(true)
    );
    assert_eq!(
        native["sandbox_workspace_write"]["exclude_slash_tmp"].as_bool(),
        Some(true)
    );
    assert_eq!(native["web_search"].as_str(), Some("disabled"));
    let session = started["session"].as_str().ok_or("no session")?;
    let client = Client::new(table.dir.path().join("runner.sock"), Arc::clone(&table.key));
    assert!(matches!(
        client.ask(&Act::Wait {
            session: session.to_owned(),
            cursor: Some(0),
            pattern: "profile-read".to_owned(),
            regex: false
        })?,
        Answer::Matched { .. }
    ));
    table.post(&format!("/agents/{agent}/policy"), &json!({"version": 1, "rules": [{
        "id": "owner-deny", "tool": "Read", "kind": "Tool", "target": null, "authority": "Hard"
    }]})).await?;
    let (status, refused) = post(
        &table.service,
        &format!("/agents/{agent}/start-command"),
        Auth::Cookie(&table.cookie),
        &json!({"machine": machine, "operation": op()?}),
    )
    .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "PolicyUnrepresentable", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("owner-deny")),
        "{refused}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn codex_form_defaults_start_a_real_runner_with_workspace_network_off() -> Outcome {
    let table = Table::open().await?;
    let result = walk(&table).await;
    match (result, table.close().await) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(walk), Err(close)) => {
            Err(format!("start failed: {walk}; cleanup failed: {close}").into())
        }
    }
}
