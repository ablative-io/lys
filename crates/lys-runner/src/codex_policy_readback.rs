//! Check the selected native config against the authenticated plan before launch.
//! Config readback is evidence of loaded settings, not a sandbox readiness receipt.

use serde_json::{Value, json};

use crate::codex_policy::{PROFILE, render};
use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

fn mismatch(field: &str) -> RunnerError {
    RunnerError::refused(
        "codex_effective_policy_mismatch",
        format!("native {field} differs from the admitted containment plan"),
    )
}

fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key.clone(), without_nulls(value)))
                .collect(),
        ),
        value => value.clone(),
    }
}

/// Verify the `config` object returned by native config/read, for the same
/// session-owned transport which will launch the agent. Return the validated
/// plan digest; the caller still must reject later thread/resume overrides.
/// No source cursor or enforcement state is advanced by this function.
pub fn verify(plan: &Plan, expected: &Binding, config: &Value) -> Result<String, RunnerError> {
    // Includes validation of native representability, not just the hash.
    render(plan, expected)?;
    for (field, value) in [
        ("approval_policy", "never"),
        ("default_permissions", PROFILE),
    ] {
        if config.get(field).and_then(Value::as_str) != Some(value) {
            return Err(mismatch(field));
        }
    }
    for field in ["sandbox_mode", "sandbox_workspace_write", "profile"] {
        if config.get(field).is_some_and(|value| !value.is_null()) {
            return Err(mismatch(field));
        }
    }
    let mut filesystem = serde_json::Map::new();
    filesystem.insert("/".to_owned(), json!("deny"));
    for path in &plan.policy.runtime_reads {
        filesystem.insert(path.display().to_string(), json!("read"));
    }
    for path in [&plan.policy.home, &plan.policy.workspace] {
        filesystem.insert(path.display().to_string(), json!("write"));
    }
    for path in &plan.policy.protected {
        filesystem.insert(path.display().to_string(), json!("deny"));
    }
    let mut domains = serde_json::Map::new();
    for host in &plan.policy.allowed_hosts {
        domains.insert(host.clone(), json!("allow"));
    }
    for host in &plan.policy.denied_hosts {
        domains.insert(host.clone(), json!("deny"));
    }
    let profile = config
        .get("permissions")
        .and_then(|profiles| profiles.get(PROFILE))
        .ok_or_else(|| mismatch("permissions.lys-bound"))?;
    let mut actual = without_nulls(profile);
    // A profile description has no enforcement effect.
    if let Some(fields) = actual.as_object_mut() {
        fields.remove("description");
    }
    let desired = json!({"filesystem":filesystem, "network":{
        "enabled":true, "mode":"full", "enable_socks5":false,
        "enable_socks5_udp":false, "allow_upstream_proxy":false,
        "dangerously_allow_non_loopback_proxy":false,
        "dangerously_allow_all_unix_sockets":false, "allow_local_binding":false,
        "domains":domains
    }});
    if actual != desired {
        return Err(mismatch("permission profile"));
    }
    Ok(expected.digest.clone())
}
