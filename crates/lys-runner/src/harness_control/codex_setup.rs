//! Setup frames for the existing single managed connection. This codec opens
//! no connection and makes no package, containment or launch-capability claim.
//! Shapes are pinned to bd3798fa, protocol/v1 Initialize and v2/config ConfigRead.
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Initialize,
    Configuration,
    Verified,
    Refused,
}

/// One initialize/config-read exchange, owned by the same reader and writer
/// that will subsequently own the bound thread. Callers never retry its frames
/// after uncertain writes; they must close that owned transport instead.
pub struct Configuration {
    initialize_id: String,
    config_id: String,
    config_home: PathBuf,
    plan: Plan,
    binding: Binding,
    phase: Phase,
}

fn refuse(name: &str, words: &str) -> RunnerError {
    RunnerError::refused(name, words)
}

impl Configuration {
    /// Prepare initialize for the declared package's existing connection.
    /// The config home is resolved by the launch owner inside its isolated home;
    /// cwd comes from the authenticated plan. The policy is fixed for this
    /// exchange, so a later response cannot be checked against another binding.
    pub fn begin(
        initialize_id: &str,
        config_id: &str,
        config_home: &Path,
        plan: &Plan,
        expected: &Binding,
    ) -> Result<(Self, Value), RunnerError> {
        plan.validate(expected)?;
        if initialize_id.is_empty()
            || config_id.is_empty()
            || initialize_id == config_id
            || !config_home.is_absolute()
            || !config_home.starts_with(&plan.policy.home)
            || config_home.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
            || config_home.to_str().is_none()
        {
            return Err(refuse(
                "control_setup_invalid",
                "setup needs distinct request ids and resolved absolute UTF-8 paths",
            ));
        }
        let frame = json!({"id":initialize_id,"method":"initialize","params":{
            "clientInfo":{"name":"lys-managed-control","title":"Lys","version":env!("CARGO_PKG_VERSION")}
        }});
        Ok((
            Self {
                initialize_id: initialize_id.to_owned(),
                config_id: config_id.to_owned(),
                config_home: config_home.to_owned(),
                plan: plan.clone(),
                binding: expected.clone(),
                phase: Phase::Initialize,
            },
            frame,
        ))
    }

    /// Correlate initialize and verify its actual config home. Return the
    /// initialized notification followed by config/read for this exact cwd;
    /// reading without cwd would omit project layers. Neither frame writes config.
    pub fn initialized(&mut self, response: &Value) -> Result<[Value; 2], RunnerError> {
        if self.phase != Phase::Initialize {
            return Err(refuse(
                "control_setup_unresolved",
                "initialize is not awaiting a response; do not resend",
            ));
        }
        let result = match response_result(response, &self.initialize_id) {
            Ok(result) => result,
            Err(error) => {
                self.phase = Phase::Refused;
                return Err(error);
            }
        };
        if result.get("codexHome").and_then(Value::as_str) != self.config_home.to_str()
            || result
                .get("userAgent")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            || result
                .get("platformFamily")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            || result
                .get("platformOs")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            self.phase = Phase::Refused;
            return Err(refuse(
                "control_source_mismatch",
                "initialize does not prove this connection's expected config home and native identity",
            ));
        }
        self.phase = Phase::Configuration;
        Ok([
            json!({"method":"initialized"}),
            json!({"id":self.config_id,"method":"config/read","params":{
                "includeLayers":false,"cwd":self.plan.policy.workspace
            }}),
        ])
    }

    /// Pass the correlated effective config to the existing 065 verifier.
    /// The verifier owns policy interpretation and returns the admitted digest;
    /// this codec owns response correlation and single-connection ordering.
    /// Success does not start a thread or establish native containment readiness.
    pub fn readback(&mut self, response: &Value) -> Result<String, RunnerError> {
        if self.phase != Phase::Configuration {
            return Err(refuse(
                "control_setup_unresolved",
                "effective config is not awaiting a response; do not resend",
            ));
        }
        // Every failure closes this setup occurrence, including verifier errors.
        self.phase = Phase::Refused;
        let result = response_result(response, &self.config_id)?;
        let config = result
            .get("config")
            .filter(|value| value.is_object())
            .ok_or_else(|| {
                refuse(
                    "control_protocol_unsupported",
                    "config/read returned no effective config object",
                )
            })?;
        let digest = crate::codex_policy_readback::verify(&self.plan, &self.binding, config)?;
        self.phase = Phase::Verified;
        Ok(digest)
    }

    /// Close the occurrence on a write/read gap; it cannot generate a retry.
    pub fn lost(&mut self) {
        self.phase = Phase::Refused;
    }
}

fn response_result<'a>(response: &'a Value, id: &str) -> Result<&'a Value, RunnerError> {
    if response.get("id").and_then(Value::as_str) != Some(id) {
        return Err(refuse(
            "control_source_mismatch",
            "native reply does not name this setup request",
        ));
    }
    if response.get("error").is_some() {
        return Err(refuse(
            "control_request_refused",
            "native setup request failed; its raw payload is not exported",
        ));
    }
    response
        .get("result")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            refuse(
                "control_protocol_unsupported",
                "native setup response has no result object",
            )
        })
}
