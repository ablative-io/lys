//! Minimal directory and policy stores for policy scenarios.
use identity_contract::apps::BEA;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::agent_policy_store::PolicyStore;
use lys_identity_server::routes::open_directory;
use lys_runner::judge::Policy;
use std::error::Error;
use std::sync::Arc;

pub async fn table(keep_explicit: bool) -> Result<(Service, [String; 2]), Box<dyn Error>> {
    Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        move |config| {
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let mut directory = open_directory(config)?;
            let (person, _) = directory.setup_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Owner")?,
                1,
            )?;
            let (missing, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new("Missing")?,
                2,
            )?;
            let (other, _) = directory.register_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Other")?,
                3,
            )?;
            directory.bind_login(
                actor.clone(),
                OperationId::generate()?,
                other,
                LoginBinding::new(&config.issuer, BEA)?,
                4,
            )?;
            directory.transition(
                actor.clone(),
                OperationId::generate()?,
                IdentityId::Person(other),
                Transition::Activate,
                "",
                5,
            )?;
            let (explicit, _) = directory.register_agent(
                actor,
                OperationId::generate()?,
                other,
                Profile::new("Explicit")?,
                3,
            )?;
            let mut policies = PolicyStore::open(
                config.policies_dir.as_deref().ok_or("policies missing")?,
                Arc::new(Ed25519Identity::load(&config.event_key_file)?),
            )?;
            for version in 0..if keep_explicit { 2 } else { 0 } {
                policies.set(
                    Policy {
                        version: version + 1,
                        agent: explicit.to_string(),
                        rules: Vec::new(),
                    },
                    version,
                )?;
            }
            let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
            drop(
                lys_identity_server::configuration_store::ConfigurationStore::open(
                    &config.log_dir.with_file_name("organisation"),
                    Arc::clone(&key),
                )?,
            );
            drop(lys_identity_server::runner_acts::ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            drop(lys_identity::start::LaunchRecords::open(
                &config.log_dir.with_file_name("launch-records"),
                Ed25519Identity::load(&config.event_key_file)?,
            )?);
            drop(lys_identity_server::apps_api::opened(config, key, &|_| {})?);
            Ok([missing.to_string(), explicit.to_string()])
        },
    )
    .await
}
