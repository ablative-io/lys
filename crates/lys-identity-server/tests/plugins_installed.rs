#![cfg(test)]
//! The agent page asks the run's own computer which plugins its config folder
//! has installed: `GET /harnesses/plugins` asks the machine's runner, started
//! here, which reads the folder by the rule its start check uses. Only an
//! administrator asks; a machine not kept, a machine naming no runner, a
//! folder that is not plain and absolute, and a plugins file the runner
//! cannot read are each refused by name.

use std::error::Error;
use std::sync::Arc;

use axum::Router;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn broken_config(dir: &std::path::Path) -> Result<String, Box<dyn Error>> {
    let broken = dir.join("broken-config");
    std::fs::create_dir_all(broken.join("plugins"))?;
    std::fs::write(broken.join("plugins/installed_plugins.json"), b"not json")?;
    let broken = broken.to_str().ok_or("the test folder is not UTF-8")?;
    Ok(broken.to_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_machines_runner_names_the_plugins_installed_in_a_config_folder() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let broker = Router::new().fallback(|| async { axum::Json(json!({"handles": []})) });
    let broker = tokio::spawn(async move { axum::serve(listener, broker).await });
    let dir = tempfile::tempdir()?;
    let key_file = dir.path().join("broker.key");
    Ed25519Identity::load_or_generate(&key_file)?;
    let settings = SecretsSettings {
        broker: format!("http://{address}"),
        service: "identity".to_owned(),
        service_key_file: key_file,
    };
    let socket = dir.path().join("runner.sock");
    let adjusted = socket.clone();
    let state = dir.path().join("state");
    let (service, serving) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        Some(settings),
        None,
        move |config| {
            config.runner_socket = Some(adjusted);
        },
        move |config| {
            seed_configured(config, [ADMINISTRATOR, BEA])?;
            let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
            let runner = Runner::open(&Options {
                socket,
                state,
                server_key: key.public_key_bytes(),
                scrollback: 4096,
            })?;
            Ok(runner.spawn())
        },
    )
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;

    let config = dir.path().join("claude-config");
    std::fs::create_dir_all(config.join("plugins"))?;
    std::fs::write(
        config.join("plugins/installed_plugins.json"),
        br#"{"version":2,"plugins":{"manifold@manifold":[{}],"argus@ablative":[{}]}}"#,
    )?;
    let named = config.to_str().ok_or("the test folder is not UTF-8")?;

    let machine = operation()?;
    let made = service
        .post(
            "/network/machines",
            Some(&ada),
            &json!({ "operation": machine, "name": "Seat", "kind": "laptop", "runtime": "sh",
                "slots": 2, "may_run": [], "may_reach": [] }),
        )
        .await?;
    assert_eq!(made.0, 200, "{}", made.1);
    let asked =
        |config_dir: &str| format!("/harnesses/plugins?machine={machine}&config_dir={config_dir}");

    let unnamed = service.get(&asked(named), Some(&ada)).await?;
    refused(&unnamed, 409, "runner_absent");

    let runner = service
        .post(
            &format!("/network/machines/{machine}/runner"),
            Some(&ada),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
    assert_eq!(runner.0, 200, "{}", runner.1);

    let (status, answer) = service.get(&asked(named), Some(&ada)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer,
        json!({ "machine": machine, "config_dir": named,
            "installed": ["argus@ablative", "manifold@manifold"] })
    );

    let empty = dir.path().join("empty-config");
    std::fs::create_dir(&empty)?;
    let empty = empty.to_str().ok_or("the test folder is not UTF-8")?;
    let (status, none) = service.get(&asked(empty), Some(&ada)).await?;
    assert_eq!(status, 200, "{none}");
    assert_eq!(
        none["installed"],
        json!([]),
        "a folder with no plugins file has none"
    );

    let (status, home) = service
        .get(&format!("/harnesses/plugins?machine={machine}"), Some(&ada))
        .await?;
    // The test runs under a login with a HOME, so the folder is named: one answer, never either.
    assert_eq!(status, 200, "{home}");
    let folder = home["config_dir"].as_str().ok_or("no folder named")?;
    assert!(folder.ends_with("/.claude"), "{home}");

    let unreadable = service
        .get(&asked(&broken_config(dir.path())?), Some(&ada))
        .await?;
    refused(&unreadable, 409, "plugins_unreadable");

    let by_bea = service.get(&asked(named), Some(&bea)).await?;
    refused(&by_bea, 403, "NotAdmitted");

    let relative = service.get(&asked("claude-config"), Some(&ada)).await?;
    refused(&relative, 400, "RequestMalformed");

    let unknown = service
        .get(
            &format!(
                "/harnesses/plugins?machine={}&config_dir={named}",
                operation()?
            ),
            Some(&ada),
        )
        .await?;
    refused(&unknown, 404, "MachineUnknown");

    serving.stop()?;
    broker.abort();
    Ok(())
}
