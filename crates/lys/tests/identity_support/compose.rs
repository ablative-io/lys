//! The compose runtime for a test deployment: render its environment
//! through `lys identity prepare`, drive deploy/identity/compose.yaml, and
//! wait for readiness by `lys identity health`.

use std::process::{Command, Output};
use std::time::{Duration, Instant};

use super::fixtures::{Deployment, TestResult, output_text, repository_root, succeeded};
use super::processes::output;

/// How long a fresh deployment may take to become ready once its images
/// are present.
pub const READY_WITHIN: Duration = Duration::from_secs(180);

/// Refuses to go on without a container runtime answering `docker info`.
pub fn require_runtime() -> TestResult {
    let answered = output(Command::new("docker").arg("info"), "docker info")
        .is_ok_and(|answer| answer.status.success());
    if answered {
        Ok(())
    } else {
        Err("container_runtime_missing: no container runtime answers docker info".into())
    }
}

/// Renders the deployment's configuration into the environment compose.yaml
/// interpolates, with the real `lys identity prepare`.
pub fn render(deployment: &Deployment) -> TestResult<Output> {
    let output = deployment.lys("prepare")?;
    succeeded(&output, "lys identity prepare")?;
    Ok(output)
}

/// Runs `docker compose` for the deployment with `args`.
pub fn compose(deployment: &Deployment, args: &[&str]) -> TestResult<Output> {
    let file = repository_root().join("deploy/identity/compose.yaml");
    let mut command = Command::new("docker");
    command
        .arg("compose")
        .arg("-f")
        .arg(file)
        .arg("--env-file")
        .arg(deployment.state.join("compose.env"))
        .args(["-p", &deployment.project]);
    if deployment.bundled {
        command.args(["--profile", "bundled-db"]);
    }
    output(command.args(args), &format!("docker compose {}", args.join(" ")))
}

/// The resolved compose model, as JSON.
pub fn config_json(deployment: &Deployment) -> TestResult<serde_json::Value> {
    let output = compose(deployment, &["config", "--format", "json"])?;
    succeeded(&output, "docker compose config")?;
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Starts every service of the deployment, or only `services`.
pub fn up(deployment: &Deployment, services: &[&str]) -> TestResult {
    let mut args = vec!["up", "-d"];
    args.extend_from_slice(services);
    succeeded(&compose(deployment, &args)?, "docker compose up")
}

/// Stops one service.
pub fn stop(deployment: &Deployment, service: &str) -> TestResult {
    succeeded(
        &compose(deployment, &["stop", service])?,
        "docker compose stop",
    )
}

/// Starts one stopped service.
pub fn start(deployment: &Deployment, service: &str) -> TestResult {
    succeeded(
        &compose(deployment, &["start", service])?,
        "docker compose start",
    )
}

/// Removes the deployment's containers, networks and volumes.
pub fn down(deployment: &Deployment) -> TestResult {
    if !deployment.state.join("compose.env").exists() {
        return Ok(());
    }
    succeeded(
        &compose(deployment, &["down", "-v", "--remove-orphans"])?,
        "docker compose down",
    )
}

/// Polls `lys identity health` until it exits 0, returning its output.
pub fn wait_ready(deployment: &Deployment) -> TestResult<Output> {
    let started = Instant::now();
    loop {
        let output = deployment.lys("health")?;
        if output.status.success() {
            return Ok(output);
        }
        if started.elapsed() > READY_WITHIN {
            return Err(format!(
                "not ready within {READY_WITHIN:?}: {}",
                output_text(&output)
            )
            .into());
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Runs one statement through psql inside the bundled `PostgreSQL` service,
/// as `role`, over its local socket.
pub fn psql(deployment: &Deployment, role: &str, sql: &str) -> TestResult<Output> {
    compose(
        deployment,
        &[
            "exec",
            "-T",
            "postgres",
            "psql",
            "-U",
            role,
            "-d",
            "identity",
            "-v",
            "ON_ERROR_STOP=1",
            "-At",
            "-c",
            sql,
        ],
    )
}
