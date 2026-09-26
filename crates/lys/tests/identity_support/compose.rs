//! `docker compose` for one venue: run from the repository root against
//! deploy/identity/compose.yaml, with the environment `lys identity prepare`
//! renders from the venue's config.
//!
//! The resolved config compose prints carries the rendered credentials, so
//! it is only ever parsed here, never printed.

use std::process::{Command, Output};

use serde_json::Value;

use super::fixtures::{Failure, Venue, json_of, repo_root, text};

/// The compose file, from the repository root.
pub const COMPOSE_FILE: &str = "deploy/identity/compose.yaml";

/// `docker compose` bound to one venue.
pub struct Compose<'v> {
    venue: &'v Venue,
}

impl<'v> Compose<'v> {
    /// Compose for `venue`.
    pub fn new(venue: &'v Venue) -> Self {
        Self { venue }
    }

    /// Renders the venue's config through `lys identity prepare` into
    /// identity.env, the environment deploy/identity/compose.yaml
    /// interpolates.
    pub fn render(&self) -> Result<Value, Failure> {
        let config = self.venue.config();
        let output = self.venue.lys(&[
            "--json",
            "identity",
            "prepare",
            "--config",
            &config.to_string_lossy(),
        ])?;
        if !output.status.success() {
            return Err(format!("prepare_failed: {}", text(&output.stderr)).into());
        }
        json_of(&output)
    }

    /// `docker compose -f deploy/identity/compose.yaml -p <project>
    /// --env-file <private>/identity.env [--profile database] <args>`.
    pub fn command(&self, args: &[&str]) -> Command {
        let env_file = self.venue.private_dir().join("identity.env");
        let mut command = Command::new("docker");
        command
            .current_dir(repo_root())
            .args(["compose", "-f", COMPOSE_FILE, "-p"]);
        command
            .arg(self.venue.project())
            .arg("--env-file")
            .arg(env_file);
        if self.venue.local_database() {
            command.args(["--profile", "database"]);
        }
        command.args(args);
        command
    }

    /// Runs compose; the status is the caller's to judge.
    pub fn run(&self, args: &[&str]) -> Result<Output, Failure> {
        Ok(self.command(args).output()?)
    }

    /// Runs compose and requires success.
    pub fn ok(&self, args: &[&str]) -> Result<Output, Failure> {
        let output = self.run(args)?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(format!(
                "compose_failed: docker compose {}: {}",
                args.join(" "),
                text(&output.stderr)
            )
            .into())
        }
    }

    /// The resolved compose config as JSON.
    pub fn config(&self) -> Result<Value, Failure> {
        let output = self.ok(&["config", "--format", "json"])?;
        json_of(&output)
    }

    /// One resolved environment variable of one service.
    pub fn environment(&self, service: &str, name: &str) -> Result<String, Failure> {
        let config = self.config()?;
        config["services"][service]["environment"][name]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("compose config has no {service} {name}").into())
    }
}
