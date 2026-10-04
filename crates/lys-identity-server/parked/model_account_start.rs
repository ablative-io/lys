//! A run on a model account. With no account named in an agent's settings
//! the run uses the machine's own login, as every other start test shows.
//! With one named, a program the catalogue gives no way to run from an
//! account is refused `ModelAccountUnsupported`; an agent the broker will
//! not let draw from it is refused `ModelAccountNotDrawable`, and nothing
//! starts; and an agent that may draw is started with the account's
//! placeholder in its program's variable, which names its draw and is never
//! kept: no answer, no record the service keeps, no line it says, and no
//! file the runner keeps carries the draw's handle.
//!
//! The broker here is a stand-in. That the real broker puts the sealed token
//! in on each call is proved in `lys-secrets` and `lys-home`; that the
//! provider takes it is proved only once a real token is registered after
//! the install.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use identity_contract::apps::{Auth, login, ok, op, post};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, LoginBinding, OperationId, Profile, Provenance};
use lys_identity_server::routes::open_directory;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Act, Answer, Client, Options, Runner, Serving};
use serde_json::{Value, json};

type Outcome<T = ()> = Result<T, Box<dyn Error>>;

const ACCOUNT: &str = "model-account-op-0123456789abcdef0123456789abcdef";
const HANDLE: &str = "0a1b2c3d4e5f60718293a4b5c6d7e8f9";
const DRAW: &str = "d7a0f3c95be14e26a8c1f04d93b27e6c5a1d8f0e3b6c9a2d7f4e1b8c5a2d9f60";

/// What the stand-in broker answers a draw with.
#[derive(Clone, Copy)]
enum Draws {
    Refused,
    Given,
}

struct Table {
    service: Service,
    cookie: String,
    dir: tempfile::TempDir,
    key: Arc<Ed25519Identity>,
    serving: Serving,
    asked: Arc<Mutex<Vec<String>>>,
    said: Arc<Mutex<Vec<String>>>,
}

fn stand_in(draws: Draws, asked: Arc<Mutex<Vec<String>>>) -> axum::Router {
    axum::Router::new().fallback(move |request: Request| {
        let asked = Arc::clone(&asked);
        async move {
            let path = request.uri().path().to_owned();
            asked.lock().expect("fixture lock poisoned").push(path.clone());
            let answer: Response = match (path.as_str(), draws) {
                ("/_lys/model/draw", Draws::Refused) => (
                    StatusCode::FORBIDDEN,
                    "ModelAccountNotDrawable: agent may not draw from model account Main: PermissionDenied (no grant)\n",
                )
                    .into_response(),
                ("/_lys/model/draw", Draws::Given) => axum::Json(json!({
                    "account": ACCOUNT, "agent": "agent", "handle": HANDLE, "token": DRAW,
                }))
                .into_response(),
                _ => axum::Json(json!({ "handles": [] })).into_response(),
            };
            answer
        }
    })
}

impl Table {
    async fn open(draws: Draws) -> Outcome<Self> {
        let dir = tempfile::tempdir()?;
        let asked = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let router = stand_in(draws, Arc::clone(&asked));
        tokio::spawn(async move { axum::serve(listener, router).await });
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
        let said = Arc::new(Mutex::new(Vec::new()));
        let saying = Arc::clone(&said);
        let say: lys_identity_server::Say = Arc::new(move |line| {
            saying
                .lock()
                .expect("fixture lock poisoned")
                .push(line.to_owned());
        });
        let (service, (serving, key)) = Box::pin(Service::start_saying(
            GRANT_MODEL,
            None,
            Some(secrets),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
                config.model_proxy = Some("http://127.0.0.1:9/anthropic".to_owned());
            },
            Some(say),
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
        ))
        .await?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        // The program says it was given the placeholder without printing it.
        let expected = format!("lys-account.{ACCOUNT}.{HANDLE}.{DRAW}");
        let program = dir.path().join("claude");
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\nif [ \"$CLAUDE_CODE_OAUTH_TOKEN\" = \"{expected}\" ]; then printf 'placeholder-given\\n'; else printf 'placeholder-missing\\n'; fi\nexec cat\n"
            ),
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        Ok(Self {
            service,
            cookie,
            dir,
            key,
            serving,
            asked,
            said,
        })
    }

    async fn post(&self, path: &str, body: &Value) -> Outcome<Value> {
        ok(post(&self.service, path, Auth::Cookie(&self.cookie), body).await?)
    }

    /// An active agent whose reviewed profile names `account` with
    /// `description`, a computer it may run on, and its start's answer.
    async fn start(&self, description: &Value, account: &str) -> Outcome<(u16, Value)> {
        let registered = self
            .post(
                "/agents",
                &json!({"operation": op()?, "display_name": "Drawing"}),
            )
            .await?;
        let agent = registered["agent"].as_str().ok_or("no agent")?.to_owned();
        self.post(
            &format!("/identities/{agent}/transitions"),
            &json!({"operation": op()?, "transition": "activate", "reason": "Run on an account"}),
        )
        .await?;
        self.post(&format!("/agents/{agent}/provisioning"), &json!({
            "operation": op()?, "from_version": 0, "working_folder": "/tmp",
            "model_access": ["default"], "tools": [], "skills": [], "mcp_servers": [],
            "instructions": "", "instructions_mode": "keep", "note": "Run on an account",
            "permissions": {"default_mode": "workspace-only"},
            "model_account": account,
            "harness": {"name": "Claude Code", "description": description,
                        "program": self.dir.path().join("claude"), "package": "2.1.287"},
        }))
        .await?;
        self.post(
            &format!("/agents/{agent}/provisioning/1/review"),
            &json!({"operation": op()?}),
        )
        .await?;
        let machine = op()?;
        self.post("/network/machines", &json!({
            "operation": machine, "name": "This computer", "kind": "Computer", "runtime": "lys-runner",
            "slots": 0, "may_run": [agent], "may_run_roles": [], "may_reach": []
        }))
        .await?;
        self.post(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner": {"kind": "lys"}}),
        )
        .await?;
        post(
            &self.service,
            &format!("/agents/{agent}/start-command"),
            Auth::Cookie(&self.cookie),
            &json!({"machine": machine, "operation": op()?}),
        )
        .await
    }

    fn close(self) -> Outcome {
        self.serving.stop()?;
        Ok(())
    }
}

fn description() -> Outcome<Value> {
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    Ok(catalogue["description"].clone())
}

/// Every file under `dir`, none of which may carry `text`.
fn never_written(dir: &Path, text: &str) -> Outcome {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            never_written(&path, text)?;
        } else {
            let bytes = std::fs::read(&path)?;
            assert!(
                !bytes
                    .windows(text.len())
                    .any(|window| window == text.as_bytes()),
                "{} carries the draw",
                path.display()
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_program_the_catalogue_gives_no_account_way_is_refused_by_name() -> Outcome {
    let table = Table::open(Draws::Given).await?;
    let mut bare = description()?;
    bare.as_object_mut()
        .ok_or("description is not an object")?
        .remove("model_account");
    let (status, answer) = table.start(&bare, ACCOUNT).await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "ModelAccountUnsupported");
    assert!(
        answer["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("cannot be run from a model account")),
        "{answer}"
    );
    assert!(
        !table
            .asked
            .lock()
            .expect("fixture lock poisoned")
            .iter()
            .any(|path| path == "/_lys/model/draw")
    );
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_the_broker_will_not_let_draw_is_refused_and_nothing_starts() -> Outcome {
    let table = Table::open(Draws::Refused).await?;
    let (status, answer) = table.start(&description()?, ACCOUNT).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "ModelAccountNotDrawable");
    let (_, live) = table
        .service
        .get("/runtime/live", Some(&table.cookie))
        .await?;
    assert_eq!(live["sessions"], json!([]), "{live}");
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drawing_agent_is_given_the_placeholder_and_the_draw_is_never_kept() -> Outcome {
    let table = Table::open(Draws::Given).await?;
    let (status, answer) = table.start(&description()?, ACCOUNT).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["model_account"]["account"], ACCOUNT);
    assert!(!answer.to_string().contains(DRAW), "{answer}");
    let session = answer["session"].as_str().ok_or("no session")?;
    let client = Client::new(table.dir.path().join("runner.sock"), Arc::clone(&table.key));
    match client.ask(&Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: "placeholder-(given|missing)".to_owned(),
        regex: true,
    })? {
        Answer::Matched { matched, .. } => {
            assert_eq!(matched, "placeholder-given", "the run was not given its placeholder");
        }
        other => return Err(format!("the run said nothing of its placeholder: {other:?}").into()),
    }
    let (_, live) = table
        .service
        .get("/runtime/live", Some(&table.cookie))
        .await?;
    assert!(!live.to_string().contains(DRAW));
    never_written(table.service.dir.path(), DRAW)?;
    never_written(&table.dir.path().join("runner-state"), DRAW)?;
    assert!(
        !table
            .said
            .lock()
            .expect("fixture lock poisoned")
            .iter()
            .any(|line| line.contains(DRAW))
    );
    table.close()
}
