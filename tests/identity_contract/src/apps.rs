//! What the app tests share: a seeded service, requests carrying a session
//! or an app's credential, and two fixture apps named for these tests only.
//! No product is named: the apps are `fixture_notes` and `fixture_files`,
//! registered here through the API as any app would register, and Lys
//! holds nothing of either until a test registers it.

use std::error::Error;

use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

use crate::fake_issuer::Login;
use crate::harness::{ADMINISTRATOR, Service};

/// A test's outcome.
pub type TestResult = Result<(), Box<dyn Error>>;

/// A status and a JSON body, or the body's text when it is not JSON.
pub type Answer = (u16, Value);

/// The second seeded person's subject.
pub const BEA: &str = "bea-subject";

/// The fixture app every workspace test registers.
pub const NOTES: &str = "fixture_notes";

/// A second fixture app, declaring a kind of the same name as the first.
pub const FILES: &str = "fixture_files";

/// How a request authenticates.
#[derive(Debug, Clone, Copy)]
pub enum Auth<'a> {
    /// No authentication.
    Nobody,
    /// A session cookie.
    Cookie(&'a str),
    /// A bearer credential.
    Bearer(&'a str),
}

/// A login at the fake issuer.
pub fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

/// A new operation id.
pub fn op() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// The service with the administrator's person and Bea seeded.
pub async fn seeded() -> Result<(Service, Seeded), Box<dyn Error>> {
    let broker = crate::app_custody::start().await?;
    Service::start_adjusted(
        crate::harness::GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            })
        },
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await
}

/// Send `method` to `path` with `body`, authenticated as `auth`.
pub async fn send(
    service: &Service,
    method: reqwest::Method,
    path: &str,
    auth: Auth<'_>,
    body: Option<&Value>,
) -> Result<Answer, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut request = client.request(method, format!("{}{path}", service.base));
    match auth {
        Auth::Nobody => {}
        Auth::Cookie(cookie) => request = request.header(reqwest::header::COOKIE, cookie),
        Auth::Bearer(credential) => {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {credential}"),
            );
        }
    }
    if let Some(body) = body {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string());
    }
    let response = request.send().await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    Ok((status, value))
}

/// POST `body` to `path` as `auth`.
pub async fn post(
    service: &Service,
    path: &str,
    auth: Auth<'_>,
    body: &Value,
) -> Result<Answer, Box<dyn Error>> {
    send(service, reqwest::Method::POST, path, auth, Some(body)).await
}

/// GET `path` as `auth`.
pub async fn get(service: &Service, path: &str, auth: Auth<'_>) -> Result<Answer, Box<dyn Error>> {
    send(service, reqwest::Method::GET, path, auth, None).await
}

/// PUT `body` to `path` as `auth`.
pub async fn put(
    service: &Service,
    path: &str,
    auth: Auth<'_>,
    body: &Value,
) -> Result<Answer, Box<dyn Error>> {
    send(service, reqwest::Method::PUT, path, auth, Some(body)).await
}

/// Refuse the test unless `answer` is the refusal `name` at `status`.
pub fn refused(answer: &Answer, status: u16, name: &str) -> TestResult {
    if answer.0 != status || answer.1["refusal"] != name {
        return Err(format!("expected {status} {name}, got {} {}", answer.0, answer.1).into());
    }
    Ok(())
}

/// Refuse the test unless `answer` is a success, answering its body.
pub fn ok(answer: Answer) -> Result<Value, Box<dyn Error>> {
    if answer.0 != 200 {
        return Err(format!("expected 200, got {} {}", answer.0, answer.1).into());
    }
    Ok(answer.1)
}

/// The workspace schema: a workspace whose members reach its channels, and
/// a document kind, all under `app`'s prefix. It is written in the form the
/// service keeps a schema in, every member present and every list in order,
/// so a version read back compares equal to it.
pub fn workspace_schema(app: &str) -> Value {
    json!({"kinds": {
        format!("{app}.workspace"): {
            "actions": ["read", "write"],
            "relations": {"member": ["read"], "owner": ["read", "write"]},
            "parents": []
        },
        format!("{app}.channel"): {
            "actions": ["read", "write"],
            "relations": {"poster": ["read", "write"]},
            "parents": [format!("{app}.workspace")]
        },
        format!("{app}.doc"): {
            "actions": ["read", "write"],
            "relations": {"editor": ["read", "write"], "reader": ["read"]},
            "parents": []
        }
    }})
}

/// The registration body of `app` with `schema`.
pub fn registration(app: &str, schema: &Value) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": op()?,
        "id": app,
        "name": format!("The {app} fixture"),
        "redirects": ["https://app.example.test/signed-in"],
        "schema": schema,
    }))
}

/// Register `app` with the workspace schema, as the administrator.
pub async fn register(service: &Service, admin: &str, app: &str) -> Result<Value, Box<dyn Error>> {
    let body = registration(app, &workspace_schema(app))?;
    ok(post(service, "/apps", Auth::Cookie(admin), &body).await?)
}

/// Approve `app` as the administrator, answering the approval.
pub async fn approve(service: &Service, admin: &str, app: &str) -> Result<Value, Box<dyn Error>> {
    let body = json!({"operation": op()?});
    ok(post(
        service,
        &format!("/apps/{app}/approve"),
        Auth::Cookie(admin),
        &body,
    )
    .await?)
}

/// Register and approve `app`, answering its credential.
pub async fn registered(
    service: &Service,
    admin: &str,
    app: &str,
) -> Result<String, Box<dyn Error>> {
    register(service, admin, app).await?;
    let approval = approve(service, admin, app).await?;
    if approval["credentials"]["app"] != app || !approval["client"].is_null() {
        return Err("approval did not confirm fixture broker custody".into());
    }
    Ok(crate::app_custody::credential(app))
}

/// A root grant of `relation` on `kind:id` to `holder`, as the administrator.
pub async fn root(
    service: &Service,
    admin: &str,
    holder: &str,
    (kind, id): (&str, &str),
    relation: &str,
) -> Result<Answer, Box<dyn Error>> {
    let body = json!({
        "operation": op()?,
        "route": "api",
        "holder": holder,
        "resource": {"kind": kind, "id": id},
        "relation": relation,
        "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
    });
    post(service, "/grants/roots", Auth::Cookie(admin), &body).await
}

/// One check of a batch.
pub fn check(subject: &str, kind: &str, id: &str, action: &str) -> Value {
    json!({"subject": subject, "kind": kind, "id": id, "action": action})
}
