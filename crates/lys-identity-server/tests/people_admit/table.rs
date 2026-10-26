//! The admit tests’ issuer, service and existing route helpers.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use identity_contract::mail_sink::MailSink;
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::mail::{MailSettings, Tls};
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "people-admit-first-run-code";
const ADMIN_EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
pub(super) const BEA: &str = "bea@example.test";

pub(super) async fn table_with_mail(
    mail: bool,
) -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    // Configured letters go to a sink of their own; without mail the act
    // hands the link. The handle may go; the sink stays.
    let sink = MailSink::start().await?;
    let port = sink.port();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        move |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
                organisation: "Example organisation".to_owned(),
            });
            config.mail = if mail {
                Some(MailSettings {
                    host: "127.0.0.1".to_owned(),
                    port,
                    user: None,
                    password_file: None,
                    from: "Lys <lys@example.test>".to_owned(),
                    tls: Tls::None,
                    sent_file: config.log_dir.with_file_name("letters-sent.jsonl"),
                })
            } else {
                None
            };
        },
        |config| {
            let setup = config.setup.as_ref().ok_or("setup is configured")?;
            let digest: Vec<String> = Sha256::digest(CODE.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let pending = json!({ "purpose": "first-run", "sha256": digest.concat() });
            std::fs::write(&setup.code_file, pending.to_string())?;
            let key = Arc::new(load_service_key(&config.event_key_file)?);
            drop(open_directory(config)?);
            drop(ConfigurationStore::open(
                &config.log_dir.with_file_name("organisation"),
                Arc::clone(&key),
            )?);
            drop(ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            Ok(())
        },
    )
    .await?;
    rauthy.link(service.issuer.clone())?;
    let made = reqwest::Client::new()
        .post(format!("{}/setup/administrator", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "code": CODE,
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Ada Lovelace",
                "email": ADMIN_EMAIL,
                "password": PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await?;
    let ada = session_cookie(made).await?;
    Ok((service, rauthy, ada))
}

/// The act admitting Bea with a first grant under `relation`.
pub(super) fn admit(operation: &str, relation: &str) -> Value {
    json!({
        "operation": operation,
        "display_name": "Bea",
        "email": BEA,
        "grant": {
            "route": "browser",
            "resource": {"kind": "doc", "id": "alpha"},
            "relation": relation,
        },
    })
}

/// Sign in as the person an admit answer names, at the issuer their login
/// was bound at.
pub(super) async fn signed_in_as(
    service: &Service,
    admitted: &Value,
    email: &str,
) -> Result<String, Box<dyn Error>> {
    let subject = admitted["subject"]
        .as_str()
        .ok_or("an issuer subject")?
        .to_owned();
    service
        .sign_in(Login {
            subject,
            email: email.to_owned(),
        })
        .await
}

/// The durable link and letter records, read before and after a refused act.
pub(super) fn admission_files(
    dir: &std::path::Path,
) -> Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>, Box<dyn Error>> {
    let mut files = std::collections::BTreeMap::new();
    let mut directories = vec![dir.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                directories.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name == "foundings.json" || name == "letters-sent.jsonl")
            {
                files.insert(path.clone(), std::fs::read(path)?);
            }
        }
    }
    Ok(files)
}
