//! Native containment authority is bound to the same versioned tool policy.
//! Canonical bytes cover every declared enforcement input; a digest is never
//! accepted as a caller's label. Signature verification belongs to the existing
//! runner protocol, and OS enforcement belongs to the native launch owner.

use std::collections::BTreeSet;
use std::path::{Component, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::judge::Policy;
use crate::protocol::hex;

/// Domain and version of the canonical policy encoding. A changed encoding
/// requires a new domain; historical bytes are never silently re-encoded.
pub const POLICY_DOMAIN: &str = "lys-containment-policy/v1";

/// An OS facility required before any untrusted instruction is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Writes are confined by the kernel, including descendants.
    Filesystem,
    /// Hostname egress cannot be bypassed by a direct connection.
    HostnameEgress,
    /// Kernel denials are attributable and their source can be followed.
    KernelAudit,
    /// Tool-only rules have an independently proved enforcement owner.
    ToolJudge,
}

/// Server-derived authority for one policy version. These are declared
/// inputs, not an agent-supplied configuration or a runner-local allow list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePolicy {
    /// The exact policy the shared tool judge uses.
    pub tools: Policy,
    /// The session's isolated writable home directory.
    pub home: PathBuf,
    /// The session's writable workspace directory.
    pub workspace: PathBuf,
    /// Exact declared runtime inputs readable by the contained process.
    pub runtime_reads: BTreeSet<PathBuf>,
    /// Paths containing control, audit or credential material, outside writes.
    pub protected: BTreeSet<PathBuf>,
    /// Names the policy allows the egress owner to resolve and connect to.
    pub allowed_hosts: BTreeSet<String>,
    /// Explicit denials remain denials even beside another allowed name.
    pub denied_hosts: BTreeSet<String>,
    /// Every required facility must have actual native readiness evidence.
    pub required: BTreeSet<Capability>,
}

fn refused(name: &str, words: impl Into<String>) -> RunnerError {
    RunnerError::refused(name, words)
}

fn plain(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
        && path.to_str().is_some_and(|text| {
            !text.contains('\0')
                && !text.contains("//")
                && !text.contains("/./")
                && !text.ends_with("/.")
                && (text == "/" || !text.ends_with('/'))
        })
}

fn host(name: &str) -> bool {
    !name.is_empty()
        && name.is_ascii()
        && name == name.to_ascii_lowercase()
        && name.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
        && name.parse::<std::net::IpAddr>().is_err()
}

impl NativePolicy {
    /// Validate declared inputs before encoding. Real directory objects and
    /// current privileges are checked separately at native preparation time.
    pub fn validate(&self) -> Result<(), RunnerError> {
        self.tools
            .clone()
            .checked()
            .map_err(|error| refused(error.refusal, error.words))?;
        for path in [&self.home, &self.workspace]
            .into_iter()
            .chain(&self.runtime_reads)
            .chain(&self.protected)
        {
            if !plain(path) {
                return Err(refused(
                    "containment_path_invalid",
                    format!("{} is not an absolute plain path", path.display()),
                ));
            }
        }
        for root in [&self.home, &self.workspace] {
            if root.parent().is_none() {
                return Err(refused(
                    "containment_path_invalid",
                    "the filesystem root cannot be a writable root",
                ));
            }
            for protected in &self.protected {
                if protected.starts_with(root) || root.starts_with(protected) {
                    return Err(refused(
                        "containment_control_exposed",
                        format!(
                            "writable root {} overlaps protected path {}",
                            root.display(),
                            protected.display()
                        ),
                    ));
                }
            }
            for readonly in &self.runtime_reads {
                if readonly.starts_with(root) || root.starts_with(readonly) {
                    return Err(refused(
                        "containment_readonly_writable",
                        format!(
                            "runtime input {} lies inside writable root {}",
                            readonly.display(),
                            root.display()
                        ),
                    ));
                }
            }
        }
        for name in self.allowed_hosts.union(&self.denied_hosts) {
            if !host(name) {
                return Err(refused(
                    "containment_host_invalid",
                    format!("{name} is not a canonical destination hostname"),
                ));
            }
        }
        for capability in [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ] {
            if !self.required.contains(&capability) {
                return Err(refused(
                    "containment_capability_missing",
                    format!("policy omits required {capability:?}"),
                ));
            }
        }
        if !self.tools.rules.is_empty() && !self.required.contains(&Capability::ToolJudge) {
            return Err(refused(
                "containment_capability_missing",
                "tool policy requires ToolJudge evidence",
            ));
        }
        Ok(())
    }

    /// Canonical, versioned bytes. Struct fields have declared order, sets
    /// have sorted order and rule order is preserved because the judge uses it.
    pub fn canonical(&self) -> Result<Vec<u8>, RunnerError> {
        self.validate()?;
        let mut bytes = POLICY_DOMAIN.as_bytes().to_vec();
        bytes.push(b'\n');
        serde_json::to_writer(&mut bytes, self).map_err(|error| {
            refused(
                "containment_policy_encoding",
                format!("policy {}: {error}", self.tools.version),
            )
        })?;
        Ok(bytes)
    }

    /// Digest recomputed from all enforcement-relevant declared inputs.
    pub fn digest(&self) -> Result<String, RunnerError> {
        Ok(hex(&Sha256::digest(self.canonical()?)))
    }
}

/// The immutable audience of a prepared plan; compared with the server's
/// admitted launch, never with an agent's self-description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Provisioned runner identity.
    pub runner: String,
    /// Admitted session identity.
    pub session: String,
    /// Fresh session lifetime identity, not a reusable OS process id.
    pub incarnation: String,
    /// Policy's bound agent identity.
    pub agent: String,
    /// Current server-authorised policy version.
    pub revision: u64,
    /// Expected canonical policy digest from the authenticated launch.
    pub digest: String,
}

/// A plan received only inside the server-signed launch protocol. Checking
/// this value does not substitute for verifying that signature and challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Expected launch audience and policy authority.
    pub binding: Binding,
    /// The complete policy, from which its digest is recomputed.
    pub policy: NativePolicy,
}

impl Plan {
    /// Refuse any stale, differently addressed or modified policy before
    /// native preparation. The expected binding belongs to the launch owner.
    pub fn validate(&self, expected: &Binding) -> Result<(), RunnerError> {
        for (field, value) in [
            ("runner", &expected.runner),
            ("session", &expected.session),
            ("incarnation", &expected.incarnation),
            ("agent", &expected.agent),
        ] {
            if value.is_empty() {
                return Err(refused(
                    "containment_binding_invalid",
                    format!("expected {field} is empty"),
                ));
            }
        }
        if &self.binding != expected {
            return Err(refused(
                "containment_binding_mismatch",
                format!(
                    "plan does not match admitted session {} on runner {}",
                    expected.session, expected.runner
                ),
            ));
        }
        if self.policy.tools.agent != expected.agent
            || self.policy.tools.version != expected.revision
        {
            return Err(refused(
                "containment_policy_stale",
                format!(
                    "session {} requires agent {} policy revision {}",
                    expected.session, expected.agent, expected.revision
                ),
            ));
        }
        if self.policy.digest()? != expected.digest {
            return Err(refused(
                "containment_policy_digest_mismatch",
                format!(
                    "session {} policy bytes differ from the admitted digest",
                    expected.session
                ),
            ));
        }
        Ok(())
    }
}
