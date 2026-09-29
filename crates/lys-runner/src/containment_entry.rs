//! Native launch preparation never substitutes for owned audit and egress readiness.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::containment_macos::{EXECUTABLE, Profile, Roots, Stdio};
use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

/// Sets agent variables only after Seatbelt has been applied. This fixed
/// executable must itself be covered by the plan's declared runtime reads.
pub const ENV_EXECUTABLE: &str = "/usr/bin/env";

/// Private runner-to-helper input, carried only after the signed launch has
/// been checked by the runner. This is not an additional network protocol.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Complete admitted policy.
    pub plan: Plan,
    /// Independently admitted launch audience supplied by the runner.
    pub expected: Binding,
    /// Pipes or the actual slave terminal created and owned by that runner.
    pub stdio: Stdio,
    /// Harness executable, passed to sandbox-exec without a shell.
    pub program: PathBuf,
    /// Harness argv, not shell source.
    pub arguments: Vec<String>,
    /// Explicit launch variables; the runner's environment is never inherited.
    /// Account secrets must remain at their existing broker, outside this input.
    pub environment: BTreeMap<String, String>,
}

fn refused(words: impl Into<String>) -> RunnerError {
    RunnerError::refused("containment_entry_refused", words)
}

impl Input {
    /// Prepare an exact native command for inspection or helper exec. The
    /// helper entry point is fixed, HOME belongs to the plan, and agent
    /// variables take effect only inside the applied sandbox.
    pub fn command(&self) -> Result<Command, RunnerError> {
        let profile = Profile::compile(&self.plan, &self.expected, &self.stdio)?;
        let environment_program = std::path::Path::new(ENV_EXECUTABLE);
        if !self
            .plan
            .policy
            .runtime_reads
            .iter()
            .any(|root| environment_program.starts_with(root))
            || self
                .plan
                .policy
                .protected
                .iter()
                .any(|root| environment_program.starts_with(root))
        {
            return Err(refused(format!(
                "declared runtime reads do not permit {ENV_EXECUTABLE}"
            )));
        }
        let program = self
            .program
            .to_str()
            .filter(|value| self.program.is_absolute() && !value.contains('\0'))
            .ok_or_else(|| refused("harness program must be an absolute UTF-8 path without NUL"))?;
        if program.contains('=') {
            return Err(refused(
                "harness program contains '=' and would be parsed as an environment assignment",
            ));
        }
        for (key, value) in &self.environment {
            if key.is_empty() || key.contains(['\0', '=']) || value.contains('\0') {
                return Err(refused(format!(
                    "environment variable {key:?} is not representable"
                )));
            }
            if key.starts_with("DYLD_") || key.starts_with("LD_") {
                return Err(refused(format!(
                    "environment variable {key} changes the trusted native loader"
                )));
            }
            if key == "HOME" && std::ffi::OsStr::new(value) != self.plan.policy.home.as_os_str() {
                return Err(refused("HOME differs from the admitted isolated home"));
            }
        }
        let mut environment = self.environment.clone();
        environment.insert(
            "HOME".to_owned(),
            self.plan.policy.home.display().to_string(),
        );
        let mut inside = vec!["-i".to_owned(), "--".to_owned()];
        inside.extend(
            environment
                .iter()
                .map(|(key, value)| format!("{key}={value}")),
        );
        inside.push(program.to_owned());
        inside.extend(self.arguments.iter().cloned());
        let arguments = profile.arguments(&self.expected, environment_program, &inside)?;
        let mut command = Command::new(EXECUTABLE);
        command
            .args(arguments)
            .env_clear()
            .current_dir(&self.plan.policy.workspace);
        Ok(command)
    }
}

/// Refuse native admission until the runner has an actual bound audit owner.
/// Structured input cannot supply readiness, and lowering `required` cannot
/// bypass this check. Command preparation remains available for controlled
/// backend probes; it is never an admitted agent launch or an enforced receipt.
pub fn exec(input: &Input) -> Result<Infallible, RunnerError> {
    if !cfg!(target_os = "macos") {
        return Err(refused("Seatbelt requires a native macOS host"));
    }
    input.command()?;
    crate::containment_stdio::verify(&input.stdio)?;
    let roots = Roots::open(&input.plan, &input.expected)?;
    roots.verify(&input.plan, &input.expected)?;
    Err(RunnerError::refused(
        "containment_unavailable",
        format!(
            "session {} on runner {} requires KernelAudit: no bound native audit lifetime is installed; no agent was executed",
            input.expected.session, input.expected.runner
        ),
    ))
}
