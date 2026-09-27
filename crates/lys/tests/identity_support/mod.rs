//! Support for the container-backed identity targets (`identity_deploy`,
//! `identity_refusals`, `identity_shared_db`, `identity_restart`,
//! `identity_theme`). They run only on the identity leg of `.land/gates.sh`,
//! never in `cargo test --workspace`, and use only the container runtime and
//! crates already in `Cargo.lock`.
//!
//! Each target declares this module `pub`, so every item here is reachable
//! from its crate root whichever subset that target calls.

pub mod compose;
pub mod fixtures;
pub mod server;

use std::time::Duration;

use compose::Compose;
use fixtures::{Fixture, output_text, repo_root};

/// A test's result; every failure carries its own message.
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// How long a stack may take to become ready, image pulls included.
pub const READY_LIMIT: Duration = Duration::from_secs(300);

/// A running stack on its own compose project and volumes, removed on drop.
pub struct Stack {
    /// The venue directory and its configuration.
    pub fixture: Fixture,
    /// Its compose project.
    pub compose: Compose,
}

impl Stack {
    /// Prepare a fresh venue on the local database service, bring every
    /// dependency up, and wait until Rauthy and SpiceDB are ready.
    pub fn up(label: &str) -> TestResult<Self> {
        let fixture = Fixture::new(label, "postgres")?;
        let compose = compose::render(&fixture)?;
        let stack = Self { fixture, compose };
        stack.compose.run_ok(&["up", "-d"])?;
        stack.wait_ready()?;
        Ok(stack)
    }

    /// Wait until Rauthy and SpiceDB both answer ready.
    pub fn wait_ready(&self) -> TestResult {
        let rauthy = self.fixture.rauthy();
        let spicedb = self.fixture.spicedb();
        server::wait_for("rauthy", READY_LIMIT, || server::rauthy_healthy(&rauthy))?;
        server::wait_for("spicedb", READY_LIMIT, || server::spicedb_serving(&spicedb))
    }

    /// Run `lys identity configure` with the shipped theme mapping, failing
    /// unless it exits 0; returns its stdout.
    pub fn configure(&self) -> TestResult<String> {
        let themes = repo_root().join("deploy/identity/rauthy-themes.json");
        let themes = themes.to_string_lossy().to_string();
        let output = self.fixture.identity("configure", &["--themes", themes.as_str()])?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(format!("lys identity configure failed: {}", output_text(&output)).into())
        }
    }

    /// Remove the containers (keeping the volumes) and bring them back.
    pub fn restart(&self) -> TestResult {
        self.compose.run_ok(&["down"])?;
        self.compose.run_ok(&["up", "-d"])?;
        self.wait_ready()
    }

    /// `GET` or `POST` a Rauthy admin path with the bootstrap API key.
    pub fn rauthy_admin(&self, method: &str, path: &str) -> TestResult<server::Reply> {
        let key = self.fixture.api_key_header()?;
        server::request(&self.fixture.rauthy(), method, path, &[("Authorization", key.as_str())], None)
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        if let Err(error) = self.compose.down() {
            eprintln!("warning: removing compose project {} failed: {error}", self.compose.project);
        }
    }
}
