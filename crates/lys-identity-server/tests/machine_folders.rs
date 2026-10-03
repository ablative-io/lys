#![cfg(test)]
//! A person chooses where an agent works from the folders its computer
//! holds: `POST /network/machines/{id}/folders` asks the machine's runner,
//! started here, and answers the folders directly inside one folder. Only
//! an administrator asks; a machine not kept, a machine naming no runner, a
//! path that is not a plain absolute folder and a folder the runner cannot
//! read are each refused by name.

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

#[tokio::test(flavor = "multi_thread")]
async fn a_machines_runner_names_the_folders_inside_one_folder() -> TestResult {
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

    let held = dir.path().join("held");
    std::fs::create_dir(&held)?;
    std::fs::create_dir(held.join("receipts"))?;
    std::fs::create_dir(held.join("archive"))?;
    std::fs::write(held.join("notes.txt"), b"a file is not a folder")?;
    let named = held.to_str().ok_or("the test folder is not UTF-8")?;

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
    let folders = format!("/network/machines/{machine}/folders");

    let unnamed = service
        .post(&folders, Some(&ada), &json!({ "under": named }))
        .await?;
    refused(&unnamed, 409, "runner_absent");

    let runner = service
        .post(
            &format!("/network/machines/{machine}/runner"),
            Some(&ada),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
    assert_eq!(runner.0, 200, "{}", runner.1);

    let (status, answer) = service
        .post(&folders, Some(&ada), &json!({ "under": named }))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer,
        json!({ "machine": machine, "under": named, "folders": ["archive", "receipts"] })
    );

    let (status, home) = service.post(&folders, Some(&ada), &json!({})).await?;
    if status == 200 {
        let under = home["under"]
            .as_str()
            .ok_or("the home answer names no folder")?;
        assert!(std::path::Path::new(under).is_absolute(), "{home}");
        assert!(home["folders"].is_array(), "{home}");
    } else {
        assert_eq!(home["refusal"], "home_unknown", "{home}");
    }

    let by_bea = service
        .post(&folders, Some(&bea), &json!({ "under": named }))
        .await?;
    refused(&by_bea, 403, "NotAdmitted");

    let relative = service
        .post(&folders, Some(&ada), &json!({ "under": "held/receipts" }))
        .await?;
    refused(&relative, 400, "RequestMalformed");

    let unknown = service
        .post(
            &format!("/network/machines/{}/folders", operation()?),
            Some(&ada),
            &json!({ "under": named }),
        )
        .await?;
    refused(&unknown, 404, "MachineUnknown");

    let missing = service
        .post(
            &folders,
            Some(&ada),
            &json!({ "under": format!("{named}/nothing-here") }),
        )
        .await?;
    refused(&missing, 409, "folder_unreadable");

    serving.stop()?;
    broker.abort();
    Ok(())
}
