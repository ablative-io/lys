#![cfg(test)]
//! R7's browser acceptance, run against the standalone identity server: the
//! server started over a disposable `SpiceDB`, the fixture set up through its
//! own routes, and `surface/identity/tests/acceptance/permission_why.spec.ts`
//! run by vitest against it, driving the why view as a person does and
//! comparing each verdict and reason it shows with the server's own answer.
//!
//! The fixture extends R6's: person P (Ada, the root authority) and a second
//! person Q (Bea), agent A (Ada's agent) and identity B (Bea's agent). The
//! root authority issues P a root grant on project X and Q one on project Y;
//! P grants A read on X (grant PA); Q grants B read on Y (grant QB); B holds
//! nothing on X. The run's whole output is printed, and the count of tests
//! vitest ran and passed is asserted, not only its exit.
//!
//! The browser surface's own packages are installed from its lock file with
//! `npm ci` when they are not held.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

/// The root authority's root grant of read on `project` to `holder`, which
/// the holder may pass on to agents.
fn root(holder: &str, project: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "browser",
        "holder": holder,
        "resource": { "kind": "project", "id": project },
        "relation": "alpha",
        "pass_on": { "kind": "to", "actions": ["read"], "recipients": ["agent"] },
        "window": { "starts_at": 0, "ends_at": null },
    }))
}

/// `responsible`'s grant of read on `project` to its agent `recipient`,
/// derived from `source`.
fn read_to(
    source: &str,
    recipient: &str,
    responsible: &str,
    project: &str,
) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "browser",
        "source": source,
        "recipient": recipient,
        "responsible": responsible,
        "resource": { "kind": "project", "id": project },
        "relation": "beta",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 0, "ends_at": null },
    }))
}

/// POST `body` to `path` as `cookie`, answering the grant it recorded.
async fn recorded(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
) -> Result<String, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer["grant"]
        .as_str()
        .ok_or_else(|| format!("{path} recorded no grant: {answer}"))?
        .to_owned())
}

/// The browser surface, with its packages installed.
fn surface() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let surface = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../surface/identity");
    let vitest = surface.join("node_modules/.bin/vitest");
    if !vitest.exists() {
        let installed = Command::new("npm")
            .args(["ci", "--no-audit", "--no-fund"])
            .current_dir(&surface)
            .output()?;
        if !installed.status.success() {
            return Err(format!(
                "npm ci in {} exited {}: {}{}",
                surface.display(),
                installed.status,
                String::from_utf8_lossy(&installed.stdout),
                String::from_utf8_lossy(&installed.stderr)
            )
            .into());
        }
    }
    Ok((surface, vitest))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_why_view_shows_the_standalone_servers_answers_for_a_and_b() -> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let (ada, bea) = (&seeded.people[0], &seeded.people[1]);
    let (person_p, person_q) = (ada.id.to_string(), bea.id.to_string());
    let (agent_a, identity_b) = (ada.agents[0].id.to_string(), bea.agents[0].id.to_string());
    let root_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let q_cookie = service.sign_in(login(BEA)).await?;

    let rp = recorded(
        &service,
        "/grants/roots",
        &root_cookie,
        &root(&person_p, "x")?,
    )
    .await?;
    let rq = recorded(
        &service,
        "/grants/roots",
        &root_cookie,
        &root(&person_q, "y")?,
    )
    .await?;
    let pa = read_to(&rp, &agent_a, &person_p, "x")?;
    recorded(&service, "/grants", &root_cookie, &pa).await?;
    let qb = read_to(&rq, &identity_b, &person_q, "y")?;
    recorded(&service, "/grants", &q_cookie, &qb).await?;

    let fixture = json!({
        "service": service.base,
        "session": root_cookie,
        "agent_a": agent_a,
        "identity_b": identity_b,
        "person_p": person_p,
        "resource_x": { "kind": "project", "id": "x" },
    });
    let (dir, vitest) = surface()?;
    let mut run = Command::new(vitest);
    run.args(["run", "--config", "vitest.acceptance.config.ts"])
        .env("LYS_WHY_ACCEPTANCE", fixture.to_string())
        .env("NO_COLOR", "1")
        .current_dir(dir);
    let ran = tokio::task::spawn_blocking(move || run.output()).await??;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    println!("{said}");
    assert!(ran.status.success(), "vitest exited {}: {said}", ran.status);
    let tests = said
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("Tests "))
        .collect::<Vec<_>>();
    assert_eq!(
        tests.len(),
        1,
        "vitest reported no single test count: {said}"
    );
    assert!(
        tests[0].ends_with(" 1 passed (1)"),
        "vitest did not run and pass exactly one acceptance test: {said}"
    );
    Ok(())
}
