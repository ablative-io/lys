#![cfg(test)]
//! DIRECTORY-098 R1 at the route: the app's own credential or the
//! administrator declares one return address, judged as a saved line is;
//! an address already listed writes nothing; a person or another app's
//! credential is refused `NotAdmitted` by name; a retired app's credential
//! is refused as its credential is today.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use serde_json::{Value, json};

use crate::apps_binding::sha256_hex;
use crate::apps_state::{Approved, By, Client, Decided, Line, Registered, SignInSet};
use crate::apps_store::AppStore;
use identity_contract::lys_identity_server::Config;
use identity_contract::lys_identity_server::dev_seed::seed_configured;

type Outcome = Result<(), Box<dyn Error>>;

const LISTED: &str = "https://app.example.test/callback";
const CONSOLE: &str = "https://console.example/signin/return";
const PLAIN: &str = "http://console.example/signin/return";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "scope@example.test".to_owned(),
    }
}

fn secret(app: &str) -> String {
    format!("{app}-fixture-secret")
}

/// The app's own credential, as `apps_binding::bearer` reads it.
fn credential(app: &str) -> String {
    format!("Bearer lys-app.{app}.{}", secret(app))
}

fn fixture(config: &Config) -> Result<(), Box<dyn Error>> {
    seed_configured(config, [ADMINISTRATOR, "member"])?;
    let mut apps = AppStore::open(
        &config.apps_dir(),
        Arc::new(Ed25519Identity::load(&config.event_key_file)?),
    )?;
    for id in ["liminal", "other", "retired"] {
        apps.keep(Line::Registered(Registered {
            operation: format!("register-{id}"),
            app: id.to_owned(),
            name: id.to_owned(),
            redirects: vec![LISTED.to_owned()],
            schema: json!({"kinds": {format!("{id}.doc"): {
                "actions": ["read"], "relations": {"reader": ["read"]}, "parents": []}}}),
            service_account: None,
            by: By::Start,
            at: 1,
        }))?;
        apps.keep(Line::Approved(Approved {
            operation: format!("approve-{id}"),
            app: id.to_owned(),
            client: Client {
                client_id: id.to_owned(),
                secret_sha256: sha256_hex(&secret(id)),
            },
            binding: None,
            by: By::Start,
            at: 2,
        }))?;
        apps.keep(Line::SignInSet(SignInSet {
            operation: format!("sign-in-{id}"),
            app: id.to_owned(),
            redirects: vec![LISTED.to_owned()],
            profile: false,
            by: By::Start,
            at: 2,
        }))?;
        if id == "retired" {
            apps.keep(Line::Retired(Decided {
                operation: "retire".to_owned(),
                app: id.to_owned(),
                reason: "withdrawn".to_owned(),
                by: By::Start,
                at: 3,
            }))?;
        }
    }
    Ok(())
}

fn body(operation: &str, address: &str) -> Vec<u8> {
    json!({"operation": operation, "address": address})
        .to_string()
        .into_bytes()
}

fn op() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// Declare `address` for `app` with the credential `holder` holds.
async fn declare_as(
    service: &Service,
    app: &str,
    holder: &str,
    address: &str,
    operation: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    service
        .post_signed(
            &format!("/apps/{app}/sign_in/returns"),
            ("authorization", &credential(holder)),
            body(operation, address),
        )
        .await
}

#[tokio::test]
async fn the_app_declares_its_return_address_once_and_a_repeat_writes_nothing() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let first = op()?;
    let (status, answer) = declare_as(&service, "liminal", "liminal", CONSOLE, &first).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["sign_in"]["redirects"], json!([LISTED, CONSOLE]));
    assert_eq!(
        answer["sign_in"]["declared"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(answer["sign_in"]["declared"][0]["address"], json!(CONSOLE));
    assert_eq!(
        answer["sign_in"]["declared"][0]["by"],
        json!({"kind": "service_account", "id": "liminal"})
    );
    // the same operation again answers what it did
    let (status, same) = declare_as(&service, "liminal", "liminal", CONSOLE, &first).await?;
    assert_eq!(status, 200, "{same}");
    assert_eq!(same["sign_in"], answer["sign_in"]);
    // a new operation for an address already listed writes nothing
    let (status, again) = declare_as(&service, "liminal", "liminal", CONSOLE, &op()?).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["sign_in"], answer["sign_in"]);
    // the provider's own list, as the administrator reads it, carries it
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, read) = service.get("/apps/liminal", Some(&administrator)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["sign_in"]["redirects"], json!([LISTED, CONSOLE]));
    Ok(())
}

#[tokio::test]
async fn the_administrator_declares_and_a_whole_list_save_makes_it_a_saved_address() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, answer) = service
        .post(
            "/apps/liminal/sign_in/returns",
            Some(&administrator),
            &json!({"operation": op()?, "address": CONSOLE}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["sign_in"]["redirects"], json!([LISTED, CONSOLE]));
    assert_eq!(
        answer["sign_in"]["declared"][0]["by"]["kind"],
        json!("person")
    );
    let (status, saved) = service
        .post(
            "/apps/liminal/sign_in",
            Some(&administrator),
            &json!({"operation": op()?, "redirects": [LISTED, CONSOLE], "profile": false}),
        )
        .await?;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["sign_in"]["redirects"], json!([LISTED, CONSOLE]));
    assert_eq!(saved["sign_in"]["declared"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_person_and_another_apps_credential_are_refused_not_admitted() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let (status, refused) = declare_as(&service, "liminal", "other", CONSOLE, &op()?).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], json!("NotAdmitted"));
    // Lys words a NotAdmitted reason with its name in front, as every other does.
    assert_eq!(
        refused["reason"],
        json!(
            "NotAdmitted: only the administrator or the app itself declares the app's return addresses"
        )
    );
    let member = service.sign_in(login("member")).await?;
    let (status, refused) = service
        .post(
            "/apps/liminal/sign_in/returns",
            Some(&member),
            &json!({"operation": op()?, "address": CONSOLE}),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], json!("NotAdmitted"));
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (_, read) = service.get("/apps/liminal", Some(&administrator)).await?;
    assert_eq!(read["sign_in"]["redirects"], json!([LISTED]));
    Ok(())
}

#[tokio::test]
async fn an_http_address_off_this_machine_is_refused_by_the_saved_lines_rule() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let (status, refused) = declare_as(&service, "liminal", "liminal", PLAIN, &op()?).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], json!("redirect_invalid"));
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("is neither https nor http to this machine")),
        "{refused}"
    );
    Ok(())
}

#[tokio::test]
async fn a_retired_apps_credential_is_refused_as_retired() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let (status, refused) = declare_as(&service, "retired", "retired", CONSOLE, &op()?).await?;
    assert_eq!(
        refused["refusal"],
        json!("app_retired"),
        "{status} {refused}"
    );
    Ok(())
}
