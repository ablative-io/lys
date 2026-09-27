//! `docker compose` over `deploy/identity/compose.yaml`, with the environment
//! `lys identity prepare` renders from a fixture's configuration. `psql` runs
//! inside the PostgreSQL container; nothing here needs a client on the host.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

use super::TestResult;
use super::fixtures::{Fixture, output_text, repo_root};

/// The compose file every stack runs.
pub fn compose_file() -> PathBuf {
    repo_root().join("deploy/identity/compose.yaml")
}

/// One compose project with its rendered environment.
pub struct Compose {
    /// The compose project name.
    pub project: String,
    /// The private env file `prepare` wrote.
    pub env_file: PathBuf,
}

/// Render the fixture's configuration through `lys identity prepare` into
/// the environment compose interpolates.
pub fn render(fixture: &Fixture) -> TestResult<Compose> {
    let prepared = fixture.identity("prepare", &[])?;
    if !prepared.status.success() {
        return Err(format!("lys identity prepare failed: {}", output_text(&prepared)).into());
    }
    Ok(Compose {
        project: fixture.project.clone(),
        env_file: fixture.state_dir().join("identity.env"),
    })
}

impl Compose {
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("docker");
        command
            .arg("compose")
            .arg("--env-file")
            .arg(&self.env_file)
            .arg("-f")
            .arg(compose_file())
            .arg("-p")
            .arg(&self.project)
            .args(args);
        command
    }

    /// Run `docker compose <args>`.
    pub fn run(&self, args: &[&str]) -> TestResult<Output> {
        Ok(self.command(args).output()?)
    }

    /// Run `docker compose <args>`, failing with its output unless it exits 0.
    pub fn run_ok(&self, args: &[&str]) -> TestResult<String> {
        let output = self.run(args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(format!("docker compose {} failed: {}", args.join(" "), output_text(&output)).into())
        }
    }

    /// The resolved configuration, `docker compose config` as JSON.
    pub fn config(&self) -> TestResult<Value> {
        Ok(serde_json::from_str(&self.run_ok(&["config", "--format", "json"])?)?)
    }

    /// Run one SQL statement as `role` inside the PostgreSQL container.
    pub fn psql(&self, role: &str, sql: &str) -> TestResult<Output> {
        self.run(&[
            "exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1", "-U", role, "-d", "identity",
            "-Atc", sql,
        ])
    }

    /// Dump the identity database, custom format, to `path`.
    pub fn dump(&self, path: &Path) -> TestResult {
        let file = File::create(path)?;
        let status = self
            .command(&["exec", "-T", "postgres", "pg_dump", "-U", "postgres", "-d", "identity", "-Fc"])
            .stdout(Stdio::from(file))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("pg_dump exited {status}").into())
        }
    }

    /// Restore a dump written by [`Compose::dump`] into the identity database.
    pub fn restore(&self, path: &Path) -> TestResult {
        let output = self
            .command(&[
                "exec", "-T", "postgres", "pg_restore", "-U", "postgres", "-d", "identity", "--clean",
                "--if-exists", "--exit-on-error",
            ])
            .stdin(Stdio::from(File::open(path)?))
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!("pg_restore failed: {}", output_text(&output)).into())
        }
    }

    /// Stop and remove the project's containers and volumes.
    pub fn down(&self) -> TestResult {
        self.run_ok(&["down", "-v", "--remove-orphans"]).map(drop)
    }
}
