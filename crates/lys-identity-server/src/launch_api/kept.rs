//! Runner inputs reconstructed from the exact kept provisioning version.
use super::{HandleName, ServerError, Value, reviewed};

/// Build the signed launch using only the kept template and its exact version.
///
/// # Errors
/// Refuses unreadable starts, missing kept versions and invalid template bytes.
pub fn kept_launch(
    store: &crate::provisioning_store::ProvisioningStore,
    view: &Value,
) -> Result<lys_runner::Launch, ServerError> {
    let text = |member: &str| {
        view.get(member)
            .and_then(Value::as_str)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {member}"),
            })
    };
    let number = view
        .get("provisioning_version")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the kept start names no profile version".to_owned(),
        })?;
    let version = store
        .profile(text("agent")?)
        .and_then(|profile| {
            profile
                .versions
                .iter()
                .find(|version| version.number == number)
        })
        .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
    reviewed(version)?;
    let mut native = crate::launch_template::from_template(
        version,
        text("template")?,
        text("template_sha256")?,
    )?;
    if let Some(window) = version
        .settings
        .seat_fields()
        .map_err(|reason| ServerError::LaunchUnrenderable { reason })?
        .context_window
    {
        native
            .environment
            .insert("LYS_CONTEXT_WINDOW".to_owned(), window.to_string());
    }
    let handles: Vec<HandleName> =
        serde_json::from_value(view.get("handles").cloned().ok_or_else(|| {
            ServerError::LaunchUnrenderable {
                reason: "the kept start names no handles".to_owned(),
            }
        })?)
        .map_err(|error| ServerError::LaunchUnrenderable {
            reason: format!("the kept handles do not read: {error}"),
        })?;
    native.environment.insert(
        "LYS_LAUNCH_TEMPLATE".to_owned(),
        text("template_sha256")?.to_owned(),
    );
    native.environment.insert(
        "LYS_HANDLES".to_owned(),
        handles
            .iter()
            .map(|handle| handle.id.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );
    let files = native
        .files
        .into_iter()
        .map(|file| lys_runner::launch_config::File {
            path: file.path,
            text: file.text,
            sha256: file.sha256,
        })
        .collect();
    Ok(lys_runner::Launch {
        session: text("session")?.to_owned(),
        program: native.program,
        arguments: native.arguments,
        directory: text("directory")?.to_owned(),
        environment: native.environment,
        config: Some(lys_runner::launch_config::Config {
            requires_controls: crate::launch_template::requires_controls(version),
            files,
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
            .and_then(|session| session.accounts.clone()),
        policy: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::launch_template::{Start, render};
    use crate::provisioning_store::{ProvisioningStore, Version};
    use serde_json::json;

    #[test]
    fn a_direct_kept_launch_uses_the_exact_profiles_declared_window()
    -> Result<(), Box<dyn std::error::Error>> {
        for session_form in [false, true] {
            let dir = tempfile::tempdir()?;
            let mut value = json!({
                "number":1, "operation":"profile-original", "settings":{
                    "model_access":["fixture-model"], "tools":[], "skills":[], "mcp_servers":[],
                    "instructions":"", "note":"", "harness":crate::harness_description::declared(),
                    "permissions":{"default_mode":"plan"}
                }, "set_by":"person", "set_at":1,
                "reviewed":{"operation":"review-original", "by":"person", "at":1}
            });
            if session_form {
                value["settings"]["session"] = json!({"context_window":128_000});
            } else {
                value["settings"]["context_window"] = json!(128_000);
            }
            let version: Version = serde_json::from_value(value)?;
            let mut store = ProvisioningStore::open(&dir.path().join("profiles.json"))?;
            store.set("agent", 0, version.clone())?;
            let rendered = render(
                &Start {
                    agent: "agent",
                    session: "session",
                    machine: "machine",
                    runtime: "sh",
                    version: &version,
                    skills: &[],
                    policy: None,
                    model_proxy: None,
                },
                &[],
            )?;
            let view = json!({
                "agent":"agent", "session":"session", "directory":"/",
                "provisioning_version":1, "handles":[],
                "template":rendered.template, "template_sha256":rendered.template_sha256,
            });
            let launch = super::kept_launch(&store, &view)?;
            assert_eq!(
                launch
                    .environment
                    .get("LYS_CONTEXT_WINDOW")
                    .map(String::as_str),
                Some("128000")
            );
            assert_eq!(super::kept_launch(&store, &view)?, launch);
        }
        Ok(())
    }
}
