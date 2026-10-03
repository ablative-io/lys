#![cfg(test)]
//! DIRECTORY-077 path proof: an agent Lys starts finds Lys's MCP address and
//! its own pass in the config its seat reads, lists the tools, acts on one
//! route inside its grant, is refused outside it, and once stopped its pass
//! is refused.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const PASS_HEADER: &str = "lys-agent-pass";
const SEAT_HEADER: &str = "lys-seat";

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

struct Held {
    service: Service,
    seeded: Seeded,
    ada: String,
    dir: tempfile::TempDir,
    serving: Option<Serving>,
    passes: PathBuf,
    machine: String,
    broker: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Held {
    async fn open() -> TestResult<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = Router::new().fallback(|| async { axum::Json(json!({"handles": []})) });
        let broker = tokio::spawn(async move { axum::serve(listener, broker).await });
        let dir = tempfile::tempdir()?;
        let program = dir.path().join("seat");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf 'ready\\n'\nwhile IFS= read -r line; do printf '%s\\n' \"$line\"; done\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
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
        let (service, (seeded, serving, passes)) = Service::start_adjusted(
            &lys_identity::grants::shipped_model(),
            None,
            Some(settings),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
            },
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 4096,
                })?;
                let passes = config.log_dir.with_file_name("agent-passes.json");
                Ok((seeded, runner.spawn(), passes))
            },
        )
        .await?;
        let ada = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "person@example.test".to_owned(),
            })
            .await?;
        let held = Self {
            service,
            seeded,
            ada,
            dir,
            serving: Some(serving),
            passes,
            machine: operation()?,
            broker,
        };
        held.ok("/network/machines", &json!({ "operation": held.machine, "name": "Seat", "kind": "laptop", "runtime": program.display().to_string(), "slots": 2, "may_run": [held.agent()], "may_reach": [] })).await?;
        held.ok(
            &format!("/network/machines/{}/runner", held.machine),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
        let path = format!("/agents/{}/provisioning", held.agent());
        let mut harness = harness_description::declared();
        harness["program"] = json!(program.display().to_string());
        held.ok(&path, &json!({ "operation": operation()?, "from_version": 0, "working_folder": "/tmp",
            "model_access": ["model-1"], "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "", "permissions": {"default_mode": "plan"}, "harness": harness, "session": { "compact": "compact" } })).await?;
        held.ok(
            &format!("/agents/{}/provisioning/1/review", held.agent()),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(held)
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    fn person(&self) -> String {
        self.seeded.people[0].id.to_string()
    }

    async fn ok(&self, path: &str, body: &Value) -> TestResult<Value> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    /// The lys entry the session's native config holds: its url, its pass,
    /// and the runner's seat signature over the pass.
    fn rendered(&self, session: &str) -> TestResult<(String, String, String)> {
        let dir: PathBuf = self
            .dir
            .path()
            .join("state/sessions")
            .join(session)
            .join("config");
        for entry in std::fs::read_dir(&dir)? {
            let text = std::fs::read_to_string(entry?.path())?;
            let Ok(native) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let lys = &native["mcpServers"]["lys"];
            if let (Some(url), Some(pass), Some(seat)) = (
                lys["url"].as_str(),
                lys["headers"][PASS_HEADER].as_str(),
                lys["headers"][SEAT_HEADER].as_str(),
            ) {
                return Ok((url.to_owned(), pass.to_owned(), seat.to_owned()));
            }
        }
        Err(format!("no lys entry under {}", dir.display()).into())
    }

    async fn mcp(
        &self,
        url: &str,
        (pass, seat): (&str, Option<&str>),
        method: &str,
        params: Value,
    ) -> TestResult<(u16, Value)> {
        let mut request = reqwest::Client::new()
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header(PASS_HEADER, pass);
        if let Some(seat) = seat {
            request = request.header(SEAT_HEADER, seat);
        }
        let response = request
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await.unwrap_or(Value::Null)))
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        self.broker.abort();
        if let Some(serving) = self.serving.take() {
            if let Err(error) = serving.stop() {
                eprintln!("runner cleanup failed: {error}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_lys_starts_acts_inside_its_grant_through_mcp_and_its_pass_ends_with_it()
-> TestResult {
    let held = Held::open().await?;
    // The person grants the agent one act: changing the person's profile.
    let person = held.person();
    let resource = json!({"kind": "person", "id": person});
    let root = held.ok("/grants/roots", &json!({"operation": operation()?, "route": "api", "holder": person, "resource": resource, "relation": "owner", "pass_on": {"kind": "to", "actions": ["person.profile.set"], "recipients": ["agent"]}, "window": {"starts_at": 0, "ends_at": null}})).await?;
    held.ok("/grants", &json!({"operation": operation()?, "route": "api", "source": root["grant"], "recipient": held.agent(), "responsible": person, "resource": resource, "relation": "only.person.profile.set", "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}})).await?;

    // An agent changes a person's record only within a team they share.
    let team = operation()?;
    held.ok("/teams", &json!({"operation": team, "name": "Shared"}))
        .await?;
    for member in [&person, &held.agent()] {
        held.ok(
            &format!("/teams/{team}/members"),
            &json!({"operation": operation()?, "member": member}),
        )
        .await?;
    }

    let started = held
        .ok(
            &format!("/agents/{}/start-command", held.agent()),
            &json!({ "operation": operation()?, "machine": held.machine }),
        )
        .await?;
    let session = started["session"].as_str().ok_or("no session")?;
    let (url, pass, seat) = held.rendered(session)?;
    assert!(url.ends_with("/api/mcp"), "{url}");
    // The screens nest the API under /api; the harness serves it bare.
    let url = url.replace("/api/mcp", "/mcp");
    eprintln!(
        "proof: rendered config names {url} and a pass of {} bytes",
        pass.len()
    );

    let signed = (pass.as_str(), Some(seat.as_str()));
    let (status, bare) = held
        .mcp(&url, (pass.as_str(), None), "tools/list", json!({}))
        .await?;
    assert_eq!(status, 401, "a pass without its seat is refused: {bare}");
    // The runner's signature with its last digit changed: the same seat, a
    // signature it never made.
    let mut forged = seat.clone();
    let last = forged.pop().ok_or("the seat is empty")?;
    forged.push(if last == '0' { '1' } else { '0' });
    let (status, refused) = held
        .mcp(
            &url,
            (pass.as_str(), Some(forged.as_str())),
            "tools/list",
            json!({}),
        )
        .await?;
    assert_eq!(
        status, 401,
        "a seat that did not sign the pass is refused: {refused}"
    );
    eprintln!("proof: a bare pass and a forged seat are both refused 401");

    let (status, tools) = held.mcp(&url, signed, "tools/list", json!({})).await?;
    assert_eq!(status, 200, "{tools}");
    eprintln!(
        "proof: tools/list answered {} tools",
        tools["result"]["tools"].as_array().map_or(0, Vec::len)
    );

    let call = |path: String| json!({"name": "change", "arguments": {"method": "POST", "path": path, "body": {"operation": OperationId::generate().map(|id| id.to_string()).unwrap_or_default(), "display_name": "Changed by its agent"}}});
    let (status, inside) = held
        .mcp(
            &url,
            signed,
            "tools/call",
            call(format!("/identities/{person}/profile")),
        )
        .await?;
    assert_eq!(status, 200, "{inside}");
    assert_eq!(
        inside["result"]["structuredContent"]["status"], 200,
        "{inside}"
    );
    eprintln!("proof: inside its grant, POST /identities/{{person}}/profile answered 200");

    let other = held.seeded.people[1].id.to_string();
    let (_, outside) = held
        .mcp(
            &url,
            signed,
            "tools/call",
            call(format!("/identities/{other}/profile")),
        )
        .await?;
    let refusal = &outside["result"]["structuredContent"]["body"];
    assert_ne!(
        outside["result"]["structuredContent"]["status"], 200,
        "{outside}"
    );
    eprintln!(
        "proof: outside its grant, refused {} naming {}",
        refusal["refusal"], refusal["can_grant"]
    );

    held.ok(
        &format!("/agents/{}/stop", held.agent()),
        &json!({ "operation": operation()?, "reason": "proof" }),
    )
    .await?;
    let (status, after) = held.mcp(&url, signed, "tools/list", json!({})).await?;
    assert_eq!(status, 401, "{after}");
    eprintln!(
        "proof: after stop, the pass is refused {status} {}",
        after["refusal"]
    );
    Ok(())
}

/// A stop whose pass end fails still withdraws, keeps the stop whole and
/// tells the runners, and reports the failed end.
#[tokio::test(flavor = "multi_thread")]
async fn stop_api_a_failed_pass_end_still_keeps_the_stop_and_reports_the_failure() -> TestResult {
    let held = Held::open().await?;
    let started = held
        .ok(
            &format!("/agents/{}/start-command", held.agent()),
            &json!({ "operation": operation()?, "machine": held.machine }),
        )
        .await?;
    assert!(started["session"].as_str().is_some(), "{started}");
    // The pass table can no longer be written, so ending the pass fails.
    std::fs::remove_file(&held.passes)?;
    std::fs::create_dir(&held.passes)?;
    let (status, answer) = held
        .service
        .post(
            &format!("/agents/{}/stop", held.agent()),
            Some(&held.ada),
            &json!({ "operation": operation()?, "reason": "proof" }),
        )
        .await?;
    assert_ne!(status, 200, "a failed pass end was not reported: {answer}");
    let (status, kept) = held
        .service
        .get(&format!("/agents/{}/stops", held.agent()), Some(&held.ada))
        .await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["stops"].as_array().map(Vec::len), Some(1), "{kept}");
    assert_eq!(kept["stops"][0]["done"], true, "{kept}");
    assert_eq!(
        kept["stops"][0]["sessions_asked"].as_array().map(Vec::len),
        Some(1),
        "{kept}"
    );
    Ok(())
}
