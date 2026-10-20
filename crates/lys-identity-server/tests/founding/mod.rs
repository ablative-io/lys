//! The table and the helpers the founding tests share (DIRECTORY-111):
//! a service with a fake issuer, Ada as its first-run administrator of
//! Acme, and its letters pointed at a sink of its own.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use identity_contract::mail_sink::{Captured, MailSink};
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

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const CODE: &str = "organisations-found-first-run-code";
pub(crate) const ADMIN_EMAIL: &str = "ada@example.test";
pub(crate) const PASSWORD: &str = "Analytical-Engine-1843";
pub(crate) const BEA: &str = "bea@example.test";
pub(crate) const FOUND: &str = "/organisations/found";
pub(crate) const PROVE: &str = "/organisations/found/prove";

/// A service with a fake issuer, Ada as its first-run administrator of
/// Acme and, when `mail` is set, its letters pointed at a sink of its own.
pub(crate) async fn table(
    mail: bool,
) -> Result<(Service, FakeRauthy, String, MailSink), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
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
                organisation: "Acme".to_owned(),
            });
            if mail {
                config.mail = Some(MailSettings {
                    host: "127.0.0.1".to_owned(),
                    port,
                    user: None,
                    password_file: None,
                    from: "Lys <lys@example.test>".to_owned(),
                    tls: Tls::None,
                    sent_file: config.log_dir.with_file_name("letters-sent.jsonl"),
                });
            }
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
    Ok((service, rauthy, ada, sink))
}

/// Bea founds Bea's Bakery.
pub(crate) fn found(operation: &str) -> Value {
    json!({
        "operation": operation,
        "organisation": "Bea's Bakery",
        "display_name": "Bea Baker",
        "email": BEA,
    })
}

/// The token in a founded letter's link.
pub(crate) fn token_in(letter: &Captured) -> Result<String, Box<dyn Error>> {
    let (_, after) = letter
        .body
        .split_once("/#/found/")
        .ok_or("the letter carries the proving link")?;
    Ok(after
        .split(|c: char| c.is_whitespace())
        .next()
        .ok_or("a token follows the link's path")?
        .to_owned())
}

/// How many directory leaves the service has, read from the public
/// receipts: the first index that answers no receipt.
pub(crate) async fn leaves(service: &Service) -> Result<u64, Box<dyn Error>> {
    let mut count = 0;
    loop {
        let (status, _) = service.get(&format!("/receipts/{count}"), None).await?;
        if status != 200 {
            return Ok(count);
        }
        count += 1;
        if count > 10_000 {
            return Err("the receipts never end".into());
        }
    }
}

/// The file called `name` under the service's directory, if one is.
pub(crate) fn file_under(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.file_name().is_some_and(|found| found == name) {
                return Some(path);
            }
        }
    }
    None
}

/// The foundings file, parsed.
pub(crate) fn foundings(service: &Service) -> Result<Option<Value>, Box<dyn Error>> {
    let Some(path) = file_under(service.dir.path(), "foundings.json") else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&std::fs::read_to_string(path)?)?))
}

/// The accounts the issuer holds for `email`.
pub(crate) fn accounts_for(rauthy: &FakeRauthy, email: &str) -> Result<usize, Box<dyn Error>> {
    Ok(rauthy
        .users()?
        .iter()
        .filter(|user| user["email"] == email)
        .count())
}

/// Post `token` to prove, answering the status, the body and the cookie.
pub(crate) async fn prove(
    service: &Service,
    token: &str,
) -> Result<(u16, Value, Option<String>), Box<dyn Error>> {
    let response = reqwest::Client::new()
        .post(format!("{}{PROVE}", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({ "proof": token }).to_string())
        .send()
        .await?;
    let status = response.status().as_u16();
    let cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    let body: Value = response.json().await?;
    Ok((status, body, cookie))
}
