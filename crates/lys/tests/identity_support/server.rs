//! A running deployment for one venue, taken down with its volumes when
//! dropped, and the `lys identity` subcommands run against it.

use std::process::{Command, Output, Stdio};
use std::time::Duration;

use serde_json::Value;

use super::compose::Compose;
use super::fixtures::{Failure, Venue, json_of, repo_root, text};

/// Refuses by name when no container runtime answers `docker info`.
pub fn require_container_runtime() -> Result<(), Failure> {
    let answered = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if answered {
        Ok(())
    } else {
        Err("container_runtime_missing: no container runtime answered `docker info`".into())
    }
}

/// A deployment started for one venue.
pub struct Stack<'v> {
    venue: &'v Venue,
    compose: Compose<'v>,
}

impl<'v> Stack<'v> {
    /// Checks the runtime, renders the venue, confirms the database host
    /// that reaches compose is the configured one, and starts every service.
    pub fn up(venue: &'v Venue) -> Result<Self, Failure> {
        require_container_runtime()?;
        let compose = Compose::new(venue);
        compose.render()?;
        let host = compose.environment("rauthy", "PG_HOST")?;
        if host != venue.database_host() {
            return Err(format!(
                "compose resolved PG_HOST {host}, not {}",
                venue.database_host()
            )
            .into());
        }
        let stack = Self { venue, compose };
        stack.compose.ok(&["up", "-d"])?;
        Ok(stack)
    }

    /// The venue's compose.
    pub fn compose(&self) -> &Compose<'v> {
        &self.compose
    }

    /// `lys --json identity <subcommand> --config <venue config>`, with the
    /// repository's theme mapping for `configure`. The status is the
    /// caller's to judge; a printed secret is refused by [`Venue::lys`].
    pub fn identity(&self, subcommand: &str) -> Result<Output, Failure> {
        let config = self.venue.config().to_string_lossy().into_owned();
        let themes = repo_root()
            .join("deploy/identity/rauthy-themes.json")
            .to_string_lossy()
            .into_owned();
        let mut args = vec![
            "--json",
            "identity",
            subcommand,
            "--config",
            config.as_str(),
        ];
        if subcommand == "configure" {
            args.extend(["--themes", themes.as_str()]);
        }
        self.venue.lys(&args)
    }

    /// One `health` run.
    pub fn health(&self) -> Result<Output, Failure> {
        self.identity("health")
    }

    /// One `configure` run, required to succeed; its JSON report.
    pub fn configure(&self) -> Result<Value, Failure> {
        let output = self.identity("configure")?;
        if !output.status.success() {
            return Err(format!("configure_failed: {}", text(&output.stderr)).into());
        }
        json_of(&output)
    }

    /// Polls `health` until every declared service is ready and, for a
    /// local database, the administrator's `select 1` answers; returns the
    /// health report. It sets no deadline of its own: the gate running the
    /// identity leg bounds how long it waits.
    pub fn wait_ready(&self) -> Result<Value, Failure> {
        loop {
            let output = self.health()?;
            if output.status.success()
                && (!self.venue.local_database()
                    || self.psql("identity_admin", "select 1")?.status.success())
            {
                return json_of(&output);
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    /// `psql` inside the `PostgreSQL` container as `user`, with the password
    /// passed through the environment rather than the command line.
    pub fn psql(&self, user: &str, sql: &str) -> Result<Output, Failure> {
        let secret = match user {
            "rauthy" => "rauthy_db_password",
            "spicedb" => "spicedb_db_password",
            _ => "pg_admin_password",
        };
        let mut command = self.compose.command(&[
            "exec",
            "-T",
            "-e",
            "PGPASSWORD",
            "postgres",
            "psql",
            "-h",
            "127.0.0.1",
            "-U",
            user,
            "-d",
            "identity",
            "-v",
            "ON_ERROR_STOP=1",
            "-Atc",
            sql,
        ]);
        command.env("PGPASSWORD", self.venue.secret(secret)?);
        Ok(command.output()?)
    }
}

impl Drop for Stack<'_> {
    fn drop(&mut self) {
        match self.compose.run(&["down", "-v", "--remove-orphans"]) {
            Ok(output) if output.status.success() => {}
            Ok(output) => eprintln!("compose down failed: {}", text(&output.stderr)),
            Err(error) => eprintln!("compose down did not run: {error}"),
        }
    }
}
