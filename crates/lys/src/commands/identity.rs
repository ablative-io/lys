//! `lys identity prepare`, `configure` and `health`: load the configuration,
//! run the act, and report it in lines or as one JSON object. Nothing printed
//! here is a secret: paths, modes, operation identifiers and readiness only.

use std::path::Path;

use serde_json::{Value, json};

use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::identity::{
    DeployConfig, configure as configure_act, health as health_act, prepare as prepare_act,
};

/// Validate the configuration and write the venue.
pub fn prepare(config_path: &Path, json: bool) -> CliResult<()> {
    let config = DeployConfig::load(config_path)?;
    let prepared = prepare_act::run(&config)?;
    let mut out = Emitter::new(json);
    out.field("venue", "venue", prepared.venue.display().to_string());
    out.field(
        "credentials",
        "credentials",
        if prepared.credentials_generated {
            "generated"
        } else {
            "reused"
        },
    );
    let files: Vec<Value> = prepared
        .files
        .iter()
        .map(|file| {
            json!({
                "name": file.name,
                "path": file.path.display().to_string(),
                "outcome": file.outcome.word(),
                "mode": format!("{:04o}", file.mode),
            })
        })
        .collect();
    if out.is_json() {
        out.field("files", "files", Value::Array(files));
    } else {
        for file in &prepared.files {
            out.note(&format!(
                "{:<20} {:<9} mode {:04o}",
                file.name,
                file.outcome.word(),
                file.mode
            ));
        }
    }
    out.field("files written", "count", prepared.files.len());
    out.finish();
    Ok(())
}

/// Reconcile the clients and themes on Rauthy.
pub fn configure(config_path: &Path, json: bool) -> CliResult<()> {
    let config = DeployConfig::load(config_path)?;
    let done = configure_act::run(&config)?;
    let mut out = Emitter::new(json);
    out.field("rauthy", "rauthy", config.rauthy_origin());
    let operations: Vec<Value> = done
        .iter()
        .map(|item| json!({"operation": item.operation, "change": item.change.word()}))
        .collect();
    if out.is_json() {
        out.field("operations", "operations", Value::Array(operations));
    } else {
        for item in &done {
            out.note(&format!("{:<28} {}", item.operation, item.change.word()));
        }
    }
    out.finish();
    Ok(())
}

/// Name each declared service as ready or why not.
pub fn health(config_path: &Path, json: bool) -> CliResult<()> {
    let config = DeployConfig::load(config_path)?;
    let (checks, outcome) = health_act::run(&config);
    let mut out = Emitter::new(json);
    let services: Vec<Value> = checks
        .iter()
        .map(|check| {
            json!({
                "service": check.service,
                "address": check.address,
                "ready": check.unready.is_none(),
                "unready": check.unready,
            })
        })
        .collect();
    if out.is_json() {
        out.field("services", "services", Value::Array(services));
        if outcome.is_ok() {
            out.finish();
        }
    } else {
        for check in &checks {
            match &check.unready {
                None => out.note(&format!(
                    "{:<9} ready      {}",
                    check.service, check.address
                )),
                Some(why) => out.note(&format!(
                    "{:<9} NOT READY  {}: {why}",
                    check.service, check.address
                )),
            }
        }
    }
    outcome?;
    Ok(())
}
