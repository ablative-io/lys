//! Native Codex permissions are rendered from the bound containment plan.
//! This is a config fragment for the isolated home owner, not an enforcement
//! receipt. Provider, notify and MCP settings remain owned by that writer.

use std::collections::BTreeMap;

use serde_json::json;

use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

/// The profile selected by a managed Codex session, never inherited from a
/// project configuration or supplied by an agent's launch arguments.
pub const PROFILE: &str = "lys-bound";

fn line(out: &mut String, key: &str, value: &str) {
    out.push_str(&json!(key).to_string());
    out.push_str(" = ");
    out.push_str(&json!(value).to_string());
    out.push('\n');
}

fn table(out: &mut String, section: &str) {
    out.push_str("\n[permissions.");
    out.push_str(PROFILE);
    out.push('.');
    out.push_str(section);
    out.push_str("]\n");
}

fn path_text(path: &std::path::Path) -> Result<&str, RunnerError> {
    let text = path.to_str().ok_or_else(|| {
        RunnerError::refused(
            "codex_policy_unrepresentable",
            format!("{} is not a UTF-8 path", path.display()),
        )
    })?;
    if text.contains(['*', '?', '[', ']', '{', '}']) {
        return Err(RunnerError::refused(
            "codex_policy_unrepresentable",
            format!(
                "{} contains native glob syntax; a literal directory cannot become a pattern",
                path.display()
            ),
        ));
    }
    Ok(text)
}

/// Render only the policy-owned config keys after verifying the complete
/// plan binding and digest. The launch owner merges admitted non-policy
/// settings, refuses widening overrides and verifies native effective config.
/// Tool rules still require the plan's separately proved judge capability.
pub fn render(plan: &Plan, expected: &Binding) -> Result<String, RunnerError> {
    plan.validate(expected)?;
    let policy = &plan.policy;
    let mut files = BTreeMap::new();
    files.insert("/", "deny");
    for path in &policy.runtime_reads {
        files.insert(path_text(path)?, "read");
    }
    for path in [&policy.home, &policy.workspace] {
        files.insert(path_text(path)?, "write");
    }
    for path in &policy.protected {
        for read in &policy.runtime_reads {
            if read.starts_with(path) || path.starts_with(read) {
                return Err(RunnerError::refused(
                    "codex_policy_unrepresentable",
                    format!(
                        "runtime input {} overlaps protected path {}",
                        read.display(),
                        path.display(),
                    ),
                ));
            }
        }
        files.insert(path_text(path)?, "deny");
    }
    let mut out = String::new();
    line(&mut out, "approval_policy", "never");
    line(&mut out, "default_permissions", PROFILE);
    table(&mut out, "filesystem");
    for (path, access) in files {
        line(&mut out, path, access);
    }
    table(&mut out, "network");
    out.push_str(concat!(
        "enabled = true\n",
        "mode = \"full\"\n",
        "enable_socks5 = false\n",
        "enable_socks5_udp = false\n",
        "allow_upstream_proxy = false\n",
        "dangerously_allow_non_loopback_proxy = false\n",
        "dangerously_allow_all_unix_sockets = false\n",
        "allow_local_binding = false\n",
    ));
    // Full here permits HTTP methods, not unrestricted hosts or filesystem
    // access. An empty domain map still means no permitted destination.
    table(&mut out, "network.domains");
    let mut hosts = BTreeMap::new();
    for host in &policy.allowed_hosts {
        hosts.insert(host.as_str(), "allow");
    }
    for host in &policy.denied_hosts {
        hosts.insert(host.as_str(), "deny");
    }
    for (host, access) in hosts {
        line(&mut out, host, access);
    }
    Ok(out)
}
