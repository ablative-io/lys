//! A reviewed seat owns its settings; Lys owns its identity and standard streams.
use crate::RunnerError;
use crate::session::{Table, append};
use crate::tracking_store::{Body, Coverage};

mod launch;
mod pipe;
mod reader;
pub(crate) use launch::prepare;
pub(crate) use pipe::{Pipe, Writer};

pub(crate) const SPEC_VARIABLE: &str = "LYS_SEAT_PROGRAM";
pub(crate) const PROOF_VARIABLE: &str = "LYS_SEAT_PROGRAM_PROOF";

#[cfg(test)]
#[path = "seat_program/model_account_environment_tests.rs"]
mod model_account_environment_tests;

pub(crate) fn environment(
    given: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    use lys_home::harness::claude_code::launch_env::credential_shaped;
    let mut kept: std::collections::BTreeMap<_, _> = std::env::vars_os()
        .filter(|(name, _)| {
            !credential_shaped(&name.to_string_lossy())
                && !matches!(name.to_str(), Some(SPEC_VARIABLE | PROOF_VARIABLE))
        })
        .collect();
    for (name, value) in given {
        if name != SPEC_VARIABLE
            && name != PROOF_VARIABLE
            && (name == "LYS_AGENT_PASS" || !credential_shaped(name))
        {
            kept.insert(
                std::ffi::OsString::from(name),
                std::ffi::OsString::from(value),
            );
        }
    }
    kept
}

pub(crate) fn record(
    table: &mut Table,
    id: &str,
    state: &str,
    value: &serde_json::Value,
) -> Result<(), RunnerError> {
    let words = serde_json::to_string(value)
        .map_err(|error| RunnerError::refused("seat_program_record_invalid", error.to_string()))?;
    append(
        table,
        id,
        vec![Body::Coverage(Coverage {
            state: state.to_owned(),
            words,
            source: None,
            generation: 0,
            offset: None,
            executable: value
                .get("executable")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            harness_version: None,
            adapter: Some("acp/1".to_owned()),
        })],
        None,
    )
}
