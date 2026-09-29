//! Compile a bound policy to Seatbelt arguments without granting launch readiness.
//! Paths are parameters, never profile source. This module neither starts an
//! agent nor claims kernel audit or hostname-proxy readiness.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::containment_inputs::Inputs;
use crate::containment_paths::Directory;
use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

/// Apple's fixed entry point; never resolved through an agent's PATH.
pub const EXECUTABLE: &str = "/usr/bin/sandbox-exec";

/// The runner-owned standard I/O transport. Pipes grant no terminal path.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Stdio {
    /// Three inherited pipes owned by the managed transport.
    Pipes,
    /// The exact pseudo-terminal owned by this launch.
    Terminal {
        /// Native slave path obtained from the PTY owner.
        path: PathBuf,
    },
}

/// A compiled profile and its literal path parameters. Possession of this
/// value is preparation only, not evidence that a process is contained.
#[derive(Debug)]
pub struct Profile {
    binding: Binding,
    source: String,
    parameters: Vec<String>,
}

fn refused(words: impl Into<String>) -> RunnerError {
    RunnerError::refused("containment_unavailable", words)
}

fn rule(source: &mut String, prefix: &str, value: &str) {
    source.push_str(prefix);
    source.push_str(value);
    source.push_str("))\n");
}

fn parameter(parameters: &mut Vec<String>, path: &Path) -> Result<String, RunnerError> {
    let text = path
        .to_str()
        .ok_or_else(|| refused(format!("Seatbelt cannot encode path {}", path.display())))?;
    if text.contains('\0') {
        return Err(refused(format!(
            "Seatbelt path {} contains NUL",
            path.display()
        )));
    }
    let name = format!("LYS_PATH_{}", parameters.len());
    parameters.push(format!("-D{name}={text}"));
    Ok(format!("(param \"{name}\")"))
}

impl Profile {
    /// Compile the filesystem and deny-all-network policy. A policy allowing
    /// destinations requires the owned hostname egress service and is refused
    /// here until that service can supply its bound endpoint. It is never
    /// silently converted to direct network access or to a weaker host list.
    /// The terminal path must come from the runner's newly opened PTY, not a
    /// request field. Runtime reads come only from the authenticated plan.
    pub fn compile(plan: &Plan, expected: &Binding, stdio: &Stdio) -> Result<Self, RunnerError> {
        plan.validate(expected)?;
        if plan
            .policy
            .allowed_hosts
            .difference(&plan.policy.denied_hosts)
            .next()
            .is_some()
        {
            return Err(refused("HostnameEgress: no bound hostname egress service"));
        }
        if let Stdio::Terminal { path: terminal } = stdio {
            let tty = terminal
                .to_str()
                .ok_or_else(|| refused("PTY path is not UTF-8"))?;
            let suffix = tty
                .strip_prefix("/dev/ttys")
                .ok_or_else(|| refused("PTY is not a macOS slave terminal"))?;
            if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(refused("PTY is not a macOS slave terminal"));
            }
        }
        let mut source = String::from(concat!(
            "(version 1)\n(deny default)\n",
            "(allow process-exec)\n(allow process-fork)\n",
            "(allow signal (target same-sandbox))\n",
            "(allow process-info* (target same-sandbox))\n",
            "(deny system-fcntl (fcntl-command 80 110))\n",
            // dyld opens the root directory while locating its shared cache.
            // This permits that directory object, never its descendant data.
            "(allow file-read-data (require-all (literal \"/\") (vnode-type DIRECTORY)))\n",
        ));
        let mut parameters = Vec::new();
        for root in [&plan.policy.home, &plan.policy.workspace] {
            let value = parameter(&mut parameters, root)?;
            rule(
                &mut source,
                "(allow file-read* file-write* (subpath ",
                &value,
            );
            // A process must not move its root and acquire authority to a
            // replacement path under the already compiled pathname rule.
            rule(&mut source, "(deny file-write-unlink (literal ", &value);
        }
        for root in &plan.policy.runtime_reads {
            let value = parameter(&mut parameters, root)?;
            rule(&mut source, "(allow file-read* (subpath ", &value);
        }
        // Explicit denials remain effective beneath a declared readable tree.
        for root in &plan.policy.protected {
            let value = parameter(&mut parameters, root)?;
            rule(
                &mut source,
                "(deny file-read* file-write* (subpath ",
                &value,
            );
        }
        if let Stdio::Terminal { path: terminal } = stdio {
            let value = parameter(&mut parameters, terminal)?;
            rule(
                &mut source,
                "(allow file-read* file-write-data file-ioctl (literal ",
                &value,
            );
        }
        Ok(Self {
            binding: expected.clone(),
            source,
            parameters,
        })
    }

    /// Inspect the actual source for review and native venue fixtures.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Produce argv with a fixed sandbox entry point and no shell evaluation.
    /// The caller must independently establish native readiness, keep the
    /// writable roots held, and clear its inherited environment/descriptors.
    pub fn arguments(
        &self,
        expected: &Binding,
        program: &Path,
        arguments: &[String],
    ) -> Result<Vec<String>, RunnerError> {
        if &self.binding != expected {
            return Err(refused(
                "Seatbelt profile belongs to another policy or session",
            ));
        }
        if !program.is_absolute() {
            return Err(refused(format!(
                "Seatbelt program {} is not absolute",
                program.display()
            )));
        }
        let program = program
            .to_str()
            .filter(|value| !value.contains('\0'))
            .ok_or_else(|| refused("Seatbelt program is not a UTF-8 executable path"))?;
        if arguments.iter().any(|value| value.contains('\0')) {
            return Err(refused("Seatbelt executable argument contains NUL"));
        }
        let mut result = vec!["-p".to_owned(), self.source.clone()];
        result.extend(self.parameters.iter().cloned());
        result.push(program.to_owned());
        result.extend(arguments.iter().cloned());
        Ok(result)
    }
}

/// Held writable directory identities belonging to one authenticated plan.
/// This is not a kernel readiness receipt. A native owner must recheck at
/// launch and account for pathname replacement races at its exec boundary.
#[derive(Debug)]
pub struct Roots {
    binding: Binding,
    home: Directory,
    workspace: Directory,
    paths: [PathBuf; 2],
    inputs: Inputs,
}

impl Roots {
    /// Open both roots without following any symlink in either path.
    pub fn open(plan: &Plan, expected: &Binding) -> Result<Self, RunnerError> {
        plan.validate(expected)?;
        Ok(Self {
            binding: expected.clone(),
            home: Directory::open(&plan.policy.home)?,
            workspace: Directory::open(&plan.policy.workspace)?,
            paths: [plan.policy.home.clone(), plan.policy.workspace.clone()],
            inputs: Inputs::open(plan, expected)?,
        })
    }

    /// Refuse stale preparation or replacement before issuing launch arguments.
    pub fn verify(&self, plan: &Plan, expected: &Binding) -> Result<(), RunnerError> {
        plan.validate(expected)?;
        if &self.binding != expected
            || self.paths != [plan.policy.home.clone(), plan.policy.workspace.clone()]
        {
            return Err(refused(
                "held directories belong to another containment plan",
            ));
        }
        self.home.verify()?;
        self.workspace.verify()?;
        self.inputs.verify(plan, expected)
    }
}
