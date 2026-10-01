//! Build issuer-independent stores through their normal typed constructors.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_identity::signer::load_service_key;
use lys_identity_server::Config;
use lys_identity_server::{
    agent_policy_store::PolicyStore,
    apps_api,
    budgets_store::BudgetStore,
    certificates_store::CertificateStore,
    configuration_store::ConfigurationStore,
    goals_store::GoalStore,
    mcp_requests_store::McpRequestStore,
    requests_store::RequestStore,
    reviews_store::ReviewStore,
    routes::open_directory,
    runner_acts::ActStore,
    runtime_store::RuntimeStore,
    service_accounts_store::ServiceAccountStore,
    stops_store::StopStore,
    teams_state::{Checked, Line},
    teams_store::TeamStore,
};

pub(crate) fn paths(config: &Config) -> Vec<(&'static str, PathBuf)> {
    let mut paths = vec![
        ("log", config.log_dir.clone()),
        (
            "organisation",
            config.log_dir.with_file_name("organisation"),
        ),
        ("runner-acts", config.log_dir.with_file_name("runner-acts")),
        (
            "launch-records",
            config.log_dir.with_file_name("launch-records"),
        ),
        ("apps", config.apps_dir()),
    ];
    for (name, path) in [
        ("requests", &config.requests_dir),
        ("certificates", &config.certificates_dir),
        ("runtime", &config.runtime_dir),
        ("service-accounts", &config.service_accounts_dir),
        ("reviews", &config.reviews_dir),
        ("teams", &config.teams_dir),
        ("stops", &config.stops_dir),
        ("budgets", &config.budgets_dir),
        ("policies", &config.policies_dir),
        ("goals", &config.goals_dir),
    ] {
        if let Some(path) = path {
            paths.push((name, path.clone()));
        }
    }
    if let Some(path) = &config.requests_dir {
        paths.push(("mcp-requests", path.with_file_name("mcp-requests")));
    }
    paths
}

fn snapshot_result(failure: Option<&str>) -> Result<(), Box<dyn Error>> {
    match failure {
        Some(reason) => Err(format!("service template snapshot failed: {reason}").into()),
        None => Ok(()),
    }
}

pub(crate) fn build(name: &str, path: &Path, config: &Config) -> Result<(), Box<dyn Error>> {
    let key = Arc::new(load_service_key(&config.event_key_file)?);
    match name {
        "log" => {
            let mut configured = config.clone();
            path.clone_into(&mut configured.log_dir);
            snapshot_result(open_directory(&configured)?.log()?.snapshot_failure())?;
        }
        "organisation" => {
            drop(ConfigurationStore::open(path, key)?);
        }
        "runner-acts" => {
            snapshot_result(ActStore::open(path, key)?.snapshot_failure())?;
        }
        "launch-records" => {
            snapshot_result(
                lys_identity::start::LaunchRecords::open(
                    path,
                    load_service_key(&config.event_key_file)?,
                )?
                .snapshot_failure(),
            )?;
        }
        "apps" => {
            let mut configured = config.clone();
            configured.grant_log_dir = path.with_file_name("grant-log");
            snapshot_result(apps_api::opened(&configured, key, &|_| {})?.snapshot_failure())?;
        }
        "requests" => {
            snapshot_result(RequestStore::open(path, key)?.snapshot_failure())?;
        }
        "certificates" => {
            snapshot_result(CertificateStore::open(path, key)?.snapshot_failure())?;
        }
        "runtime" => {
            snapshot_result(RuntimeStore::open(path, key)?.snapshot_failure())?;
        }
        "service-accounts" => {
            snapshot_result(ServiceAccountStore::open(path, key)?.snapshot_failure())?;
        }
        "reviews" => {
            snapshot_result(ReviewStore::open(path, key)?.snapshot_failure())?;
        }
        "teams" => {
            let mut teams = TeamStore::open(path, key)?;
            teams.stage_migration(vec![Line::Checked(Checked {
                operation: "lys/teams/legacy-membership/v1/checked".to_owned(),
                at: 0,
            })])?;
            teams.finish_migration()?;
            snapshot_result(teams.snapshot_failure())?;
        }
        "stops" => {
            snapshot_result(StopStore::open(path, key)?.snapshot_failure())?;
        }
        "budgets" => {
            snapshot_result(BudgetStore::open(path, key)?.snapshot_failure())?;
        }
        "policies" => {
            snapshot_result(PolicyStore::open(path, key)?.snapshot_failure())?;
        }
        "goals" => {
            snapshot_result(GoalStore::open(path, key)?.snapshot_failure())?;
        }
        "mcp-requests" => {
            drop(McpRequestStore::open(path, key)?);
        }
        _ => return Err(format!("unknown service template store {name}").into()),
    }
    // Constructors can keep a snapshot failure for later reporting. A cache
    // entry must contain that snapshot before it can be published.
    std::fs::File::open(path.join("snapshot.bin"))?;
    Ok(())
}
