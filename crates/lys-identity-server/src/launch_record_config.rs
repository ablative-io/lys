//! A launch record selects one exact reviewed provisioning operation.
use crate::error::ServerError;
use crate::provisioning_store::ProvisioningStore;
use lys_identity::start::LaunchRecord;
use lys_runner::Launch;
use lys_runner::admitted::Admitted;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "launch_record_config_tests.rs"]
mod tests;

pub(crate) fn build(
    store: &ProvisioningStore,
    record: &LaunchRecord,
    session: String,
    runtime: &str,
    policy: Option<Box<Admitted>>,
    model_proxy: Option<&str>,
) -> Result<Launch, ServerError> {
    let (agent, version) =
        store
            .named(&record.profile_version)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: "the launch record names no kept provisioning operation".to_owned(),
            })?;
    if agent != record.agent {
        return Err(ServerError::LaunchUnrenderable {
            reason: "the launch record's provisioning operation belongs to another agent"
                .to_owned(),
        });
    }
    crate::start_checks::reviewed(version)?;
    if runtime.is_empty() {
        return Err(ServerError::MachineWithoutRuntime);
    }
    let skills = crate::launch_harness::skill_files(store, version)?;
    // The rendering gives a Claude Code run the proxy under a key minted for
    // this start alone (`start_checks::fields`), and says the key and the
    // profile version in the template's environment, which the start act
    // reads back to tell the runner.
    let rendered = crate::launch_template::render(
        &crate::launch_template::Start {
            agent: &record.agent,
            session: &session,
            machine: &record.machine,
            runtime,
            version,
            skills: &skills,
            policy: policy.as_ref().map(|admitted| &admitted.policy),
            model_proxy,
        },
        &[],
    )?;
    let mut native = crate::launch_template::from_template(
        version,
        &rendered.template,
        &rendered.template_sha256,
    )?;
    if native.program != record.executable || native.arguments != record.arguments {
        return Err(ServerError::LaunchUnrenderable {
            reason: "the signed command differs from its exact reviewed provisioning operation"
                .to_owned(),
        });
    }
    native.environment.extend(BTreeMap::from([
        ("LYS_AGENT".to_owned(), record.agent.clone()),
        ("LYS_SESSION".to_owned(), session.clone()),
        ("LYS_LAUNCH_RECORD".to_owned(), record.id.clone()),
        ("LYS_HANDLES".to_owned(), record.credential_ids.join(",")),
        ("LYS_LAUNCH_TEMPLATE".to_owned(), rendered.template_sha256),
    ]));
    Ok(Launch {
        session,
        program: native.program,
        arguments: native.arguments,
        directory: record.working_directory.clone(),
        environment: native.environment,
        config: Some(lys_runner::launch_config::Config {
            files: native.files,
            argument_files: native.argument_files,
            environment_paths: native.environment_paths,
            working_directory: false,
            harness: version
                .settings
                .harness
                .as_ref()
                .and_then(crate::launch_template::runner_harness),
        }),
        columns: crate::runner_sessions::COLUMNS,
        rows: crate::runner_sessions::ROWS,
        rotation: version
            .settings
            .session
            .as_ref()
            .and_then(|settings| settings.accounts.clone()),
        policy,
    })
}
